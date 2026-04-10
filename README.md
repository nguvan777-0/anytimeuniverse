# anytimeuniverse

A cross-platform simulation in Rust where time is the direct input. Step to any point in time — past or future — and the field evaluates in O(1). Every parameter at every generation comes from a single sin-based hash, and cumulative wave energy across any number of generations is computable in closed form via the Dirichlet kernel.

Three wave branches run against that field. Each accumulates energy, copies parameters forward with drift, or phases in and out. You can watch the visualization forward, rewind, or jump to any moment.

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

Pre-built binaries are on the [releases page](https://github.com/nguvan777-0/anytimeuniverse/releases).

- **macOS Apple Silicon** — download the `.tar.gz`, extract, and run
- **Linux** — download, `chmod +x`, and run
- **Windows** — download and run the `.exe`

---

## you don't need the past

Each wave contributes a periodic signal to the field, localized by a Gaussian envelope whose radius is driven by the wave's energy. When all waves use the default sine shape, the field is a spatially-localized Fourier sum:

$$F(x, y, t) = \frac{1}{A} \sum_{i}\ A_i \exp\!\left(-\frac{\|\mathbf{x} - \mathbf{c}_i(t)\|^2}{R_i(E_i)^2}\right) \sin\!\left(\omega_i\, \mathbf{d}_i \cdot \mathbf{x} - \omega_i t + \phi_i\right)$$

The sinusoidal foundation defines the field's speed and slope at every point. Every pixel evaluates directly from the formula in constant time ($O(1)$); the field is as fast to compute at $T=0$ as it is at $T=1e15$. Direct formulas for the time-derivative ($\frac{\partial F}{\partial t}$) and spatial gradient ($\nabla_{\mathbf{x}} F$):

$$\frac{\partial F}{\partial t} = -\frac{1}{A}\sum_i A_i\omega_i \cos(\cdot) \qquad \nabla_{\mathbf{x}} F = \frac{1}{A}\sum_i A_i\omega_i \cos(\cdot)\,\mathbf{d}_i$$

The same sinusoidal structure makes cumulative energy across generations computable in O(1) via the Dirichlet kernel — the closed form for a partial sum of cosines:

$$E(i, N) = 1 - \frac{1}{2} \cdot \frac{\sin\!\left(\frac{(N+1)\alpha}{2}\right)\cos\!\left(\frac{N\alpha}{2} + \beta_i\right)}{\sin\!\left(\frac{\alpha}{2}\right)}$$

where $\alpha = 2\varphi$, $\beta_i = i\varphi\pi$, and $\varphi$ is the golden ratio. Using $\alpha = 2\varphi$ makes the trajectory quasiperiodic — it never repeats, and each wave gets a unique energy path through the same generation space.

The Dirichlet kernel provides the closed form for $O(1)$ energy calculation: it determines if a wave has accumulated enough resonance to stay active at any generation $N$, without summing $N$ terms. This makes $t$ a direct input rather than a clock—there is no loop over history, only a formula evaluated at a point.

Natural selection enters through the carry blend. A generation's parameters are a linear interpolation between its own hash and the previous generation's, weighted by carry:

$$p(i,n,c) = (1 - \text{carry})\cdot\text{hash}(i,n,c) + \text{carry}\cdot\text{hash}(i,n-1,c)$$

$$\text{carry}(i,n) = \frac{1}{2}\,\text{hash}(i,n,c{+}10) + \frac{1}{2}\,\text{power}(i,n)$$

High-energy generations carry more of their parameters forward. This is selection without a selector — it falls out of the blend formula.

---

## Parameters from the hash

Every parameter of every generation of every wave comes from a single sin-based hash:

```rust
fn fhash(wave_i: u32, gn: u32, ch: u32) -> f32 {
    let x = f32(wave_i) * 97.0 + f32(gn) * PHI + f32(ch) * 7.321;
    let v = sin(x) + sin(x * EUL) * 0.5 + sin(x * PI) * 0.25;
    return sin(v * 1.5) * 0.5 + 0.5;
}
```

`wave_i` is the wave index, `gn` is the generation, `ch` is the channel — amplitude, frequency, direction angle, wave shape, spatial warp and carry weights. The irrational multipliers (PHI, e, π) ensure the output is aperiodic across all (wave_i, gn, ch) triples.

## Step → generation

The step counter maps to a generation number via the accumulator:

```rust
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

```rust
fn memory(wave_i: u32, gn: u32, ch: u32) -> f32 {
    let own   = fhash(wave_i, gn,      ch);
    let past  = fhash(wave_i, gn - 1u, ch);
    let power = clamp(amp * freq * 0.4 * cb, 0.0, 1.0);
    let carry = mix(fhash(wave_i, gn, ch + 10u), power, 0.5);
    return mix(own, past, carry);
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
    let beta    = f32(wave_i) * PHI * PI;
    let N       = f32(gn);
    let cos_sum = sin((N + 1.0) * alpha * 0.5)
                * cos(N * alpha * 0.5 + beta)
                / sin(alpha * 0.5);
    return 1.0 - 0.5 * cos_sum;
}
```

Energy drives the physical size of the wave's Gaussian envelope. Because `cos_sum` oscillates, energy constantly cycles, expanding and contracting the wave's spatial radius as it pulses through generations without ever dying permanently. Forks use energy as a threshold to determine when they branch in and out of existence.

## The field

Each pixel sums the contributions of all active waves and their forks:

```wgsl
let field = (wave0.sin_term + wave1.sin_term + wave2.sin_term
           + f0.sin_term + f1.sin_term + f2.sin_term) / amp_sum;
```

The time derivative and spatial gradient are computed analytically:

```wgsl
// dfield/dt — positive: field rising, negative: field falling
let dfield_dt = -(wave0.cos_freq + wave1.cos_freq + wave2.cos_freq
               +  f0.cos_freq    + f1.cos_freq    + f2.cos_freq) / amp_sum;

// gradient — spatial direction of steepest ascent
let gradient = vec2(
    sum(cos_freq * dir.x),
    sum(cos_freq * dir.y),
) / amp_sum;
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

