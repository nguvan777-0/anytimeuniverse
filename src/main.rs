#![allow(clippy::type_complexity)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::empty_line_after_doc_comments)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
mod engine;
mod ui;

use engine::sim::spawn_sim;
use pollster::block_on;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

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

/// Returns `(background_noise, first_wave_weights)` derived from the seed.
/// `background_noise` scales the gen_acc wiggle in the shader.
/// `first_wave_weights` seed the 12-anchor color projection matrix.
pub fn init_seed_params(seed: &str) -> (f32, [[f32; 14]; 12]) {
    let mut rng = Lcg::new(hash_seed(seed));

    let background_noise = rng.next_f32() * 0.30 + 0.05;

    let mut first_wave_weights = [[0.0f32; 14]; 12];
    for weights in &mut first_wave_weights {
        for w in weights.iter_mut() {
            *w = (rng.next_f32() * 2.0 - 1.0) * 256.0;
        }
    }

    (background_noise, first_wave_weights)
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
        println!("  --ascii             terminal — interactive ASCII renderer");
        println!("  --tape              terminal — scrolling data log, exits on last signal");
        println!();
        println!("  --step <f64>        T units advanced per frame  [ascii: 0.1, tape: 1000.0]");
        println!("  --width <usize>     grid columns                [ascii: 60]");
        println!("  --height <usize>    grid rows                   [ascii: 20]");
        println!("  --t0 <f64>          starting T value            [default: 0.0]");
        println!();
        println!("CONTROLS (--ascii):");
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

    let ascii_mode = cli_args.iter().any(|a| a == "--ascii" || a == "--tape");

    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let seed = generate_seed(time);
    let (background_noise, first_wave_weights) = init_seed_params(&seed);

    // ascii — skip GPU init, banner, and window entirely
    if ascii_mode {
        let wave_colors = ui::espresso_walk::generate(3, &seed, ui::espresso_walk::Palette::Bright);
        let colors: [[f64; 3]; 3] = std::array::from_fn(|i| {
            let c = wave_colors[i];
            [c.r() as f64 / 255.0, c.g() as f64 / 255.0, c.b() as f64 / 255.0]
        });
        ui::ascii_render::run(&seed, background_noise, colors);
        return;
    }

    println!("┌─────────────────────────────────────┐");
    println!(
        "│       anytimeuniverse  v{}       │",
        env!("CARGO_PKG_VERSION")
    );
    println!("│       hardware wave evolution       │");
    println!("│   cross-platform O(1) time travel   │");
    println!("└─────────────────────────────────────┘");
    println!();

    let event_loop = winit::event_loop::EventLoop::<()>::with_user_event()
        .build()
        .expect("failed to create event loop");

    let (instance, adapter, device, queue) = block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("no GPU adapter found");
        println!("[ gpu ] {}", adapter.get_info().name);

        let required_features = wgpu::Features::empty();

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

    let sim_handle = spawn_sim();

    println!("[ world ] seed: {}  noise: {:.3}", seed, background_noise);

    // GPU window — the render loop is self-contained and VSync-paced.
    // T is advanced in the frame callback; the sim thread is just a stats ticker.
    ui::run(
            event_loop,
            instance,
            adapter,
            device,
            queue,
            sim_handle,
            seed,
            background_noise,
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
 
