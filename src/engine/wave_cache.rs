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

const FORK_MUTATION: f32 = 0.35;
const FORK_WEIGHT:   f32 = 0.6;

// ── WaveData ─────────────────────────────────────────────────────────────────
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WaveData {
    pub amp:       f32,
    pub freq:      f32,
    pub dir_x:     f32,
    pub dir_y:     f32,
    pub shape:     f32,
    pub warp:      f32,
    pub cx:        f32,
    pub cy:        f32,
    pub cos_v:     f32,
    pub sin_v:     f32,
    pub stretch:   f32,
    pub radius:    f32,
    pub phase_off: f32,
    pub origin_ph: f32,
    pub alive:     f32,
    pub base_amp:  f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

#[inline(always)]
fn fhash(wave_i: u32, gn: u32, ch: u32) -> f32 {
    let gn = gn % (1 << 24);
    let x = wave_i as f32 * 97.0 + gn as f32 * PHI + ch as f32 * 7.321;
    let v = x.sin() + (x * EUL).sin() * 0.5 + (x * PI).sin() * 0.25;
    (v * 1.5).sin() * 0.5 + 0.5
}

#[inline(always)]
fn gp(wave_i: u32, gn: u32, ch: u32) -> f32 { fhash(wave_i, gn, ch) }

/// Caches the three gp values that are shared across every channel call for a
/// given (wave_i, gn) pair: gp(_, gn, 0), gp(_, gn, 1), gp(_, gn, 8).
/// Without this, calling memory(wi, gn, ch) for ch in 0..5 would recompute
/// those three values five times each — 60 redundant fhash calls per wave.
struct MemoryCtx { power: f32 }

impl MemoryCtx {
    fn new(wave_i: u32, gn: u32) -> Self {
        let amp_n     = 0.3 + gp(wave_i, gn, 0) * 2.4;
        let freq_n    = 0.4 + gp(wave_i, gn, 1) * 1.2;
        let resonance = gp(wave_i, gn, 8);
        let cb    = 1.0 - ((resonance - RES_TARGET).abs() / RES_TARGET).clamp(0.0, 1.0);
        let power = (amp_n * freq_n * 0.4 * (0.5 + 0.5 * cb)).clamp(0.0, 1.0);
        Self { power }
    }

    #[inline(always)]
    fn memory(&self, wave_i: u32, gn: u32, ch: u32) -> f32 {
        let own   = gp(wave_i, gn, ch);
        let past  = gp(wave_i, gn.wrapping_sub(1), ch);
        let carry = gp(wave_i, gn, ch + 10) * 0.5 + self.power * 0.5;
        own * (1.0 - carry) + past * carry
    }
}

/// High-precision f64 version of gen_acc.
fn gen_acc_f64(drift_freq: f64, drift_phase: f64, t: f64, noise: f64) -> f64 {
    const PHI_F64: f64 = 1.618_033_988_749_895_f64;
    let base   = t * drift_freq * 2.0;
    let wiggle = noise * 0.5 * (
        (drift_freq * PHI_F64 * t + drift_phase).sin() +
        (drift_freq * std::f64::consts::PI * t + drift_phase * std::f64::consts::E).sin()
    );
    (base + wiggle).max(0.0)
}

fn wave_energy(wave_i: u32, gn: u32) -> f32 {
    let alpha   = 2.0 * PHI;
    let beta    = wave_i as f32 * PHI * PI;
    let n       = gn as f32;
    let cos_sum = ((n + 1.0) * alpha * 0.5).sin()
                * (n * alpha * 0.5 + beta).cos()
                / (alpha * 0.5).sin();
    1.0 - 0.5 * cos_sum
}

/// Computes wave center and velocity together, sharing arg_x/arg_y computation
/// and using sin_cos() to get both sin and cos of each argument in one call.
/// Previously two separate functions that each recomputed fx, fy, arg_x, arg_y.
fn wave_center_and_velocity(wave_i: u32, t_f64: f64) -> ([f32; 2], [f32; 2]) {
    let s  = wave_i as f64;
    let fx = 0.13 + s * 0.07;
    let fy = 0.11 + s * 0.03;
    let r  = 4.0 + s * 2.0;
    // sin_cos() computes both in one call — center needs sin_x/cos_y,
    // velocity needs cos_x/sin_y (the derivatives).
    let (sin_x, cos_x) = (t_f64 * fx + s * 1.3).rem_euclid(std::f64::consts::TAU).sin_cos();
    let (sin_y, cos_y) = (t_f64 * fy + s * 2.3).rem_euclid(std::f64::consts::TAU).sin_cos();
    let exp = (t_f64 * 0.01).clamp(0.0, 1.0) as f32;
    let r32 = r as f32;
    (
        [r32 * sin_x as f32 * exp, r32 * cos_y as f32 * exp],
        [(r * fx * cos_x) as f32, (r * -fy * sin_y) as f32],
    )
}

fn fork_split_at(fork_i: u32) -> u32 {
    (gp(fork_i, 0, 20) * 80.0) as u32 + 5
}

/// Computes fork center. Accepts precomputed origin center (px, py),
/// normalized fork direction (fork_dx, fork_dy), and push distance
/// (dist_p * (1 - exp_term)) — all hoisted to the caller to avoid
/// recomputing wave_center, the direction normalization, and the exp.
fn fork_center(
    px: f32, py: f32,
    fork_id: u32,
    fork_dx: f32, fork_dy: f32,
    t_f64: f64,
    push: f32,
) -> [f32; 2] {
    let s  = fork_id as f64;
    let wx = (t_f64 * 0.41 + s).rem_euclid(std::f64::consts::TAU).sin() as f32 * 0.4;
    let wy = (t_f64 * 0.33 + s).rem_euclid(std::f64::consts::TAU).cos() as f32 * 0.4;
    [px + fork_dx * push + wx, py + fork_dy * push + wy]
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

pub fn compute(env: &[f32; 24], t_epoch: i64, t_residual: f64, noise: f32) -> [WaveData; 6] {
    let _t: f32 = t_epoch as f32 * (TAU / 0.1) + t_residual as f32;
    const PERIOD_F64: f64 = std::f64::consts::TAU / 0.1;
    let t_f64: f64 = t_epoch as f64 * PERIOD_F64 + t_residual;
    const GN_WRAP: u64 = 1 << 24;

    let mut out = [WaveData::zeroed(); 6];

    let heat_index   = env[7];
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
        let acc_f64 = gen_acc_f64(drift_freq as f64, drift_ph as f64, t_f64, noise as f64);
        let gn      = (acc_f64 as u64 % GN_WRAP) as u32;
        let frac    = acc_f64.fract() as f32;
        let blend   = smoothstep(0.9, 1.0, frac);

        // MemoryCtx precomputes gp(wi, gn, 0/1/8) once, shared across all ch calls.
        let ctx_gn  = MemoryCtx::new(wi, gn);
        let ctx_gn1 = MemoryCtx::new(wi, gn.wrapping_add(1));

        let amp_a  = env_amp  * (0.3 + ctx_gn.memory(wi, gn,                  0) * 2.4);
        let amp_b  = env_amp  * (0.3 + ctx_gn1.memory(wi, gn.wrapping_add(1), 0) * 2.4);
        let freq_a = env_freq * (0.4 + ctx_gn.memory(wi, gn,                  1) * 1.2);
        let freq_b = env_freq * (0.4 + ctx_gn1.memory(wi, gn.wrapping_add(1), 1) * 1.2);
        let ang_a  = ctx_gn.memory(wi, gn,                  2) * TAU;
        let ang_b  = ctx_gn1.memory(wi, gn.wrapping_add(1), 2) * TAU;
        let shp_a  = ctx_gn.memory(wi, gn,                  3);
        let shp_b  = ctx_gn1.memory(wi, gn.wrapping_add(1), 3);
        let wrp_a  = ctx_gn.memory(wi, gn,                  4);
        let wrp_b  = ctx_gn1.memory(wi, gn.wrapping_add(1), 4);

        let amp_v  = lerp(amp_a, amp_b, blend);
        let freq_t = lerp(freq_a, freq_b, blend);
        let ang_t  = lerp(ang_a, ang_b, blend);

        // Merged: center and velocity share arg_x/arg_y computation and trig.
        let ([cx, cy], [vel_x, vel_y]) = wave_center_and_velocity(wi, t_f64);
        let speed_t = (vel_x * vel_x + vel_y * vel_y).sqrt();
        let angle_v = vel_y.atan2(vel_x);
        let amp_t   = amp_v * (0.8 + 0.4 * smoothstep(0.1, 1.5, speed_t));
        let angle_t = ang_t * 0.4 + angle_v * 0.6;

        let energy = wave_energy(wi, gn);
        let radius = 1.5 + energy.clamp(0.0, 2.0) * heat_index * birth_factor;

        // sin_cos() computes both in one instruction instead of two separate calls.
        let (sin_t, cos_t) = angle_t.sin_cos();
        let (sin_v, cos_v) = angle_v.sin_cos();

        out[i] = WaveData {
            amp:       amp_t,
            freq:      freq_t,
            dir_x:     cos_t,
            dir_y:     sin_t,
            shape:     lerp(shp_a, shp_b, blend),
            warp:      lerp(wrp_a, wrp_b, blend),
            cx,
            cy,
            cos_v,
            sin_v,
            stretch:   1.0 + speed_t * 0.5,
            radius,
            phase_off: (-(freq_t as f64) * t_f64 + env_phase as f64).rem_euclid(std::f64::consts::TAU) as f32,
            origin_ph: env_phase,
            alive:     1.0,
            base_amp:  env_amp,
        };

        // ── Fork (index i+3) ─────────────────────────────────────────────────
        let fork_i    = i as u32;
        let fork_id   = fork_i + 10;
        let origin_gn = (acc_f64 as u64 % 256) as u32;
        let gn_fork   = fork_split_at(fork_i) % 256;

        if origin_gn < gn_fork {
            out[3 + i] = WaveData { alive: 0.0, base_amp: env_amp, ..WaveData::zeroed() };
            continue;
        }

        let fork_age   = (origin_gn - gn_fork) as f32 + frac;
        let fork_age_u = origin_gn - gn_fork;
        let fork_energy = wave_energy(fork_id, fork_age_u);

        // Precompute normalized fork direction once — previously duplicated
        // between compute() and the interior of fork_center().
        let s_f  = fork_id as f32;
        let fdx  = (s_f * 7.7).sin();
        let fdy  = (s_f * 9.9).cos();
        let fmag = (fdx * fdx + fdy * fdy).sqrt().max(1e-9);
        let (fork_dx, fork_dy) = (fdx / fmag, fdy / fmag);

        // exp_term computed once — previously computed separately for sep_vel
        // (here) and for push (inside fork_center).
        let dist_p   = 2.5 + fhash(fork_id, 0, 30) * 3.0;
        let exp_term = (-(fork_age + 0.1) * 0.15).exp();
        let push     = dist_p * (1.0 - exp_term);
        let sep_vel  = dist_p * 0.15 * exp_term;

        let fvx      = vel_x + fork_dx * sep_vel;
        let fvy      = vel_y + fork_dy * sep_vel;
        let fspeed   = (fvx * fvx + fvy * fvy).sqrt();
        let fangle_v = fvy.atan2(fvx);

        let mix_fk = |a: f32, b: f32| a * (1.0 - FORK_MUTATION) + b * FORK_MUTATION;

        let ctx_fk  = MemoryCtx::new(wi, gn_fork);
        let ctx_fk1 = MemoryCtx::new(wi, gn_fork + 1);

        let famp_a  = env_amp  * (0.3 + mix_fk(ctx_fk.memory(wi,  gn_fork,     0), gp(fork_id, fork_age_u,     0)) * 2.4);
        let famp_b  = env_amp  * (0.3 + mix_fk(ctx_fk1.memory(wi, gn_fork + 1, 0), gp(fork_id, fork_age_u + 1, 0)) * 2.4);
        let ffreq_a = env_freq * (0.4 + mix_fk(ctx_fk.memory(wi,  gn_fork,     1), gp(fork_id, fork_age_u,     1)) * 1.2);
        let ffreq_b = env_freq * (0.4 + mix_fk(ctx_fk1.memory(wi, gn_fork + 1, 1), gp(fork_id, fork_age_u + 1, 1)) * 1.2);
        let fang_a  = mix_fk(ctx_fk.memory(wi,  gn_fork,     2), gp(fork_id, fork_age_u,     2)) * TAU;
        let fang_b  = mix_fk(ctx_fk1.memory(wi, gn_fork + 1, 2), gp(fork_id, fork_age_u + 1, 2)) * TAU;
        let fshp_a  = mix_fk(ctx_fk.memory(wi,  gn_fork,     3), gp(fork_id, fork_age_u,     3));
        let fshp_b  = mix_fk(ctx_fk1.memory(wi, gn_fork + 1, 3), gp(fork_id, fork_age_u + 1, 3));
        let fwrp_a  = mix_fk(ctx_fk.memory(wi,  gn_fork,     4), gp(fork_id, fork_age_u,     4));
        let fwrp_b  = mix_fk(ctx_fk1.memory(wi, gn_fork + 1, 4), gp(fork_id, fork_age_u + 1, 4));

        let famp_v   = lerp(famp_a,  famp_b,  blend);
        let ffreq_t  = lerp(ffreq_a, ffreq_b, blend);
        let fang_t   = lerp(fang_a,  fang_b,  blend);
        let famp_t   = famp_v * (0.8 + 0.4 * smoothstep(0.1, 1.5, fspeed)) * FORK_WEIGHT;
        let fangle_t = fang_t * 0.4 + fangle_v * 0.6;

        let fradius = 1.0 + fork_energy.clamp(0.0, 2.0) * (heat_index * 0.5);
        // fork_center receives the already-computed center, direction, and push.
        let [fcx, fcy] = fork_center(cx, cy, fork_id, fork_dx, fork_dy, t_f64, push);

        let (fsin_t, fcos_t) = fangle_t.sin_cos();
        let (fsin_v, fcos_v) = fangle_v.sin_cos();

        out[3 + i] = WaveData {
            amp:       famp_t,
            freq:      ffreq_t,
            dir_x:     fcos_t,
            dir_y:     fsin_t,
            shape:     lerp(fshp_a, fshp_b, blend),
            warp:      lerp(fwrp_a, fwrp_b, blend),
            cx:        fcx,
            cy:        fcy,
            cos_v:     fcos_v,
            sin_v:     fsin_v,
            stretch:   1.0 + fspeed * 0.5,
            radius:    fradius,
            phase_off: (-(ffreq_t as f64) * t_f64 + env_phase as f64).rem_euclid(std::f64::consts::TAU) as f32,
            origin_ph: env_phase,
            alive:     if fork_energy > 0.0 { 1.0 } else { 0.0 },
            base_amp:  env_amp,
        };
    }

    out
}
