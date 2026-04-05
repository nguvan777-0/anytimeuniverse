use std::collections::VecDeque;
use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use crate::engine::sim::{Command, PcaAxesData, SimHandle, Stats};

use super::espresso_walk;

const DISPLAY_H: u32 = 600; // logical pixel height of the window ("zoom level")

/// Compress current history into old_history (20-entry ghost) and clear history for new run.
fn archive_history(history: &mut VecDeque<Vec<u32>>, old_history: &mut VecDeque<Vec<u32>>) {
    if !history.is_empty() {
        let step = (history.len() / 20).max(1);
        *old_history = history.iter().step_by(step).take(20).cloned().collect();
    }
    history.clear();
}

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
/// Three scale tiers — medium / fast / slow — keep organism density rich
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
        // data[base+7] reserved
    }
    data
}

fn speed_label(speed: u8) -> &'static str {
    match speed {
        0 => "¼ T/s",
        1 => "1 T/s",
        2 => "10 T/s",
        3 => "100 T/s",
        4 => "1K T/s",
        _ => "∞",
    }
}

pub fn run(
    event_loop: EventLoop<()>,
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    sim_handle: SimHandle,
    seed: String,
    substrate_noise: f32,
    first_wave_weights: [[f32; 14]; 12],
) {
    let espresso = espresso_walk::generate(12, &seed, espresso_walk::Palette::Bright);
    let espresso_rgb: [[f32; 3]; 12] = std::array::from_fn(|i| {
        let c = espresso[i];
        [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0]
    });
    let color_data = crate::engine::color_math::build(&first_wave_weights, &espresso_rgb);
    let wave_colors = espresso_walk::generate(3, &seed, espresso_walk::Palette::Bright);

    let mut app = App {
        theme: Theme::Dew,
        instance,
        adapter,
        device,
        queue,
        sim_handle,
        branch_colors: espresso,
        wave_colors,
        color_data,
        old_colors: Vec::new(),
        seed: seed.clone(),
        substrate_noise,
        state: None,
        last_stats: None,
        history: VecDeque::new(),
        old_history: VecDeque::new(),
        archive_time: None,
        last_frame: std::time::Instant::now(),
        fps: 60.0,
        t_per_sec: 0.0,
        last_tps_t_epoch: 0,
        last_tps_t_residual: 0.0,
        last_tps_update: std::time::Instant::now(),
        show_branch: true,
        show_pop_metrics: true,
        show_strategy: true,
        show_metrics: true,
        pca_density_latest: None,
        pca_density_dirty: false,
        last_pca_axes_tick: 0,
        last_bounds_instant: None,
        circle_pc: ([0.0; 14], [0.0; 14], [0.0; 14]),
        last_sent_bounds: [-15.0, 15.0, -15.0, 15.0],
        speed: 1,
        is_paused: false,
        t_epoch: 0,
        t_residual: 0.0,
        wave_speed: 1.0,
        custom_speed: 1.0,
        speed_text: "1.00 T/s".to_string(),
        time_text: "0.0 T".to_string(),
        seed_text: seed.clone(),
        env_data: make_env_data(&seed),
        title: "anytimeuniverse".to_string(),
        title_text: "anytimeuniverse".to_string(),
    };
    let _ = app
        .sim_handle
        .cmd_tx
        .send(Command::SetSpeed(speed_duration(1)));
    event_loop.run_app(&mut app).expect("event loop failed");
}

struct RenderState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    bg: wgpu::BindGroup,
    tick_buf: wgpu::Buffer,
    env_buf: wgpu::Buffer,
    color_buf: wgpu::Buffer,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
}

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

struct App {
    theme: Theme,
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    sim_handle: SimHandle,
    seed: String,
    substrate_noise: f32,
    state: Option<RenderState>,
    last_stats: Option<Stats>,
    history: VecDeque<Vec<u32>>,
    old_history: VecDeque<Vec<u32>>,
    branch_colors: Vec<egui::Color32>,
    wave_colors: Vec<egui::Color32>,
    color_data: crate::engine::color_math::ColorData,
    old_colors: Vec<egui::Color32>,
    archive_time: Option<std::time::Instant>,
    last_frame: std::time::Instant,
    fps: f32,
    t_per_sec: f64,        // T units advanced per real second
    last_tps_t_epoch: i64,
    last_tps_t_residual: f64,
    last_tps_update: std::time::Instant,
    show_branch: bool,
    show_pop_metrics: bool,
    show_strategy: bool,
    show_metrics: bool,
    pca_density_latest: Option<Vec<u32>>,
    pca_density_dirty: bool,
    last_pca_axes_tick: u32,
    last_bounds_instant: Option<std::time::Instant>,
    /// PCA axes and mean, computed on first tick and reused for bounds updates.
    circle_pc: ([f32; 14], [f32; 14], [f32; 14]),
    /// Bounds [min_x, max_x, min_y, max_y] last sent to the GPU density shader.
    last_sent_bounds: [f32; 4],
    speed: u8,
    is_paused: bool,
    /// Integer number of full universe periods elapsed (P = 2π/freq_min ≈ 62.83).
    /// Combined with `residual`, this gives lossless T at any scale.
    t_epoch: i64,
    /// Fractional phase within the current period: always in [0, P).
    /// This is what gets sent to the GPU as f32.
    t_residual: f64,
    wave_speed: f32,  // T units per second
    custom_speed: f64, // user-typed T/s (active when speed == 6)
    speed_text: String,
    time_text: String,
    seed_text: String,
    title: String,
    title_text: String,
    /// CPU copy of the wave parameters so we can evaluate dominance analytically.
    env_data: [f32; 24],
}

/// Mirror of the shader's generation system — same hash, same accumulator.
/// Returns fractional amplitude dominance [0,1] × 3 for the COLOR RIVER.
fn wave_dominance_at(env_data: &[f32; 24], t: f64, noise: f32) -> [f32; 3] {
    const PHI: f64 = 1.6180339887;
    const EUL: f64 = 2.7182818284;

    // Hash: same constants as shader — (wave_i * 97 + gn * 1031 + ch * 7)
    let uhash = |x: u32| -> u32 {
        let mut v = x ^ (x >> 17);
        v = v.wrapping_mul(0xbf324c81);
        v ^= v >> 13;
        v = v.wrapping_mul(0x9a813f77);
        v ^= v >> 16;
        v
    };
    let gen_param = |wave_i: u32, gn: u32, ch: u32| -> f64 {
        uhash(wave_i.wrapping_mul(97).wrapping_add(gn.wrapping_mul(1031)).wrapping_add(ch.wrapping_mul(7))) as f64
            / u32::MAX as f64
    };

    let mut amps = [0.0f32; 3];
    for i in 0..3usize {
        let base        = i * 8;
        let amp         = env_data[base]     as f64;
        let drift_freq  = env_data[base + 5] as f64;
        let drift_phase = env_data[base + 6] as f64;

        // Accumulator — matches gen_acc in shader
        let base_acc = t * drift_freq * 2.0;
        let wiggle   = noise as f64 * 0.5 * (
            (drift_freq * PHI * t + drift_phase).sin() +
            (drift_freq * std::f64::consts::PI * t + drift_phase * EUL).sin()
        );
        let acc  = (base_acc + wiggle).max(0.0);
        let gn   = acc.floor() as u32;
        let frac = acc.fract();

        let wi = i as u32;
        // mirror of memory() in shader — power-based carry (copying)
        let copy = |g: u32, ch: u32| -> f64 {
            let own    = gen_param(wi, g, ch);
            let prev  = gen_param(wi, g.wrapping_sub(1), ch);
            let amp_n  = 0.3 + gen_param(wi, g, 0) * 2.4;
            let freq_n = 0.4 + gen_param(wi, g, 1) * 1.2;
            let power = (amp_n * freq_n * 0.4).clamp(0.0, 1.0);
            let carry = gen_param(wi, g, ch + 10) * 0.5 + power * 0.5;
            prev + (own - prev) * carry
        };
        let amp_a = amp * (0.3 + copy(gn,                  0) * 2.4);
        let amp_b = amp * (0.3 + copy(gn.wrapping_add(1), 0) * 2.4);

        // smoothstep(0.9, 1.0, frac)
        let blend = if frac < 0.9 { 0.0 } else {
            let x = (frac - 0.9) / 0.1;
            x * x * (3.0 - 2.0 * x)
        };
        amps[i] = (amp_a + (amp_b - amp_a) * blend).max(0.01) as f32;
    }
    let total = amps[0] + amps[1] + amps[2];
    [amps[0] / total, amps[1] / total, amps[2] / total]
}

/// Compute a branch's display color by applying the 14×3 color projection matrix.
fn blend_branch_color(weights: &[f32; 14], color_data: &crate::engine::color_math::ColorData, first_wave_id: usize) -> egui::Color32 {
    let rgb = crate::engine::color_math::apply(color_data, weights, first_wave_id);
    let tone = |v: f32| (v * 255.0).clamp(0.0, 255.0) as u8;
    egui::Color32::from_rgb(tone(rgb[0]), tone(rgb[1]), tone(rgb[2]))
}

/// Returns `(projected_points, pc1, pc2, mean)`.
fn compute_pca_2d(points: &[[f32; 14]; 12], valid: &[bool; 12])
    -> ([(f32, f32); 12], [f32; 14], [f32; 14], [f32; 14])
{
    let mut valid_count = 0;
    let mut mean = [0.0; 14];
    for i in 0..12 {
        if valid[i] {
            valid_count += 1;
            for j in 0..14 { mean[j] += points[i][j]; }
        }
    }
    if valid_count < 2 { return ([(0.0, 0.0); 12], [0.0; 14], [0.0; 14], mean); }
    for j in 0..14 { mean[j] /= valid_count as f32; }

    let mut centered = [[0.0; 14]; 12];
    for i in 0..12 {
        if valid[i] {
            for j in 0..14 { centered[i][j] = points[i][j] - mean[j]; }
        }
    }

    let get_pc = |data: &mut [[f32; 14]; 12]| -> [f32; 14] {
        let mut t = [1.0 / 14f32.sqrt(); 14];
        for _ in 0..10 { // Power iteration
            let mut new_t = [0.0; 14];
            for i in 0..12 {
                if !valid[i] { continue; }
                let mut dot = 0.0;
                for j in 0..14 { dot += data[i][j] * t[j]; }
                for j in 0..14 { new_t[j] += dot * data[i][j]; }
            }
            let norm = new_t.iter().map(|v| v * v).sum::<f32>().sqrt();
            if norm > 0.0 { for j in 0..14 { t[j] = new_t[j] / norm; } }
        }
        t
    };

    let p1 = get_pc(&mut centered.clone());

    // Remove p1 projection
    let mut data_p2 = centered.clone();
    for i in 0..12 {
        if !valid[i] { continue; }
        let mut dot = 0.0;
        for j in 0..14 { dot += data_p2[i][j] * p1[j]; }
        for j in 0..14 { data_p2[i][j] -= dot * p1[j]; }
    }
    let p2 = get_pc(&mut data_p2);

    let mut out = [(0.0, 0.0); 12];
    for i in 0..12 {
        if valid[i] {
            let mut d1 = 0.0;
            let mut d2 = 0.0;
            for j in 0..14 {
                d1 += centered[i][j] * p1[j];
                d2 += centered[i][j] * p2[j];
            }
            out[i] = (d1, d2);
        }
    }
    (out, p1, p2, mean)
}

impl App {
    fn update_colors(_queue: &wgpu::Queue, _color_buf: Option<()>, _color_data: &crate::engine::color_math::ColorData) {
        // color_data is no longer bound to the shader — PCA/branch colors live CPU-side only.
    }

    fn reset_simulation(&mut self, change_seed: bool) {
        if change_seed {
            let num = crate::hash_seed(&self.seed);
            self.seed = crate::generate_seed(num);
        }

        let (w, noise, fw) = crate::init_world(&self.seed);
        self.substrate_noise = noise;
        
        if change_seed {
            println!("[ world ] change channel to new seed: {} (noise: {:.3})", self.seed, noise);
        } else {
            println!("[ world ] rewind current seed: {} (noise: {:.3})", self.seed, noise);
        }

        let _ = self.sim_handle.cmd_tx.send(Command::Reset(w.data, noise));
        self.old_colors = self.wave_colors.clone();
        self.branch_colors = super::espresso_walk::generate(12, &self.seed, super::espresso_walk::Palette::Bright);
        self.wave_colors = super::espresso_walk::generate(3, &self.seed, super::espresso_walk::Palette::Bright);
        
        let wc_data: [f32; 12] = {
            let wc = &self.wave_colors;
            [
                wc[0].r() as f32 / 255.0, wc[0].g() as f32 / 255.0, wc[0].b() as f32 / 255.0, 0.0,
                wc[1].r() as f32 / 255.0, wc[1].g() as f32 / 255.0, wc[1].b() as f32 / 255.0, 0.0,
                wc[2].r() as f32 / 255.0, wc[2].g() as f32 / 255.0, wc[2].b() as f32 / 255.0, 0.0,
            ]
        };
        let env_data = make_env_data(&self.seed);
        self.env_data = env_data;
        if let Some(state) = &self.state {
            self.queue.write_buffer(&state.env_buf, 0, bytemuck::cast_slice(&env_data));
            self.queue.write_buffer(&state.color_buf, 0, bytemuck::cast_slice(&wc_data));
            state.window.request_redraw();
        }
        
        let espresso_rgb: [[f32; 3]; 12] = std::array::from_fn(|i| {
            let c = self.branch_colors[i];
            [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0]
        });
        self.color_data = crate::engine::color_math::build(&fw, &espresso_rgb);
        
        archive_history(&mut self.history, &mut self.old_history);
        self.archive_time = None;
        
        self.pca_density_latest = None;
        self.pca_density_dirty = false;
        self.last_pca_axes_tick = 0;
        self.last_bounds_instant = None;
        self.circle_pc = ([0.0; 14], [0.0; 14], [0.0; 14]);
        self.last_sent_bounds = [-15.0, 15.0, -15.0, 15.0];
        self.last_stats = None; // Reset stale stats
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

        // Use LogicalSize so the 260px panel is always 260 logical px regardless of DPI,
        // and DISPLAY_H controls the zoom level.
        let window = Arc::new(
            event_loop
                .create_window(
                    winit::window::WindowAttributes::default()
                        .with_title("anytimeuniverse")
                        .with_inner_size(winit::dpi::LogicalSize::new(DISPLAY_H + 260 + 260, DISPLAY_H))
                        .with_min_inner_size(winit::dpi::LogicalSize::new(320u32, 240u32)),
                )
                .expect("failed to create window"),
        );

        let surface: wgpu::Surface<'static> = self
            .instance
            .create_surface(Arc::clone(&window))
            .expect("failed to create surface");

        let caps = surface.get_capabilities(&self.adapter);
        let format = caps.formats[0];

        let inner = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: inner.width.max(1),
            height: inner.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&self.device, &config);

        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("render"),
                source: wgpu::ShaderSource::Wgsl(include_str!("render.wgsl").into()),
            });

        // Tick uniform: [tick_f32, pad, pad, pad] = 16 bytes (std140)
        let tick_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tick-uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // EnvUniform: 3 waves × 8 f32 = 96 bytes
        // Wave layout: amp, freq, phase, dir_x, dir_y, _p0, _p1, _p2
        let env_data = make_env_data(&self.seed);
        let env_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("env-uniform"),
            size: (env_data.len() * 4) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&env_buf, 0, bytemuck::cast_slice(&env_data));

        // WaveColors uniform: 3 × vec4<f32> = 48 bytes
        let wc_data: [f32; 12] = {
            let wc = &self.wave_colors;
            [
                wc[0].r() as f32 / 255.0, wc[0].g() as f32 / 255.0, wc[0].b() as f32 / 255.0, 0.0,
                wc[1].r() as f32 / 255.0, wc[1].g() as f32 / 255.0, wc[1].b() as f32 / 255.0, 0.0,
                wc[2].r() as f32 / 255.0, wc[2].g() as f32 / 255.0, wc[2].b() as f32 / 255.0, 0.0,
            ]
        };
        let color_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wave-colors-uniform"),
            size: (wc_data.len() * 4) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&color_buf, 0, bytemuck::cast_slice(&wc_data));

        let bgl = self
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    // binding 0 — sim uniform (wave time T)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // binding 1 — env uniform (3 waves)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // binding 2 — wave colors (3 seed-derived colors)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render-bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: tick_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: env_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: color_buf.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(&bgl)],
                immediate_size: 0,
            });

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("render"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        let egui_ctx = egui::Context::default();
        Theme::Dew.provider().apply_theme(&egui_ctx);

        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(&self.device, format, egui_wgpu::RendererOptions {
            msaa_samples: 1,
            depth_stencil_format: None,
            dithering: false,
            ..Default::default()
        });

        self.state = Some(RenderState {
            window,
            surface,
            config,
            pipeline,
            bg,
            tick_buf,
            env_buf,
            color_buf,
            egui_ctx,
            egui_state,
            egui_renderer,
        });
        self.state.as_ref().unwrap().window.focus_window();
        self.state.as_ref().unwrap().window.request_redraw();
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {
        let mut got_stats = false;
        
        while self.sim_handle.stats_buffer.update() {
            let stats = self.sim_handle.stats_buffer.read().clone();
            if let Some(prev) = &self.last_stats {
                if stats.tick < prev.tick {
                    self.pca_density_latest = None;
                    self.pca_density_dirty = false;
                    self.last_pca_axes_tick = 0;
                    self.last_bounds_instant = None;
                    self.last_sent_bounds = [-15.0, 15.0, -15.0, 15.0];
                    self.circle_pc = ([0.0; 14], [0.0; 14], [0.0; 14]);
                    self.history.clear();
                }
            }
            if !self.old_history.is_empty() && self.archive_time.is_none() {
                self.archive_time = Some(std::time::Instant::now());
            }
            self.history.push_back(stats.color_counts.clone());
            if self.history.len() > 240 {
                self.history.pop_front();
            }
            if let Some(density) = stats.pca_density.clone() {
                self.pca_density_latest = Some(density);
                self.pca_density_dirty = true;
            }
            self.last_stats = Some(stats);
            got_stats = true;
        }

        if got_stats {
            // Stats updated — mark dirty but don't request redraw.
            // The vsync render loop runs continuously when unpaused and will
            // pick up the latest stats on the next frame automatically.
            let _ = got_stats; // suppress unused warning
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let Some(state) = &mut self.state {
            let res = state.egui_state.on_window_event(&state.window, &event);
            if res.repaint {
                state.window.request_redraw();
            }
            if res.consumed {
                return;
            }
        }

        match event {
            WindowEvent::Resized(physical_size) => {
                if let Some(state) = &mut self.state {
                    if physical_size.width > 0 && physical_size.height > 0 {
                        state.config.width = physical_size.width;
                        state.config.height = physical_size.height;
                        state.surface.configure(&self.device, &state.config);
                        state.window.request_redraw();
                    }
                }
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == winit::event::ElementState::Pressed {
                    if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
                        match code {
                            winit::keyboard::KeyCode::KeyF => {
                                if let Some(state) = &self.state {
                                    let is_fullscreen = state.window.fullscreen().is_some();
                                    state.window.set_fullscreen(if is_fullscreen {
                                        None
                                    } else {
                                        Some(winit::window::Fullscreen::Borderless(None))
                                    });
                                }
                            }
                            winit::keyboard::KeyCode::KeyR => {
                                // Rewind: zero T, speed=1, resume, same seed
                                self.t_epoch = 0;
                                self.t_residual = 0.0;
                                self.wave_speed = 1.0;
                                self.custom_speed = 1.0;
                                self.is_paused = false;
                                let _ = self.sim_handle.cmd_tx.send(Command::Resume);
                                self.reset_simulation(false);
                            }
                            winit::keyboard::KeyCode::KeyC => {
                                // New seed: zero T, speed=1, resume, new seed
                                self.t_epoch = 0;
                                self.t_residual = 0.0;
                                self.wave_speed = 1.0;
                                self.custom_speed = 1.0;
                                self.is_paused = false;
                                let _ = self.sim_handle.cmd_tx.send(Command::Resume);
                                self.reset_simulation(true);
                            }
                            winit::keyboard::KeyCode::Space => {
                                self.is_paused = !self.is_paused;
                                if self.is_paused {
                                    let _ = self.sim_handle.cmd_tx.send(Command::Pause);
                                    println!("[ world ] pause");
                                } else {
                                    let _ = self.sim_handle.cmd_tx.send(Command::Resume);
                                    println!("[ world ] resume");
                                }
                                if let Some(state) = &self.state {
                                    state.window.request_redraw();
                                }
                            }
                            winit::keyboard::KeyCode::Digit0
                            | winit::keyboard::KeyCode::Digit1
                            | winit::keyboard::KeyCode::Digit2
                            | winit::keyboard::KeyCode::Digit3
                            | winit::keyboard::KeyCode::Digit4
                            | winit::keyboard::KeyCode::Digit5
                            | winit::keyboard::KeyCode::Numpad0
                            | winit::keyboard::KeyCode::Numpad1
                            | winit::keyboard::KeyCode::Numpad2
                            | winit::keyboard::KeyCode::Numpad3
                            | winit::keyboard::KeyCode::Numpad4
                            | winit::keyboard::KeyCode::Numpad5 => {
                                let s = match code {
                                    winit::keyboard::KeyCode::Digit0
                                    | winit::keyboard::KeyCode::Numpad0 => 0u8,
                                    winit::keyboard::KeyCode::Digit1
                                    | winit::keyboard::KeyCode::Numpad1 => 1,
                                    winit::keyboard::KeyCode::Digit2
                                    | winit::keyboard::KeyCode::Numpad2 => 2,
                                    winit::keyboard::KeyCode::Digit3
                                    | winit::keyboard::KeyCode::Numpad3 => 3,
                                    winit::keyboard::KeyCode::Digit4
                                    | winit::keyboard::KeyCode::Numpad4 => 4,
                                    _ => 5,
                                };
                                self.speed = s;
                                self.wave_speed = match s {
                                    0 => 0.25,
                                    1 => 1.0,
                                    2 => 10.0,
                                    3 => 100.0,
                                    4 => 1_000.0,
                                    _ => 1_000_000.0,
                                };
                                self.custom_speed = self.wave_speed as f64; // keep slider in sync
                                let _ = self
                                    .sim_handle
                                    .cmd_tx
                                    .send(Command::SetSpeed(speed_duration(s)));
                            }
                            winit::keyboard::KeyCode::ArrowLeft
                            | winit::keyboard::KeyCode::ArrowRight => {
                                const FREQ_MIN: f64 = 0.1;
                                const PERIOD: f64 = std::f64::consts::TAU / FREQ_MIN;
                                let current_t = self.t_epoch as f64 * PERIOD + self.t_residual;
                                let jump = if current_t.abs() < 1.0 {
                                    PERIOD
                                } else {
                                    let magnitude = 10f64.powi(current_t.abs().log10().floor() as i32);
                                    (current_t.abs() / magnitude).floor() * magnitude
                                };
                                if code == winit::keyboard::KeyCode::ArrowLeft {
                                    self.t_residual -= jump;
                                } else {
                                    self.t_residual += jump;
                                }
                                // Normalise residual into [0, PERIOD)
                                if self.t_residual >= PERIOD {
                                    let extra = (self.t_residual / PERIOD).floor() as i64;
                                    self.t_epoch += extra;
                                    self.t_residual -= extra as f64 * PERIOD;
                                } else if self.t_residual < 0.0 {
                                    let borrow = (-self.t_residual / PERIOD).ceil() as i64;
                                    self.t_epoch -= borrow;
                                    self.t_residual += borrow as f64 * PERIOD;
                                }
                                self.is_paused = true;
                                let _ = self.sim_handle.cmd_tx.send(Command::Pause);
                                if let Some(state) = &self.state {
                                    state.window.request_redraw();
                                }
                            }
                            winit::keyboard::KeyCode::ArrowUp
                            | winit::keyboard::KeyCode::ArrowDown => {
                                // Step through symmetric speed ladder crossing zero (pause)
                                const LADDER: &[f64] = &[
                                    -1e12, -1e11, -1e10, -1e9, -1e8, -1e7,
                                    -1_000_000.0, -1_000.0, -100.0, -10.0, -1.0, -0.25,
                                    0.0, // pause
                                    0.25, 1.0, 10.0, 100.0, 1_000.0, 1_000_000.0,
                                    1e7, 1e8, 1e9, 1e10, 1e11, 1e12,
                                ];
                                let cur = self.wave_speed as f64;
                                // Find closest ladder index
                                let idx = LADDER
                                    .iter()
                                    .enumerate()
                                    .min_by(|(_, a), (_, b)| {
                                        ((**a - cur).abs()).partial_cmp(&((**b - cur).abs())).unwrap()
                                    })
                                    .map(|(i, _)| i)
                                    .unwrap_or(7); // default to 1.0
                                let new_idx = if code == winit::keyboard::KeyCode::ArrowUp {
                                    (idx + 1).min(LADDER.len() - 1)
                                } else {
                                    idx.saturating_sub(1)
                                };
                                let new_speed = LADDER[new_idx];
                                if new_speed == 0.0 {
                                    self.is_paused = true;
                                    let _ = self.sim_handle.cmd_tx.send(Command::Pause);
                                } else {
                                    self.is_paused = false;
                                    let _ = self.sim_handle.cmd_tx.send(Command::Resume);
                                }
                                self.wave_speed = new_speed as f32;
                                self.custom_speed = new_speed;
                                if let Some(state) = &self.state {
                                    state.window.request_redraw();
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let mut pending_reset = None;
                let mut pending_title = None;
                let state = self.state.as_mut().unwrap();

                let dt = self.last_frame.elapsed().as_secs_f32();
                let _dt = dt; // clear warning
                self.last_frame = std::time::Instant::now();

                if let Some(stats) = &self.last_stats {
                    let elapsed = self.last_tps_update.elapsed().as_secs_f64();
                    if elapsed >= 0.5 {
                        const PERIOD: f64 = std::f64::consts::TAU / 0.1;
                        let cur = self.t_epoch as f64 * PERIOD + self.t_residual;
                        let prev = self.last_tps_t_epoch as f64 * PERIOD + self.last_tps_t_residual;
                        self.t_per_sec = (cur - prev) / elapsed;
                        self.last_tps_t_epoch = self.t_epoch;
                        self.last_tps_t_residual = self.t_residual;
                        self.last_tps_update = std::time::Instant::now();
                    }
                }

                let monitor_hz = state
                    .window
                    .current_monitor()
                    .and_then(|m| m.refresh_rate_millihertz())
                    .map(|mhz| (mhz as f32 / 1000.0).round() as f32)
                    .unwrap_or(60.0);

                let raw_input = state.egui_state.take_egui_input(&state.window);
                let mut rewind_req = false;
                let mut reroll_req = false;
                let mut pause_req = false;
                let mut speed_req: Option<u8> = None;
                let mut arrow_up_req = false;
                let mut arrow_down_req = false;
                let mut arrow_left_req = false;
                let mut arrow_right_req = false;
                let mut exit_req = false;
                let mut minimize_req = false;
                let mut fullscreen_req = false;


                let full_output = state.egui_ctx.run(raw_input, |ctx| {
                    let navy_blue = egui::Color32::from_rgb(0, 0, 128);
                    if matches!(self.theme, Theme::Rect) {
                        let term_bg   = egui::Color32::BLACK;
                        let term_green = egui::Color32::from_rgb(0, 230, 65);
                        let term_dim  = term_green;
                        let mut visuals = egui::Visuals::dark();
                        visuals.panel_fill = term_bg;
                        visuals.window_fill = term_bg;
                        visuals.selection.bg_fill = term_dim;
                        visuals.selection.stroke = egui::Stroke::new(1.0, term_bg);
                        visuals.widgets.noninteractive.bg_fill = term_bg;
                        visuals.widgets.noninteractive.weak_bg_fill = term_bg;
                        visuals.widgets.inactive.bg_fill = term_bg;
                        visuals.widgets.inactive.weak_bg_fill = term_bg;
                        visuals.widgets.hovered.bg_fill = term_dim;
                        visuals.widgets.hovered.weak_bg_fill = term_dim;
                        visuals.widgets.active.bg_fill = term_dim;
                        visuals.widgets.active.weak_bg_fill = term_dim;
                        visuals.override_text_color = Some(term_green);
                        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, term_dim);
                        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, term_green);
                        visuals.widgets.active.bg_stroke  = egui::Stroke::new(1.0, term_green);
                        visuals.widgets.noninteractive.corner_radius = egui::Rounding::ZERO;
                        visuals.widgets.inactive.corner_radius = egui::Rounding::ZERO;
                        visuals.widgets.hovered.corner_radius  = egui::Rounding::ZERO;
                        visuals.widgets.active.corner_radius   = egui::Rounding::ZERO;
                        visuals.window_corner_radius = egui::Rounding::ZERO;
                        visuals.menu_corner_radius   = egui::Rounding::ZERO;
                        ctx.set_visuals(visuals);
                        let mut fonts = egui::FontDefinitions::default();
                        if let Some(mono) = fonts.families.get(&egui::FontFamily::Monospace).cloned() {
                            fonts.families.insert(egui::FontFamily::Proportional, mono);
                        }
                        ctx.set_fonts(fonts);
                    }

                    let mut term_title_bar = |ui: &mut egui::Ui, title_text: &mut String, real_title: &mut String, pending_title: &mut Option<String>, subtitle: Option<&str>, sub_suffix: Option<&str>| -> (bool, bool, bool) {
                        let height = 18.0;
                        let term_bar_bg    = egui::Color32::BLACK;
                        let term_bar_green = egui::Color32::from_rgb(0, 230, 65);
                        // removed dim
                        let (rect, _resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::hover());
                        ui.painter().rect_filled(rect, egui::Rounding::ZERO, term_bar_bg);
                        ui.painter().rect_stroke(rect, egui::Rounding::ZERO, egui::Stroke::new(1.0, term_bar_green), egui::StrokeKind::Outside);

                        let mut right_edge = rect.max.x - 2.0;

                        let mut draw_term_btn = |ui: &mut egui::Ui, x: f32, id: &str, text: &str, offset_y: f32| -> bool {
                            let btn_rect = egui::Rect::from_min_size(egui::pos2(x, rect.min.y + 2.0), egui::vec2(14.0, 14.0));
                            let resp = ui.interact(btn_rect, egui::Id::new(id), egui::Sense::click());

                            let is_down = resp.is_pointer_button_down_on();
                            let is_hov  = resp.hovered();
                            let bg = term_bar_bg;
                            let fg = term_bar_green;
                            ui.painter().rect_filled(btn_rect, egui::Rounding::ZERO, bg);
                            ui.painter().rect_stroke(btn_rect, egui::Rounding::ZERO, if is_down || is_hov { egui::Stroke::NONE } else { egui::Stroke::new(1.0, fg) }, egui::StrokeKind::Outside);

                            let text_pos = btn_rect.center() + egui::vec2(0.0, offset_y);
                            ui.painter().text(
                                text_pos,
                                egui::Align2::CENTER_CENTER,
                                text,
                                egui::FontId::monospace(11.0),
                                fg,
                            );

                            resp.clicked()
                        };

                        right_edge -= 14.0;
                        let max_clicked = draw_term_btn(ui, right_edge, "c_btn_m", "~", 0.0);

                        right_edge -= 16.0;
                        let min_clicked = draw_term_btn(ui, right_edge, "c_btn_n", ".", 0.0);

                        right_edge -= 16.0;
                        let exit_clicked = draw_term_btn(ui, right_edge, "c_btn_x", "*", 0.0);

                        right_edge -= 6.0;

                        if let Some(sub) = subtitle {
                            if let Some(suffix) = sub_suffix {
                                let suffix_rect = ui.painter().text(
                                    egui::pos2(right_edge, rect.min.y + 6.0),
                                    egui::Align2::RIGHT_TOP,
                                    suffix,
                                    egui::FontId::monospace(11.0),
                                    term_bar_green,
                                );
                                right_edge = suffix_rect.min.x - 1.0;
                            }

                            ui.painter().text(
                                egui::pos2(right_edge, rect.min.y + 4.0),
                                egui::Align2::RIGHT_TOP,
                                sub,
                                egui::FontId::monospace(11.0),
                                term_bar_green,
                            );
                        }
                        
                        let text_rect = egui::Rect::from_min_max(rect.min + egui::vec2(4.0, 2.0), egui::pos2(right_edge, rect.max.y));
                        let mut child_ui = ui.child_ui(text_rect, egui::Layout::left_to_right(egui::Align::TOP), None);
                        child_ui.visuals_mut().extreme_bg_color = egui::Color32::TRANSPARENT;
                        child_ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                        child_ui.visuals_mut().widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                        
                        let edit = egui::TextEdit::singleline(title_text)
                            .frame(egui::Frame::NONE)
                            .font(egui::FontId::monospace(13.0))
                            .text_color(term_bar_green)
                            .margin(egui::vec2(0.0, -1.0));
                        let action = child_ui.add(edit);
                        
                        if action.gained_focus() {
                            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), action.id) {
                                state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                                    egui::text::CCursor::new(0),
                                    egui::text::CCursor::new(title_text.chars().count()),
                                )));
                                egui::TextEdit::store_state(ui.ctx(), action.id, state);
                            }
                        }
                        
                        if (action.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || action.lost_focus() {
                            if *title_text != *real_title {
                                if title_text.trim().is_empty() {
                                    *title_text = real_title.clone();
                                } else {
                                    *real_title = title_text.clone();
                                    *pending_title = Some(real_title.clone());
                                }
                            }
                        } else if !action.has_focus() {
                            *title_text = real_title.clone();
                        }

                        ui.add_space(2.0);

                        (exit_clicked, min_clicked, max_clicked)
                    };

                    let mut frame = egui::Frame::side_top_panel(&ctx.global_style());
                    frame.shadow = egui::epaint::Shadow::NONE;
                    frame.corner_radius = egui::Rounding::ZERO;

                    let mut left_frame = frame;
                    left_frame.inner_margin = egui::Margin { left: 0, right: 8, top: 8, bottom: 8 };

                    let mut right_frame = frame;
                    right_frame.inner_margin = egui::Margin { left: 8, right: 0, top: 8, bottom: 8 };

                    // SidePanel fills the exact height of the window; since LogicalSize ensures
                    // no pillarboxing, this matches the sim grid height perfectly.

                    // --- Left panel: Space Strategy ---
                    let left_panel_res = egui::SidePanel::left("wight_strategy")
                        .resizable(false)
                        .exact_width(260.0)
                        .frame(left_frame)
                        .show(ctx, |ui| {
                            match self.theme {
                                Theme::Dew => crate::ui::dew::draw_stripes(ui.painter(), ui.max_rect()),
                                Theme::Future => crate::ui::future::draw_scan_lines(ui.painter(), ui.max_rect()),
                                _ => {}
                            }
                            ui.add_space(2.0);
                            if self.theme.provider().collapsible_header(ui, "SPACE STRATEGY", self.show_strategy) {
                                self.show_strategy = !self.show_strategy;
                            }
                            ui.add_space(4.0);
                            if self.show_strategy {

                            let pcolors = self.color_data;
                            if let Some(stats) = &self.last_stats {
                                // Send axes once on first tick.
                                if self.last_pca_axes_tick == 0 && stats.tick > 0 {
                                    let mut pc1 = [0.0; 14];
                                    let mut pc2 = [0.0; 14];
                                    pc1[0] = 1.0;
                                    pc2[1] = 1.0;
                                    let mean = [0.0; 14];
                                    self.circle_pc = (pc1, pc2, mean);
                                    self.last_pca_axes_tick = stats.tick;
                                    self.last_sent_bounds = [-1.0, 1.0, -1.0, 1.0];
                                    let b = self.last_sent_bounds;
                                    let _ = self.sim_handle.cmd_tx.send(Command::SetPcaAxes(PcaAxesData {
                                        pc1, pc2, mean,
                                        min_x: b[0], max_x: b[1], min_y: b[2], max_y: b[3],
                                        color_data: pcolors,
                                    }));
                                    self.pca_density_latest = None;
                                    self.pca_density_dirty = false;
                                    self.last_bounds_instant = Some(std::time::Instant::now());
                                }

                                // Data-driven bounds update.
                                let bounds_now = std::time::Instant::now();
                                let bounds_elapsed = self.last_bounds_instant
                                    .map_or(f64::MAX, |t| bounds_now.duration_since(t).as_secs_f64());
                                if self.pca_density_dirty && self.last_pca_axes_tick > 0 && bounds_elapsed >= 1.0 / 60.0 {
                                    if let Some(density) = &self.pca_density_latest {
                                        let mut cx0 = 128i32; let mut cx1 = -1i32;
                                        let mut cy0 = 128i32; let mut cy1 = -1i32;
                                        for (slot, &packed) in density.iter().enumerate() {
                                            if packed > 0 {
                                                cx0 = cx0.min((slot % 128) as i32);
                                                cx1 = cx1.max((slot % 128) as i32);
                                                cy0 = cy0.min((slot / 128) as i32);
                                                cy1 = cy1.max((slot / 128) as i32);
                                            }
                                        }
                                        if cx1 >= cx0 {
                                            let b = self.last_sent_bounds;
                                            let rx = b[1] - b[0]; let ry = b[3] - b[2];
                                            let wx0 = b[0] + cx0 as f32 / 128.0 * rx;
                                            let wx1 = b[0] + (cx1 + 1) as f32 / 128.0 * rx;
                                            let wy0 = b[2] + (1.0 - (cy1 + 1) as f32 / 128.0) * ry;
                                            let wy1 = b[2] + (1.0 - cy0 as f32 / 128.0) * ry;
                                            let new_x0 = if cx0 == 0   { b[0] - rx } else { wx0 - (wx1-wx0)*0.1 };
                                            let new_x1 = if cx1 == 127 { b[1] + rx } else { wx1 + (wx1-wx0)*0.1 };
                                            let new_y0 = if cy1 == 127 { b[2] - ry } else { wy0 - (wy1-wy0)*0.1 };
                                            let new_y1 = if cy0 == 0   { b[3] + ry } else { wy1 + (wy1-wy0)*0.1 };
                                            self.last_sent_bounds = [new_x0, new_x1, new_y0, new_y1];
                                            self.pca_density_dirty = false;
                                            self.last_bounds_instant = Some(bounds_now);
                                            let b = self.last_sent_bounds;
                                            let (pc1, pc2, mean) = self.circle_pc;
                                            let _ = self.sim_handle.cmd_tx.send(Command::SetPcaAxes(PcaAxesData {
                                                pc1, pc2, mean,
                                                min_x: b[0], max_x: b[1], min_y: b[2], max_y: b[3],
                                                color_data: pcolors,
                                            }));
                                        }
                                    }
                                }
                            }

                            // Same size as it was in the right panel: full width × 200px tall.
                            let plot_w = ui.available_width();
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(plot_w, 200.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, egui::Rounding::ZERO,
                                egui::Color32::from_rgb(8, 8, 12));
                            self.theme.provider().draw_sunken(ui.painter(), rect);

                            if let Some(density) = &self.pca_density_latest {
                                let cell_w = rect.width()  / 128.0;
                                let cell_h = rect.height() / 128.0;
                                let dot_w = cell_w.max(4.0);
                                let dot_h = cell_h.max(4.0);
                                for (slot, &packed) in density.iter().enumerate() {
                                    if packed > 0 {
                                        let px = (slot % 128) as f32;
                                        let py = (slot / 128) as f32;
                                        let sx = rect.min.x + px * cell_w;
                                        let sy = rect.min.y + py * cell_h;
                                        let e = (packed >> 24) as f32 / 255.0;
                                        let total_bits = packed & 0xFFFFFF;
                                        let r = (total_bits >> 16) as u8;
                                        let g = (total_bits >> 8) as u8;
                                        let b = total_bits as u8;
                                        let color = egui::Color32::from_rgba_unmultiplied(
                                            r, g, b, (e * 200.0 + 55.0) as u8);
                                        ui.painter().rect_filled(
                                            egui::Rect::from_min_size(
                                                egui::pos2(sx - dot_w * 0.5, sy - dot_h * 0.5),
                                                egui::vec2(dot_w, dot_h),
                                            ),
                                            egui::Rounding::ZERO,
                                            color,
                                        );
                                    }
                                }
                            }
                            }

                            // --- Left panel: System Metrics ---
                            ui.add_space(8.0);
                            ui.add_space(4.0);
                            if self.theme.provider().collapsible_header(ui, "SYSTEM METRICS", self.show_metrics) {
                                self.show_metrics = !self.show_metrics;
                            }
                            ui.add_space(4.0);

                            if self.show_metrics {
                                let info = self.adapter.get_info();
                                let dim = if matches!(self.theme, Theme::Rect) {
                                    egui::Color32::from_rgb(0, 230, 65) /* TERM_GREEN */
                                } else if matches!(self.theme, Theme::Future) {
                                    egui::Color32::from_rgb(192, 202, 222) /* FUTURE TEXT */
                                } else {
                                    egui::Color32::from_rgb(100, 100, 105)
                                };

                                let wrap_w = ui.available_width() - 40.0;
                                let gpu_galley = ui.painter().layout(info.name.clone(), egui::FontId::monospace(11.0), dim, wrap_w);
                                // The horizontal layouts enforce `interact_size.y` due to alignment!
                                let base_h = ui.spacing().interact_size.y.max(
                                    ui.painter().layout("A".to_string(), egui::FontId::monospace(11.0), dim, wrap_w).size().y
                                );
                                let row_h = gpu_galley.size().y.max(base_h);
                                let total_h = row_h + (base_h * 3.0) + (3.0 * 3.0) + 8.0; // 8.0 padding total

                                let (metrics_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), total_h),
                                    egui::Sense::hover()
                                );
                                self.theme.provider().draw_sunken(ui.painter(), metrics_rect);

                                let mut child_ui = ui.child_ui(metrics_rect.shrink(4.0), *ui.layout(), None);
                                child_ui.style_mut().spacing.item_spacing = egui::vec2(4.0, 3.0);

                                let rows: &[(&str, egui::RichText)] = &[
                                    ("GPU",   egui::RichText::new(info.name.clone()).monospace().size(11.0)),
                                    ("API",   egui::RichText::new(format!("{:?}", info.backend)).monospace().size(11.0)),
                                    ("CPU",   egui::RichText::new(std::env::consts::ARCH).monospace().size(11.0)),
                                    ("Frame", egui::RichText::new(format!("{:.1} ms", if self.fps > 0.0 { 1000.0 / self.fps } else { 0.0 })).monospace().size(11.0)),
                                ];
                                for (label, val) in rows {
                                    child_ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(*label).monospace().size(11.0).color(dim));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(val.clone());
                                        });
                                    });
                                }
                            }

                            // Pin theme selector to the very bottom of the left panel
                            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                                ui.add_space(4.0);
                                let mut ds = self.theme;
                                let btn_text = match ds {
                                    Theme::Rect   => "Rect   ^",
                                    Theme::Dew    => "Dew    ^",
                                    Theme::Future => "Future ^",
                                };
                                let resp = crate::ui::widgets::button_w(self.theme.provider(), ui, btn_text, 0.0);
                                let popup_id = ui.make_persistent_id("theme_popup");
                                if resp.clicked() {
                                    ui.ctx().memory_mut(|mem| mem.toggle_popup(popup_id));
                                }
                                if matches!(self.theme, Theme::Rect) {
                                    let is_open = egui::Popup::is_id_open(ui.ctx(), popup_id);
                                    if is_open {
                                        let term_bg    = egui::Color32::BLACK;
                                        let term_green = egui::Color32::from_rgb(0, 230, 65);
                                        let z = egui::Rounding::ZERO;
                                        let area_resp = egui::Area::new(popup_id)
                                            .order(egui::Order::Foreground)
                                            .kind(egui::UiKind::Popup)
                                            .fixed_pos(resp.rect.left_top())
                                            .pivot(egui::Align2::LEFT_BOTTOM)
                                            .constrain_to(ui.ctx().content_rect())
                                            .show(ui.ctx(), |ui| {
                                                egui::Frame::NONE
                                                    .fill(term_bg)
                                                    .stroke(egui::Stroke::new(1.0, term_green))
                                                    .inner_margin(egui::Margin::same(6))
                                                    .show(ui, |ui| {
                                                        ui.visuals_mut().override_text_color = Some(term_green);
                                                        // Keep bg_stroke=NONE on all states so the frame budget
                                                        // (inner_margin = button_padding + expansion - stroke_width)
                                                        // is never disturbed and items never shift position.
                                                        // The green border is painted separately via the Painter.
                                                        ui.visuals_mut().widgets.inactive.bg_fill      = egui::Color32::TRANSPARENT;
                                                        ui.visuals_mut().widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                                                        ui.visuals_mut().widgets.inactive.bg_stroke    = egui::Stroke::NONE;
                                                        ui.visuals_mut().widgets.inactive.corner_radius = z;
                                                        ui.visuals_mut().widgets.hovered.bg_fill       = term_bg;
                                                        ui.visuals_mut().widgets.hovered.weak_bg_fill  = term_bg;
                                                        ui.visuals_mut().widgets.hovered.bg_stroke     = egui::Stroke::NONE;
                                                        ui.visuals_mut().widgets.hovered.corner_radius  = z;
                                                        ui.visuals_mut().widgets.active.bg_fill        = term_bg;
                                                        ui.visuals_mut().widgets.active.weak_bg_fill   = term_bg;
                                                        ui.visuals_mut().widgets.active.bg_stroke      = egui::Stroke::NONE;
                                                        ui.visuals_mut().widgets.active.corner_radius   = z;
                                                        ui.visuals_mut().selection.bg_fill = term_bg;
                                                        ui.visuals_mut().selection.stroke  = egui::Stroke::new(1.0, term_green);
                                                        ui.set_min_width(resp.rect.width());
                                                        let border = egui::Stroke::new(1.0, term_green);
                                                        let mut add_btn = |ui: &mut egui::Ui, t: Theme, label: &str| {
                                                            let text = egui::RichText::new(label).monospace().size(13.0);
                                                            let is_selected = ds == t;
                                                            let btn_resp = ui.selectable_value(&mut ds, t, text);
                                                            if btn_resp.hovered() != is_selected {
                                                                ui.painter().rect_stroke(btn_resp.rect, z, border, egui::StrokeKind::Outside);
                                                            }
                                                            if btn_resp.clicked() {
                                                                ui.ctx().memory_mut(|mem| mem.close_popup(popup_id));
                                                            }
                                                        };
                                                        add_btn(ui, Theme::Rect, "Rect    ");
                                                        add_btn(ui, Theme::Dew, "Dew     ");
                                                        add_btn(ui, Theme::Future, "Future  ");
                                                    });
                                            });
                                        let close = ui.input(|i| i.pointer.any_pressed())
                                            && ui.input(|i| i.pointer.interact_pos())
                                                .map_or(false, |p| !area_resp.response.rect.contains(p));
                                        if close {
                                            ui.ctx().memory_mut(|mem| mem.close_popup(popup_id));
                                        } else {
                                            ui.ctx().memory_mut(|mem| mem.keep_popup_open(popup_id));
                                        }
                                    }
                                } else {
                                    // Make the entire popup use smaller margins/padding
                                    let prev_style = (*ui.ctx().style()).clone();
                                    let mut popup_style = prev_style.clone();
                                    popup_style.spacing.window_margin = egui::Margin::same(4);
                                    popup_style.spacing.button_padding = egui::vec2(6.0, 4.0);
                                    popup_style.spacing.item_spacing = egui::vec2(4.0, 2.0);
                                    ui.ctx().set_style(popup_style);

                                    egui::containers::popup_above_or_below_widget(ui, popup_id, &resp, egui::AboveOrBelow::Above, egui::PopupCloseBehavior::CloseOnClickOutside, |ui: &mut egui::Ui| {
                                        ui.visuals_mut().widgets.hovered.expansion = 0.0;
                                        ui.visuals_mut().widgets.active.expansion  = 0.0;
                                        ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::NONE;
                                        ui.visuals_mut().widgets.hovered.bg_stroke = egui::Stroke::NONE;
                                        ui.visuals_mut().widgets.active.bg_stroke  = egui::Stroke::NONE;
                                        
                                        // Sync hover highlights with text selection colors across themes
                                        if matches!(self.theme, Theme::Dew) {
                                            ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::from_rgb(180, 210, 255);
                                            ui.visuals_mut().widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(180, 210, 255);
                                        } else if matches!(self.theme, Theme::Future) {
                                            ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::from_rgb(130, 148, 192); // FUTURE_GLOW matches the button reflection
                                            ui.visuals_mut().widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(130, 148, 192);
                                            // Make text dark when hovering over the bright accent
                                            ui.visuals_mut().widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::BLACK);
                                        }

                                        ui.set_min_width(resp.rect.width());
                                        let mut add_btn = |ui: &mut egui::Ui, t: Theme, label: &str| {
                                            let text = egui::RichText::new(label).monospace().size(13.0);
                                            if ui.selectable_value(&mut ds, t, text).clicked() {
                                                ui.ctx().memory_mut(|mem| mem.close_popup(popup_id));
                                            }
                                        };
                                        add_btn(ui, Theme::Rect, "Rect    ");
                                        add_btn(ui, Theme::Dew, "Dew     ");
                                        add_btn(ui, Theme::Future, "Future  ");
                                    });
                                    
                                    ui.ctx().set_style(prev_style);
                                }
                                if ds != self.theme {
                                    self.theme = ds;
                                    self.theme.provider().apply_theme(ctx);
                                }
                            });

                        });
                        
                    if matches!(self.theme, Theme::Rect) {
                        let rect = left_panel_res.response.rect;
                        ctx.layer_painter(egui::LayerId::background()).vline(
                            rect.right(),
                            rect.y_range(),
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 230, 65))
                        );
                    }

                    let right_panel_res = egui::SidePanel::right("wight_control")
                        .resizable(false)
                        .exact_width(260.0)
                        .frame(right_frame)
                        .show(ctx, |ui| {
                            match self.theme {
                                Theme::Dew => crate::ui::dew::draw_stripes(ui.painter(), ui.max_rect()),
                                Theme::Future => crate::ui::future::draw_scan_lines(ui.painter(), ui.max_rect()),
                                _ => {}
                            }

                            match self.theme {
                                Theme::Rect => {
                                    let (c_exit, c_min, c_max) = term_title_bar(ui, &mut self.title_text, &mut self.title, &mut pending_title, None, None);
                                    if c_exit { exit_req = true; }
                                    if c_min { minimize_req = true; }
                                    if c_max { fullscreen_req = true; }
                                }
                                Theme::Dew | Theme::Future => {
                                    let height = 26.0;
                                    let (rect, _resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::hover());
                                    let left = rect.min.x;
                                    let right = rect.max.x;

                                    if matches!(self.theme, Theme::Dew) {
                                        // 7 lines for Dew stripe titlebar layout
                                        for i in 0..7 {
                                            let y = rect.min.y + 2.0 + i as f32 * 3.5;
                                            ui.painter().line_segment([egui::pos2(left, y), egui::pos2(right, y)], egui::Stroke::new(1.0, egui::Color32::from_rgb(205, 210, 218)));
                                            ui.painter().line_segment([egui::pos2(left, y+1.0), egui::pos2(right, y+1.0)], egui::Stroke::new(1.0, egui::Color32::from_rgb(250, 255, 255)));
                                        }
                                    }
                                    
                                    let r = 6.0;
                                    let cy = rect.center().y;
                                    
                                    // Dew behaviour: hover ANY of the 3 buttons, and ALL 3 show their icons.
                                    let group_rect = egui::Rect::from_min_max(
                                        egui::pos2(right - 50.0 - r - 2.0, cy - r - 2.0),
                                        egui::pos2(right - 14.0 + r + 2.0, cy + r + 2.0),
                                    );
                                    let group_hovered = ui.rect_contains_pointer(group_rect);
                                    let hover_t = ui.ctx().animate_value_with_time(
                                        egui::Id::new("tb_group_hover"),
                                        if group_hovered { 1.0 } else { 0.0 },
                                        0.1,
                                    );
                                    
                                    let btn_color = if matches!(self.theme, Theme::Future) {
                                        egui::Color32::from_rgb(88, 94, 112) // FUTURE_BODY — match future big buttons
                                    } else {
                                        egui::Color32::from_rgb(50, 130, 240) // DEW_BODY — match dew big buttons
                                    };

                                    let mut draw_anim_gumdrop = |ui: &mut egui::Ui, id: &str, cx: f32, base_color: egui::Color32, symbol: &str| -> bool {
                                        let center = egui::pos2(right - cx, cy);
                                        let btn_size = egui::vec2(r * 2.0 + 2.0, r * 2.0 + 2.0);
                                        let resp = ui.interact(egui::Rect::from_center_size(center, btn_size), egui::Id::new(id), egui::Sense::click());
                                        if matches!(self.theme, Theme::Future) {
                                            crate::ui::future::draw_orb_btn(ui, &resp, r, base_color, symbol, Some(hover_t));
                                        } else {
                                            crate::ui::dew::draw_dot_btn(ui, &resp, r, base_color, symbol, Some(hover_t));
                                        }
                                        resp.clicked()
                                    };
                                    
                                    if draw_anim_gumdrop(ui, "tb_red", 50.0, btn_color, "*") { exit_req = true; }
                                    if draw_anim_gumdrop(ui, "tb_yellow", 32.0, btn_color, ".") { minimize_req = true; }
                                    if draw_anim_gumdrop(ui, "tb_green", 14.0, btn_color, "~") { fullscreen_req = true; }
                                    
                                    let title_color = if matches!(self.theme, Theme::Future) {
                                        egui::Color32::WHITE
                                    } else {
                                        egui::Color32::from_rgb(30, 30, 30)
                                    };
                                    
                                    let font_id = egui::FontId::proportional(14.0);
                                    let galley = ui.painter().layout_no_wrap(self.title_text.clone(), font_id.clone(), title_color);
                                    let text_pos = egui::pos2(left + 8.0, cy - galley.size().y / 2.0);
                                    
                                    let text_rect = egui::Rect::from_min_size(text_pos, galley.size());
                                    
                                    if matches!(self.theme, Theme::Future) {
                                        ui.painter().rect_filled(
                                            text_rect.expand2(egui::vec2(4.0, -1.0)),
                                            2.0,
                                            egui::Color32::BLACK,
                                        );
                                    }
                                    
                                    let mut child_ui = ui.child_ui(text_rect, egui::Layout::left_to_right(egui::Align::TOP), None);
                                    child_ui.visuals_mut().extreme_bg_color = egui::Color32::TRANSPARENT;
                                    child_ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                                    child_ui.visuals_mut().widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                                    
                                    let edit = egui::TextEdit::singleline(&mut self.title_text)
                                        .frame(egui::Frame::NONE)
                                        .font(font_id)
                                        .text_color(title_color)
                                        .margin(egui::vec2(0.0, 0.0))
                                        .desired_width(150.0);
                                        
                                    let action = child_ui.add(edit);
                                    
                                    if action.gained_focus() {
                                        if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), action.id) {
                                            state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                                                egui::text::CCursor::new(0),
                                                egui::text::CCursor::new(self.title_text.chars().count()),
                                            )));
                                            egui::TextEdit::store_state(ui.ctx(), action.id, state);
                                        }
                                    }
                                    
                                    if (action.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || action.lost_focus() {
                                        if self.title_text != self.title {
                                            if self.title_text.trim().is_empty() {
                                                self.title_text = self.title.clone();
                                            } else {
                                                self.title = self.title_text.clone();
                                                pending_title = Some(self.title.clone());
                                            }
                                        }
                                    } else if !action.has_focus() {
                                        self.title_text = self.title.clone();
                                    }
                                    
                                }
                            }

                            egui::ScrollArea::vertical().show(ui, |ui| {
                                // Performance stats row — Dew inset pill / Editable seed
                                {
                                    let noise_str = format!("noise:{:.2}", self.substrate_noise);
                                    let full_text = format!("{}  ·  {}  ·  {:.0}fps", self.seed_text, noise_str, self.fps);
                                    let stat_color = if matches!(self.theme, Theme::Rect) {
                                        egui::Color32::from_rgb(0, 210, 60)
                                    } else if matches!(self.theme, Theme::Future) {
                                        egui::Color32::WHITE
                                    } else {
                                        egui::Color32::from_rgb(120, 120, 130)
                                    };

                                    let galley = ui.painter().layout_no_wrap(
                                        full_text,
                                        egui::FontId::monospace(11.0),
                                        stat_color,
                                    );
                                    let padding = egui::vec2(8.0, 4.0);
                                    let size = egui::vec2(ui.available_width(), galley.size().y + padding.y * 2.0);
                                    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                                    
                                    if ui.is_rect_visible(rect) {
                                        let p = ui.painter();
                                        if matches!(self.theme, Theme::Rect) {
                                            p.rect_filled(rect, egui::Rounding::ZERO, egui::Color32::BLACK);
                                            p.rect_stroke(rect, egui::Rounding::ZERO, egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 230, 65)), egui::StrokeKind::Outside);
                                        } else {
                                            let bg = if matches!(self.theme, Theme::Future) { egui::Color32::BLACK } else { egui::Color32::from_rgba_premultiplied(0, 0, 0, 18) };
                                            p.rect_filled(rect, rect.height() / 2.0, bg);
                                            crate::ui::dew::draw_inset(p, rect);
                                        }
                                        
                                        let inner_rect = egui::Rect::from_center_size(rect.center(), galley.size());
                                        let mut child_ui = ui.child_ui(inner_rect, *ui.layout(), None);
                                        child_ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |child_ui| {
                                            child_ui.spacing_mut().item_spacing.x = 0.0;
                                            
                                            let font_id = egui::FontId::monospace(11.0);
                                            let seed_w = child_ui.painter().layout_no_wrap(self.seed_text.clone(), font_id.clone(), stat_color).size().x;
                                            
                                            // Invisible editable text
                                            child_ui.visuals_mut().extreme_bg_color = egui::Color32::TRANSPARENT;
                                            child_ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                                            child_ui.visuals_mut().widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                                            
                                            let edit = egui::TextEdit::singleline(&mut self.seed_text)
                                                .frame(egui::Frame::NONE)
                                                .font(font_id.clone())
                                                .text_color(stat_color)
                                                .desired_width(seed_w.max(5.0)); // Prevent total collapse of width
                                                
                                            let seed_action = child_ui.add(edit);
                                            
                                            child_ui.label(egui::RichText::new(format!("  ·  {}  ·  {:.0}fps", noise_str, self.fps))
                                                .font(font_id)
                                                .color(stat_color));
                                                
                                            if seed_action.gained_focus() {
                                                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), seed_action.id) {
                                                    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                                                        egui::text::CCursor::new(0),
                                                        egui::text::CCursor::new(self.seed_text.chars().count()),
                                                    )));
                                                    egui::TextEdit::store_state(ui.ctx(), seed_action.id, state);
                                                }
                                            }
                                            
                                            if (seed_action.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || seed_action.lost_focus() {
                                                if self.seed_text != self.seed {
                                                    if self.seed_text.trim().is_empty() {
                                                        self.seed_text = self.seed.clone(); // Revert if blanked
                                                    } else {
                                                        self.seed = self.seed_text.clone();
                                                        pending_reset = Some(false);
                                                    }
                                                }
                                            } else if !seed_action.has_focus() {
                                                self.seed_text = self.seed.clone();
                                            }
                                        });
                                    }
                                }
                                ui.add_space(4.0);

                                // Transport row: 3 equal-width buttons filling the panel
                                {
                                    let avail = ui.available_width();
                                    let spacing = ui.spacing().item_spacing.x;
                                    let btn_w = ((avail - 2.0 * spacing) / 3.0).floor();
                                    ui.horizontal(|ui| {
                                        if crate::ui::widgets::button_w(self.theme.provider(), ui, "<< R", btn_w).on_hover_text("Rewind").clicked() { rewind_req = true; }
                                        if crate::ui::widgets::button_w(self.theme.provider(), ui, "⟳ C", btn_w).on_hover_text("Reroll").clicked() { reroll_req = true; }
                                        let label = if self.is_paused { "▶ Space" } else { "⏸ Space" };
                                        if crate::ui::widgets::button_w(self.theme.provider(), ui, label, btn_w).on_hover_text(if self.is_paused { "Play" } else { "Pause" }).clicked() { pause_req = true; }
                                    });
                                }

                                // Speed slider: left = reverse, center = 0, right = forward
                                let speed_resp = ui.vertical(|ui| {
                                    ui.style_mut().spacing.item_spacing.y = 2.0;
                                    let lbl_galley = ui.painter().layout_no_wrap("TIME TRAVEL".to_string(), egui::FontId::monospace(8.0), egui::Color32::BLACK);
                                    let label_w = lbl_galley.size().x;
                                    let label_h = lbl_galley.size().y;
                                    let btn_side = ((label_w - 2.0) / 2.0).ceil();
                                    
                                    let field_h = btn_side + 2.0 + label_h;
                                    ui.horizontal(|ui| {
                                        {
                                            ui.vertical(|ui| {
                                                ui.style_mut().spacing.item_spacing.y = 2.0;
                                                let label_top_y = ui.cursor().min.y;
                                                ui.add_space(label_h);
                                                let btn_row = ui.horizontal(|ui| {
                                                    ui.style_mut().spacing.item_spacing.x = 2.0;
                                                    let r = self.theme.provider().key_cap_small(ui, "↓", btn_side);
                                                    if r.clicked() { arrow_down_req = true; }
                                                    let r = self.theme.provider().key_cap_small(ui, "↑", btn_side);
                                                    if r.clicked() { arrow_up_req = true; }
                                                });
                                                let center_x = btn_row.response.rect.center().x;
                                                let color = if matches!(self.theme, Theme::Rect) { egui::Color32::from_rgb(0, 230, 65) } else if matches!(self.theme, Theme::Future) { egui::Color32::WHITE } else { egui::Color32::from_rgb(100, 100, 110) };
                                                let font = egui::FontId::monospace(8.0);
                                                let text = "SLOW  FAST";
                                                let galley = ui.painter().layout_no_wrap(text.to_string(), font, color);
                                                let text_rect = egui::Align2::CENTER_TOP.anchor_size(egui::pos2(center_x, label_top_y), galley.size());
                                                
                                                if matches!(self.theme, Theme::Future) {
                                                    ui.painter().rect_filled(
                                                        text_rect.expand2(egui::vec2(2.0, -1.0)),
                                                        2.0,
                                                        egui::Color32::BLACK,
                                                    );
                                                }
                                                ui.painter().galley(text_rect.min, galley, color);
                                            });
                                        }
                                        let speed_action = self.theme.provider().text_field_edit(ui, &mut self.speed_text, 16.0, field_h);
                                        if speed_action.gained_focus() {
                                            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), speed_action.id) {
                                                state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                                                    egui::text::CCursor::new(0),
                                                    egui::text::CCursor::new(self.speed_text.chars().count()),
                                                )));
                                                egui::TextEdit::store_state(ui.ctx(), speed_action.id, state);
                                            }
                                        }
                                        if speed_action.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) || speed_action.lost_focus() {
                                            if let Ok(v) = self.speed_text.trim().trim_end_matches("T/s").trim().parse::<f64>() {
                                                self.custom_speed = v.clamp(-1e12, 1e12);
                                                self.wave_speed = self.custom_speed as f32;
                                            }
                                            let av2 = self.custom_speed.abs();
                                            let sign2 = if self.custom_speed < 0.0 { "-" } else { "" };
                                            self.speed_text = if av2 < 0.01 { "0.00 T/s".to_string() }
                                                else if av2 < 10.0 { format!("{}{:.2} T/s", sign2, av2) }
                                                else { format!("{}{:.0} T/s", sign2, av2) };
                                        } else if !speed_action.has_focus() {
                                            let av = self.custom_speed.abs();
                                            let sign = if self.custom_speed < 0.0 { "-" } else { "" };
                                            self.speed_text = if av < 0.01 { "0.00 T/s".to_string() }
                                                else if av < 10.0 { format!("{}{:.2} T/s", sign, av) }
                                                else { format!("{}{:.0} T/s", sign, av) };
                                        }
                                    });
                                    ui.add_space(2.0);
                                    let r = crate::ui::widgets::slider_symlog_f64(self.theme.provider(), 
                                        ui,
                                        &mut self.custom_speed,
                                        1e12f64,
                                        "",
                                        |v| {
                                            let av = v.abs();
                                            let sign = if v < 0.0 { "-" } else { "" };
                                            if av < 0.01 { "0.00 T/s".to_string() }
                                            else if av < 10.0 { format!("{}{:.2} T/s", sign, av) }
                                            else { format!("{}{:.0} T/s", sign, av) }
                                        }
                                    );
                                    let av = self.custom_speed.abs();
                                    let sign = if self.custom_speed < 0.0 { "-" } else { "" };
                                    let val_str = if av < 0.01 { "0.00 T/s".to_string() }
                                        else if av < 10.0 { format!("{}{:.2} T/s", sign, av) }
                                        else { format!("{}{:.0} T/s", sign, av) };
                                    if r.changed() { self.speed_text = val_str; }
                                    r
                                }).inner;
                                if speed_resp.changed() {
                                    self.wave_speed = self.custom_speed as f32;
                                }

                                // Time slider: center = T=0, left = past, right = future
                                // Display value is full T = epoch*P + residual, max ±1e15
                                const PERIOD_SL: f64 = std::f64::consts::TAU / 0.1;
                                let t_display_max = 1e15f64;
                                let mut t_display = (self.t_epoch as f64 * PERIOD_SL + self.t_residual).clamp(-t_display_max, t_display_max);
                                let time_resp = ui.vertical(|ui| {
                                    ui.style_mut().spacing.item_spacing.y = 2.0;
                                    let lbl_galley = ui.painter().layout_no_wrap("TIME TRAVEL".to_string(), egui::FontId::monospace(8.0), egui::Color32::BLACK);
                                    let label_w = lbl_galley.size().x;
                                    let label_h = lbl_galley.size().y;
                                    let btn_side = ((label_w - 2.0) / 2.0).ceil();
                                    
                                    let field_h = btn_side + 2.0 + label_h;
                                    let av = t_display.abs();
                                    let sign = if t_display < 0.0 { "-" } else { "" };
                                    ui.horizontal(|ui| {
                                        {
                                            ui.vertical(|ui| {
                                                ui.style_mut().spacing.item_spacing.y = 2.0;
                                                let label_top_y = ui.cursor().min.y;
                                                ui.add_space(label_h);
                                                let btn_row = ui.horizontal(|ui| {
                                                    ui.style_mut().spacing.item_spacing.x = 2.0;
                                                    let r = self.theme.provider().key_cap_small_rotated(ui, "↑", -std::f32::consts::FRAC_PI_2, btn_side);
                                                    if r.clicked() { arrow_left_req = true; }
                                                    let r = self.theme.provider().key_cap_small_rotated(ui, "↑", std::f32::consts::FRAC_PI_2, btn_side);
                                                    if r.clicked() { arrow_right_req = true; }
                                                });
                                                let center_x = btn_row.response.rect.center().x;
                                                let color = if matches!(self.theme, Theme::Rect) { egui::Color32::from_rgb(0, 230, 65) } else if matches!(self.theme, Theme::Future) { egui::Color32::WHITE } else { egui::Color32::from_rgb(100, 100, 110) };
                                                let font = egui::FontId::monospace(8.0);
                                                let text = "TIME TRAVEL";
                                                let galley = ui.painter().layout_no_wrap(text.to_string(), font, color);
                                                let text_rect = egui::Align2::CENTER_TOP.anchor_size(egui::pos2(center_x, label_top_y), galley.size());
                                                
                                                if matches!(self.theme, Theme::Future) {
                                                    ui.painter().rect_filled(
                                                        text_rect.expand2(egui::vec2(2.0, -1.0)),
                                                        2.0,
                                                        egui::Color32::BLACK,
                                                    );
                                                }
                                                ui.painter().galley(text_rect.min, galley, color);
                                            });
                                        }
                                        let time_action = self.theme.provider().text_field_edit(ui, &mut self.time_text, 16.0, field_h);
                                        if time_action.gained_focus() {
                                            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), time_action.id) {
                                                state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                                                    egui::text::CCursor::new(0),
                                                    egui::text::CCursor::new(self.time_text.chars().count()),
                                                )));
                                                egui::TextEdit::store_state(ui.ctx(), time_action.id, state);
                                            }
                                        }
                                        if time_action.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) || time_action.lost_focus() {
                                            if let Ok(v) = self.time_text.trim().trim_end_matches('T').trim().parse::<f64>() {
                                                let new_t = v.clamp(-t_display_max, t_display_max);
                                                let new_residual = new_t.rem_euclid(PERIOD_SL);
                                                let new_epoch = ((new_t - new_residual) / PERIOD_SL).round() as i64;
                                                self.t_epoch = new_epoch;
                                                self.t_residual = new_residual;
                                                self.is_paused = true;
                                                let _ = self.sim_handle.cmd_tx.send(Command::Pause);
                                                let av2 = new_t.abs();
                                                let sign2 = if new_t < 0.0 { "-" } else { "" };
                                                self.time_text = if av2 < 0.01 { "0.0 T".to_string() }
                                                    else if av2 < 1e5 { format!("{}{:.1} T", sign2, av2) }
                                                    else { let e = av2.log10().floor() as i32; format!("{}{:.2}e{} T", sign2, av2 / 10f64.powi(e), e) };
                                            } else {
                                                self.time_text = if av < 0.01 { "0.0 T".to_string() }
                                                    else if av < 1e5 { format!("{}{:.1} T", sign, av) }
                                                    else { let e = av.log10().floor() as i32; format!("{}{:.2}e{} T", sign, av / 10f64.powi(e), e) };
                                            }
                                        } else if !time_action.has_focus() {
                                            self.time_text = if av < 0.01 { "0.0 T".to_string() }
                                                else if av < 1e5 { format!("{}{:.1} T", sign, av) }
                                                else { let e = av.log10().floor() as i32; format!("{}{:.2}e{} T", sign, av / 10f64.powi(e), e) };
                                        }
                                    });
                                    ui.add_space(2.0);
                                    let r = crate::ui::widgets::slider_symlog_f64(self.theme.provider(), 
                                        ui,
                                        &mut t_display,
                                        t_display_max,
                                        "",
                                        |v| {
                                            let av = v.abs();
                                            let sign = if v < 0.0 { "-" } else { "" };
                                            if av < 0.01 { "0.0 T".to_string() }
                                            else if av < 1e5 { format!("{}{:.1} T", sign, av) }
                                            else {
                                                let e = av.log10().floor() as i32;
                                                format!("{}{:.2}e{} T", sign, av / 10f64.powi(e), e)
                                            }
                                        }
                                    );
                                    let val_str = if av < 0.01 { "0.0 T".to_string() }
                                        else if av < 1e5 { format!("{}{:.1} T", sign, av) }
                                        else { let e = av.log10().floor() as i32; format!("{}{:.2}e{} T", sign, av / 10f64.powi(e), e) };
                                    if r.changed() { self.time_text = val_str; }
                                    r
                                }).inner;
                                
                                if time_resp.drag_started() {
                                    let was_playing = !self.is_paused;
                                    ui.memory_mut(|mem| mem.data.insert_temp(time_resp.id.with("was_playing"), was_playing));
                                }

                                if time_resp.changed() {
                                    // decompose rewind position back into epoch + residual
                                    let new_residual = t_display.rem_euclid(PERIOD_SL);
                                    let new_epoch = ((t_display - new_residual) / PERIOD_SL).round() as i64;
                                    self.t_epoch = new_epoch;
                                    self.t_residual = new_residual;
                                    self.is_paused = true;
                                    let _ = self.sim_handle.cmd_tx.send(Command::Pause);
                                }

                                if time_resp.drag_stopped() {
                                    let was_playing = ui.memory_mut(|mem| mem.data.get_temp(time_resp.id.with("was_playing")).unwrap_or(false));
                                    if was_playing {
                                        self.is_paused = false;
                                        let _ = self.sim_handle.cmd_tx.send(Command::Resume);
                                    }
                                }
                                
                                if self.theme.provider().collapsible_header(ui, "SUPERPOSITION", self.show_pop_metrics) {
                                    self.show_pop_metrics = !self.show_pop_metrics;
                                }
                                ui.add_space(4.0);
                                if self.show_pop_metrics {
                                    let env_data = make_env_data(&self.seed);
                                    let current_t = self.t_epoch as f64 * PERIOD_SL + self.t_residual;
                                    let metrics = crate::ui::ascii_render::get_population_metrics(&env_data, current_t, self.substrate_noise as f64);

                                    egui::Frame::none()
                                        .fill(egui::Color32::from_rgb(10, 10, 10))
                                        .inner_margin(egui::Margin { left: 8, right: 0, top: 8, bottom: 8 })
                                        .show(ui, |ui| {
                                            let inner_w = ui.available_width();
                                            let col_width = ((inner_w - 40.0) / 5.0).floor().max(20.0);
                                            egui::Grid::new("population_metrics_grid")
                                                .num_columns(5)
                                                .spacing([10.0, 6.0])
                                                .min_col_width(col_width)
                                                .show(ui, |ui| {
                                                    let th_col = egui::Color32::LIGHT_GRAY;
                                                    ui.label(egui::RichText::new("WAVE").color(th_col).strong());
                                                    ui.label(egui::RichText::new("GN").color(th_col).strong());
                                                    ui.label(egui::RichText::new("BITS").color(th_col).strong());
                                                    ui.label(egui::RichText::new("PWR").color(th_col).strong());
                                                    ui.label(egui::RichText::new("ENERGY").color(th_col).strong());
                                                    ui.end_row();

                                                    for i in 0..3 {
                                                        let (gn, bits, power, energy) = metrics[i];
                                                        let r = self.wave_colors.get(i).map(|c| c.r()).unwrap_or(255);
                                                        let g = self.wave_colors.get(i).map(|c| c.g()).unwrap_or(255);
                                                        let b = self.wave_colors.get(i).map(|c| c.b()).unwrap_or(255);
                                                        let color = egui::Color32::from_rgb(r, g, b);
                                                        let zeroed = energy <= 0.0;
                                                        let th_col = egui::Color32::LIGHT_GRAY;
                                                        let bit_color = if bits == 16 && !zeroed { egui::Color32::YELLOW } else { th_col };

                                                        ui.label(egui::RichText::new(format!("W{}", i)).color(color).strong());
                                                        ui.label(egui::RichText::new(format!("{:>4}", gn)).color(th_col));
                                                        ui.label(egui::RichText::new(format!("{:>3}", bits)).color(bit_color));
                                                        ui.label(egui::RichText::new(format!("{:.4}", power)).color(th_col));
                                                        let e_text = if zeroed { "zeroed".to_string() } else { format!("{energy:>+.3}") };
                                                        ui.label(egui::RichText::new(e_text).color(th_col));
                                                        ui.end_row();
                                                    }
                                                });
                                        });
                                }
                                ui.add_space(4.0);

                                if self.theme.provider().collapsible_header(ui, "COLOR RIVER", self.show_branch) {
                                    self.show_branch = !self.show_branch;
                                }
                                ui.add_space(4.0);

                                if self.show_branch {
                                    // Chart — always visible, fills from history
                                let chart_height = 160.0;
                                let chart_w = ui.available_width();
                                let (rect, _response) = ui.allocate_exact_size(
                                    egui::vec2(chart_w, chart_height),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    rect,
                                    egui::Rounding::ZERO,
                                    egui::Color32::from_rgb(10, 10, 10),
                                );
                                self.theme.provider().draw_sunken(ui.painter(), rect);

                                // Previous run squishes full→0 in 0.2s with ease-out
                                let old_zone_w = if self.old_history.is_empty() {
                                    0.0f32
                                } else {
                                    let anim = self
                                        .archive_time
                                        .map(|t| (t.elapsed().as_secs_f32() / 0.2).min(1.0))
                                        .unwrap_or(1.0);
                                    rect.width() * (1.0 - anim).powi(2)
                                };

                                // Previous run — squishes into left zone
                                if !self.old_history.is_empty() && old_zone_w > 0.5 {
                                    let dx_old = old_zone_w / self.old_history.len() as f32;
                                    for (i, slice) in self.old_history.iter().enumerate() {
                                        let total: f32 =
                                            slice.iter().map(|&c| c as f32).sum::<f32>().max(1.0);
                                        let mut current_y = rect.max.y;
                                        let x = rect.min.x + (i as f32) * dx_old;
                                        for (generation, &count) in slice.iter().enumerate() {
                                            let h = (count as f32 / total) * chart_height;
                                            if h > 0.0 {
                                                let c = self.old_colors
                                                    [generation % self.old_colors.len()];
                                                let top = (current_y - h).floor();
                                                ui.painter().rect_filled(
                                                    egui::Rect::from_min_max(
                                                        egui::pos2(x.floor(), top),
                                                        egui::pos2(
                                                            (x + dx_old).ceil(),
                                                            current_y.ceil(),
                                                        ),
                                                    ),
                                                    egui::Rounding::ZERO,
                                                    c,
                                                );
                                                current_y -= h;
                                            }
                                        }
                                    }
                                }

                                // Blended colors derived from current mean weights — matches grid/strategy.

                                let bucket_color = |idx: usize| {
                                    self.wave_colors[idx % self.wave_colors.len()]
                                };

                                if !self.history.is_empty() {
                                    let new_zone_w = rect.width() - old_zone_w;
                                    let n = self.history.len() as f32;
                                    let dx = new_zone_w / n;
                                    let x0 = rect.min.x + old_zone_w;
                                    for (i, slice) in self.history.iter().enumerate() {
                                        let total: f32 =
                                            slice.iter().map(|&c| c as f32).sum::<f32>().max(1.0);
                                        let mut current_y = rect.max.y;
                                        let x = x0 + (i as f32) * dx;
                                        for (generation, &count) in slice.iter().enumerate() {
                                            let h = (count as f32 / total) * chart_height;
                                            if h > 0.0 {
                                                let color = bucket_color(generation); // blended_colors
                                                    
                                                let top = (current_y - h).floor();
                                                ui.painter().rect_filled(
                                                    egui::Rect::from_min_max(
                                                        egui::pos2(x.floor(), top),
                                                        egui::pos2(
                                                            (x + dx).ceil(),
                                                            current_y.ceil(),
                                                        ),
                                                    ),
                                                    egui::Rounding::ZERO,
                                                    color,
                                                );
                                                current_y -= h;
                                            }
                                        }
                                    }
                                }

                                // Legend — top colors
                                if let Some(stats) = &self.last_stats {
                                    ui.add_space(4.0);
                                    ui.horizontal_wrapped(|ui| {
                                        let mut active_colors: Vec<_> = stats.color_counts.iter().enumerate().filter(|(_, c)| **c > 0).collect();
                                        active_colors.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
                                        for (generation, &count) in active_colors.into_iter().take(40) {
                                            ui.horizontal(|ui| {
                                                let color = bucket_color(generation);
                                                let (lbl_rect, _) = ui.allocate_exact_size(
                                                    egui::vec2(8.0, 8.0),
                                                    egui::Sense::hover(),
                                                );
                                                ui.painter().rect_filled(
                                                    lbl_rect,
                                                    egui::Rounding::ZERO,
                                                    color,
                                                );
                                                ui.label(format!("{count}"));
                                                ui.add_space(4.0);
                                            });
                                        }
                                    });
                                }

                                    ui.add_space(8.0);
                                } // Ends if self.show_branch

                                ui.add_space(8.0);
                            }); // scroll area
                        }); // side panel
                    
                    if matches!(self.theme, Theme::Rect) {
                        let rect = right_panel_res.response.rect;
                        ctx.layer_painter(egui::LayerId::background()).vline(
                            rect.left(),
                            rect.y_range(),
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 230, 65))
                        );
                    }
                });

                if exit_req { event_loop.exit(); }
                if minimize_req { state.window.set_minimized(true); }
                if rewind_req {                    self.t_epoch = 0;
                    self.t_residual = 0.0;
                    self.wave_speed = 1.0;
                    self.custom_speed = 1.0;
                    self.is_paused = false;
                    let _ = self.sim_handle.cmd_tx.send(Command::Resume);
                    state.window.request_redraw();
                }
                if reroll_req {
                    self.t_epoch = 0;
                    self.t_residual = 0.0;
                    self.wave_speed = 1.0;
                    self.custom_speed = 1.0;
                    self.is_paused = false;
                    let _ = self.sim_handle.cmd_tx.send(Command::Resume);
                    pending_reset = Some(true);
                    state.window.request_redraw();
                }
                if pause_req {
                    self.is_paused = !self.is_paused;
                    if self.is_paused {
                        let _ = self.sim_handle.cmd_tx.send(Command::Pause);
                    } else {
                        let _ = self.sim_handle.cmd_tx.send(Command::Resume);
                    }
                    state.window.request_redraw();
                }
                if let Some(s) = speed_req {
                    self.speed = s;
                    self.wave_speed = match s {
                        0 => 0.25,
                        1 => 1.0,
                        2 => 10.0,
                        3 => 100.0,
                        4 => 1_000.0,
                        _ => 1_000_000.0,
                    };
                    self.custom_speed = self.wave_speed as f64; // keep slider in sync
                    let _ = self
                        .sim_handle
                        .cmd_tx
                        .send(Command::SetSpeed(speed_duration(s)));
                }
                if arrow_up_req || arrow_down_req {
                    const LADDER: &[f64] = &[
                        -1e12, -1e11, -1e10, -1e9, -1e8, -1e7,
                        -1_000_000.0, -1_000.0, -100.0, -10.0, -1.0, -0.25,
                        0.0,
                        0.25, 1.0, 10.0, 100.0, 1_000.0, 1_000_000.0,
                        1e7, 1e8, 1e9, 1e10, 1e11, 1e12,
                    ];
                    let cur = self.wave_speed as f64;
                    let idx = LADDER.iter().enumerate()
                        .min_by(|(_, a), (_, b)| ((**a - cur).abs()).partial_cmp(&((**b - cur).abs())).unwrap())
                        .map(|(i, _)| i).unwrap_or(7);
                    let new_idx = if arrow_up_req { (idx + 1).min(LADDER.len() - 1) } else { idx.saturating_sub(1) };
                    let new_speed = LADDER[new_idx];
                    if new_speed == 0.0 {
                        self.is_paused = true;
                        let _ = self.sim_handle.cmd_tx.send(Command::Pause);
                    } else {
                        self.is_paused = false;
                        let _ = self.sim_handle.cmd_tx.send(Command::Resume);
                    }
                    self.wave_speed = new_speed as f32;
                    self.custom_speed = new_speed;
                    let av = new_speed.abs();
                    let sign = if new_speed < 0.0 { "-" } else { "" };
                    self.speed_text = if av < 0.01 { "0.00 T/s".to_string() }
                        else if av < 10.0 { format!("{}{:.2} T/s", sign, av) }
                        else { format!("{}{:.0} T/s", sign, av) };
                    state.window.request_redraw();
                }
                if arrow_left_req || arrow_right_req {
                    const FREQ_MIN: f64 = 0.1;
                    const PERIOD: f64 = std::f64::consts::TAU / FREQ_MIN;
                    let current_t = self.t_epoch as f64 * PERIOD + self.t_residual;
                    let jump = if current_t.abs() < 1.0 {
                        PERIOD
                    } else {
                        let magnitude = 10f64.powi(current_t.abs().log10().floor() as i32);
                        (current_t.abs() / magnitude).floor() * magnitude
                    };
                    if arrow_left_req { self.t_residual -= jump; } else { self.t_residual += jump; }
                    if self.t_residual >= PERIOD {
                        let extra = (self.t_residual / PERIOD).floor() as i64;
                        self.t_epoch += extra;
                        self.t_residual -= extra as f64 * PERIOD;
                    } else if self.t_residual < 0.0 {
                        let borrow = (-self.t_residual / PERIOD).ceil() as i64;
                        self.t_epoch -= borrow;
                        self.t_residual += borrow as f64 * PERIOD;
                    }
                    self.is_paused = true;
                    let _ = self.sim_handle.cmd_tx.send(Command::Pause);
                    let new_t = self.t_epoch as f64 * PERIOD + self.t_residual;
                    let av_t = new_t.abs();
                    let sign_t = if new_t < 0.0 { "-" } else { "" };
                    self.time_text = if av_t < 0.01 { "0.0 T".to_string() }
                        else if av_t < 1e5 { format!("{}{:.1} T", sign_t, av_t) }
                        else { let e = av_t.log10().floor() as i32; format!("{}{:.2}e{} T", sign_t, av_t / 10f64.powi(e), e) };
                    state.window.request_redraw();
                }
                state
                    .egui_state
                    .handle_platform_output(&state.window, full_output.platform_output);
                let tris = state
                    .egui_ctx
                    .tessellate(full_output.shapes, state.window.scale_factor() as f32);
                for (id, image_delta) in &full_output.textures_delta.set {
                    state
                        .egui_renderer
                        .update_texture(&self.device, &self.queue, *id, image_delta);
                }

                let frame = match state.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
                    _ => return,
                };
                let view = frame.texture.create_view(&Default::default());

                let mut encoder = self.device.create_command_encoder(&Default::default());
                
                let screen_descriptor = egui_wgpu::ScreenDescriptor {
                    size_in_pixels: [
                        state.window.inner_size().width,
                        state.window.inner_size().height,
                    ],
                    pixels_per_point: state.window.scale_factor() as f32,
                };
                state.egui_renderer.update_buffers(
                    &self.device,
                    &self.queue,
                    &mut encoder,
                    &tris,
                    &screen_descriptor,
                );

                // Wave time: advance this frame using epoch+residual split.
                // residual stays in [0, PERIOD) and overflows cleanly into epoch.
                const FREQ_MIN: f64 = 0.1;
                const PERIOD: f64 = std::f64::consts::TAU / FREQ_MIN; // ≈ 62.83
                if !self.is_paused {
                    let frame_dt = 1.0 / self.fps.max(1.0) as f64;
                    self.t_residual += frame_dt * self.wave_speed as f64;
                    // Normalise: residual must stay in [0, PERIOD)
                    if self.t_residual >= PERIOD {
                        let extra = (self.t_residual / PERIOD).floor() as i64;
                        self.t_epoch += extra;
                        self.t_residual -= extra as f64 * PERIOD;
                    } else if self.t_residual < 0.0 {
                        let borrow = (-self.t_residual / PERIOD).ceil() as i64;
                        self.t_epoch -= borrow;
                        self.t_residual += borrow as f64 * PERIOD;
                    }
                }
                // GPU always receives a small, precise f32 residual — full accuracy forever.
                let t_wrapped = self.t_residual as f32;
                self.queue.write_buffer(
                    &state.tick_buf,
                    0,
                    bytemuck::cast_slice(&[t_wrapped, self.substrate_noise, 0.0f32, 0.0f32]),
                );

                // Recompute COLOR RIVER as 240 evenly-spaced T samples ending at current T.
                // Always in sync: rewind, jump, reverse — the river instantly shows that epoch.
                // Window = 10 full slow-drift cycles so you always see meaningful shape.
                {
                    const PERIOD: f64 = std::f64::consts::TAU / 0.1;
                    let t_now = self.t_epoch as f64 * PERIOD + self.t_residual;
                    let window = (10.0 * std::f64::consts::TAU / 0.005)
                        .max(self.wave_speed.abs() as f64 * 4.0);
                    const SAMPLES: usize = 240;
                    self.history.clear();
                    for i in 0..SAMPLES {
                        let frac = i as f64 / (SAMPLES - 1) as f64;
                        let t_sample = t_now - window * (1.0 - frac);
                        let dom = wave_dominance_at(&self.env_data, t_sample, self.substrate_noise);
                        self.history.push_back(dom.iter().map(|&f| (f * 1000.0) as u32).collect());
                    }
                }

                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    pass.set_pipeline(&state.pipeline);

                    let w = screen_descriptor.size_in_pixels[0] as f32;
                    let h = screen_descriptor.size_in_pixels[1] as f32;

                    // Pillarbox: keep the sim square within the area between both panels
                    let panel_physical = 260.0 * screen_descriptor.pixels_per_point;
                    let available_w = (w - panel_physical * 2.0).max(1.0);

                    let side = available_w.min(h);
                    let vx = panel_physical + (available_w - side) / 2.0;
                    let vy = (h - side) / 2.0;
                    pass.set_viewport(vx, vy, side, side, 0.0, 1.0);

                    pass.set_bind_group(0, &state.bg, &[]);
                    pass.draw(0..4, 0..1);

                    pass.set_viewport(0.0, 0.0, w, h, 0.0, 1.0);
                    state.egui_renderer.render(
                        &mut pass.forget_lifetime(),
                        &tris,
                        &screen_descriptor,
                    );
                }
                self.queue.submit([encoder.finish()]);

                // TRUE FPS calculation:
                // We use the exact physical time elapsed since the very start of the last frame (dt).
                // If the user doesn't move the mouse and the simulation is at 4 TPS, this naturally
                // reads 4 FPS. When they interact with the UI, it instantly jumps up to monitor_hz.
                let inst_fps = if dt > 0.0 {
                    1.0 / dt
                } else {
                    monitor_hz
                };
                
                // Clamp to monitor refresh rate so a 1ms frame (1000 FPS) doesn't swing the average wildly.
                let inst_fps = inst_fps.min(monitor_hz); 
                self.fps = self.fps * 0.9 + inst_fps * 0.1;

                frame.present();

                for id in &full_output.textures_delta.free {
                    state.egui_renderer.free_texture(id);
                }

                let repaint_delay = full_output
                    .viewport_output
                    .get(&egui::ViewportId::ROOT)
                    .map(|v| v.repaint_delay)
                    .unwrap_or(std::time::Duration::MAX);

                // "Game Loop" pattern: If unpaused, ALWAYS request the next frame.
                // The `frame.present()` call above blocks on hardware VSync, inherently 
                // capping this loop to exactly your monitor's refresh rate (60/120hz)
                // without spinning the CPU or trusting OS thread-wakeup jitter.
                if repaint_delay.is_zero() || !self.is_paused {
                    state.window.request_redraw();
                    self.sim_handle.ui_requested_frame.store(true, std::sync::atomic::Ordering::Relaxed);
                }

                if repaint_delay < std::time::Duration::from_secs(10) {
                    let target = std::time::Instant::now() + repaint_delay;
                    event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(target));
                } else {
                    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
                }

                // Apply fullscreen AFTER present() so no SurfaceTexture is alive
                // when the platform fires the synchronous Resized event that reconfigures the surface.
                if fullscreen_req {
                    let is_fs = state.window.fullscreen().is_some();
                    state.window.set_fullscreen(if is_fs { None } else { Some(winit::window::Fullscreen::Borderless(None)) });
                    // Reset cursor: platform briefly shows a native resize cursor during the
                    // window-bounds change; override it back to the default arrow.
                    state.window.set_cursor(winit::window::CursorIcon::Default);
                }

                if let Some(new_title) = pending_title {
                    state.window.set_title(&new_title);
                }

                if let Some(reset) = pending_reset {
                    self.reset_simulation(reset);
                }
            }
            _ => {}
        }
    }
}
