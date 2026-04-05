use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::thread;
use std::time::{Duration, Instant};

use super::triple_buffer::TripleBuffer;

// ── Public types (kept compatible with window.rs) ────────────────────────────

/// CPU representation of the PCA projection axes — kept for API compatibility.
#[derive(Clone, Copy)]
pub struct PcaAxesData {
    pub pc1:   [f32; 14],
    pub pc2:   [f32; 14],
    pub mean:  [f32; 14],
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub color_data: crate::engine::color_math::ColorData,
}

#[derive(Clone)]
pub struct Stats {
    pub tick: u32,
    /// Empty in analytical wave mode — reserved for future creature census.
    pub color_counts: Vec<u32>,
    /// None in analytical wave mode.
    pub pca_density: Option<Vec<u32>>,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            tick: 0,
            color_counts: Vec::new(),
            pca_density: None,
        }
    }
}

pub enum Command {
    SetSpeed(Duration),
    Pause,
    Resume,
    Reset(Vec<f32>, f32),
    /// No-op in analytical mode — kept for window.rs API compatibility.
    SetPcaAxes(PcaAxesData),
}

pub struct SimHandle {
    pub stats_buffer: Arc<TripleBuffer<Stats>>,
    pub ui_requested_frame: Arc<std::sync::atomic::AtomicBool>,
    pub cmd_tx: Sender<Command>,
}

// ── Spawn ─────────────────────────────────────────────────────────────────────

/// Spawns a lightweight stats thread.  No GPU compute — all visuals are
/// generated analytically in the fragment shader using wave_time T.
pub fn spawn_sim(
    proxy: winit::event_loop::EventLoopProxy<()>,
) -> SimHandle {
    let stats_buffer = TripleBuffer::new();
    let ui_requested_frame = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let stats_buf_clone = stats_buffer.clone();
    let (cmd_tx, cmd_rx) = channel();

    thread::spawn(move || {
        run_sim(cmd_rx, stats_buf_clone, proxy);
    });

    SimHandle { stats_buffer, ui_requested_frame, cmd_tx }
}

// ── Sim thread ────────────────────────────────────────────────────────────────

fn run_sim(
    cmd_rx: Receiver<Command>,
    stats_buffer: Arc<TripleBuffer<Stats>>,
    proxy: winit::event_loop::EventLoopProxy<()>,
) {
    let mut tick = 0u32;
    let mut is_paused = false;
    // Default: 1 tick / 66 ms (~15 tps) — wave_time speed is controlled separately in UI.
    let mut _speed_limit = Duration::from_millis(66);
    let display_interval = Duration::from_millis(16); // ~60 fps stats push
    let mut last_push = Instant::now();

    loop {
        // Drain the command queue.
        loop {
            match cmd_rx.try_recv() {
                Ok(Command::SetSpeed(s)) => _speed_limit = s,
                Ok(Command::Pause)       => is_paused = true,
                Ok(Command::Resume)      => is_paused = false,
                Ok(Command::Reset(_, _)) => { tick = 0; }
                Ok(Command::SetPcaAxes(_)) => {} // no-op
                Err(TryRecvError::Empty)        => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        if is_paused {
            thread::sleep(Duration::from_millis(16));
            continue;
        }

        tick = tick.wrapping_add(1);

        // Push a stats snapshot at ~60 fps so the UI tick counter advances.
        if last_push.elapsed() >= display_interval {
            last_push = Instant::now();
            {
                let stat = stats_buffer.write();
                stat.tick = tick;
                // color_counts / pca_density intentionally empty —
                // creature census can be added here analytically later.
                stats_buffer.publish();
            }
            let _ = proxy.send_event(());
        }

        // Sleep a fixed 1 ms to avoid CPU spinning; the render rate is
        // entirely governed by VSync in the UI thread.
        thread::sleep(Duration::from_millis(1));
    }
}
