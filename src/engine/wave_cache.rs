use bytemuck::Zeroable;

///
/// Every computation inside `fhash / gen_param / memory / gen_acc / wave_energy
/// / wave_center / wave_velocity / fork_center / fork_split_at` produces the
/// SAME result for every pixel — it only depends on `wave_i`, `gn` (derived
/// from T), and `ch`.  By computing these once per frame on the CPU and
/// uploading the results as a 384-byte uniform buffer, the fragment shader's
/// per-pixel trig drops from ~1 200 calls to ~30.
///
/// CRITICAL: all arithmetic uses f32 to match WGSL.  The constants
/// (97.0, 1.61803398, 7.321, 2.71828, 3.14159, 1.5 ...) must match render.wgsl.

const PI:  f32 = std::f32::consts::PI;
const PHI: f32 = std::f32::consts::GOLDEN_RATIO;
const EUL: f32 = std::f32::consts::E;
const TAU: f32 = std::f32::consts::TAU;
const RES_TARGET: f32 = PHI - 1.0;

const FORK_MUTATION:    f32 = 0.35;
const FORK_WEIGHT:      f32 = 0.6;

// ── WaveData ─────────────────────────────────────────────────────────────────
// One struct per wave source (3 main waves + 3 forks = 6 total).
// Layout must match the WGSL `WaveData` struct in render.wgsl.
// 16 × f32 = 64 bytes.  Six of these = 384 bytes.  All within one uniform.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WaveData {
    /// Final effective amplitude (kinetic pulse applied; forks ×FORK_WEIGHT).
    pub amp:       f32,
    /// Final frequency (blended across gn and gn+1).
    pub freq:      f32,
    /// cos(angle_t) — wave propagation direction.
    pub dir_x:     f32,
    /// sin(angle_t).
    pub dir_y:     f32,
    /// Sine / abs-sine blend weight [0, 1].
    pub shape:     f32,
    /// Spatial warp multiplier.
    pub warp:      f32,
    /// Envelope Gaussian centre x.
    pub cx:        f32,
    /// Envelope Gaussian centre y.
    pub cy:        f32,
    /// cos(velocity direction) for ellipsoid rotation.
    pub cos_v:     f32,
    /// sin(velocity direction).
    pub sin_v:     f32,
    /// Ellipsoid stretch factor (1 + speed×0.5).
    pub stretch:   f32,
    /// Gaussian envelope radius.
    pub radius:    f32,
    /// Pre-computed −freq×t + origin_phase.  Shader adds freq×dot(dir,pos).
    pub phase_off: f32,
    /// Origin wave phase (stored for fork spatial-warp computation).
    pub origin_ph: f32,
    /// 1.0 = alive (energy > 0 or wave not yet gated); 0.0 = inactive (fork).
    pub alive:    f32,
    /// env.waves[origin_i].amp — base amplitude for per-branch colour ratio.
    pub base_amp:  f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers — mirror render.wgsl, all f32.
// ─────────────────────────────────────────────────────────────────────────────

#[inline(always)]
fn fhash(wave_i: u32, gn: u32, ch: u32) -> f32 {
    // Wrap gn to the exact-integer range of f32 (2^24) so the hash cycles
    // instead of freezing when gn grows beyond f32 precision.
    let gn = gn % (1 << 24);
    let x = wave_i as f32 * 97.0 + gn as f32 * PHI + ch as f32 * 7.321;
    let v = x.sin() + (x * EUL).sin() * 0.5 + (x * PI).sin() * 0.25;
    (v * 1.5).sin() * 0.5 + 0.5
}

/// mirrors gen_param / gen_resonance in the shader.
#[inline(always)]
fn gp(wave_i: u32, gn: u32, ch: u32) -> f32 { fhash(wave_i, gn, ch) }

/// mirrors memory(wave_i, gn, ch) — mix(own, past, carry).
fn memory(wave_i: u32, gn: u32, ch: u32) -> f32 {
    let own   = gp(wave_i, gn, ch);
    let past  = gp(wave_i, gn.wrapping_sub(1), ch);
    let amp_n = 0.3 + gp(wave_i, gn, 0) * 2.4;
    let freq_n = 0.4 + gp(wave_i, gn, 1) * 1.2;
    let resonance = gp(wave_i, gn, 8);
    let cb = 1.0 - ((resonance - RES_TARGET).abs() / RES_TARGET).clamp(0.0, 1.0);
    let power = (amp_n * freq_n * 0.4 * (0.5 + 0.5 * cb)).clamp(0.0, 1.0);
    // mix(gp(ch+10), power, 0.5)
    let carry = gp(wave_i, gn, ch + 10) * 0.5 + power * 0.5;
    // mix(own, past, carry) = own×(1−carry) + past×carry
    own * (1.0 - carry) + past * carry
}

/// mirrors gen_acc in the shader.
fn gen_acc(drift_freq: f32, drift_phase: f32, t: f32, noise: f32) -> f32 {
    let base   = t * drift_freq * 2.0;
    let wiggle = noise * 0.5 * (
        (drift_freq * PHI * t + drift_phase).sin() +
        (drift_freq * PI  * t + drift_phase * EUL).sin()
    );
    (base + wiggle).max(0.0)
}

/// High-precision f64 version of gen_acc — used in compute() so gn never
/// saturates at u32::MAX regardless of T magnitude.
fn gen_acc_f64(drift_freq: f64, drift_phase: f64, t: f64, noise: f64) -> f64 {
    const PHI_F64: f64 = 1.6180339887498948_f64;
    let base   = t * drift_freq * 2.0;
    let wiggle = noise * 0.5 * (
        (drift_freq * PHI_F64 * t + drift_phase).sin() +
        (drift_freq * std::f64::consts::PI * t + drift_phase * std::f64::consts::E).sin()
    );
    (base + wiggle).max(0.0)
}

/// mirrors wave_energy in the shader.
fn wave_energy(wave_i: u32, gn: u32) -> f32 {
    let alpha   = 2.0 * PHI;
    let beta    = wave_i as f32 * PHI * PI;
    let n       = gn as f32;
    let cos_sum = ((n + 1.0) * alpha * 0.5).sin()
                * (n * alpha * 0.5 + beta).cos()
                / (alpha * 0.5).sin();
    1.0 - 0.5 * cos_sum
}

/// mirrors wave_center in the shader.
/// t_f64 is used for trig arguments to avoid f32 precision loss at extreme T.
fn wave_center(wave_i: u32, t_f64: f64) -> [f32; 2] {
    let s  = wave_i as f64;
    let fx = 0.13 + s * 0.07;
    let fy = 0.11 + s * 0.03;
    let r  = (4.0 + s * 2.0) as f32;
    // clamp expansion: saturates at 1.0 well before precision matters
    let exp = (t_f64 * 0.01).clamp(0.0, 1.0) as f32;
    // Reduce arguments mod TAU in f64 before sin/cos so precision is kept at any T.
    let arg_x = (t_f64 * fx + s * 1.3).rem_euclid(std::f64::consts::TAU);
    let arg_y = (t_f64 * fy + s * 2.3).rem_euclid(std::f64::consts::TAU);
    [r * arg_x.sin() as f32 * exp,
     r * arg_y.cos() as f32 * exp]
}

/// mirrors wave_velocity in the shader.
fn wave_velocity(wave_i: u32, t_f64: f64) -> [f32; 2] {
    let s  = wave_i as f64;
    let fx = 0.13 + s * 0.07;
    let fy = 0.11 + s * 0.03;
    let r  = 4.0 + s * 2.0;
    // Same mod-TAU reduction as wave_center.
    let arg_x = (t_f64 * fx + s * 1.3).rem_euclid(std::f64::consts::TAU);
    let arg_y = (t_f64 * fy + s * 2.3).rem_euclid(std::f64::consts::TAU);
    [(r * fx * arg_x.cos()) as f32,
     (r * -fy * arg_y.sin()) as f32]
}

/// mirrors fork_split_at in the shader.
fn fork_split_at(fork_i: u32) -> u32 {
    (gp(fork_i, 0, 20) * 80.0) as u32 + 5
}

/// mirrors fork_center in the shader.
fn fork_center(origin_i: u32, fork_id: u32, t_f64: f64, fork_age: f32) -> [f32; 2] {
    let [px, py] = wave_center(origin_i, t_f64);
    let s  = fork_id as f64;
    let dx = (s * 7.7).sin() as f32;
    let dy = (s * 9.9).cos() as f32;
    let mag = (dx * dx + dy * dy).sqrt().max(1e-9);
    let (dx, dy) = (dx / mag, dy / mag);
    let max_dist = 2.5 + fhash(fork_id, 0, 30) * 3.0;
    let push     = max_dist * (1.0 - (-(fork_age + 0.1) * 0.15).exp());
    // Reduce wiggle arguments mod TAU in f64.
    let wx = (t_f64 * 0.41 + s).rem_euclid(std::f64::consts::TAU).sin() as f32 * 0.4;
    let wy = (t_f64 * 0.33 + s).rem_euclid(std::f64::consts::TAU).cos() as f32 * 0.4;
    [px + dx * push + wx, py + dy * push + wy]
}

#[inline(always)]
fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let tt = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    tt * tt * (3.0 - 2.0 * tt)
}

#[inline(always)]
fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Compute all pre-baked wave data for one frame.
///
/// `env`        — the 24-float env uniform (3 waves × 8 floats, same layout
///                as EnvUniform in the shader).
/// `t_epoch`    — integer period counter.
/// `t_residual` — fractional phase within the current period [0, 2π/freq_min).
/// `noise`      — background noise from seed.
///
/// Returns `[WaveData; 6]`: indices 0–2 are the 3 main waves, 3–5 are their
/// forks.  Ready to `bytemuck::bytes_of` and upload via `write_buffer`.
pub fn compute(env: &[f32; 24], t_epoch: i64, t_residual: f64, noise: f32) -> [WaveData; 6] {
    // f32 t — matches the shader's spatial computations (wave_center, shimmer).
    let t: f32 = t_epoch as f32 * (TAU / 0.1) + t_residual as f32;
    // f64 t — used for gn and phase_off so precision is maintained at any T.
    const PERIOD_F64: f64 = std::f64::consts::TAU / 0.1;
    let t_f64: f64 = t_epoch as f64 * PERIOD_F64 + t_residual;
    // gn wraps at 2^24 — the exact-integer limit of f32 — so fhash cycles
    // instead of freezing when T grows beyond ~266K epochs.
    const GN_WRAP: u64 = 1 << 24;

    let mut out = [WaveData::zeroed(); 6];

    // Universe heat index — stored in env[7] (wave-0 reserved slot) by make_env_data.
    // Cold (2.0) = intense blobs; hot (5.0) = large white balls; supernova when multiple peaks align.
    let heat_index = env[7];

    // Birth factor: ties radius expansion to wave-center expansion so the universe
    // emerges from a single point — both position AND size start minimal and grow together.
    // Saturates at 1.0 by t ≈ 100 T (~1.6 epochs), matching wave_center's exp factor.
    let birth_factor = (t_f64 * 0.01).clamp(0.0, 1.0) as f32;
    for i in 0..3usize {
        let base       = i * 8;
        let env_amp    = env[base];
        let env_freq   = env[base + 1];
        let env_phase  = env[base + 2];
        let drift_freq = env[base + 5];
        let drift_ph   = env[base + 6];
        let wi         = i as u32;

        // ── Main wave (index i) ───────────────────────────────────────────────
        // Compute acc in f64 — avoids u32::MAX saturation at extreme T.
        let acc_f64 = gen_acc_f64(drift_freq as f64, drift_ph as f64, t_f64, noise as f64);
        let gn   = (acc_f64 as u64 % GN_WRAP) as u32;
        let frac = acc_f64.fract() as f32;
        let blend = smoothstep(0.9, 1.0, frac);

        let amp_a   = env_amp  * (0.3 + memory(wi, gn,                  0) * 2.4);
        let amp_b   = env_amp  * (0.3 + memory(wi, gn.wrapping_add(1), 0) * 2.4);
        let freq_a  = env_freq * (0.4 + memory(wi, gn,                  1) * 1.2);
        let freq_b  = env_freq * (0.4 + memory(wi, gn.wrapping_add(1), 1) * 1.2);
        let ang_a   = memory(wi, gn,                  2) * TAU;
        let ang_b   = memory(wi, gn.wrapping_add(1), 2) * TAU;
        let shp_a   = memory(wi, gn,                  3);
        let shp_b   = memory(wi, gn.wrapping_add(1), 3);
        let wrp_a   = memory(wi, gn,                  4);
        let wrp_b   = memory(wi, gn.wrapping_add(1), 4);

        let amp_v  = lerp(amp_a, amp_b, blend);
        let freq_t = lerp(freq_a, freq_b, blend);
        let ang_t  = lerp(ang_a, ang_b, blend);

        let [vel_x, vel_y] = wave_velocity(wi, t_f64);
        let speed_t = (vel_x * vel_x + vel_y * vel_y).sqrt();
        let angle_v = vel_y.atan2(vel_x);

        let amp_t   = amp_v * (0.8 + 0.4 * smoothstep(0.1, 1.5, speed_t));
        let angle_t = ang_t * 0.4 + angle_v * 0.6;

        let energy  = wave_energy(wi, gn);
        let radius  = 1.5 + energy.clamp(0.0, 2.0) * heat_index * birth_factor;
        let [cx, cy] = wave_center(wi, t_f64);

        out[i] = WaveData {
            amp:       amp_t,
            freq:      freq_t,
            dir_x:     angle_t.cos(),
            dir_y:     angle_t.sin(),
            shape:     lerp(shp_a, shp_b, blend),
            warp:      lerp(wrp_a, wrp_b, blend),
            cx,
            cy,
            cos_v:     angle_v.cos(),
            sin_v:     angle_v.sin(),
            stretch:   1.0 + speed_t * 0.5,
            radius,
            phase_off: (-(freq_t as f64) * t_f64 + env_phase as f64).rem_euclid(std::f64::consts::TAU) as f32,
            origin_ph: env_phase,
            alive:    1.0,
            base_amp:  env_amp,
        };

        // ── Fork (index i+3) ─────────────────────────────────────────────────
        let fork_i  = i as u32;
        let fork_id = fork_i + 10;
        let origin_gn = (acc_f64 as u64 % 256) as u32;
        let gn_fork   = fork_split_at(fork_i) % 256;

        if origin_gn < gn_fork {
            // Fork hasn't split yet — emit a zeroed-out (inactive) WaveData.
            out[3 + i] = WaveData { alive: 0.0, base_amp: env_amp, ..WaveData::zeroed() };
            continue;
        }

        let fork_age   = (origin_gn - gn_fork) as f32 + frac;
        let fork_age_u = origin_gn - gn_fork;   // = u32(floor(fork_age))
        let fork_energy = wave_energy(fork_id, fork_age_u);

        // Velocity-aligned propulsion & deformation (mirrors shader).
        let [vel_px, vel_py] = wave_velocity(wi, t_f64);
        let s_f  = fork_id as f32;
        let dx   = (s_f * 7.7).sin();
        let dy   = (s_f * 9.9).cos();
        let mag  = (dx * dx + dy * dy).sqrt().max(1e-9);
        let (dx, dy) = (dx / mag, dy / mag);
        let dist_p = 2.5 + fhash(fork_id, 0, 30) * 3.0;
        let sep_vel = dist_p * 0.15 * (-(fork_age + 0.1) * 0.15).exp();
        let fvx  = vel_px + dx * sep_vel;
        let fvy  = vel_py + dy * sep_vel;
        let fspeed  = (fvx * fvx + fvy * fvy).sqrt();
        let fangle_v = fvy.atan2(fvx);

        // Fork parameter blends: mix(memory(origin, gn_fork, ch), gp(fork_id, floor(fork_age), ch), FORK_MUTATION)
        let mix_fk = |a: f32, b: f32| a * (1.0 - FORK_MUTATION) + b * FORK_MUTATION;

        let famp_a  = env_amp  * (0.3 + mix_fk(memory(wi, gn_fork,     0), gp(fork_id, fork_age_u,     0)) * 2.4);
        let famp_b  = env_amp  * (0.3 + mix_fk(memory(wi, gn_fork + 1, 0), gp(fork_id, fork_age_u + 1, 0)) * 2.4);
        let ffreq_a = env_freq * (0.4 + mix_fk(memory(wi, gn_fork,     1), gp(fork_id, fork_age_u,     1)) * 1.2);
        let ffreq_b = env_freq * (0.4 + mix_fk(memory(wi, gn_fork + 1, 1), gp(fork_id, fork_age_u + 1, 1)) * 1.2);
        let fang_a  = mix_fk(memory(wi, gn_fork,     2), gp(fork_id, fork_age_u,     2)) * TAU;
        let fang_b  = mix_fk(memory(wi, gn_fork + 1, 2), gp(fork_id, fork_age_u + 1, 2)) * TAU;
        let fshp_a  = mix_fk(memory(wi, gn_fork,     3), gp(fork_id, fork_age_u,     3));
        let fshp_b  = mix_fk(memory(wi, gn_fork + 1, 3), gp(fork_id, fork_age_u + 1, 3));
        let fwrp_a  = mix_fk(memory(wi, gn_fork,     4), gp(fork_id, fork_age_u,     4));
        let fwrp_b  = mix_fk(memory(wi, gn_fork + 1, 4), gp(fork_id, fork_age_u + 1, 4));

        let famp_v  = lerp(famp_a,  famp_b,  blend);
        let ffreq_t = lerp(ffreq_a, ffreq_b, blend);
        let fang_t  = lerp(fang_a,  fang_b,  blend);
        let famp_t  = famp_v * (0.8 + 0.4 * smoothstep(0.1, 1.5, fspeed)) * FORK_WEIGHT;
        let fangle_t = fang_t * 0.4 + fangle_v * 0.6;

        let fradius = 1.0 + fork_energy.clamp(0.0, 2.0) * (heat_index * 0.5);
        let [fcx, fcy] = fork_center(wi, fork_id, t_f64, fork_age);

        out[3 + i] = WaveData {
            amp:       famp_t,
            freq:      ffreq_t,
            dir_x:     fangle_t.cos(),
            dir_y:     fangle_t.sin(),
            shape:     lerp(fshp_a, fshp_b, blend),
            warp:      lerp(fwrp_a, fwrp_b, blend),
            cx:        fcx,
            cy:        fcy,
            cos_v:     fangle_v.cos(),
            sin_v:     fangle_v.sin(),
            stretch:   1.0 + fspeed * 0.5,
            radius:    fradius,
            phase_off: (-(ffreq_t as f64) * t_f64 + env_phase as f64).rem_euclid(std::f64::consts::TAU) as f32,
            origin_ph: env_phase,
            alive:    if fork_energy > 0.0 { 1.0 } else { 0.0 },
            base_amp:  env_amp,
        };
    }

    out
}
