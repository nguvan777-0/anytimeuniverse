# anytimeuniverse

A cross-platform simulation in Rust where time is the direct input. Step to any point in time — past or future — and the field evaluates in $\mathcal{O}(1)$. Every parameter at every generation comes from a single sin-based hash, and cumulative wave energy across any number of generations is computable in closed form via the Dirichlet kernel.

Three wave branches run against that field. Each accumulates energy, copies parameters forward with drift, or phases in and out. You can watch the visualization forward, rewind, or jump to any moment.

![screenshot](https://github.com/nguvan777-0/anytimeuniverse/releases/download/v0.3.1/anytimeuniverse-wvh7Tc3t-11-61.84961621.png)

![screenshot](https://github.com/nguvan777-0/anytimeuniverse/releases/download/v0.3.1/anytimeuniverse-DX8zqdXS-12345678910_0.png)

![screenshot](https://github.com/nguvan777-0/anytimeuniverse/releases/download/v0.3.0/anytimeuniverse-TKQg7X9w-40792701337273597952_0.png)

![screenshot](https://github.com/nguvan777-0/anytimeuniverse/releases/download/v0.3.0/anytimeuniverse-yHxJ7gPS-579521556646169804800_0.png)

![screenshot](https://github.com/nguvan777-0/anytimeuniverse/releases/download/v0.3.0/anytimeuniverse-C7Qw2Jt1-2765314241196514816_0.png)

```bash
cargo run                        # GPU: Hardware-accelerated real-time field
cargo run -- --ascii             # CLI: Interactive 24-bit terminal renderer
cargo run -- --tape              # Tape: scrolling data log
cargo run -- --help              # full options
```

## Download compiled binary

Pre-built binaries are on the [releases page](https://github.com/nguvan777-0/anytimeuniverse/releases)

- **macOS Apple Silicon** — download the `.tar.gz`, extract, and run
- **Linux** — download, `chmod +x`, and run
- **Windows** — download and run the `.exe`

---

## you don't need the past

Each wave contributes a periodic signal to the field, localized by a Gaussian envelope whose radius is driven by the wave's energy. When all waves use the default sine shape, the field is a spatially-localized Fourier sum:

$$F(x, y, t) = \frac{1}{A} \sum_{i}\ A_i \exp\!\left(-\frac{\|\mathbf{x} - \mathbf{c}_i(t)\|^2}{R_i(E_i)^2}\right) \sin\!\left(\omega_i\, \mathbf{d}_i \cdot \mathbf{x} - \omega_i t + \phi_i\right)$$

The sinusoidal foundation defines the field's speed and slope at every point. Every pixel evaluates directly from the formula in constant time $\mathcal{O}(1)$; the field is as fast to compute at $T=0$ as it is at $T=1e15$. Direct formulas for the time-derivative ($\frac{\partial F}{\partial t}$) and spatial gradient ($\nabla_{\mathbf{x}} F$):

$$\frac{\partial F}{\partial t} = -\frac{1}{A}\sum_i A_i\omega_i \cos(\cdot) \qquad \nabla_{\mathbf{x}} F = \frac{1}{A}\sum_i A_i\omega_i \cos(\cdot)\,\mathbf{d}_i$$

The same sinusoidal structure makes cumulative energy across generations computable in $\mathcal{O}(1)$ via the Dirichlet kernel — the closed form for a partial sum of cosines:

$$E(i, N) = 1 - \frac{1}{2} \cdot \frac{\sin\!\left(\frac{(N+1)\alpha}{2}\right)\cos\!\left(\frac{N\alpha}{2} + \beta_i\right)}{\sin\!\left(\frac{\alpha}{2}\right)}$$

where $\alpha = 2\varphi$, $\beta_i = i\varphi\pi$, and $\varphi$ is the golden ratio. Using $\alpha = 2\varphi$ makes the trajectory quasiperiodic — it never repeats, and each wave gets a unique energy path through the same generation space.

The Dirichlet kernel provides the closed form for $\mathcal{O}(1)$ energy calculation: it determines if a wave has accumulated enough resonance to stay active at any generation $N$, without summing $N$ terms. This makes $t$ a direct input rather than a clock—there is no loop over history, only a formula evaluated at a point.

Natural selection enters through the carry blend. A generation's parameters are a linear interpolation between its own hash and the previous generation's, weighted by carry:

$$p(i,n,c) = (1 - \text{carry})\cdot\text{hash}(i,n,c) + \text{carry}\cdot\text{hash}(i,n-1,c)$$

$$\text{carry}(i,n) = \frac{1}{2}\,\text{hash}(i,n,c{+}10) + \frac{1}{2}\,\text{power}(i,n)$$

High-energy generations carry more of their parameters forward. This is selection without a selector — it falls out of the blend formula.

---

## Parameters from the hash

Every parameter of every generation of every wave comes from a single sin-based hash:

```rust
fn fhash(wave_i: u32, gn: u32, ch: u32) -> f32 {
    let gn = gn % (1 << 24);
    let x = wave_i as f32 * 97.0 + gn as f32 * PHI + ch as f32 * 7.321;
    let v = x.sin() + (x * EUL).sin() * 0.5 + (x * PI).sin() * 0.25;
    (v * 1.5).sin() * 0.5 + 0.5
}
```

`wave_i` is the wave index, `gn` is the generation, `ch` is the channel — amplitude, frequency, direction angle, wave shape, spatial warp and carry weights. The irrational multipliers (PHI, e, π) ensure the output is aperiodic across all (wave_i, gn, ch) triples.

## Step → generation

The step counter maps to a generation number via the accumulator:

```rust
fn gen_acc(drift_freq: f64, drift_phase: f64, t: f64, noise: f64) -> f64 {
    let base   = t * drift_freq * 2.0;
    let wiggle = noise * 0.5 * (
        (drift_freq * PHI * t + drift_phase).sin() +
        (drift_freq * PI  * t + drift_phase * EUL).sin()
    );
    (base + wiggle).max(0.0)
}
```

`floor(gen_acc(t))` is the generation at step `t`. Parameters interpolate between `gn` and `gn+1` using `fract(gen_acc(t))`. Each wave has its own `drift_freq` and `drift_phase`, so they advance through generations at different rates.

## Parameters across generations

A generation's parameters are a weighted blend of its own hash and the previous generation's. The weight is power — high-energy generations carry more forward:

```rust
fn memory(wave_i: u32, gn: u32, ch: u32) -> f32 {
    let own   = fhash(wave_i, gn, ch);
    let past  = fhash(wave_i, gn.wrapping_sub(1), ch);
    let power = (amp * freq * 0.4 * cb).clamp(0.0, 1.0);
    let carry = fhash(wave_i, gn, ch + 10) * 0.5 + power * 0.5;
    own * (1.0 - carry) + past * carry
}
```

High energy → high carry → parameters stay close to the previous generation.
Low energy → low carry → parameters drift further.

`complexity_bonus` (cb) is derived from how close the resonance value is to the resonance attractor ($\Phi - 1 \approx 0.618$) — generations near that target score highest.

## Energy in O(1)

Whether a wave is active at generation `gn` depends on its cumulative energy — the sum of `(power − threshold)` across all generations up to `gn`. A step-by-step sum is O(N). Instead it uses the Dirichlet kernel, a closed-form cosine sum:

```rust
fn wave_energy(wave_i: u32, gn: u32) -> f32 {
    let alpha   = 2.0 * PHI;
    let beta    = wave_i as f32 * PHI * PI;
    let n       = gn as f32;
    let cos_sum = ((n + 1.0) * alpha * 0.5).sin()
                * (n * alpha * 0.5 + beta).cos()
                / (alpha * 0.5).sin();
    1.0 - 0.5 * cos_sum
}
```

Energy drives the physical size of the wave's Gaussian envelope. Because `cos_sum` oscillates, energy constantly cycles, expanding and contracting the wave's spatial radius as it pulses through generations without ever dying permanently. Forks use energy as a threshold to determine when they branch in and out of existence.

## The field

Each pixel sums the contributions of all active waves and their forks:

```wgsl
    // identical for every pixel, so the division cost is paid once.
    let inv_amp = 1.0 / (wc.waves[0].base_amp + wc.waves[1].base_amp + wc.waves[2].base_amp);

    let field = (wave0.sin_term + wave1.sin_term + wave2.sin_term
               + f0.sin_term + f1.sin_term + f2.sin_term) * inv_amp;
```

The time derivative and spatial gradient are computed analytically:

```wgsl
// dfield/dt — positive: field rising, negative: field falling
let dfield_dt = -(wave0.cos_freq + wave1.cos_freq + wave2.cos_freq
               +  f0.cos_freq    + f1.cos_freq    + f2.cos_freq) * inv_amp;

// gradient — spatial direction of steepest ascent
let gradient = vec2(
    sum(cos_freq * dir.x),
    sum(cos_freq * dir.y),
) * inv_amp;
```

`dfield_dt` tints the color — rising and falling regions render at different temperatures. `gradient` magnitude adds brightness at wave edges.

## Architecture

```
(seed, T)  →  fhash / gen_acc / memory / wave_energy  →  WaveData[6]   (CPU, once per frame)
           →  dot(dir, pos) / sin / exp                →  field pixel   (GPU, per pixel)
           →  egui                                     →  UI overlay    (GPU, per frame)
```

The CPU computes all non-positional values—hash lookups, blends, and energy—once per frame. By uploading these as a uniform buffer, the GPU evaluates the spatial carrier and Gaussian envelope.

Simulation parameters are passed to the renderer directly on the main thread, while a separate audio thread (`synth_engine.rs`) drives the additive synthesizer. The sound reflects the live physics: as parameters drift across generations, the timbre changes in sync.

### Wave Data Structure
The CPU passes wave parameters to the GPU in a fixed-size byte buffer. The simulation runs 3 waves, mapped to 3 `EnvUniform` definitions. Each wave packs 8 `f32` components (32 bytes). The total uniform buffer is 3 × 32 = 96 bytes:
*   `[0]` — Amplitude ($A$)
*   `[1]` — Frequency ($\omega$)
*   `[2]` — Phase Offset ($\phi$)
*   `[3]` — Direction X ($d_x$)
*   `[4]` — Direction Y ($d_y$)
*   `[5]` — Drift Frequency
*   `[6]` — Drift Phase
*   `[7]` — Heat Index (wave-0) / Padding

### Numeric Precision & Domain
Large time jumps require 64-bit precision to avoid rounding errors. The CPU handles all time and physics math using `f64`. Data is cast to `f32` only when sent to the GPU. Consumer GPUs lack fast 64-bit hardware, making `f64` compute heavily penalized, and graphics APIs are built around 32-bit types.

| Domain | Types | Purpose |
|---|---|---|
| **Epoch & Residual** | `i64` + `f64` | Time is stored as whole periods (`t_epoch`) plus a fractional offset (`t_residual` in `[0, P)`). This prevents precision loss at scales like $T = 10^{20}$. |
| **Speed & Delta** | `f64` | `wave_speed` and frametime (`dt`) use `f64` to prevent integration drift at speeds up to $10^{18}$ T/s. |
| **GPU Shaders** | `f32` | Spatial pixel evaluation. Time offsets are wrapped by the period ($T \bmod 2\pi$) before GPU upload, preventing precision loss in the shader. |
| **UI Constraints** | `f64` mapping | Sliders use a symmetric log mapper (`slider_symlog_f64`) to handle inputs from $T = 10^{-4}$ to $10^{20}$ without overflow. |

## Terms

| term | meaning |
|---|---|
| **power** | `amp × freq × 0.4` — instantaneous output of a generation |
| **energy** | power accumulated across generations — goes negative, wave phases out |
| **carry** | how much of the previous generation's parameters survive into the next |
| **resonance** | how close the generation's hash is to the golden ratio — high resonance amplifies power |
| **generation** | one complete cycle of the accumulator |
| **branch** | a branch path from a fixed seed — wave0, wave1, wave2 never cross |
| **fork** | splits off from a branch at a hash-determined generation, runs its own energy trajectory |

## License

BSD 3-Clause

