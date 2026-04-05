mod engine;
mod ui;

use engine::{World, sim::spawn_sim};
use pollster::block_on;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

// 12 first_wave branches — each has one dominant weight channel (4..16)
// that locks in its color. weights can mutate later.
pub const FIRST_WAVES: usize = 12;

pub fn hash_seed(seed: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    hasher.finish()
}

pub fn generate_seed(state: u64) -> String {
    let mut rng = Lcg::new(state);
    rng.next_u32(); // advance PRNG state
    let chars = b"0123456789BCDFGHJKLMNPQRSTVWXYZbcdfghjklmnpqrstvwxyz";
    let mut res = String::with_capacity(8);
    for _ in 0..8 {
        res.push(chars[(rng.next_u32() as usize) % chars.len()] as char);
    }
    res
}

pub fn init_world(seed: &str) -> (World, f32, [[f32; 14]; 12]) {
    let mut w = World::new(1024, 1024);
    let num_seed = hash_seed(seed);

    let mut rng = Lcg::new(num_seed);

    // substrate noise varies per seed — range [0.05, 0.35]
    let substrate_noise = rng.next_f32() * 0.30 + 0.05;

    // seed food randomly across the grid
    for y in 0..1024usize {
        for x in 0..1024usize {
            w.pixel_mut(x, y)[0] = rng.next_f32() * 0.3;
        }
    }

    // spawn 12 first_waves at random positions, saving their weight vectors
    // so the UI can calibrate the color projection matrix.
    let mut first_wave_weights = [[0.0f32; 14]; 12];
    for i in 0..FIRST_WAVES {
        let x = (rng.next_u32() % 1024) as usize;
        let y = (rng.next_u32() % 1024) as usize;
        let px = w.pixel_mut(x, y);
        px[0] = 0.0;   // food — eaten on first tick
        px[1] = 100.0; // energy (0..100 scale)
        px[2] = rng.next_f32() * 2.0 * std::f32::consts::PI; // heading (radians)
        px[3] = 0.0;   // accumulator — starts empty
        
        // All 14 weight channels uniformly ±256.
        // Speed/spin formulas in shader.wgsl are scaled to expect this range.
        for c in 4..18 {
            let v = (rng.next_f32() * 2.0 - 1.0) * 256.0;
            px[c] = v;
            first_wave_weights[i][c - 4] = v;
        }

        // Store true descent ID in the new channel 18
        px[18] = i as f32;
    }
    (w, substrate_noise, first_wave_weights)
}

fn main() {
    let cli_args: Vec<String> = std::env::args().collect();

    if cli_args.iter().any(|a| a == "--help" || a == "-h") {
        println!("anytimeuniverse v{}", env!("CARGO_PKG_VERSION"));
        println!();
        println!("USAGE:");
        println!("  cargo run [-- OPTIONS]");
        println!();
        println!("OPTIONS:");
        println!("  (none)              GPU window — full interference field, real-time");
        println!("  --headless          terminal — interactive ASCII renderer");
        println!("  --tape              terminal — scrolling data log, exits on last signal");
        println!();
        println!("  --step <f64>        T units advanced per frame  [headless: 10.0, tape: 1000.0]");
        println!("  --width <usize>     grid columns                [headless: 60]");
        println!("  --height <usize>    grid rows                   [headless: 20]");
        println!("  --t0 <f64>          starting T value            [default: 0.0]");
        println!();
        println!("CONTROLS (--headless):");
        println!("  space               pause / resume");
        println!("  ← →                 rewind backward / forward in T");
        println!("  ↑ ↓                 speed up / slow down step");
        println!("  1–5                 step presets: 0.1, 1, 10, 100, 1000");
        println!("  r                   rewind to T=0");
        println!("  c                   new seed");
        println!("  q / ESC / ctrl-c    quit");
        println!();
        println!("PIPE (--tape):");
        println!("  cargo run -- --tape | grep last-signal");
        println!("  cargo run -- --tape --step 500 | tee run.log");
        return;
    }

    let headless = cli_args.iter().any(|a| a == "--headless" || a == "--tape");

    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let seed = generate_seed(time);
    let (w, substrate_noise, first_wave_weights) = init_world(&seed);

    // headless — skip GPU init, banner, and window entirely
    if headless {
        let wave_colors = ui::espresso_walk::generate(3, &seed, ui::espresso_walk::Palette::Bright);
        let colors: [[f64; 3]; 3] = std::array::from_fn(|i| {
            let c = wave_colors[i];
            [c.r() as f64 / 255.0, c.g() as f64 / 255.0, c.b() as f64 / 255.0]
        });
        ui::ascii_render::run(&seed, substrate_noise, colors);
        return;
    }

    println!("┌─────────────────────────────────────┐");
    println!(
        "│       anytimeuniverse  v{}       │",
        env!("CARGO_PKG_VERSION")
    );
    println!("│         hardware evolution          │");
    println!("│        cross-platform · GPU         │");
    println!("└─────────────────────────────────────┘");
    println!();

    let event_loop = winit::event_loop::EventLoop::<()>::with_user_event()
        .build()
        .expect("failed to create event loop");
    let proxy = event_loop.create_proxy();

    let (instance, adapter, device, queue) = block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("no GPU adapter found");
        println!("[ gpu ] {}", adapter.get_info().name);

        let mut required_features = wgpu::Features::empty();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features,
                    ..Default::default()
                },
            )
            .await
            .expect("failed to get GPU device");

        (instance, adapter, device, queue)
    });

    let device = Arc::new(device);
    let queue = Arc::new(queue);

    let sim_handle = spawn_sim(proxy);

    println!(
        "[ world ] pixelate {} wights on {}x{} grid (seed: {}, noise: {:.3})",
        FIRST_WAVES, 1024, 1024, seed, substrate_noise
    );

    // UI mode (default). The sim thread posts user_events to drive redraws;
    // if max_ticks is set it calls std::process::exit(0) when done.
    ui::run(
            event_loop,
            instance,
            adapter,
            device,
            queue,
            sim_handle,
            seed,
            substrate_noise,
            first_wave_weights,
        );
}

// simple LCG for deterministic seeding — no extra crate needed
pub struct Lcg(u64);

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }
    pub fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    pub fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }
}
