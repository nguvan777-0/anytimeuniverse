const PI:  f32 = 3.14159265;
const PHI: f32 = 1.6180339887;
const EUL: f32 = 2.7182818284;
const TAU: f32 = 6.2831853071;

struct SimUniform { tick: f32, noise: f32, _pad1: f32, _pad2: f32 }
@group(0) @binding(0) var<uniform> sim: SimUniform;

struct EnvWave {
    amp:         f32,
    freq:        f32,
    phase:       f32,
    dir_x:       f32,
    dir_y:       f32,
    drift_freq:  f32,
    drift_phase: f32,
    _p2:         f32,
}
struct EnvUniform { waves: array<EnvWave, 3> }
@group(0) @binding(1) var<uniform> env: EnvUniform;

struct WaveColors { c: array<vec4<f32>, 3> }
@group(0) @binding(2) var<uniform> wave_colors: WaveColors;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0)       uv:  vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VOut {
    var positions = array<vec2<f32>, 4>(
        vec2(-1.0,  1.0), vec2(-1.0, -1.0),
        vec2( 1.0,  1.0), vec2( 1.0, -1.0),
    );
    var uvs = array<vec2<f32>, 4>(
        vec2(0.0, 0.0), vec2(0.0, 1.0),
        vec2(1.0, 0.0), vec2(1.0, 1.0),
    );
    var out: VOut;
    out.pos = vec4(positions[vi], 0.0, 1.0);
    out.uv  = uvs[vi];
    return out;
}

// ── Hash ─────────────────────────────────────────────────────────────────────
fn uhash(x: u32) -> u32 {
    var v = x ^ (x >> 17u);
    v  = v * 0xbf324c81u;
    v ^= v >> 13u;
    v  = v * 0x9a813f77u;
    v ^= v >> 16u;
    return v;
}

fn gen_param(wave_i: u32, gen: u32, ch: u32) -> f32 {
    return f32(uhash(wave_i * 97u + gen * 1031u + ch * 7u)) / 4294967295.0;
}

// ── Hardware bit count (Population Count) ────────────────────────────────────
fn gen_bits(wave_i: u32, gn: u32) -> f32 {
    let h = uhash(wave_i * 97u + gn * 1031u + 8u * 7u); // Channel 8 reserved for raw genome
    return f32(countOneBits(h));
}

// ── Energy ───────────────────────────────────────────────────────────────────
// cumulative power across all generations up to gn, in O(1).
// power per generation = amp × freq × 0.4 — instantaneous output.
// energy = Σ(power - threshold) — positive means active, negative means zeroed.
// uses closed-form cosine sum (Dirichlet kernel) — no loop needed.
const ENERGY_THRESHOLD: f32 = 0.51;
fn wave_energy(wave_i: u32, gn: u32) -> f32 {
    // alpha irrational → cos_sum never repeats
    // beta uses PHI offset per lineage → each lineage has a unique trajectory
    let alpha   = 2.0 * PHI;
    let beta    = f32(wave_i) * PHI * PI;
    let N       = f32(gn);
    let cos_sum = sin((N + 1.0) * alpha * 0.5) * cos(N * alpha * 0.5 + beta) / sin(alpha * 0.5);
    // +1.0 birth budget: all lineages start active (energy ≥ 0.49 at gn=0).
    // drift = -0.01/gen; cos_sum bounded ≈ ±1; zeroed permanently after ~100 gens.
    return 1.0 + (N + 1.0) * (0.5 - ENERGY_THRESHOLD) - 0.5 * cos_sum;
}

// ── Memory ───────────────────────────────────────────────────────────────────
// parameters at generation N are a weighted blend of hash(N) and hash(N-1) — copying.
// carry weight = mix(random, power) — high-power generations copy more of the parent.
// high power → high carry → stable, keeps what worked.
// low power  → low carry  → drifts further from parent.
fn memory(wave_i: u32, gn: u32, ch: u32) -> f32 {
    let own  = gen_param(wave_i, gn,      ch);
    let past = gen_param(wave_i, gn - 1u, ch);

    // power: amp × freq × 0.4 — instantaneous output of this generation.
    let amp_n  = 0.3 + gen_param(wave_i, gn, 0u) * 2.4;
    let freq_n = 0.4 + gen_param(wave_i, gn, 1u) * 1.2;

    // bit complexity (popcnt) — favors sequences near 16 set bits (50%)
    let bits = gen_bits(wave_i, gn);
    let complexity_bonus = 1.0 - (abs(bits - 16.0) / 16.0);

    let power = clamp(amp_n * freq_n * 0.4 * (0.5 + 0.5 * complexity_bonus), 0.0, 1.0);

    let carry = mix(gen_param(wave_i, gn, ch + 10u), power, 0.5);
    return mix(own, past, carry);
}

// ── Accumulator ──────────────────────────────────────────────────────────────
fn gen_acc(drift_freq: f32, drift_phase: f32, t: f32, noise: f32) -> f32 {
    let base   = t * drift_freq * 2.0;
    let wiggle = noise * 0.5 * (
        sin(drift_freq * PHI * t + drift_phase) +
        sin(drift_freq * PI  * t + drift_phase * EUL)
    );
    return max(base + wiggle, 0.0);
}

// ── Math VM (Structural Genetics) ────────────────────────────────────────────
fn evaluate_dna_ops(dna: u32, phase_arg: f32, pos: vec2<f32>, freq: f32) -> f32 {
    let op_wave_shape = dna & 3u;
    let op_space_warp = (dna >> 2u) & 3u;
    let op_invert     = (dna >> 4u) & 1u;

    // 1. Shape Evaluation (Branchless)
    let is_sh_0 = f32(op_wave_shape == 0u);
    let is_sh_1 = f32(op_wave_shape == 1u);
    let is_sh_2 = f32(op_wave_shape == 2u);
    let is_sh_3 = f32(op_wave_shape == 3u);

    let sh_0 = sin(phase_arg);
    let sh_1 = abs(sin(phase_arg)) * 2.0 - 1.0;
    let sh_2 = fract(phase_arg / TAU) * 2.0 - 1.0;
    let sh_3 = smoothstep(-1.0, 1.0, sin(phase_arg)) * 2.0 - 1.0;

    var wave_val = sh_0 * is_sh_0 
                 + sh_1 * is_sh_1 
                 + sh_2 * is_sh_2 
                 + sh_3 * is_sh_3;

    // 2. Spatial Warp (Branchless)
    let is_w_0 = f32(op_space_warp == 0u);
    let is_w_1 = f32(op_space_warp == 1u);
    let is_w_2 = f32(op_space_warp == 2u);
    let is_w_3 = f32(op_space_warp == 3u);

    // Some visually interesting multipliers to warp the sine-field
    let w_1 = cos(pos.x * 2.0);
    let w_2 = sin(pos.y * 2.0);
    let w_3 = sin(length(pos) * 5.0);

    wave_val *= is_w_0 * 1.0 
              + is_w_1 * w_1 
              + is_w_2 * w_2 
              + is_w_3 * w_3;

    // 3. Inversion
    if op_invert == 1u {
        wave_val = -wave_val;
    }

    return wave_val;
}

// ── Fork ─────────────────────────────────────────────────────────────────────
// Each parent wave forks once — a child wave born at a specific parent generation.
// Child inherits parent parameters at that moment, blended with its own hash.
// Child lives and dies on its own health trajectory from birth.
// All O(1): fork generation is a hash, not a simulation result.
// Each pixel independently pulls its fork contribution — pure pull model.

const FORK_MUTATION: f32 = 0.35; // how far child drifts from parent (0=clone, 1=random)
const FORK_WEIGHT:   f32 = 0.6;  // fork contribution relative to parent

fn fork_born_at(fork_i: u32) -> u32 {
    // generation of parent at which fork occurs — unique per fork, range [5, 85)
    return u32(gen_param(fork_i, 0u, 20u) * 80.0) + 5u;
}

fn sample_fork(parent: EnvWave, parent_i: u32, fork_i: u32, pos: vec2<f32>, t: f32, noise: f32) -> WaveSample {
    var out: WaveSample;
    out.sin_term = 0.0; out.cos_freq = 0.0;
    out.dir = vec2(1.0, 0.0); out.freq = 1.0;

    let acc       = gen_acc(parent.drift_freq, parent.drift_phase, t, noise);
    let parent_gn = u32(floor(acc));
    let gn_fork   = fork_born_at(fork_i);
    if parent_gn < gn_fork { return out; }  // not born yet

    let fork_age  = parent_gn - gn_fork;
    let child_id  = fork_i + 10u;           // offset to avoid colliding with parent hash space
    let here      = select(0.0, 1.0, wave_energy(child_id, fork_age) > 0.0);
    if here == 0.0 { return out; }

    let frac = fract(acc);

    // parameters: mix of parent-at-fork-generation and child's own hash
    // FORK_MUTATION controls how far the child drifts from the parent
    let amp_a  = parent.amp  * (0.3 + mix(memory(parent_i, gn_fork,       0u), gen_param(child_id, fork_age,       0u), FORK_MUTATION) * 2.4);
    let freq_a = parent.freq * (0.4 + mix(memory(parent_i, gn_fork,       1u), gen_param(child_id, fork_age,       1u), FORK_MUTATION) * 1.2);
    let ang_a  =                      mix(memory(parent_i, gn_fork,       2u), gen_param(child_id, fork_age,       2u), FORK_MUTATION) * TAU;
    let shp_a  =                      mix(memory(parent_i, gn_fork,       3u), gen_param(child_id, fork_age,       3u), FORK_MUTATION);
    let wrp_a  =                      mix(memory(parent_i, gn_fork,       4u), gen_param(child_id, fork_age,       4u), FORK_MUTATION);

    let amp_b  = parent.amp  * (0.3 + mix(memory(parent_i, gn_fork + 1u,  0u), gen_param(child_id, fork_age + 1u,  0u), FORK_MUTATION) * 2.4);
    let freq_b = parent.freq * (0.4 + mix(memory(parent_i, gn_fork + 1u,  1u), gen_param(child_id, fork_age + 1u,  1u), FORK_MUTATION) * 1.2);
    let ang_b  =                      mix(memory(parent_i, gn_fork + 1u,  2u), gen_param(child_id, fork_age + 1u,  2u), FORK_MUTATION) * TAU;
    let shp_b  =                      mix(memory(parent_i, gn_fork + 1u,  3u), gen_param(child_id, fork_age + 1u,  3u), FORK_MUTATION);
    let wrp_b  =                      mix(memory(parent_i, gn_fork + 1u,  4u), gen_param(child_id, fork_age + 1u,  4u), FORK_MUTATION);

    let blend     = smoothstep(0.9, 1.0, frac);
    let amp_t     = mix(amp_a,  amp_b,  blend);
    let freq_t    = mix(freq_a, freq_b, blend);
    let angle_t   = mix(ang_a,  ang_b,  blend);
    let shape_t   = mix(shp_a,  shp_b,  blend);
    let warpx_t   = mix(wrp_a,  wrp_b,  blend);

    let dir_t     = vec2(cos(angle_t), sin(angle_t));
    let phase_arg = freq_t * dot(dir_t, pos) - freq_t * t + parent.phase;

    // Direct mathematical morphing using float memory fields!
    // Shape 0.0 -> smooth sine | Shape 1.0 -> absolute saw-like peaks
    let osc      = mix(sin(phase_arg), abs(sin(phase_arg)) * 2.0 - 1.0, shape_t);
    
    // Spatial warp 0.0 -> uniform field | Space warp 1.0 -> lattice/pulse modulators
    let modifier = mix(1.0, cos(pos.x * 2.0) * sin(length(pos) * 3.0), warpx_t);

    let wave_val = osc * modifier;

    out.sin_term = amp_t * wave_val * here * FORK_WEIGHT;
    out.cos_freq = amp_t * freq_t * cos(phase_arg) * here * FORK_WEIGHT;
    out.dir      = dir_t;
    out.freq     = freq_t;
    return out;
}

// ── Wave sample ───────────────────────────────────────────────────────────────
// sin_term = amp * sin(phase_arg)        — field contribution
// cos_freq = amp * freq * cos(phase_arg) — derivative kernel:
//   dfield/dT  = -sum(cos_freq)
//   dfield/dx  =  sum(cos_freq * dir.x)
//   dfield/dy  =  sum(cos_freq * dir.y)
struct WaveSample { sin_term: f32, cos_freq: f32, dir: vec2<f32>, freq: f32 }

fn sample_wave(w: EnvWave, wave_i: u32, pos: vec2<f32>, t: f32, noise: f32) -> WaveSample {
    let acc  = gen_acc(w.drift_freq, w.drift_phase, t, noise);
    let gn   = u32(floor(acc));
    let frac = fract(acc);

    let amp_a   = w.amp  * (0.3 + memory(wave_i, gn,      0u) * 2.4);
    let freq_a  = w.freq * (0.4 + memory(wave_i, gn,      1u) * 1.2);
    let angle_a =                  memory(wave_i, gn,      2u) * TAU;
    let shp_a   =                  memory(wave_i, gn,      3u);
    let wrp_a   =                  memory(wave_i, gn,      4u);

    let amp_b   = w.amp  * (0.3 + memory(wave_i, gn + 1u, 0u) * 2.4);
    let freq_b  = w.freq * (0.4 + memory(wave_i, gn + 1u, 1u) * 1.2);
    let angle_b =                  memory(wave_i, gn + 1u, 2u) * TAU;
    let shp_b   =                  memory(wave_i, gn + 1u, 3u);
    let wrp_b   =                  memory(wave_i, gn + 1u, 4u);

    let blend   = smoothstep(0.9, 1.0, frac);
    let amp_t   = mix(amp_a,   amp_b,   blend);
    let freq_t  = mix(freq_a,  freq_b,  blend);
    let angle_t = mix(angle_a, angle_b, blend);
    let shape_t = mix(shp_a,   shp_b,   blend);
    let warpx_t = mix(wrp_a,   wrp_b,   blend);

    let dir_t     = vec2(cos(angle_t), sin(angle_t));
    let phase_arg = freq_t * dot(dir_t, pos) - freq_t * t + w.phase;

    let here   = select(0.0, 1.0, wave_energy(wave_i, gn) > 0.0);

    let osc      = mix(sin(phase_arg), abs(sin(phase_arg)) * 2.0 - 1.0, shape_t);
    let modifier = mix(1.0, cos(pos.x * 2.0) * sin(length(pos) * 3.0), warpx_t);

    let wave_val = osc * modifier;

    var out: WaveSample;
    out.sin_term = amp_t * wave_val * here;
    out.cos_freq = amp_t * freq_t * cos(phase_arg) * here;
    out.dir      = dir_t;
    out.freq     = freq_t;
    return out;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let pos   = (in.uv * 2.0 - vec2(1.0)) * 6.0;
    let t     = sim.tick;
    let noise = sim.noise;

    let w0 = sample_wave(env.waves[0], 0u, pos, t, noise);
    let w1 = sample_wave(env.waves[1], 1u, pos, t, noise);
    let w2 = sample_wave(env.waves[2], 2u, pos, t, noise);

    // forks — each parent spawns one child wave, born at a hash-determined generation
    let f0 = sample_fork(env.waves[0], 0u, 0u, pos, t, noise);
    let f1 = sample_fork(env.waves[1], 1u, 1u, pos, t, noise);
    let f2 = sample_fork(env.waves[2], 2u, 2u, pos, t, noise);

    let amp_sum = env.waves[0].amp + env.waves[1].amp + env.waves[2].amp;

    // field value — parents + forks
    let field = (w0.sin_term + w1.sin_term + w2.sin_term
               + f0.sin_term + f1.sin_term + f2.sin_term) / amp_sum;

    // dfield/dT — positive = rising toward a crest, negative = falling away
    let dfield_dt = -(w0.cos_freq + w1.cos_freq + w2.cos_freq
                    + f0.cos_freq + f1.cos_freq + f2.cos_freq) / amp_sum;

    // grad field — points toward nearest crest in space
    let grad = vec2(
        (w0.cos_freq * w0.dir.x + w1.cos_freq * w1.dir.x + w2.cos_freq * w2.dir.x
       + f0.cos_freq * f0.dir.x + f1.cos_freq * f1.dir.x + f2.cos_freq * f2.dir.x) / amp_sum,
        (w0.cos_freq * w0.dir.y + w1.cos_freq * w1.dir.y + w2.cos_freq * w2.dir.y
       + f0.cos_freq * f0.dir.y + f1.cos_freq * f1.dir.y + f2.cos_freq * f2.dir.y) / amp_sum,
    );

    let signal = pow(max(field, 0.0), 3.0);

    // lineage color — forks inherit parent color (same lineage, diverged parameters)
    let r0   = max(w0.sin_term + f0.sin_term, 0.0) / (env.waves[0].amp + 1e-5);
    let r1   = max(w1.sin_term + f1.sin_term, 0.0) / (env.waves[1].amp + 1e-5);
    let r2   = max(w2.sin_term + f2.sin_term, 0.0) / (env.waves[2].amp + 1e-5);
    let rsum = r0 + r1 + r2 + 1e-5;
    let species_col = (r0 / rsum) * wave_colors.c[0].xyz
                    + (r1 / rsum) * wave_colors.c[1].xyz
                    + (r2 / rsum) * wave_colors.c[2].xyz;

    // dfield/dT tints the color — rising regions cool, falling regions warm.
    // the leading edge of a moving creature is a different temperature than its trailing edge.
    let time_tint  = clamp(dfield_dt * 0.4, -1.0, 1.0);
    let tinted_col = species_col
        + select(vec3( 0.15, -0.05, -0.15),   // warm (falling)
                 vec3(-0.10,  0.05,  0.20),    // cool (rising)
                 time_tint > 0.0) * abs(time_tint);

    // grad magnitude lights the edges of creatures
    let edge = clamp(length(grad) * 0.15, 0.0, 0.4);
    let lit  = clamp(tinted_col + vec3(edge), vec3(0.0), vec3(1.0));

    let rgb = mix(lit, vec3(1.0), signal * signal) * signal;
    return vec4(pow(clamp(rgb, vec3(0.0), vec3(1.0)), vec3(2.2)), 1.0);
}
