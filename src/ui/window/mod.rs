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
        theme: Theme::Metallic,
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
        zoom: 1.0,
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
        modifiers: winit::keyboard::ModifiersState::empty(),
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
        moment_hash_text: format_moment_hash(&seed, 0, 0.0, 0.0, 0.0, 1.0),
        pending_hash_jump: None,
        pending_paste: None,
        sys_components: sysinfo::Components::new_with_refreshed_list(),
        last_temp_refresh: std::time::Instant::now(),
        sys_temps: Vec::new(),
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
pub enum Theme { Rect, Metallic, Dew, Future }

impl Theme {
    pub fn provider(self) -> &'static dyn crate::ui::theme::ThemeProvider {
        match self {
            Theme::Rect     => &crate::ui::rect::Rect,
            Theme::Dew      => &crate::ui::dew::Dew,
            Theme::Future   => &crate::ui::future::Future,
            Theme::Metallic => &crate::ui::metallic::Metallic,
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
pub fn format_moment_hash(seed: &str, t_epoch: i64, t_residual: f64, pan_x: f32, pan_y: f32, zoom: f32) -> String {
    let fmt_f = |f: f64, prec: usize| {
        if f.abs() < 1e-8 { return "".to_string(); }
        let mut s = format!("{:.*}", prec, f);
        if s.contains('.') {
            s = s.trim_end_matches('0').to_string();
            if s.ends_with('.') { s.pop(); }
        }
        if s == "-0" || s == "0" { 
            return "".to_string(); 
        }
        if s.starts_with("0.") {
            s = s[1..].to_string();
        } else if s.starts_with("-0.") {
            s = format!("-{}", &s[2..]);
        }
        s
    };

    let f_ep = if t_epoch == 0 { "".to_string() } else { t_epoch.to_string() };
    let f_res = fmt_f(t_residual, 8);
    let f_px = fmt_f(pan_x as f64, 4);
    let f_py = fmt_f(pan_y as f64, 4);
    let f_z = if (zoom - 1.0).abs() < 1e-5 { "".to_string() } else { fmt_f(zoom as f64, 4) };

    let mut parts = vec![f_ep, f_res, f_px, f_py, f_z];
    while parts.last().is_some_and(|p| p.is_empty()) {
        parts.pop();
    }

    if parts.is_empty() {
        seed.to_string()
    } else {
        format!("{}-{}", seed, parts.join("-"))
    }
}

pub fn parse_moment_hash(s: &str) -> Option<(String, i64, f64, f32, f32, f32)> {
    let (seed, rest) = if let Some((se, r)) = s.split_once('-') {
        (se, Some(r))
    } else if let Some((se, r)) = s.split_once('@') {
        (se, Some(r))
    } else {
        (s, None)
    };
    if seed.is_empty() { return None; }

    let mut epoch = 0;
    let mut residual = 0.0;
    let mut pan_x = 0.0;
    let mut pan_y = 0.0;
    let mut zoom = 1.0;

    if let Some(rest) = rest {
        let rest_normalized = rest.replace([':', '_', ','], "-"); // backward compatibility with old hashes
        
        let mut string_parts = Vec::new();
        let chars = rest_normalized.chars().collect::<Vec<char>>();
        let mut current_part = String::new();
        let mut i = 0;
        
        while i < chars.len() {
            if chars[i] == '-' {
                if i + 1 < chars.len() && chars[i + 1] == '-' {
                    // It's a double dash: the first is the delimiter, the second is a negative sign for the next number
                    string_parts.push(current_part);
                    current_part = "-".to_string();
                    i += 2;
                } else if i == 0 {
                    // starts with a dash -> the first number is negative
                    current_part = "-".to_string();
                    i += 1;
                } else {
                    // standard delimiter
                    string_parts.push(current_part);
                    current_part = String::new();
                    i += 1;
                }
            } else {
                current_part.push(chars[i]);
                i += 1;
            }
        }
        string_parts.push(current_part);

        let mut parts = string_parts.into_iter();
        
        if let Some(p) = parts.next()
            && !p.is_empty() { epoch = p.parse::<i64>().unwrap_or(0); }
        if let Some(p) = parts.next()
            && !p.is_empty() { residual = p.parse::<f64>().unwrap_or(0.0); }
        if let Some(p) = parts.next()
            && !p.is_empty() { pan_x = p.parse::<f32>().unwrap_or(0.0); }
        if let Some(p) = parts.next()
            && !p.is_empty() { pan_y = p.parse::<f32>().unwrap_or(0.0); }
        if let Some(p) = parts.next()
            && !p.is_empty()
                && let Ok(z) = p.parse::<f32>() {
                    zoom = z;
                }
    }
    
    Some((seed.to_string(), epoch, residual, pan_x, pan_y, zoom))
}

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
