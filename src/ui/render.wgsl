const PI:  f32 = 3.14159265;
const PHI: f32 = 1.61803399;
const TAU: f32 = 6.28318531;

struct SimUniform { 
    tick: f32, 
    noise: f32, 
    t_epoch: f32, 
    pan_x: f32, 
    pan_y: f32, 
    _pad1: f32, 
    _pad2: f32, 
    _pad3: f32 
}
@group(0) @binding(0) var<uniform> sim: SimUniform;

// binding 1 (env_buf) is kept in the bind group for backward compatibility
// but no longer read by this shader — all env math moved to wave_cache on CPU.

struct WaveColors { c: array<vec4<f32>, 3> }
@group(0) @binding(2) var<uniform> wave_colors: WaveColors;

// ── Wave Cache (binding 3) ────────────────────────────────────────────────────
// Pre-baked per-frame results of fhash / memory / gen_acc / wave_energy /
// wave_center / wave_velocity / fork_center.
// Computed once on the CPU in engine/wave_cache.rs, uploaded before the field
// pass.  This reduces per-pixel trig from ~1 200 calls to ~30.
// Indices 0–2 = main waves, 3–5 = their forks.
struct WaveData {
    amp:       f32,   // effective amplitude (kinetic pulse applied; forks ×FORK_WEIGHT)
    freq:      f32,   // final frequency
    dir_x:     f32,   // cos(angle_t)
    dir_y:     f32,   // sin(angle_t)
    shape:     f32,   // sine / abs-sine blend weight
    warp:      f32,   // spatial warp multiplier
    cx:        f32,   // Gaussian envelope centre x
    cy:        f32,   // Gaussian envelope centre y
    cos_v:     f32,   // cos(velocity angle) for ellipsoid rotation
    sin_v:     f32,   // sin(velocity angle)
    stretch:   f32,   // 1 + speed×0.5
    radius:    f32,   // Gaussian envelope radius
    phase_off: f32,   // precomputed −freq×t + origin_phase
    origin_ph: f32,   // origin wave phase (for fork spatial warp)
    alive:     f32,   // 1.0 = alive, 0.0 = inactive (fork not yet split)
    base_amp:  f32,   // env.waves[origin].amp for per-branch colour ratio
}
struct WaveCache { waves: array<WaveData, 6> }
@group(0) @binding(3) var<uniform> wc: WaveCache;

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

// ── Wave sample ───────────────────────────────────────────────────────────────
// sin_term = amp × sin(phase_arg)         — field contribution
// cos_freq = amp × freq × cos(phase_arg)  — derivative kernel:
//   dfield/dT  = −∑(cos_freq)
//   dfield/dx  =  ∑(cos_freq × dir.x)
//   dfield/dy  =  ∑(cos_freq × dir.y)
struct WaveSample { sin_term: f32, cos_freq: f32, dir: vec2<f32>, freq: f32 }

// Per-pixel work for a main wave: ~4 trig + 1 exp (vs ~200 trig before).
fn sample_wave(i: u32, pos: vec2<f32>, t: f32) -> WaveSample {
    let w   = wc.waves[i];
    var out: WaveSample;
    out.sin_term = 0.0; out.cos_freq = 0.0;
    out.dir = vec2(w.dir_x, w.dir_y); out.freq = w.freq;
    if w.alive < 0.5 { return out; }

    let dir       = vec2(w.dir_x, w.dir_y);
    let phase_arg = w.freq * dot(dir, pos) + w.phase_off;
    let s         = sin(phase_arg);
    let osc       = mix(s, abs(s) * 2.0 - 1.0, w.shape);
    // Shimmer: surface texture — 2 trig per wave, genuinely pos-dependent.
    let shimmer   = cos(pos.x * 2.0 + t) * sin(pos.y * 1.5 - t * 0.5);
    let wave_val  = 1.0 + 0.3 * osc + w.warp * 0.2 * shimmer;

    let rel    = pos - vec2(w.cx, w.cy);
    let p_long =  rel.x * w.cos_v + rel.y * w.sin_v;
    let p_lat  = -rel.x * w.sin_v + rel.y * w.cos_v;
    let d_sq   = (p_long * p_long) / (w.stretch * w.stretch)
               + (p_lat  * p_lat)  * (w.stretch * w.stretch);
    let env    = exp(-d_sq / (w.radius * w.radius));

    out.sin_term = w.amp * wave_val * env;
    out.cos_freq = w.amp * w.freq * cos(phase_arg) * env;
    out.dir      = dir;
    out.freq     = w.freq;
    return out;
}

// Per-pixel work for a fork: ~6 trig + 1 exp (vs ~200 trig before).
fn sample_fork(i: u32, pos: vec2<f32>, t: f32) -> WaveSample {
    let w   = wc.waves[3u + i];
    var out: WaveSample;
    out.sin_term = 0.0; out.cos_freq = 0.0;
    out.dir = vec2(w.dir_x, w.dir_y); out.freq = w.freq;
    if w.alive < 0.5 { return out; }

    let dir = vec2(w.dir_x, w.dir_y);
    // Spatial warp: 2 trig per fork, genuinely pos-dependent.
    let warped_pos = pos + vec2(
        sin(pos.y * 0.15 + w.origin_ph) * 2.5,
        cos(pos.x * 0.15 - w.origin_ph) * 2.5,
    );
    let phase_arg = w.freq * dot(dir, warped_pos) + w.phase_off;
    let s         = sin(phase_arg);
    let osc       = mix(s, abs(s) * 2.0 - 1.0, w.shape);
    // Fork texture: 2 trig per fork, genuinely pos-dependent.
    let texture   = cos(pos.x * 1.5 + pos.y * 0.5) * sin(length(pos) * 2.0 + t);
    let wave_val  = 1.0 + 0.3 * osc + w.warp * 0.2 * texture;

    let rel    = pos - vec2(w.cx, w.cy);
    let p_long =  rel.x * w.cos_v + rel.y * w.sin_v;
    let p_lat  = -rel.x * w.sin_v + rel.y * w.cos_v;
    let d_sq   = (p_long * p_long) / (w.stretch * w.stretch)
               + (p_lat  * p_lat)  * (w.stretch * w.stretch);
    let env    = exp(-d_sq / (w.radius * w.radius));

    // FORK_WEIGHT (0.6) already baked into w.amp by wave_cache.rs.
    out.sin_term = w.amp * wave_val * env;
    out.cos_freq = w.amp * w.freq * cos(phase_arg) * env;
    out.dir      = dir;
    out.freq     = w.freq;
    return out;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let t = sim.t_epoch * (TAU / 0.1) + sim.tick;

    // Causal horizon: wave phase velocity = 1, so the field at position x
    // is zero until t = |x|. The universe grows because physics says so.
    let camera_span = 12.5;
    let pos         = (in.uv * 2.0 - vec2(1.0)) * camera_span + vec2(sim.pan_x, sim.pan_y);

    let wave0 = sample_wave(0u, pos, t);
    let wave1 = sample_wave(1u, pos, t);
    let wave2 = sample_wave(2u, pos, t);

    // forks — each branch forks once at a hash-determined generation
    let f0 = sample_fork(0u, pos, t);
    let f1 = sample_fork(1u, pos, t);
    let f2 = sample_fork(2u, pos, t);

    let amp_sum = wc.waves[0].base_amp + wc.waves[1].base_amp + wc.waves[2].base_amp;

    // field value — branches + forks
    let field = (wave0.sin_term + wave1.sin_term + wave2.sin_term
               + f0.sin_term + f1.sin_term + f2.sin_term) / amp_sum;

    // dfield/dT — positive = rising toward a crest, negative = falling away
    let dfield_dt = -(wave0.cos_freq + wave1.cos_freq + wave2.cos_freq
                    + f0.cos_freq + f1.cos_freq + f2.cos_freq) / amp_sum;

    // gradient field — points toward nearest crest in space
    let gradient = vec2(
        (wave0.cos_freq * wave0.dir.x + wave1.cos_freq * wave1.dir.x + wave2.cos_freq * wave2.dir.x
       + f0.cos_freq * f0.dir.x + f1.cos_freq * f1.dir.x + f2.cos_freq * f2.dir.x) / amp_sum,
        (wave0.cos_freq * wave0.dir.y + wave1.cos_freq * wave1.dir.y + wave2.cos_freq * wave2.dir.y
       + f0.cos_freq * f0.dir.y + f1.cos_freq * f1.dir.y + f2.cos_freq * f2.dir.y) / amp_sum,
    );

    let signal = pow(max(field, 0.0), 3.0);

    // branch color — forks share their origin's color (same branch, diverged parameters)
    let r0   = max(wave0.sin_term + f0.sin_term, 0.0) / (wc.waves[0].base_amp + 1e-5);
    let r1   = max(wave1.sin_term + f1.sin_term, 0.0) / (wc.waves[1].base_amp + 1e-5);
    let r2   = max(wave2.sin_term + f2.sin_term, 0.0) / (wc.waves[2].base_amp + 1e-5);
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

    // gradient magnitude lights the edges of creatures
    let edge = clamp(length(gradient) * 0.15, 0.0, 0.4);
    let lit  = clamp(tinted_col + vec3(edge), vec3(0.0), vec3(1.0));

    let rgb = mix(lit, vec3(1.0), signal * signal) * signal;
    return vec4(pow(clamp(rgb, vec3(0.0), vec3(1.0)), vec3(2.2)), 1.0);
}
