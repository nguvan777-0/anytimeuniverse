/// Terminal renderer — mirrors render.wgsl.
/// Every pixel is O(1) in T: same hash → memory → energy → sample_wave pipeline.
/// ANSI true color (24-bit). Run with --ascii.

use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;

const PI:  f64 = std::f64::consts::PI;
const PHI: f64 = std::f64::consts::GOLDEN_RATIO;
const EUL: f64 = std::f64::consts::E;
const TAU: f64 = std::f64::consts::TAU;
const RES_TARGET: f64 = PHI - 1.0;

// ── Hash (mirrors shader) ─────────────────────────────────────────────────────
fn fhash(wave_i: u32, gn: u64, ch: u32) -> f64 {
    let x = (wave_i as f64) * 97.0 + (gn as f64) * PHI + (ch as f64) * 7.321;
    let v = x.sin() + (x * std::f64::consts::E).sin() * 0.5 + (x * std::f64::consts::PI).sin() * 0.25;
    (v * 1.5).sin() * 0.5 + 0.5
}

fn gen_param(wave_i: u32, gn: u64, ch: u32) -> f64 {
    fhash(wave_i, gn, ch)
}

// ── Golden Attractor (Resonance) ──────────────────────────────────────────────
fn gen_resonance(wave_i: u32, gn: u64) -> f64 {
    fhash(wave_i, gn, 8) // Channel 8 reserved for raw params
}

// ── Accumulator ───────────────────────────────────────────────────────────────
fn gen_acc(drift_freq: f64, drift_phase: f64, t: f64, noise: f64) -> f64 {
    let base   = t * drift_freq * 2.0;
    let wiggle = noise * 0.5 * (
        (drift_freq * PHI * t + drift_phase).sin() +
        (drift_freq * PI  * t + drift_phase * EUL).sin()
    );
    (base + wiggle).max(0.0)
}

// ── Memory (mirrors shader) ───────────────────────────────────────────────────
fn memory(wave_i: u32, gn: u64, ch: u32) -> f64 {
    let own    = gen_param(wave_i, gn, ch);
    let past   = gen_param(wave_i, gn.wrapping_sub(1), ch);
    let amp_n  = 0.3 + gen_param(wave_i, gn, 0) * 2.4;
    let freq_n = 0.4 + gen_param(wave_i, gn, 1) * 1.2;
    
    // Golden Attractor resonance — favors sequences near 0.618 (Golden ratio)
    let resonance = gen_resonance(wave_i, gn);
    let complexity_bonus = 1.0 - ((resonance - RES_TARGET).abs() / RES_TARGET).clamp(0.0, 1.0);

    let power = (amp_n * freq_n * 0.4 * (0.5 + 0.5 * complexity_bonus)).clamp(0.0, 1.0);
    let carry = gen_param(wave_i, gn, ch + 10) * 0.5 + power * 0.5;
    past + (own - past) * carry
}

// ── Energy (mirrors shader) ───────────────────────────────────────────────────
const ENERGY_THRESHOLD: f64 = 0.50;

fn wave_energy(wave_i: u32, gn_in: u64) -> f64 {
    let n = (gn_in % 256) as f64;
    let alpha   = 2.0 * PHI;
    let beta    = wave_i as f64 * PHI * PI;
    let cos_sum = ((n + 1.0) * alpha * 0.5).sin()
                * (n * alpha * 0.5 + beta).cos()
                / (alpha * 0.5).sin();
    1.4 + (n + 1.0) * (0.5 - ENERGY_THRESHOLD) - 0.4 * cos_sum
}

// ── Branch Projection / Omniscience Samplers ─────────────────────────────────
pub fn get_gn_at_time(env: &[f32; 24], wave_i: usize, t: f64, noise: f64) -> u64 {
    let drift_freq  = env[wave_i * 8 + 5] as f64;
    let drift_phase = env[wave_i * 8 + 6] as f64;
    gen_acc(drift_freq, drift_phase, t, noise).floor() as u64
}

pub fn get_params(env: &[f32; 24], wave_i: usize, gn: u64) -> [f64; 5] {
    let base_amp  = env[wave_i * 8]     as f64;
    let base_freq = env[wave_i * 8 + 1] as f64;
    
    let amp_a   = base_amp  * (0.3 + memory(wave_i as u32, gn, 0) * 2.4);
    let freq_a  = base_freq * (0.4 + memory(wave_i as u32, gn, 1) * 1.2);
    let angle_a = memory(wave_i as u32, gn, 2) * TAU;
    let shp_a   = memory(wave_i as u32, gn, 3);
    let wrp_a   = memory(wave_i as u32, gn, 4);
    
    [amp_a, freq_a, angle_a, shp_a, wrp_a]
}


// ── Analytical Kinematics ───────────────────────────────────────────────────
// Harmonic Random Walk: O(1) path that looks like searching movement.
fn wave_center(wave_i: u32, t: f64) -> [f64; 2] {
    let s  = wave_i as f64;
    let fx = 0.13 + s * 0.07;
    let fy = 0.11 + s * 0.03;
    let r  = 4.0 + s * 2.0;

    let exp = (t * 0.01).clamp(0.0, 1.0);
    // Simplified single harmonic jitter
    [r * (t * fx + s * 1.3).sin() * exp, r * (t * fy + s * 2.3).cos() * exp]
}

fn wave_velocity(wave_i: u32, t: f64) -> [f64; 2] {
    let s  = wave_i as f64;
    let fx = 0.13 + s * 0.07;
    let fy = 0.11 + s * 0.03;
    let r  = 4.0 + s * 2.0;
    [r * fx * (t * fx + s * 1.3).cos(), r * -fy * (t * fy + s * 2.3).sin()]
}

// Binary Fission splitting logic
fn fork_center(origin_i: u32, fork_id: u32, t: f64, fork_age: f64) -> [f64; 2] {
    let parent = wave_center(origin_i, t);
    let s  = fork_id as f64;
    let dir = [(s * 7.7).sin(), (s * 9.9).cos()];
    let mag = (dir[0]*dir[0] + dir[1]*dir[1]).sqrt() + 1e-9;
    let dir_norm = [dir[0] / mag, dir[1] / mag];
    
    let max_dist = 2.5 + fhash(fork_id, 0, 30) * 3.0;
    let push     = max_dist * (1.0 - (-(fork_age + 0.1) * 0.15).exp());
    let wiggle = [(t * 0.41 + s).sin() * 0.4, (t * 0.33 + s).cos() * 0.4];
    
    [parent[0] + dir_norm[0] * push + wiggle[0], parent[1] + dir_norm[1] * push + wiggle[1]]
}

// ── Wave sample ───────────────────────────────────────────────────────────────
struct WaveSample {
    sin_term: f64,
    cos_freq: f64,
    #[allow(dead_code)]
    dir:      [f64; 2],
}

#[allow(clippy::too_many_arguments)]
fn sample_wave(
    amp: f64, freq: f64, phase: f64, _dir_x: f64, _dir_y: f64,
    drift_freq: f64, drift_phase: f64,
    wave_i: u32, pos: [f64; 2], t: f64, noise: f64,
) -> WaveSample {
    let acc  = gen_acc(drift_freq, drift_phase, t, noise);
    let gn   = acc.floor() as u64;
    let frac = acc.fract();

    let amp_a   = amp  * (0.3 + memory(wave_i, gn,                  0) * 2.4);
    let freq_a  = freq * (0.4 + memory(wave_i, gn,                  1) * 1.2);
    let angle_a =              memory(wave_i, gn,                  2) * TAU;
    let shp_a   =              memory(wave_i, gn,                  3);
    let wrp_a   =              memory(wave_i, gn,                  4);

    let amp_b   = amp  * (0.3 + memory(wave_i, gn.wrapping_add(1), 0) * 2.4);
    let freq_b  = freq * (0.4 + memory(wave_i, gn.wrapping_add(1), 1) * 1.2);
    let angle_b =              memory(wave_i, gn.wrapping_add(1), 2) * TAU;
    let shp_b   =              memory(wave_i, gn.wrapping_add(1), 3);
    let wrp_b   =              memory(wave_i, gn.wrapping_add(1), 4);

    let blend   = if frac < 0.9 { 0.0 } else { let x = (frac - 0.9) / 0.1; x * x * (3.0 - 2.0 * x) };
    let gn_w    = gn % 256;

    // ── Hydrodynamic Propulsion & Deformation ──
    let vel_t   = wave_velocity(wave_i, t);
    let speed_t = (vel_t[0]*vel_t[0] + vel_t[1]*vel_t[1]).sqrt();
    let angle_v = vel_t[1].atan2(vel_t[0]);
    
    // Kinetic Pulse: entity intensifies during lunge
    let amp_t   = (amp_a + (amp_b - amp_a) * blend) * (0.8 + 0.4 * (speed_t * 0.7).clamp(0.0, 1.0));
    let freq_t  = freq_a  + (freq_b  - freq_a)  * blend;
    let angle_t = (angle_a + (angle_b - angle_a) * blend) * 0.4 + angle_v * 0.6;
    let shape_t = shp_a   + (shp_b   - shp_a)   * blend;
    let warpx_t = wrp_a   + (wrp_b   - wrp_a)   * blend;

    let dir_t     = [angle_t.cos(), angle_t.sin()];
    
    // Domain Warping
    let spatial_warp = [
        (pos[1] * 0.15 + phase).sin() * 2.5,
        (pos[0] * 0.15 - phase).cos() * 2.5,
    ];
    let warped_pos = [pos[0] + spatial_warp[0], pos[1] + spatial_warp[1]];

    let phase_arg = freq_t * (dir_t[0] * warped_pos[0] + dir_t[1] * warped_pos[1]) - freq_t * t + phase;



    // ── Ellipsoidal Deformation (Squash & Stretch) ──
    let center   = wave_center(wave_i, t);
    let rel_pos  = [pos[0] - center[0], pos[1] - center[1]];
    let stretch  = 1.0 + speed_t * 0.5;
    let cos_v    = angle_v.cos();
    let sin_v    = angle_v.sin();
    let p_long   =  rel_pos[0] * cos_v + rel_pos[1] * sin_v;
    let p_lat    = -rel_pos[0] * sin_v + rel_pos[1] * cos_v;
    let d_sq     = (p_long * p_long) / (stretch * stretch) + (p_lat * p_lat) * (stretch * stretch);

    let radius   = 1.5 + (wave_energy(wave_i, gn_w)).clamp(0.0, 2.0) * 1.5;
    let envelope = (-d_sq / (radius * radius)).exp();

    let osc = (1.0 - shape_t) * phase_arg.sin() + shape_t * (phase_arg.sin().abs() * 2.0 - 1.0);
    // SOLID BODY - Base 1.0 results in constant visibility
    let shimmer  = (pos[0] * 2.0 + t).cos() * (pos[1] * 1.5 - t * 0.5).sin();
    let wave_val = 1.0 + 0.3 * osc + warpx_t * 0.2 * shimmer;

    WaveSample {
        sin_term: amp_t * wave_val * envelope,
        cos_freq: amp_t * freq_t * phase_arg.cos() * envelope,
        dir:      dir_t,
    }
}

fn fork_split_at(fork_i: u32) -> u64 {
    (fhash(fork_i, 0, 20) * 80.0) as u64 + 5
}

#[allow(clippy::too_many_arguments)]
fn sample_fork(
    origin_amp: f64, origin_freq: f64, phase: f64,
    drift_freq: f64, drift_phase: f64,
    origin_i: u32, fork_i: u32, pos: [f64; 2], t: f64, noise: f64,
) -> WaveSample {
    let acc       = gen_acc(drift_freq, drift_phase, t, noise);
    let origin_gn = (acc.floor() as u64) % 256;
    let gn_fork   = (fork_split_at(fork_i)) % 256;
    if origin_gn < gn_fork { return WaveSample { sin_term: 0.0, cos_freq: 0.0, dir: [1.0, 0.0] }; }

    let frac       = acc.fract();
    let fork_age   = (origin_gn - gn_fork) as f64 + frac;
    let fork_id    = fork_i + 10;
    let energy     = wave_energy(fork_id, origin_gn - gn_fork);

    // ── Hydrodynamic Propulsion ──
    let vel_p   = wave_velocity(origin_i, t);
    let s_f     = fork_id as f64;
    let dir_sep = [(s_f * 7.7).sin(), (s_f * 9.9).cos()];
    let mag_s   = (dir_sep[0]*dir_sep[0] + dir_sep[1]*dir_sep[1]).sqrt() + 1e-9;
    let dist_m  = 2.5 + fhash(fork_id, 0, 30) * 3.0;
    let vel_sep = [
        dir_sep[0] / mag_s * (dist_m * 0.15 * (-(fork_age + 0.1) * 0.15).exp()),
        dir_sep[1] / mag_s * (dist_m * 0.15 * (-(fork_age + 0.1) * 0.15).exp())
    ];
    let vel_t   = [vel_p[0] + vel_sep[0], vel_p[1] + vel_sep[1]];
    let speed_t = (vel_t[0]*vel_t[0] + vel_t[1]*vel_t[1]).sqrt();
    let angle_v = vel_t[1].atan2(vel_t[0]);

    let fork_mutation = 0.35;
    let gn = origin_gn - gn_fork; 
    
    // parameters at split point
    let amp_a  = origin_amp  * (0.3 + (memory(origin_i, gn_fork, 0) * (1.0 - fork_mutation) + gen_param(fork_id, gn, 0) * fork_mutation) * 2.4);
    let freq_a = origin_freq * (0.4 + (memory(origin_i, gn_fork, 1) * (1.0 - fork_mutation) + gen_param(fork_id, gn, 1) * fork_mutation) * 1.2);
    let ang_a  = (memory(origin_i, gn_fork, 2) * (1.0 - fork_mutation) + gen_param(fork_id, gn, 2) * fork_mutation) * TAU;
    let shp_a  = memory(origin_i, gn_fork, 3) * (1.0 - fork_mutation) + gen_param(fork_id, gn, 3) * fork_mutation;
    let wrp_a  = memory(origin_i, gn_fork, 4) * (1.0 - fork_mutation) + gen_param(fork_id, gn, 4) * fork_mutation;

    let amp_b  = origin_amp  * (0.3 + (memory(origin_i, gn_fork + 1, 0) * (1.0 - fork_mutation) + gen_param(fork_id, gn + 1, 0) * fork_mutation) * 2.4);
    let freq_b = origin_freq * (0.4 + (memory(origin_i, gn_fork + 1, 1) * (1.0 - fork_mutation) + gen_param(fork_id, gn + 1, 1) * fork_mutation) * 1.2);
    let ang_b  = (memory(origin_i, gn_fork + 1, 2) * (1.0 - fork_mutation) + gen_param(fork_id, gn + 1, 2) * fork_mutation) * TAU;
    let shp_b  = memory(origin_i, gn_fork + 1, 3) * (1.0 - fork_mutation) + gen_param(fork_id, gn + 1, 3) * fork_mutation;
    let wrp_b  = memory(origin_i, gn_fork + 1, 4) * (1.0 - fork_mutation) + gen_param(fork_id, gn + 1, 4) * fork_mutation;

    let blend   = if frac < 0.9 { 0.0 } else { let x = (frac - 0.9) / 0.1; x * x * (3.0 - 2.0 * x) };
    let amp_t   = (amp_a + (amp_b - amp_a) * blend) * (0.8 + 0.4 * (speed_t * 0.7).clamp(0.0, 1.0));
    let freq_t  = freq_a  + (freq_b  - freq_a)  * blend;
    let angle_t = (ang_a + (ang_b - ang_a) * blend) * 0.4 + angle_v * 0.6;
    let shape_t = shp_a   + (shp_b   - shp_a)   * blend;
    let warpx_t = wrp_a   + (wrp_b   - wrp_a)   * blend;

    let dir_t = [angle_t.cos(), angle_t.sin()];
    let phase_arg = freq_t * (dir_t[0] * pos[0] + dir_t[1] * pos[1]) - freq_t * t + phase;
    
    // ── Hydrodynamic Deformation (Squash & Stretch) ──
    let center   = fork_center(origin_i, fork_id, t, fork_age);
    let rel_pos  = [pos[0] - center[0], pos[1] - center[1]];
    let stretch  = 1.0 + speed_t * 0.5;
    let cos_v    = angle_v.cos();
    let sin_v    = angle_v.sin();
    let p_long   =  rel_pos[0] * cos_v + rel_pos[1] * sin_v;
    let p_lat    = -rel_pos[0] * sin_v + rel_pos[1] * cos_v;
    let d_sq     = (p_long * p_long) / (stretch * stretch) + (p_lat * p_lat) * (stretch * stretch);

    let radius = 1.0 + energy.clamp(0.0, 2.0) * 1.0;
    let envelope = (-d_sq / (radius * radius)).exp();

    let osc = (1.0 - shape_t) * phase_arg.sin() + shape_t * (phase_arg.sin().abs() * 2.0 - 1.0);
    // SOLID BODY
    let shimmer = (pos[0] * 1.5 + t).cos() * (pos[1] * 2.0 - t * 0.5).sin();
    let wave_val = 1.0 + 0.3 * osc + warpx_t * 0.2 * shimmer;
    
    WaveSample {
        sin_term: amp_t * wave_val * envelope * 0.6,
        cos_freq: amp_t * freq_t * phase_arg.cos() * envelope * 0.6,
        dir:      dir_t,
    }
}

// ── Pixel color (mirrors fs_main) ────────────────────────────────────────────
fn pixel_rgb(
    env: &[f32; 24],
    wave_colors: &[[f64; 3]; 3],
    pos: [f64; 2],
    t: f64,
    noise: f64,
) -> [u8; 3] {
    let w = |i: usize| sample_wave(
        env[i*8]     as f64,
        env[i*8 + 1] as f64,
        env[i*8 + 2] as f64,
        env[i*8 + 3] as f64,
        env[i*8 + 4] as f64,
        env[i*8 + 5] as f64,
        env[i*8 + 6] as f64,
        i as u32, pos, t, noise,
    );
    let wave0 = w(0); let wave1 = w(1); let wave2 = w(2);

    let f = |i: usize| sample_fork(
        env[i*8]     as f64,
        env[i*8 + 1] as f64,
        env[i*8 + 2] as f64,
        env[i*8 + 5] as f64,
        env[i*8 + 6] as f64,
        i as u32, i as u32, pos, t, noise,
    );
    let f0 = f(0); let f1 = f(1); let f2 = f(2);

    let amp_sum = (env[0] + env[8] + env[16]) as f64 + 1e-5;

    let field     = (wave0.sin_term + wave1.sin_term + wave2.sin_term + f0.sin_term + f1.sin_term + f2.sin_term) / amp_sum;
    let dfield_dt = -(wave0.cos_freq + wave1.cos_freq + wave2.cos_freq + f0.cos_freq + f1.cos_freq + f2.cos_freq) / amp_sum;

    let signal = field.max(0.0).powi(3);

    // branch color
    let r0 = (wave0.sin_term + f0.sin_term).max(0.0) / (env[0]  as f64 + 1e-5);
    let r1 = (wave1.sin_term + f1.sin_term).max(0.0) / (env[8]  as f64 + 1e-5);
    let r2 = (wave2.sin_term + f2.sin_term).max(0.0) / (env[16] as f64 + 1e-5);
    let rsum = r0 + r1 + r2 + 1e-5;

    let mut col = [
        ( (r0 / rsum) * wave_colors[0][0] + (r1 / rsum) * wave_colors[1][0] + (r2 / rsum) * wave_colors[2][0] ),
        ( (r0 / rsum) * wave_colors[0][1] + (r1 / rsum) * wave_colors[1][1] + (r2 / rsum) * wave_colors[2][1] ),
        ( (r0 / rsum) * wave_colors[0][2] + (r1 / rsum) * wave_colors[1][2] + (r2 / rsum) * wave_colors[2][2] )
    ];

    // time tint — rising = cool, falling = warm
    let time_tint = (dfield_dt * 0.4).clamp(-1.0, 1.0);
    let tint = if time_tint > 0.0 {
        [-0.10, 0.05, 0.20]   // cool
    } else {
        [0.15, -0.05, -0.15]  // warm
    };
    for c in 0..3 { col[c] = (col[c] + tint[c] * time_tint.abs()).clamp(0.0, 1.0); }

    // mix toward white at signal peaks
    let rgb: [f64; 3] = std::array::from_fn(|c| {
        let lit = col[c];
        let mixed = lit + (1.0 - lit) * signal * signal;
        (mixed * signal).clamp(0.0, 1.0)
    });

    // gamma 2.2
    [
        (rgb[0].powf(2.2) * 255.0) as u8,
        (rgb[1].powf(2.2) * 255.0) as u8,
        (rgb[2].powf(2.2) * 255.0) as u8,
    ]
}

// ── Terminal size ─────────────────────────────────────────────────────────────
fn terminal_size() -> (usize, usize) {
    if let Ok((w, h)) = crossterm::terminal::size() {
        (w as usize, h as usize)
    } else {
        (80, 24)
    }
}

// ── Raw terminal mode ─────────────────────────────────────────────────────────
// Switches stdin to raw + non-blocking. Restores on drop.
struct RawMode {}

impl RawMode {
    fn enter() -> Self {
        let _ = crossterm::terminal::enable_raw_mode();
        RawMode {}
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        // restore cursor and flush
        let _ = std::io::stdout().write_all(b"\x1b[?25h\n");
        let _ = std::io::stdout().flush();
    }
}

// ── Input ─────────────────────────────────────────────────────────────────────
pub enum Action { None, Quit, Rewind, NewSeed }

fn poll_keys(c: &mut crate::ui::controls::Controls) -> Action {
    use crossterm::event::{Event, KeyCode, KeyModifiers};
    
    let mut action = Action::None;
    
    // Poll for all available events without blocking
    while let Ok(true) = crossterm::event::poll(std::time::Duration::from_secs(0)) {
        if let Ok(Event::Key(key)) = crossterm::event::read() {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Action::Quit;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Action::Quit,
                KeyCode::Char(' ') => c.toggle_pause(),
                KeyCode::Char('r') => { c.rewind(); action = Action::Rewind; }
                KeyCode::Char('c') => { c.rewind(); action = Action::NewSeed; }
                KeyCode::Char('1') => c.preset(1),
                KeyCode::Char('2') => c.preset(2),
                KeyCode::Char('3') => c.preset(3),
                KeyCode::Char('4') => c.preset(4),
                KeyCode::Char('5') => c.preset(5),
                KeyCode::Right => c.rewind_fwd(),
                KeyCode::Left  => c.rewind_back(),
                KeyCode::Up    => c.speed_up(),
                KeyCode::Down  => c.speed_down(),
                _ => {}
            }
        }
    }
    action
}

// ── Wave status line ──────────────────────────────────────────────────────────
#[allow(dead_code)]
fn wave_status(env: &[f32; 24], noise: f64, t: f64, wave_colors: &[[f64; 3]; 3], _cols: usize) -> String {
    let mut out = String::new();
    for i in 0..3usize {
        let drift_freq  = env[i*8 + 5] as f64;
        let drift_phase = env[i*8 + 6] as f64;
        let acc = gen_acc(drift_freq, drift_phase, t, noise);
        let gn  = acc.floor() as u64;
        let energy = wave_energy(i as u32, gn);

        let amp_n  = 0.3 + gen_param(i as u32, gn, 0) * 2.4;
        let freq_n = 0.4 + gen_param(i as u32, gn, 1) * 1.2;
        let resonance = gen_resonance(i as u32, gn);
        let complexity_bonus = 1.0 - ((resonance - RES_TARGET).abs() / RES_TARGET).clamp(0.0, 1.0);
        let power = (amp_n * freq_n * 0.4 * (0.5 + 0.5 * complexity_bonus)).clamp(0.0, 1.0);

        let [r, g, b] = wave_colors[i].map(|v| (v * 255.0) as u8);
        let bar_len = 16usize;
        let filled = if energy > 0.0 { (power * bar_len as f64).round() as usize } else { 0 };

        let state_str = if energy > 0.0 { "active" } else { "zeroed" };
        let _ = write!(
            out,
            "\x1b[38;2;{r};{g};{b}m wave{i}\x1b[0m  gn:{gn:>4}  resn:{resonance:>4.2}  pwr:{power:.2}  energy:{energy:+.2}  ["
        );
        for j in 0..bar_len {
            if j < filled { out.push('█') } else { out.push('░') }
        }
        let _ = writeln!(out, "]  {state_str}");
    }
    out
}

// ── Color river (80 samples of wave prominence history) ───────────────────────
#[allow(dead_code)]
fn color_river(env: &[f32; 24], noise: f64, t_now: f64, wave_colors: &[[f64; 3]; 3], cols: usize) -> String {
    let window = 100.0;
    let mut out = String::new();
    for i in 0..cols {
        let frac     = i as f64 / (cols - 1) as f64;
        let t_sample = t_now - window * (1.0 - frac);

        let mut doms = [0.0f64; 3];
        let mut total = 0.0f64;
        for wi in 0..3usize {
            let drift_freq  = env[wi*8 + 5] as f64;
            let drift_phase = env[wi*8 + 6] as f64;
            let acc = gen_acc(drift_freq, drift_phase, t_sample, noise);
            let gn  = acc.floor() as u64;
            let amp_n = 0.3 + gen_param(wi as u32, gn, 0) * 2.4;
            let freq_n = 0.4 + gen_param(wi as u32, gn, 1) * 1.2;
            let resonance = gen_resonance(wi as u32, gn);
            let complexity_bonus = 1.0 - ((resonance - RES_TARGET).abs() / RES_TARGET).clamp(0.0, 1.0);
            let power = (amp_n * freq_n * 0.4 * (0.5 + 0.5 * complexity_bonus)).clamp(0.0, 1.0);
            doms[wi] = power;
            total += doms[wi];
        }
        total = total.max(1e-5);

        let r = (doms[0]/total * wave_colors[0][0]
               + doms[1]/total * wave_colors[1][0]
               + doms[2]/total * wave_colors[2][0]) * 255.0;
        let g = (doms[0]/total * wave_colors[0][1]
               + doms[1]/total * wave_colors[1][1]
               + doms[2]/total * wave_colors[2][1]) * 255.0;
        let b = (doms[0]/total * wave_colors[0][2]
               + doms[1]/total * wave_colors[1][2]
               + doms[2]/total * wave_colors[2][2]) * 255.0;

        let _ = write!(out, "\x1b[38;2;{};{};{}m█", r as u8, g as u8, b as u8);
    }
    out.push_str("\x1b[0m");
    out
}

// ── Main render loop ──────────────────────────────────────────────────────────
// Prints a grid frame every `step` units of T, then scrolls.
// Usage: cargo run -- --ascii [--step 0.1] [--width 60] [--height 20] [--t0 0.0]
pub fn run(seed: &str, background_noise: f32, initial_wave_colors: [[f64; 3]; 3]) {
    // parse flags from env args
    let args: Vec<String> = std::env::args().collect();
    let get = |flag: &str, default: f64| -> f64 {
        args.windows(2)
            .find(|w| w[0] == flag)
            .and_then(|w| w[1].parse().ok())
            .unwrap_or(default)
    };
    let tape   = args.iter().any(|a| a == "--tape");
    let step   = get("--step",   if tape { 1000.0 } else { 0.1 });
    
    let (term_w, term_h) = terminal_size();
    let width_default = term_w.saturating_sub(1).max(10) as f64;
    // 1 header + 3 branch statuses + 1 control hint = 5 lines. Give 1 line breathing room.
    let height_default = term_h.saturating_sub(6).max(1) as f64;

    let width  = get("--width",  width_default) as usize;
    let height = get("--height", height_default) as usize;
    let t      = get("--t0",     0.0);

    let noise  = background_noise as f64;
    let mut wave_colors = initial_wave_colors;
    let stdout = std::io::stdout();

    // character ramp: dark → bright
    let ramp: &[char] = &[' ', '·', ':', ';', '-', '=', '+', '*', '#', '%', '@'];

    // tape is pipe-friendly — no raw mode, no cursor tricks, no key handling
    let _raw = if !tape { Some(RawMode::enter()) } else { None };

    if !tape { 
        // Hide cursor and switch to Alternate Screen Buffer
        print!("\x1b[?25l\x1b[?1049h"); 
        let _ = stdout.lock().flush();
    }

    let mut s = crate::ui::controls::Controls::new(t, step);
    let mut current_seed = seed.to_string();
    let mut env = crate::ui::window::make_env_data_pub(&current_seed);

    let mut first_frame = true;

    loop {
        // ── input (interactive only — tape is pipe-friendly, ctrl-c to stop) ──
        if !tape {
            match poll_keys(&mut s) {
                Action::Quit   => break,
                Action::Rewind => { first_frame = true; } // redraw from top
                Action::NewSeed => {
                    let hash = crate::hash_seed(&current_seed);
                    current_seed = crate::generate_seed(hash);
                    env = crate::ui::window::make_env_data_pub(&current_seed);
                    let new_colors = crate::ui::espresso_walk::generate(3, &current_seed, crate::ui::espresso_walk::Palette::Wide);
                    wave_colors = std::array::from_fn(|i| {
                        let c = new_colors[i];
                        [c.r() as f64 / 255.0, c.g() as f64 / 255.0, c.b() as f64 / 255.0]
                    });
                    first_frame = true;
                }
                Action::None   => {}
            }
        }

        if s.paused {
            std::thread::sleep(std::time::Duration::from_millis(16));
            continue;
        }
        
        let (term_w, term_h) = terminal_size();
        
        // Dynamic HUD toggling: hide wave breakdown if terminal is too short
        let show_channels = term_h >= 24;
        let hud_lines = if show_channels { 1 + 3 + 1 } else { 1 + 1 }; // header + channels + controls
        
        let dyn_width  = term_w.saturating_sub(1).max(10);
        let dyn_height = term_h.saturating_sub(hud_lines).max(1);
        
        let width = if args.iter().any(|a| a == "--width") { width } else { dyn_width };
        let height = if args.iter().any(|a| a == "--height") { height } else { dyn_height };
        let frame_lines = hud_lines + height;

        let mut buf = String::with_capacity(width * frame_lines * 30);

        if tape {
            // ── tape: one data row per step ───────────────────────────────────
            // print header every 32 rows so it stays visible after scrolling
            // column layout (all widths fixed so header and data align):
            // T(12)  seed(8)  gn(5) bits(4) pwr(4) energy(7) ...  dominant
            const SEP: &str = "  ";
            if ((s.t / s.step) as u64).is_multiple_of(32) {
                let _ = writeln!(buf,
                    "{:>12}{SEP}{:>8}{SEP}{:>8} {:>4} {:>4} {:>7}{SEP}{:>8} {:>4} {:>4} {:>7}{SEP}{:>8} {:>4} {:>4} {:>7}{SEP}dominant",
                    "T", "seed",
                    "wave0:gn", "resn", "pwr", "energy",
                    "wave1:gn", "resn", "pwr", "energy",
                    "wave2:gn", "resn", "pwr", "energy");
                buf.push_str(&"─".repeat(115));
                buf.push('\n');
            }

            let mut wave_data = [(0u64, 0.0f64, 0.0f64, 0.0f64); 3];
            for i in 0..3usize {
                let drift_freq  = env[i*8 + 5] as f64;
                let drift_phase = env[i*8 + 6] as f64;
                let acc = gen_acc(drift_freq, drift_phase, s.t, noise);
                let gn  = acc.floor() as u64;
                let energy  = wave_energy(i as u32, gn);
                let amp_n   = 0.3 + gen_param(i as u32, gn, 0) * 2.4;
                let freq_n  = 0.4 + gen_param(i as u32, gn, 1) * 1.2;
                let resonance = gen_resonance(i as u32, gn);
                let complexity_bonus = 1.0 - ((resonance - RES_TARGET).abs() / RES_TARGET).clamp(0.0, 1.0);
                let power = (amp_n * freq_n * 0.4 * (0.5 + 0.5 * complexity_bonus)).clamp(0.0, 1.0);
                wave_data[i] = (gn, power, energy, resonance);
            }

            // dominant: active branch with highest power
            let dominant = (0..3usize)
                .max_by(|&a, &b| wave_data[a].1.partial_cmp(&wave_data[b].1).unwrap());

            let dom_str = match dominant {
                None    => "no signal".to_string(),
                Some(i) => format!("wave{}{}", i, if wave_data[i].1 > 0.95 { " PEAK" } else { "" }),
            };

            let annotation = "".to_string();


            // data row — same widths as header
            let _ = write!(buf, "{:>12.4e}{SEP}{:>8}", s.t, current_seed);
            for i in 0..3usize {
                let (gn, pwr, energy, resonance_val) = wave_data[i];
                let e = if energy > 0.0 { format!("{energy:>+7.3}") } else { " zeroed".to_string() };
                let res_str = format!("{:.2}", resonance_val);
                let res_str_stripped = res_str.strip_prefix("0").unwrap_or(&res_str);
                let _ = write!(buf, "{SEP}{:>8} {:>4} {:>4.2} {}", gn, res_str_stripped, pwr, e);
            }
            let _ = writeln!(buf, "{SEP}{}{}", dom_str, annotation);

        } else {
            // ── ascii interactive: full ASCII grid ─────────────────────────

            if !first_frame { let _ = write!(buf, "\x1b[{}A", frame_lines); }
            else { let _ = write!(buf, "\x1b[H\x1b[2J"); } // clear alternate screen

            // header
            let status = if s.paused { "PAUSED" } else { "      " };
            let _ = write!(buf,
                "\x1b[2K\r\x1b[2m── T: {:.4e}  seed: {}  noise: {:.3}  step: {:.2}  {status}\x1b[0m\n",
                s.t, current_seed, noise, s.step);

            // grid
            for row in 0..height {
                buf.push_str("\x1b[2K\r");
                for col in 0..width {
                    // Coordinates [0, 1]
                    let uv_x = col as f64 / width  as f64;
                    let uv_y = 1.0 - row as f64 / height as f64;
                    
                    // Characters are ~2:1 height:width, so correct visual aspect ratio for terminal
                    let visual_w = width as f64 * 0.5; 
                    let visual_h = height as f64 * 1.0;
                    let sq_size = visual_w.min(visual_h); // Largest square that fits the screen
                    
                    // [-1, 1] normalized to the largest centered square, 
                    // with visual blank padding on whichever axis is longer
                    let norm_x = (visual_w * uv_x - visual_w * 0.5) / sq_size * 2.0;
                    let norm_y = (visual_h * uv_y - visual_h * 0.5) / sq_size * 2.0;

                    // Infinite Open Universe expanding forever
                    let universe_radius = s.t * 2.0;

                    // Viewport camera width (static zoom so universe visibly expands)
                    let camera_span = 50.0_f64;
                    
                    let raw_pos = [norm_x * camera_span, norm_y * camera_span];
                    
                    // Shape Morphing (Perfect blending from Circle to Square)
                    let dist_circle = (raw_pos[0]*raw_pos[0] + raw_pos[1]*raw_pos[1]).sqrt();
                    let dist_square = raw_pos[0].abs().max(raw_pos[1].abs());
                    let shape_blend = 1.0 - (-s.t * 0.1).exp(); // 0.0 at T=0, 1.0 at T=∞
                    let dist = dist_circle * (1.0 - shape_blend) + dist_square * shape_blend;
                    
                    if dist > universe_radius {
                        let _ = write!(buf, " ");
                        continue;
                    }
                    
                    // Cosmological Expansion Factor
                    let cosmic_scale = 1.0 + 300.0 * (-s.t * 0.15).exp();
                    let pos = [raw_pos[0] * cosmic_scale, raw_pos[1] * cosmic_scale];
                    
                    let [r, g, b] = pixel_rgb(&env, &wave_colors, pos, s.t, noise);
                    let lum = (r as f64 * 0.299 + g as f64 * 0.587 + b as f64 * 0.114) / 255.0;
                    let ch  = ramp[(lum * (ramp.len() - 1) as f64) as usize];
                    let _ = write!(buf, "\x1b[38;2;{r};{g};{b}m{ch}");
                }
                buf.push_str("\x1b[0m\n");
            }

            // branch status bars
            if show_channels {
                for i in 0..3usize {
                    let drift_freq  = env[i*8 + 5] as f64;
                    let drift_phase = env[i*8 + 6] as f64;
                    let acc     = gen_acc(drift_freq, drift_phase, s.t, noise);
                    let gn      = acc.floor() as u64;
                    let energy  = wave_energy(i as u32, gn);
                    let amp_n   = 0.3 + gen_param(i as u32, gn, 0) * 2.4;
                    let freq_n  = 0.4 + gen_param(i as u32, gn, 1) * 1.2;
                    let resonance = gen_resonance(i as u32, gn);
                    let complexity_bonus = 1.0 - ((resonance - RES_TARGET).abs() / RES_TARGET).clamp(0.0, 1.0);
                    let power = (amp_n * freq_n * 0.4 * (0.5 + 0.5 * complexity_bonus)).clamp(0.0, 1.0);
                    let [r, g, b] = wave_colors[i].map(|v| (v * 255.0) as u8);
                    let bar: String = (0..12).map(|j| {
                        if j < (power * 12.0) as usize { '█' } else { '░' }
                    }).collect();
                    let _ = write!(buf,
                        "\x1b[2K\r  \x1b[38;2;{r};{g};{b}mwave{i}\x1b[0m \
                         gn:{gn:>4}  resn:{resonance:>4.2}  pwr:{power:.2}  energy:{energy:+.2}  [{bar}]\n");
                }
            }

            // controls hint
            let _ = write!(buf,
                "\x1b[2K\r\x1b[2m  space:pause  ←→:rewind  ↑↓:speed  1-5:preset  r:rewind  c:seed  q:quit\x1b[0m\n");
        }

        first_frame = false;

        {
            let mut lock = stdout.lock();
            let _ = lock.write_all(buf.as_bytes());
            let _ = lock.flush();
        }

        s.advance();
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    if !tape {
        // Restore cursor, exit Alternate Screen Buffer, and clear screen on exit
        print!("\x1b[?1049l\x1b[?25h\x1b[2J\x1b[H"); 
        let _ = stdout.lock().flush();
    }
}

pub fn get_summary_metrics(env: &[f32; 24], t: f64, noise: f64) -> [(u64, f64, f64, f64); 3] {
    let mut out = [(0, 0.0, 0.0, 0.0); 3];
    for i in 0..3usize {
        let drift_freq  = env[i*8 + 5] as f64;
        let drift_phase = env[i*8 + 6] as f64;
        let acc = gen_acc(drift_freq, drift_phase, t, noise);
        let gn  = acc.floor() as u64;
        let energy  = wave_energy(i as u32, gn);
        let amp_n   = 0.3 + gen_param(i as u32, gn, 0) * 2.4;
        let freq_n  = 0.4 + gen_param(i as u32, gn, 1) * 1.2;
        let resonance = gen_resonance(i as u32, gn);
        let complexity_bonus = 1.0 - ((resonance - RES_TARGET).abs() / RES_TARGET).clamp(0.0, 1.0);
        let power = (amp_n * freq_n * 0.4 * (0.5 + 0.5 * complexity_bonus)).clamp(0.0, 1.0);
        out[i] = (gn, resonance, power, energy);
    }
    out
}
