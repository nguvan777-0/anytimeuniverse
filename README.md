# anytime-universe

A universe that exists as a pure mathematical function of time T.

No simulation loop. No stored state. Every moment — past, future, any T — evaluates in O(1).

![screenshot](https://github.com/nguvan777-0/anytime-universe/releases/download/screenshots/screenshot_Yy3zb1B1.png)

---

## What it is

Three wave lineages superposing across a 2D field. Each lineage propagates, interferes with the others, and attenuates over time. The field at any T is whatever signal remains.

The GPU evaluates the field at the current T. Scrub backward — it recalculates. Jump to T=1,000,000 — instant. Every event in the universe's history already exists in the math. You are not running it. You are reading it.

## Vocabulary

| term | domain | meaning |
|---|---|---|
| **power** | physics | amp × freq × 0.4 — instantaneous output of a generation |
| **energy** | physics | power accumulated across generations — goes negative, signal is lost |
| **copying** | information theory | parameters carried forward from parent generation, weighted by carry |
| **lineage** | mathematics | a parallel line of descent from a fixed seed — W0, W1, W2 never cross |
| **fork** | computer science | deterministic branch — new lineage born from a parent at a hash-determined generation |
| **carry** | mathematics | how much of the parent's parameters survive into the next generation |
| **generation** | mathematics | one complete cycle of the accumulator |
| **memory** | computer science | reconstructs parameters from hash — stores nothing, derives everything |

## History without storage

The hash is the history.

```
gen_param(wave_i, gn, ch)  =  uhash(wave_i × 97 + gn × 1031 + ch × 7)
```

Every generation's parameters are a blend of `hash(gn)` and `hash(gn-1)`. The child resembles the parent because the math makes it so, not because anything was written down.

Generation 47 is two hash evaluations and a blend. Any generation, any ancestor, instant. Ancestry is a mathematical property of the hash — the same way a number's factors don't need to be stored anywhere. They just are.

## Energy and selection, O(1)

Power is `amp × freq × 0.4`. High-power generations carry more of their parameters forward into the next generation. Low-power generations drift. Selection pressure without a selector.

Energy accumulates across generations using a closed-form cosine sum — no loop. When a lineage's energy hits zero, it goes quiet. Last signal. When all three are quiet, the field is dark. The program exits.

Change the seed. A new universe begins.

## Architecture

```
seed + T  →  gen_acc()    →  generation number
          →  memory()     →  copied parameters (power-weighted blend of hash(gn), hash(gn-1))
          →  energy()     →  active or zeroed (Dirichlet kernel, O(1))
          →  sample()     →  field contribution at (x, y, T)
          →  superpose    →  pixel
```

Every pixel is independent. The GPU runs them all in parallel. No adjacency. No communication. Pure pull — each pixel samples the field, gets an answer, done.

## Run

```
cargo run                        # GPU window, real-time field
cargo run -- --headless          # ASCII terminal, interactive
cargo run -- --tape              # scrolling data log, exits on last signal
cargo run -- --help              # full options
```

## Controls (--headless)

```
space       pause / resume
← →         scrub backward / forward in T
↑ ↓         speed up / slow down
1–5         step presets: 0.1  1  10  100  1000
r           rewind to T=0
c           new seed
q           quit
```

## Tape

```bash
cargo run -- --tape                          # watch the full lifecycle
cargo run -- --tape | grep "no signal"       # last signal events
cargo run -- --tape | grep PEAK              # high-power generations
cargo run -- --tape --step 500 | tee run.log
```

Output columns:

```
           T      seed  W0:gn  pwr   energy  W1:gn  pwr   energy  W2:gn  pwr   energy  dominant
────────────────────────────────────────────────────────────────────────────────────────────────
  0.0000e0  x7Kp3mNq      0  0.11   +0.490      0  0.60   +0.809      0  0.40   +1.359  W2
  5.8000e4  x7Kp3mNq     58  0.57  zeroed       72  0.77   +0.638     94  0.43   +0.862  W2  ← W0 last signal
  1.3500e5  x7Kp3mNq    135  0.00  zeroed      115  0.00  zeroed      135  0.00  zeroed  no signal
```

---

## What's next

- More lineages — why three?
- Forks that fork — lineages that branch fast enough to stay ahead of attenuation
- Cross-lineage interference as a selection pressure
- Seeds that hold signal for thousands of generations vs ones that go dark in twenty

The block universe model means none of this requires re-architecting. Every idea is an extension of the same O(1) evaluation. Say yes, add a term, run it.
