/// Shared control state for both UI and headless modes.
/// All mutations live here — UI and headless map their inputs to these methods.

pub struct Controls {
    pub t:      f64,
    pub step:   f64,
    pub paused: bool,
}

impl Controls {
    pub fn new(t: f64, step: f64) -> Self {
        Self { t, step, paused: false }
    }

    pub fn speed_up(&mut self) {
        self.step = (self.step * 2.0).min(10000.0);
    }

    pub fn speed_down(&mut self) {
        self.step = (self.step * 0.5).max(0.01);
    }

    pub fn rewind_fwd(&mut self) {
        self.t += self.step * 100.0;
    }

    pub fn rewind_back(&mut self) {
        self.t = (self.t - self.step * 100.0).max(0.0);
    }

    pub fn rewind(&mut self) {
        self.t = 0.0;
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// 1-indexed preset: 1=0.1, 2=1, 3=10, 4=100, 5=1000
    pub fn preset(&mut self, n: u8) {
        if let Some(&s) = [0.1f64, 1.0, 10.0, 100.0, 1000.0].get(n.saturating_sub(1) as usize) {
            self.step = s;
        }
    }

    pub fn advance(&mut self) {
        if !self.paused {
            self.t += self.step;
        }
    }
}
