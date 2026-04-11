use std::collections::VecDeque;
use std::sync::Arc;
use crate::ui::{GAP_XS, GAP_SM, GAP_MD, KEY_CAP_SIDE};

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use crate::engine::sim::{Command, SimHandle, Stats};

use super::espresso_walk;

const DISPLAY_H: u32 = 600; // logical pixel height of the window ("zoom level")

/// Speed-based duration for various UI interactions.

fn speed_duration(speed: u8) -> std::time::Duration {
    match speed {
        0 => std::time::Duration::from_millis(250),
        1 => std::time::Duration::from_millis(66),
        2 => std::time::Duration::from_millis(16),
        3 => std::time::Duration::from_millis(4),
        _ => std::time::Duration::ZERO,
    }
}

/// Derive the three wave parameters (amp, freq, phase, dir) from a seed string.
/// Three scale tiers — medium / fast / slow — keep entity density rich
/// while producing completely different interference landscapes per seed.
pub fn make_env_data_pub(seed: &str) -> [f32; 24] { make_env_data(seed) }

fn make_env_data(seed: &str) -> [f32; 24] {
    use std::f32::consts::TAU;
    let mut s = crate::hash_seed(seed) | 1;
    let mut next = || -> f32 {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f32 / (1u64 << 53) as f32
    };
    // freq anchors: medium [0.75,1.35], fast [1.9,3.2], slow [0.07,0.17]
    let freq_anchor = [0.75f32, 1.90f32, 0.07f32];
    let freq_range  = [0.60f32, 1.30f32, 0.10f32];
    let mut data = [0.0f32; 24];
    for i in 0..3 {
        let amp        = next() * 2.5 + 1.0;                     // [1.0, 3.5]
        let freq       = freq_anchor[i] + next() * freq_range[i];
        let phase      = next() * TAU;
        let angle      = next() * TAU;
        // drift_freq: how fast this wave's parameters evolve over T.
        // Each wave gets its own incommensurate drift rate so they never
        // all mutate together — same trick as the wave freqs themselves.
        let drift_freq  = next() * 0.04 + 0.005;                 // [0.005, 0.045]
        let drift_phase = next() * TAU;
        let base  = i * 8;
        data[base]   = amp;
        data[base+1] = freq;
        data[base+2] = phase;
        data[base+3] = angle.cos();
        data[base+4] = angle.sin();
        data[base+5] = drift_freq;
        data[base+6] = drift_phase;
        // data[base+7] reserved for non-wave-0 blocks
    }
    // Universe heat index — 50 % cold (2.0) vs 50 % hot (5.0).
    // Stored in slot 7 of wave-0's block (the reserved slot).
    // Controls how dramatically high-energy waves expand their Gaussian radius:
    //   cold: radius max = 1.5 + 1.5 * 2.0 = 4.5  (intense blobs, no supernova)
    //   hot:  radius max = 1.5 + 1.5 * 5.0 = 9.0  (large white balls; supernova when multiple peaks align)
    data[7] = if next() < 0.5 { 2.0_f32 } else { 5.0_f32 };
    data
}


#[allow(clippy::too_many_arguments)]
pub fn run(
    event_loop: EventLoop<()>,
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    sim_handle: SimHandle,
    seed: String,
    background_noise: f32,
    first_wave_weights: [[f32; 14]; 12],
) {
    let espresso = espresso_walk::generate(12, &seed, espresso_walk::Palette::Wide);
    let espresso_rgb: [[f32; 3]; 12] = std::array::from_fn(|i| {
        let c = espresso[i];
        [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0]
    });
    let color_data = crate::engine::color_math::build(&first_wave_weights, &espresso_rgb);
    let wave_lch = espresso_walk::seed_lch(&seed, 3);
    let wave_colors = espresso_walk::generate(3, &seed, espresso_walk::Palette::Wide);
    let env_data0 = make_env_data(&seed);
    let wave_params0: Vec<[f64; 5]> = (0..3).map(|w| {
        let gn = crate::ui::ascii_render::get_gn_at_time(&env_data0, w, 0.0, background_noise as f64);
        crate::ui::ascii_render::get_params(&env_data0, w, gn)
    }).collect();

    let mut app = App {
        theme: Theme::Dew,
        instance,
        adapter,
        device,
        queue,
        sim_handle,
        branch_colors: espresso,
        wave_colors,
        wave_lch,
        wave_params0,
        color_data,
        old_colors: Vec::new(),
        seed: seed.clone(),
        background_noise,
        state: None,
        last_stats: None,
        history: VecDeque::new(),
        last_frame: std::time::Instant::now(),
        fps: 60.0,
        t_per_sec: 0.0,
        last_tps_t_epoch: 0,
        last_tps_t_residual: 0.0,
        last_tps_update: std::time::Instant::now(),
        pan_x: 0.0,
        pan_y: 0.0,
        show_branch: true,
        show_branch_metrics: true,
        take_screenshot: false,
        show_strategy: true,
        show_metrics: true,
        branch_density_latest: None,
        branch_density_dirty: false,
        last_projection_tick: 0,
        last_bounds_instant: None,
        pending_fullscreen_toggle: false,
        pending_minimize_time: None,
        circle_axes: ([0.0; 14], [0.0; 14], [0.0; 14]),
        last_sent_bounds: [-15.0, 15.0, -15.0, 15.0],
        speed: 1,
        strategy_engine: space_strategy_engine::SpaceStrategyEngine::default(),
        synth_engine: synth_engine::SynthEngine::default(),
        acoustic_volume: 50.0,
        is_muted: true,
        is_paused: false,
        t_epoch: 0,
        t_residual: 0.0,
        wave_speed: 1.0,
        custom_speed: 1.0,
        speed_text: "1.00 T/s".to_string(),
        time_text: "0.0 T".to_string(),
        seed_text: seed.clone(),
        env_data: make_env_data(&seed),
        last_rendered_epoch:    0,
        last_rendered_residual: f64::MAX, // force field render on first frame
        field_force_redraw:     true,
        title: format!("anytimeuniverse {}", env!("CARGO_PKG_VERSION")),
        title_text: format!("anytimeuniverse {}", env!("CARGO_PKG_VERSION")),
    };
    let _ = app
        .sim_handle
        .cmd_tx
        .send(Command::SetSpeed(speed_duration(1)));
    event_loop.run_app(&mut app).expect("event loop failed");
}

include!("render_state.rs");

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Theme { Rect, Dew, Future }

impl Theme {
    pub fn provider(self) -> &'static dyn crate::ui::theme::ThemeProvider {
        match self {
            Theme::Rect   => &crate::ui::rect::Rect,
            Theme::Dew    => &crate::ui::dew::Dew,
            Theme::Future => &crate::ui::future::Future,
        }
    }
}

include!("app.rs");
include!("app_handler.rs");
pub mod space_strategy_engine;
pub mod synth_engine;

/// Format a speed value (T/s) for display, filling the text box.
/// Same magnitude-aware logic as format_time_text. " T/s" is 4 chars
/// so we get ~16 chars for sign+digits, targeting 20 total.
pub fn format_speed_text(v: f64) -> String {
    let av = v.abs();
    let neg = v < 0.0;
    let s = if neg { "-" } else { "" };
    if av < 1e-4 {
        "0.00 T/s".to_string()
    } else if av < 10.0 {
        format!("{}{:.4} T/s", s, av)
    } else if av < 100.0 {
        format!("{}{:.3} T/s", s, av)
    } else if av < 1_000.0 {
        format!("{}{:.2} T/s", s, av)
    } else if av < 10_000.0 {
        format!("{}{:.1} T/s", s, av)
    } else if av < if neg { 1e15 } else { 1e16 } {
        format!("{}{:.0} T/s", s, av)
    } else {
        let e = av.log10().floor() as i32;
        let m = av / 10f64.powi(e);
        if neg {
            format!("-{:.10}e{} T/s", m, e)
        } else {
            format!("{:.11}e{} T/s", m, e)
        }
    }
}
/// Uses full decimal precision scaled to magnitude, switching to scientific
/// notation only when the integer representation would overflow (~18 chars).
pub fn format_time_text(t: f64) -> String {
    let av = t.abs();
    let neg = t < 0.0;
    let s = if neg { "-" } else { "" };
    if av < 1e-4 {
        "0.0 T".to_string()
    } else if av < 10.0 {
        format!("{}{:.4} T", s, av)
    } else if av < 100.0 {
        format!("{}{:.3} T", s, av)
    } else if av < 1_000.0 {
        format!("{}{:.2} T", s, av)
    } else if av < 10_000.0 {
        format!("{}{:.1} T", s, av)
    } else if av < if neg { 1e17 } else { 1e18 } {
        format!("{}{:.0} T", s, av)
    } else {
        let e = av.log10().floor() as i32;
        let m = av / 10f64.powi(e);
        if neg {
            format!("-{:.12}e{} T", m, e)
        } else {
            format!("{:.13}e{} T", m, e)
        }
    }
}
