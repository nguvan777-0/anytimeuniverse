/// Terminal renderer — mirrors render.wgsl exactly.
/// Every pixel is O(1) in T: same hash → memory → energy → sample_wave pipeline.
/// ANSI true color (24-bit). Run with --headless.

use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;

const PI:  f64 = std::f64::consts::PI;
const PHI: f64 = 1.6180339887;
const EUL: f64 = 2.7182818284;
const TAU: f64 = std::f64::consts::TAU;

// ── Hash (mirrors shader) ─────────────────────────────────────────────────────
fn uhash(x: u32) -> u32 {
    let mut v = x ^ (x >> 17);
    v = v.wrapping_mul(0xbf324c81);
    v ^= v >> 13;
    v = v.wrapping_mul(0x9a813f77);
    v ^= v >> 16;
    v
}

fn gen_param(wave_i: u32, gn: u32, ch: u32) -> f64 {
    uhash(wave_i.wrapping_mul(97)
        .wrapping_add(gn.wrapping_mul(1031))
        .wrapping_add(ch.wrapping_mul(7))) as f64
        / u32::MAX as f64
}

// ── Hardware bit count (Population Count) ─────────────────────────────────────
fn gen_bits(wave_i: u32, gn: u32) -> u32 {
    let h = uhash(wave_i.wrapping_mul(97)
        .wrapping_add(gn.wrapping_mul(1031))
        .wrapping_add(8u32.wrapping_mul(7))); // Channel 8 reserved for raw genome
    h.count_ones()
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
fn memory(wave_i: u32, gn: u32, ch: u32) -> f64 {
    let own    = gen_param(wave_i, gn, ch);
    let past   = gen_param(wave_i, gn.wrapping_sub(1), ch);
    let amp_n  = 0.3 + gen_param(wave_i, gn, 0) * 2.4;
    let freq_n = 0.4 + gen_param(wave_i, gn, 1) * 1.2;
    
    // bit complexity (popcnt) — favors sequences near 16 set bits (50%)
    let bits = gen_bits(wave_i, gn) as f64;
    let complexity_bonus = 1.0 - ((bits - 16.0).abs() / 16.0);

    let power = (amp_n * freq_n * 0.4 * (0.5 + 0.5 * complexity_bonus)).clamp(0.0, 1.0);
    let carry = gen_param(wave_i, gn, ch + 10) * 0.5 + power * 0.5;
    past + (own - past) * carry
}

// ── Energy (mirrors shader) ───────────────────────────────────────────────────
const ENERGY_THRESHOLD: f64 = 0.51;

fn wave_energy(wave_i: u32, gn: u32) -> f64 {
    let alpha   = 2.0 * PHI;
    let beta    = wave_i as f64 * PHI * PI;
    let n       = gn as f64;
    let cos_sum = ((n + 1.0) * alpha * 0.5).sin()
                * (n * alpha * 0.5 + beta).cos()
                / (alpha * 0.5).sin();
    1.0 + (n + 1.0) * (0.5 - ENERGY_THRESHOLD) - 0.5 * cos_sum
}

// ── Wave sample ───────────────────────────────────────────────────────────────
struct WaveSample {
    sin_term: f64,
    cos_freq: f64,
    dir:      [f64; 2],
}

fn sample_wave(
    amp: f64, freq: f64, phase: f64, dir_x: f64, dir_y: f64,
    drift_freq: f64, drift_phase: f64,
    wave_i: u32, pos: [f64; 2], t: f64, noise: f64,
) -> WaveSample {
    let acc  = gen_acc(drift_freq, drift_phase, t, noise);
    let gn   = acc.floor() as u32;
    let frac = acc.fract();

    let amp_a   = amp  * (0.3 + memory(wave_i, gn,                  0) * 2.4);
    let freq_a  = freq * (0.4 + memory(wave_i, gn,                  1) * 1.2);
    let angle_a =              memory(wave_i, gn,                  2) * TAU;
    let amp_b   = amp  * (0.3 + memory(wave_i, gn.wrapping_add(1), 0) * 2.4);
    let freq_b  = freq * (0.4 + memory(wave_i, gn.wrapping_add(1), 1) * 1.2);
    let angle_b =              memory(wave_i, gn.wrapping_add(1), 2) * TAU;

    let blend   = if frac < 0.9 { 0.0 } else { let x = (frac - 0.9) / 0.1; x * x * (3.0 - 2.0 * x) };
    let amp_t   = amp_a   + (amp_b   - amp_a)   * blend;
    let freq_t  = freq_a  + (freq_b  - freq_a)  * blend;
    let angle_t = angle_a + (angle_b - angle_a) * blend;

    let dir_t     = [angle_t.cos(), angle_t.sin()];
    let phase_arg = freq_t * (dir_t[0] * pos[0] + dir_t[1] * pos[1]) - freq_t * t + phase;

    let active = if wave_energy(wave_i, gn) > 0.0 { 1.0 } else { 0.0 };

    WaveSample {
        sin_term: amp_t * phase_arg.sin() * active,
        cos_freq: amp_t * freq_t * phase_arg.cos() * active,
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
    let w0 = w(0); let w1 = w(1); let w2 = w(2);

    let amp_sum = (env[0] + env[8] + env[16]) as f64 + 1e-5;

    let field     = (w0.sin_term + w1.sin_term + w2.sin_term) / amp_sum;
    let dfield_dt = -(w0.cos_freq + w1.cos_freq + w2.cos_freq) / amp_sum;

    let signal = field.max(0.0).powi(3);

    // branch color
    let r0 = w0.sin_term.max(0.0) / (env[0]  as f64 + 1e-5);
    let r1 = w1.sin_term.max(0.0) / (env[8]  as f64 + 1e-5);
    let r2 = w2.sin_term.max(0.0) / (env[16] as f64 + 1e-5);
    let rsum = r0 + r1 + r2 + 1e-5;

    let mut col = [
        (r0/rsum) * wave_colors[0][0] + (r1/rsum) * wave_colors[1][0] + (r2/rsum) * wave_colors[2][0],
        (r0/rsum) * wave_colors[0][1] + (r1/rsum) * wave_colors[1][1] + (r2/rsum) * wave_colors[2][1],
        (r0/rsum) * wave_colors[0][2] + (r1/rsum) * wave_colors[1][2] + (r2/rsum) * wave_colors[2][2],
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
    #[cfg(unix)]
    {
        unsafe {
            #[repr(C)]
            struct Winsize { ws_row: u16, ws_col: u16, ws_xpixel: u16, ws_ypixel: u16 }
            let mut ws = Winsize { ws_row: 0, ws_col: 0, ws_xpixel: 0, ws_ypixel: 0 };
            // TIOCGWINSZ = 0x5413 on Linux, 0x40087468 on macOS
            #[cfg(target_os = "macos")]
            let tiocgwinsz: u64 = 0x40087468;
            #[cfg(not(target_os = "macos"))]
            let tiocgwinsz: u64 = 0x5413;
            if libc_ioctl(1, tiocgwinsz, &mut ws as *mut _ as *mut u8) == 0
                && ws.ws_col > 0 && ws.ws_row > 0
            {
                return (ws.ws_col as usize, ws.ws_row as usize);
            }
        }
    }
    (80, 24)
}

#[cfg(unix)]
unsafe extern "C" {
    fn ioctl(fd: i32, request: u64, ...) -> i32;
    fn tcgetattr(fd: i32, t: *mut u8) -> i32;
    fn tcsetattr(fd: i32, action: i32, t: *const u8) -> i32;
    fn cfmakeraw(t: *mut u8);
    fn fcntl(fd: i32, cmd: i32, arg: i32) -> i32;
}

#[cfg(unix)]
unsafe fn libc_ioctl(fd: i32, req: u64, arg: *mut u8) -> i32 { ioctl(fd, req, arg) }

// ── Raw terminal mode ─────────────────────────────────────────────────────────
// Switches stdin to raw + non-blocking. Restores on drop.
#[cfg(unix)]
struct RawMode { orig_termios: [u8; 128], orig_flags: i32 }

#[cfg(unix)]
impl RawMode {
    fn enter() -> Self {
        use std::os::unix::io::AsRawFd;
        #[cfg(target_os = "macos")]      const O_NONBLOCK: i32 = 0x0004;
        #[cfg(not(target_os = "macos"))] const O_NONBLOCK: i32 = 0x0800;
        unsafe {
            let fd = std::io::stdin().as_raw_fd();
            let mut orig_termios = [0u8; 128];
            tcgetattr(fd, orig_termios.as_mut_ptr());
            let mut raw = orig_termios;
            cfmakeraw(raw.as_mut_ptr());
            tcsetattr(fd, 2 /*TCSAFLUSH*/, raw.as_ptr());
            // save original flags before setting O_NONBLOCK
            let orig_flags = fcntl(fd, 3 /*F_GETFL*/, 0);
            fcntl(fd, 4 /*F_SETFL*/, orig_flags | O_NONBLOCK);
            RawMode { orig_termios, orig_flags }
        }
    }
}

#[cfg(unix)]
impl Drop for RawMode {
    fn drop(&mut self) {
        use std::io::Write;
        use std::os::unix::io::AsRawFd;
        unsafe {
            let fd = std::io::stdin().as_raw_fd();
            // restore terminal mode
            tcsetattr(fd, 2 /*TCSAFLUSH*/, self.orig_termios.as_ptr());
            // restore original flags — this un-sets O_NONBLOCK on the shared
            // open file description so the shell stdin works normally after exit
            fcntl(fd, 4 /*F_SETFL*/, self.orig_flags);
        }
        // restore cursor and flush
        let _ = std::io::stdout().write_all(b"\x1b[?25h\n");
        let _ = std::io::stdout().flush();
    }
}

// ── Input ─────────────────────────────────────────────────────────────────────
fn read_keys() -> Vec<u8> {
    use std::io::Read;
    let mut buf = [0u8; 32];
    match std::io::stdin().read(&mut buf) {
        Ok(n) if n > 0 => buf[..n].to_vec(),
        _ => vec![],
    }
}

pub enum Action { None, Quit, Rewind, NewSeed }

fn handle_keys(bytes: &[u8], c: &mut crate::ui::controls::Controls) -> Action {
    let mut i = 0;
    let mut action = Action::None;
    while i < bytes.len() {
        match bytes[i] {
            b'q' | 3 /*Ctrl-C*/ => return Action::Quit,
            b' '  => c.toggle_pause(),
            b'r'  => { c.rewind(); action = Action::Rewind; }
            b'c'  => { c.rewind(); action = Action::NewSeed; }
            b'1'  => c.preset(1),
            b'2'  => c.preset(2),
            b'3'  => c.preset(3),
            b'4'  => c.preset(4),
            b'5'  => c.preset(5),
            27 if bytes.get(i + 1) == Some(&b'[') => {
                match bytes.get(i + 2) {
                    Some(&b'C') => c.rewind_fwd(),   // →  rewind forward
                    Some(&b'D') => c.rewind_back(),  // ←  rewind backward
                    Some(&b'A') => c.speed_up(),    // ↑  faster
                    Some(&b'B') => c.speed_down(),  // ↓  slower
                    _ => {}
                }
                i += 3;
                continue;
            }
            27 => return Action::Quit, // lone ESC
            _ => {}
        }
        i += 1;
    }
    action
}

// ── Wave status line ──────────────────────────────────────────────────────────
fn wave_status(env: &[f32; 24], noise: f64, t: f64, wave_colors: &[[f64; 3]; 3], cols: usize) -> String {
    let mut out = String::new();
    for i in 0..3usize {
        let drift_freq  = env[i*8 + 5] as f64;
        let drift_phase = env[i*8 + 6] as f64;
        let acc = gen_acc(drift_freq, drift_phase, t, noise);
        let gn  = acc.floor() as u32;
        let energy = wave_energy(i as u32, gn);

        let amp_n  = 0.3 + gen_param(i as u32, gn, 0) * 2.4;
        let freq_n = 0.4 + gen_param(i as u32, gn, 1) * 1.2;
        let bits = gen_bits(i as u32, gn);
        let complexity_bonus = 1.0 - ((bits as f64 - 16.0).abs() / 16.0);
        let power = (amp_n * freq_n * 0.4 * (0.5 + 0.5 * complexity_bonus)).clamp(0.0, 1.0);

        let [r, g, b] = wave_colors[i].map(|v| (v * 255.0) as u8);
        let bar_len = 16usize;
        let filled = if energy > 0.0 { (power * bar_len as f64).round() as usize } else { 0 };

        let state_str = if energy > 0.0 { "active" } else { "zeroed" };
        let _ = write!(
            out,
            "\x1b[38;2;{r};{g};{b}m W{i}\x1b[0m  gn:{gn:>4}  bits:{bits:>2}  pwr:{power:.2}  energy:{energy:+.2}  ["
        );
        for j in 0..bar_len {
            if j < filled { out.push('█') } else { out.push('░') }
        }
        let _ = write!(out, "]  {state_str}\n");
    }
    out
}

// ── Color river (80 samples of wave dominance history) ───────────────────────
fn color_river(env: &[f32; 24], noise: f64, t_now: f64, wave_colors: &[[f64; 3]; 3], cols: usize) -> String {
    let window = (10.0 * TAU / 0.005_f64).max(4.0);
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
            let gn  = acc.floor() as u32;
            let active = if wave_energy(wi as u32, gn) > 0.0 { 1.0 } else { 0.0 };
            let amp_n = 0.3 + gen_param(wi as u32, gn, 0) * 2.4;
            doms[wi] = amp_n * active;
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
// Usage: cargo run -- --headless [--step 10.0] [--width 60] [--height 20] [--t0 0.0]
pub fn run(seed: &str, substrate_noise: f32, initial_wave_colors: [[f64; 3]; 3]) {
    // parse flags from env args
    let args: Vec<String> = std::env::args().collect();
    let get = |flag: &str, default: f64| -> f64 {
        args.windows(2)
            .find(|w| w[0] == flag)
            .and_then(|w| w[1].parse().ok())
            .unwrap_or(default)
    };
    let tape   = args.iter().any(|a| a == "--tape");
    let step   = get("--step",   if tape { 1000.0 } else { 10.0 });
    let width  = get("--width",  60.0) as usize;
    let height = get("--height", 20.0) as usize;
    let t      = get("--t0",     0.0);

    let noise  = substrate_noise as f64;
    let mut wave_colors = initial_wave_colors;
    let stdout = std::io::stdout();

    // character ramp: dark → bright
    let ramp: &[char] = &[' ', '·', ':', ';', '-', '=', '+', '*', '#', '%', '@'];

    // tape is pipe-friendly — no raw mode, no cursor tricks, no key handling
    #[cfg(unix)]
    let _raw = if !tape { Some(RawMode::enter()) } else { None };

    if !tape { print!("\x1b[?25l"); }

    let mut s = crate::ui::controls::Controls::new(t, step);
    let mut current_seed = seed.to_string();
    let mut env = crate::ui::window::make_env_data_pub(&current_seed);

    // frame lines: 1 header + height grid + 3 wave rows + 1 controls
    let frame_lines = 1 + height + 3 + 1;
    let mut first_frame = true;

    loop {
        // ── input (interactive only — tape is pipe-friendly, ctrl-c to stop) ──
        let keys = if !tape { read_keys() } else { vec![] };
        if !keys.is_empty() {
            match handle_keys(&keys, &mut s) {
                Action::Quit   => break,
                Action::Rewind => { first_frame = true; } // redraw from top
                Action::NewSeed => {
                    let hash = crate::hash_seed(&current_seed);
                    current_seed = crate::generate_seed(hash);
                    env = crate::ui::window::make_env_data_pub(&current_seed);
                    let new_colors = crate::ui::espresso_walk::generate(3, &current_seed, crate::ui::espresso_walk::Palette::Bright);
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

        let mut buf = String::with_capacity(256);

        if tape {
            // ── tape: one data row per step ───────────────────────────────────
            // print header every 32 rows so it stays visible after scrolling
            // column layout (all widths fixed so header and data align exactly):
            // T(12)  seed(8)  gn(5) bits(4) pwr(4) energy(7) ...  dominant
            const SEP: &str = "  ";
            if (s.t / s.step) as u64 % 32 == 0 {
                let _ = write!(buf,
                    "{:>12}{SEP}{:>8}{SEP}{:>8} {:>4} {:>4} {:>7}{SEP}{:>8} {:>4} {:>4} {:>7}{SEP}{:>8} {:>4} {:>4} {:>7}{SEP}{}\n",
                    "T", "seed",
                    "wave0:gn", "bits", "pwr", "energy",
                    "wave1:gn", "bits", "pwr", "energy",
                    "wave2:gn", "bits", "pwr", "energy",
                    "dominant");
                buf.push_str(&"─".repeat(115));
                buf.push('\n');
            }

            let mut wave_data = [(0u32, 0.0f64, 0.0f64, 0u32); 3];
            for i in 0..3usize {
                let drift_freq  = env[i*8 + 5] as f64;
                let drift_phase = env[i*8 + 6] as f64;
                let acc = gen_acc(drift_freq, drift_phase, s.t, noise);
                let gn  = acc.floor() as u32;
                let energy  = wave_energy(i as u32, gn);
                let amp_n   = 0.3 + gen_param(i as u32, gn, 0) * 2.4;
                let freq_n  = 0.4 + gen_param(i as u32, gn, 1) * 1.2;
                let bits    = gen_bits(i as u32, gn);
                let complexity_bonus = 1.0 - ((bits as f64 - 16.0).abs() / 16.0);
                let power = (amp_n * freq_n * 0.4 * (0.5 + 0.5 * complexity_bonus)).clamp(0.0, 1.0);
                wave_data[i] = (gn, power, energy, bits);
            }

            // dominant: active branch with highest power
            let dominant = (0..3usize)
                .filter(|&i| wave_data[i].2 > 0.0)
                .max_by(|&a, &b| wave_data[a].1.partial_cmp(&wave_data[b].1).unwrap());

            let dom_str = match dominant {
                None    => "no signal".to_string(),
                Some(i) => format!("wave{}{}", i, if wave_data[i].1 > 0.95 { " PEAK" } else { "" }),
            };

            // annotation: last signal events
            let mut annotation = String::new();
            let active_count = (0..3).filter(|&i| wave_data[i].2 > 0.0).count();
            if active_count == 0 {
                annotation = " ← no signal".to_string();
                let _ = write!(buf, "{:>12.4e}{SEP}{:>8}{SEP}{}{}\n", s.t, current_seed, dom_str, annotation);
                let mut lock = stdout.lock();
                let _ = lock.write_all(buf.as_bytes());
                let _ = lock.flush();
                break;
            } else {
                for i in 0..3usize {
                    if wave_data[i].2 <= 0.0 && wave_data[i].0 > 0 {
                        let _ = write!(annotation, " ← wave{i} last signal");
                        break;
                    }
                }
            }

            // data row — same widths as header
            let _ = write!(buf, "{:>12.4e}{SEP}{:>8}", s.t, current_seed);
            for i in 0..3usize {
                let (gn, pwr, energy, bits) = wave_data[i];
                let e = if energy > 0.0 { format!("{energy:>+7.3}") } else { " zeroed".to_string() };
                let _ = write!(buf, "{SEP}{:>8} {:>4} {:>4.2} {}", gn, bits, pwr, e);
            }
            let _ = write!(buf, "{SEP}{}{}\n", dom_str, annotation);

        } else {
            // ── headless interactive: full ASCII grid ─────────────────────────
            buf.reserve(width * frame_lines * 30);

            if !first_frame { let _ = write!(buf, "\x1b[{frame_lines}A"); }

            // header
            let status = if s.paused { "PAUSED" } else { "      " };
            let _ = write!(buf,
                "\x1b[2K\r\x1b[2m── T: {:.4e}  seed: {}  noise: {:.3}  step: {:.2}  {status}\x1b[0m\n",
                s.t, current_seed, noise, s.step);

            // grid
            for row in 0..height {
                buf.push_str("\x1b[2K\r");
                for col in 0..width {
                    let uv_x = col as f64 / width  as f64;
                    let uv_y = 1.0 - row as f64 / height as f64;
                    let pos  = [(uv_x * 2.0 - 1.0) * 6.0, (uv_y * 2.0 - 1.0) * 6.0];
                    let [r, g, b] = pixel_rgb(&env, &wave_colors, pos, s.t, noise);
                    let lum = (r as f64 * 0.299 + g as f64 * 0.587 + b as f64 * 0.114) / 255.0;
                    let ch  = ramp[(lum * (ramp.len() - 1) as f64) as usize];
                    let _ = write!(buf, "\x1b[38;2;{r};{g};{b}m{ch}");
                }
                buf.push_str("\x1b[0m\n");
            }

            // branch status bars
            for i in 0..3usize {
                let drift_freq  = env[i*8 + 5] as f64;
                let drift_phase = env[i*8 + 6] as f64;
                let acc     = gen_acc(drift_freq, drift_phase, s.t, noise);
                let gn      = acc.floor() as u32;
                let energy  = wave_energy(i as u32, gn);
                let amp_n   = 0.3 + gen_param(i as u32, gn, 0) * 2.4;
                let freq_n  = 0.4 + gen_param(i as u32, gn, 1) * 1.2;
                let power   = (amp_n * freq_n * 0.4).clamp(0.0, 1.0);
                let [r, g, b] = wave_colors[i].map(|v| (v * 255.0) as u8);
                let state   = if energy > 0.0 { "active" } else { "zeroed" };
                let bar: String = (0..12).map(|j| {
                    if energy <= 0.0 { '─' }
                    else if j < (power * 12.0) as usize { '█' }
                    else { '░' }
                }).collect();
                let _ = write!(buf,
                    "\x1b[2K\r  \x1b[38;2;{r};{g};{b}mW{i}\x1b[0m \
                     gn:{gn:>4}  pwr:{power:.2}  energy:{energy:+.2}  [{bar}]  {state}\n");
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
}

pub fn get_population_metrics(env: &[f32; 24], t: f64, noise: f64) -> [(u32, u32, f64, f64); 3] {
    let mut out = [(0, 0, 0.0, 0.0); 3];
    for i in 0..3usize {
        let drift_freq  = env[i*8 + 5] as f64;
        let drift_phase = env[i*8 + 6] as f64;
        let acc = gen_acc(drift_freq, drift_phase, t, noise);
        let gn  = acc.floor() as u32;
        let energy  = wave_energy(i as u32, gn);
        let amp_n   = 0.3 + gen_param(i as u32, gn, 0) * 2.4;
        let freq_n  = 0.4 + gen_param(i as u32, gn, 1) * 1.2;
        let bits    = gen_bits(i as u32, gn);
        let complexity_bonus = 1.0 - ((bits as f64 - 16.0).abs() / 16.0);
        let power = (amp_n * freq_n * 0.4 * (0.5 + 0.5 * complexity_bonus)).clamp(0.0, 1.0);
        out[i] = (gn, bits, power, energy);
    }
    out
}
