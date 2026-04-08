struct App {
    theme: Theme,
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    sim_handle: SimHandle,
    seed: String,
    background_noise: f32,
    state: Option<RenderState>,
    last_stats: Option<Stats>,
    history: VecDeque<(Vec<u32>, Vec<egui::Color32>)>,
    branch_colors: Vec<egui::Color32>,
    wave_colors: Vec<egui::Color32>,
    wave_lch:    Vec<(f64, f64, f64)>, // (L, C, H) seed palette identity per wave
    wave_params0: Vec<[f64; 5]>,       // params at gn=0 — drift anchor so t=0 shows pure seed color
    color_data: crate::engine::color_math::ColorData,
    old_colors: Vec<egui::Color32>,
    last_frame: std::time::Instant,
    fps: f32,
    t_per_sec: f64,        // T units advanced per real second
    last_tps_t_epoch: i64,
    last_tps_t_residual: f64,
    last_tps_update: std::time::Instant,
    pan_x: f32,
    pan_y: f32,
    show_branch: bool,
    show_branch_metrics: bool,
    take_screenshot: bool,
    show_strategy: bool,
    show_metrics: bool,
    branch_density_latest: Option<Vec<u32>>,
    branch_density_dirty: bool,
    last_projection_tick: u32,
    last_bounds_instant: Option<std::time::Instant>,
    /// Branch projection axes and mean, computed on first tick and reused for bounds updates.
    circle_axes: ([f32; 14], [f32; 14], [f32; 14]),
    /// Bounds [min_x, max_x, min_y, max_y] last sent to the GPU density shader.
    last_sent_bounds: [f32; 4],
    speed: u8,
    strategy_engine: crate::ui::window::space_strategy_engine::SpaceStrategyEngine,
    synth_engine: crate::ui::window::synth_engine::SynthEngine,
    acoustic_volume: f32,
    is_muted: bool,
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
    /// CPU copy of the wave parameters so we can evaluate prominence analytically.
    env_data: [f32; 24],
    // Field-texture freshness tracking — field shader only re-runs when T changes.
    last_rendered_epoch:    i64,
    last_rendered_residual: f64,
    field_force_redraw:     bool,
    pending_fullscreen_toggle: bool,
}

/// Mirror of the shader's generation system — same hash, same accumulator.
/// Returns fractional amplitude prominence [0,1] × 3 for the COLOR RIVER.
fn wave_prominence_at(env_data: &[f32; 24], t: f64, noise: f32) -> [f32; 3] {
    const PHI: f64 = std::f64::consts::GOLDEN_RATIO;
    const EUL: f64 = std::f64::consts::E;

    // Hash: continuous harmonic function for smooth traits
    let fhash = |wave_i: u32, gn: u32, ch: u32| -> f64 {
        let x = (wave_i as f64) * 97.0 + (gn as f64) * PHI + (ch as f64) * 7.321;
        let v = x.sin() + (x * std::f64::consts::E).sin() * 0.5 + (x * std::f64::consts::PI).sin() * 0.25;
        (v * 1.5).sin() * 0.5 + 0.5
    };
    let gen_param = |wave_i: u32, gn: u32, ch: u32| -> f64 {
        fhash(wave_i, gn, ch)
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

/// Returns `(projected_points, axis1, axis2, mean)`.
fn compute_projection_2d(points: &[[f32; 14]; 12], valid: &[bool; 12])
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
    let mut data_p2 = centered;
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

        let (noise, fw) = crate::init_seed_params(&self.seed);
        self.background_noise = noise;

        if change_seed {
            println!("[ world ] change channel to new seed: {} (noise: {:.3})", self.seed, noise);
        } else {
            println!("[ world ] rewind current seed: {} (noise: {:.3})", self.seed, noise);
        }

        let _ = self.sim_handle.cmd_tx.send(Command::Reset);
        self.old_colors = self.wave_colors.clone();

        let env_data = make_env_data(&self.seed);
        self.env_data = env_data;

        // Espresso walk sets the seed's palette identity (L, C, H per wave).
        self.wave_lch = super::espresso_walk::seed_lch(&self.seed, 3);

        // Store params at gn=0 as drift anchor — params_to_color subtracts this
        // so the color at t=0 is the seed's palette, with no offset.
        self.wave_params0 = (0..3).map(|w| {
            let gn = crate::ui::ascii_render::get_gn_at_time(&env_data, w, 0.0, noise as f64);
            crate::ui::ascii_render::get_params(&env_data, w, gn)
        }).collect();

        self.wave_colors = (0..3).map(|w| {
            super::espresso_walk::params_to_color(self.wave_lch[w], self.wave_params0[w], self.wave_params0[w])
        }).collect();

        let wc_data: [f32; 12] = {
            let wc = &self.wave_colors;
            [
                wc[0].r() as f32 / 255.0, wc[0].g() as f32 / 255.0, wc[0].b() as f32 / 255.0, 0.0,
                wc[1].r() as f32 / 255.0, wc[1].g() as f32 / 255.0, wc[1].b() as f32 / 255.0, 0.0,
                wc[2].r() as f32 / 255.0, wc[2].g() as f32 / 255.0, wc[2].b() as f32 / 255.0, 0.0,
            ]
        };
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
        
        self.history.clear();
        
        self.branch_density_latest = None;
        self.branch_density_dirty = false;
        self.pan_x = 0.0;
        self.pan_y = 0.0;
        self.last_projection_tick = 0;
        self.last_bounds_instant = None;
        self.circle_axes = ([0.0; 14], [0.0; 14], [0.0; 14]);
        self.last_sent_bounds = [-15.0, 15.0, -15.0, 15.0];
        self.last_stats = None; // Reset stale stats
        self.field_force_redraw = true;
    }
}

