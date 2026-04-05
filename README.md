# anytimeuniverse

A cross-platform simulation in Rust where time is the direct input. Step to any point in time — past or future — and the field evaluates in O(1). Every value at every step is derived from `uhash(wave_i × 97 + gn × 1031 + ch × 7)` and the accumulator `gen_acc(t)`.

Three wave branches run natural selection across that field. Each accumulates energy, copies parameters forward with drift, or phases out. You can watch the visualization forward, rewind, or jump to any moment.

![screenshot](https://github.com/nguvan777-0/anytimeuniverse/releases/download/screenshots/anytimeuniverse-vyDqV9Ck.png)

```
cargo run                        # GPU window, real-time field
cargo run -- --headless          # ASCII terminal, interactive
cargo run -- --tape              # scrolling data log, exits on last signal
cargo run -- --help              # full options
```

---

## you don't need before

Each wave contributes a periodic signal to the field. When all waves use the default sine shape, the field is a Fourier sum over a 2D spatial field:

$$F(x, y, t) = \frac{1}{A} \sum_{i}\ \mathbf{1}[E_i > 0] \cdot A_i \sin\!\left(\omega_i\, \mathbf{d}_i \cdot \mathbf{x} - \omega_i t + \phi_i\right)$$

In practice each wave also has a shape channel (sine, absolute-value-sine, sawtooth, smoothstep-sine) and a spatial warp, so the field is a sum of periodic functions — Fourier in structure, not always in strict form. The sinusoidal carrier is what matters for the derivatives: because the dominant term is $\sin(\cdot)$, the time derivative and spatial gradient are exact with no finite differences:

$$\frac{\partial F}{\partial t} = -\frac{1}{A}\sum_i A_i\omega_i \cos(\cdot) \qquad \nabla_{\mathbf{x}} F = \frac{1}{A}\sum_i A_i\omega_i \cos(\cdot)\,\mathbf{d}_i$$

The same sinusoidal structure makes cumulative energy across generations computable in O(1) via the Dirichlet kernel — the closed form for a partial sum of cosines:

$$E(i, N) = 1 + (N+1)(0.5 - \tau) - \frac{1}{2} \cdot \frac{\sin\!\left(\frac{(N+1)\alpha}{2}\right)\cos\!\left(\frac{N\alpha}{2} + \beta_i\right)}{\sin\!\left(\frac{\alpha}{2}\right)}$$

where $\alpha = 2\varphi$, $\beta_i = i\varphi\pi$, $\tau = 0.51$, and $\varphi$ is the golden ratio. Using $\alpha = 2\varphi$ makes the trajectory quasiperiodic — it never repeats, and each wave gets a unique energy path through the same generation space.

The Dirichlet kernel is normally associated with the ringing you get when you truncate a Fourier series. Here it plays a different role: it tells you whether a wave has accumulated enough fitness to stay active at any generation $N$, without summing $N$ terms. That's what makes $t$ a direct input rather than a clock — there is no loop over history, only a formula evaluated at a point.

Natural selection enters through the carry blend. A generation's parameters are a linear interpolation between its own hash and the previous generation's, weighted by carry:

$$p(i,n,c) = (1 - \text{carry})\cdot\text{hash}(i,n,c) + \text{carry}\cdot\text{hash}(i,n-1,c)$$

$$\text{carry}(i,n) = \frac{1}{2}\,\text{hash}(i,n,c{+}10) + \frac{1}{2}\,\text{power}(i,n)$$

High-energy generations carry more of their parameters forward. This is selection without a selector — it falls out of the blend formula.

---

## How it works

Each wave has a set of parameters, an energy budget and a fork — a branch that splits off at a hash-determined generation and runs its own energy trajectory from that point. The GPU evaluates every pixel independently and in parallel, sampling and summing active waves and their forks at `(x, y, step)`.

## Parameters from the hash

Every parameter of every generation of every wave comes from a single hash:

```glsl
fn gen_param(wave_i: u32, gn: u32, ch: u32) -> f32 {
    return f32(uhash(wave_i * 97u + gn * 1031u + ch * 7u)) / 4294967295.0;
}
```

`wave_i` is the wave index, `gn` is the generation, `ch` is the channel — amplitude, frequency, direction angle, wave shape, spatial warp and carry weights.

## Step → generation

The step counter maps to a generation number via the accumulator:

```glsl
fn gen_acc(drift_freq: f32, drift_phase: f32, t: f32, noise: f32) -> f32 {
    let base   = t * drift_freq * 2.0;
    let wiggle = noise * 0.5 * (
        sin(drift_freq * PHI * t + drift_phase) +
        sin(drift_freq * PI  * t + drift_phase * EUL)
    );
    return max(base + wiggle, 0.0);
}
```

`floor(gen_acc(t))` is the generation at step `t`. Parameters interpolate between `gn` and `gn+1` using `fract(gen_acc(t))`. Each wave has its own `drift_freq` and `drift_phase`, so they advance through generations at different rates.

## Parameters across generations

A generation's parameters are a weighted blend of its own hash and the previous generation's. The weight is power — high-energy generations carry more forward:

```glsl
fn memory(wave_i: u32, gn: u32, ch: u32) -> f32 {
    let own   = gen_param(wave_i, gn,      ch);
    let past  = gen_param(wave_i, gn - 1u, ch);
    let power = clamp(amp * freq * 0.4 * complexity_bonus, 0.0, 1.0);
    let carry = mix(gen_param(wave_i, gn, ch + 10u), power, 0.5);
    return mix(own, past, carry);
}
```

High energy → high carry → parameters stay close to the previous generation.
Low energy → low carry → parameters drift further.

`complexity_bonus` is derived from the popcount of the hash (`bits` column in tape output) — generations near 16 set bits out of 32 score highest.

## Energy in O(1)

Whether a wave is active at generation `gn` depends on its cumulative energy — the sum of `(power − threshold)` across all generations up to `gn`. Computing that naively is O(N). Instead it uses the Dirichlet kernel, a closed-form cosine sum:

```glsl
fn wave_energy(wave_i: u32, gn: u32) -> f32 {
    let alpha   = 2.0 * PHI;
    let beta    = f32(wave_i) * PHI * PI;
    let N       = f32(gn);
    let cos_sum = sin((N + 1.0) * alpha * 0.5)
                * cos(N * alpha * 0.5 + beta)
                / sin(alpha * 0.5);
    return 1.0 + (N + 1.0) * (0.5 - ENERGY_THRESHOLD) - 0.5 * cos_sum;
}
```

If `wave_energy > 0`, it contributes to the field. If not, it's zeroed out. Because `cos_sum` oscillates, energy doesn't decline monotonically — a wave can phase out and in before zeroing out permanently.

## The field

Each pixel sums the contributions of all active waves and their forks:

```glsl
let field = (w0.sin_term + w1.sin_term + w2.sin_term
           + f0.sin_term + f1.sin_term + f2.sin_term) / amp_sum;
```

The time derivative and spatial gradient are computed analytically:

```glsl
// dfield/dt — positive: field rising, negative: field falling
let dfield_dt = -(w0.cos_freq + w1.cos_freq + w2.cos_freq + ...) / amp_sum;

// gradient — spatial direction of steepest ascent
let gradient = vec2(
    sum(cos_freq * dir.x),
    sum(cos_freq * dir.y),
) / amp_sum;
```

`dfield_dt` tints the color — rising and falling regions render at different temperatures. `gradient` magnitude adds brightness at wave edges.

## Architecture

```
(seed, step)  →  gen_acc()     →  generation number at this step
              →  memory()      →  parameters (blend of hash(gn) and hash(gn-1), weighted by power)
              →  wave_energy() →  active or zeroed (Dirichlet kernel, O(1))
              →  sample_wave() →  field contribution at (x, y, step)
              →  superpose     →  pixel color
```

## Vocabulary

| term | meaning |
|---|---|
| **power** | `amp × freq × 0.4` — instantaneous output of a generation |
| **energy** | power accumulated across generations — goes negative, wave phases out |
| **carry** | how much of the previous generation's parameters survive into the next |
| **generation** | one complete cycle of the accumulator |
| **branch** | a line of descent from a fixed seed — wave0, wave1, wave2 never cross |
| **fork** | splits off from a branch at a hash-determined generation, runs its own energy trajectory |

## Controls (--headless)

```
space       pause / resume
← →         rewind / forward
↑ ↓         speed up / slow down
1–5         step presets: 0.1  1  10  100  1000
r           rewind to step 0
c           new seed
q           quit
```

## Tape

```bash
cargo run -- --tape
cargo run -- --tape --step 100
```

```
           T      seed  wave0:gn bits  pwr  energy  wave1:gn bits  pwr  energy  wave2:gn bits  pwr  energy  dominant
────────────────────────────────────────────────────────────────────────────────────────────────────
    0.0000e0  xmTq5xLr         0   15 0.11  +0.490         0   18 0.56  +0.809         0   11 0.34  +1.359  wave1
    1.0000e2  xmTq5xLr         6   15 0.47  +0.475         4   17 0.69  +0.692         5   21 0.58  +1.056  wave1
    2.0000e2  xmTq5xLr        12   15 0.52  +0.525         9   17 0.44  +1.059        10   19 0.43  +1.042  wave0
    3.0000e2  xmTq5xLr        18   20 0.99  +0.604        13   18 0.98  +1.030        16   14 0.73  +0.855  wave0 PEAK
    4.0000e2  xmTq5xLr        24   20 0.31  +0.669        18   17 0.20  +0.517        21   19 0.91  +1.206  wave2
    5.0000e2  xmTq5xLr        31   15 0.60  +0.183        22   20 0.35  +0.540        26   20 0.65  +0.673  wave2
    6.0000e2  xmTq5xLr        37   20 0.59  +0.140        27   17 1.00  +0.683        32   16 0.11  +0.666  wave1 PEAK
    7.0000e2  xmTq5xLr        43    9 0.38  +0.168        31   15 0.67  +0.549        38   12 0.83  +0.720  wave2
    8.0000e2  xmTq5xLr        49   15 0.71  +0.241        36   19 0.85  +0.707        43   16 0.11  +0.718  wave1
    9.0000e2  xmTq5xLr        55   23 0.65  +0.317        40   16 0.97  +0.728        48   14 0.62  +0.846  wave1 PEAK
    1.0000e3  xmTq5xLr        62   16 0.32  zeroed        45   13 0.55  +0.210        54   13 0.76  +0.874  wave2 ← wave0 last signal
    1.2000e3  xmTq5xLr        75   14 0.15  +0.155        54   13 0.37  +0.543        65   15 0.28  +0.333  wave1
    1.3000e3  xmTq5xLr        81   16 0.16  zeroed        58   20 0.68  +0.425        70   13 0.66  +0.592  wave1 ← wave0 last signal
    1.5000e3  xmTq5xLr        93   13 0.45  zeroed        67   18 1.00  +0.353        81   12 1.00  +0.511  wave2 PEAK
    1.6000e3  xmTq5xLr        99   19 0.56  zeroed        72   15 1.00  zeroed        86   16 0.17  +0.104  wave2 ← wave1 last signal
    1.7000e3  xmTq5xLr       106   15 1.00  zeroed        76   18 0.61  zeroed        92   14 0.26  +0.012  wave2
    1.8000e3  xmTq5xLr       112   13 0.17  zeroed        81   10 0.33  +0.345        97   14 0.12  +0.428  wave1
    2.0000e3  xmTq5xLr       124   11 0.38  zeroed        90   19 0.36  zeroed       108   15 0.34  +0.101  wave2
    2.2000e3  xmTq5xLr  no signal
```

wave0 has the highest `drift_freq` — by step 1000 it's at generation 62 while wave1 is at 45. It peaks at step 300 (power 0.99) then loses energy, zeroing out around step 1000. It returns briefly at step 1200 (energy +0.155) before phasing out — the Dirichlet cosine sum oscillating across zero.

## License

BSD 3-Clause
