use egui::{Painter, Pos2, Rect, Color32, Stroke};

pub struct SpaceStrategyEngine {
    // We only recalculate when the user requests a scan
    points: Vec<StrategyPoint>,
    is_scanning: bool,
    baseline_t: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct StrategyPoint {
    pub branch: usize,
    pub generation: u32,
    pub x: f64,
    pub y: f64,
}

impl Default for SpaceStrategyEngine {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            is_scanning: false,
            baseline_t: 0.0,
        }
    }
}

impl SpaceStrategyEngine {
    pub fn is_empty(&self) -> bool {
        self.points.is_empty() && !self.is_scanning
    }

    pub fn scan(&mut self, env: &[f32; 24], noise: f64, start_t: f64, history_gens: u32) {
        self.is_scanning = true;
        self.baseline_t = start_t;
        self.points.clear();

        // 1. Collect Data
        let mut raw_data = Vec::new();
        
        for w in 0..3 {
            let start_gn = crate::ui::ascii_render::get_gn_at_time(env, w, start_t, noise);
            for offset in 0..history_gens {
                if start_gn < offset { break; }
                let gn = start_gn - offset;
                let params = crate::ui::ascii_render::get_params(env, w, gn);
                raw_data.push((w, gn, params));
            }
        }

        if raw_data.is_empty() { return; }

        let n_samples = raw_data.len() as f64;
        let mut means = [0.0; 5];
        for d in &raw_data {
            for j in 0..5 { means[j] += d.2[j]; }
        }
        for j in 0..5 { means[j] /= n_samples; }

        let mut cov = [[0.0f64; 5]; 5];
        for d in &raw_data {
            for r in 0..5 {
                for c in 0..5 {
                    cov[r][c] += (d.2[r] - means[r]) * (d.2[c] - means[c]);
                }
            }
        }
        for r in 0..5 {
            for c in 0..5 { cov[r][c] /= n_samples - 1.0; }
        }

        // 2. Power Iteration for Top 2 Projection Axes
        let axis1 = power_iteration(&cov, [1.0, 0.0, 0.0, 0.0, 0.0], 50);
        let mut cov_deflated = cov;
        let lambda1 = rayleigh_quotient(&cov, &axis1);
        for r in 0..5 {
            for c in 0..5 {
                cov_deflated[r][c] -= lambda1 * axis1[r] * axis1[c];
            }
        }
        let axis2 = power_iteration(&cov_deflated, [0.0, 1.0, 0.0, 0.0, 0.0], 50);

        for d in &raw_data {
            let mut dx = 0.0;
            let mut dy = 0.0;
            for j in 0..5 {
                let centered = d.2[j] - means[j];
                dx += centered * axis1[j];
                dy += centered * axis2[j];
            }
            // Use a fixed absolute scale instead of auto-zooming.
            // Wave parameters like amp and freq vary around 1.0 - 4.0.
            // Multiplying by 0.2 keeps them cleanly on the screen naturally.
            let screen_x = (dx * 0.2).clamp(-1.0, 1.0);
            let screen_y = (dy * 0.2).clamp(-1.0, 1.0);
            
            self.points.push(StrategyPoint { branch: d.0, generation: d.1, x: screen_x, y: screen_y });
        }

        self.is_scanning = false;
    }

    pub fn draw(&self, ui: &mut egui::Ui, rect: Rect, wave_colors: &[Color32], axis_color: Color32) {
        if self.points.is_empty() {
            ui.label("No scan data. Press Scan to begin.");
            return;
        }

        let painter = ui.painter();
        let center = rect.center();
        let scale = (rect.width().min(rect.height()) / 2.0) * 0.9;

        // Draw Axes
        painter.line_segment(
            [Pos2::new(rect.left(), center.y), Pos2::new(rect.right(), center.y)],
            Stroke::new(1.0, axis_color)
        );
        painter.line_segment(
            [Pos2::new(center.x, rect.top()), Pos2::new(center.x, rect.bottom())],
            Stroke::new(1.0, axis_color)
        );

        // Draw points
        for p in &self.points {
            let screen_x = center.x + (p.x as f32 * scale);
            let screen_y = center.y - (p.y as f32 * scale); // Invert Y for UI
            let pos = Pos2::new(screen_x, screen_y);
            let color = wave_colors.get(p.branch).copied().unwrap_or(Color32::WHITE);
            painter.circle_filled(pos, 2.5, color.linear_multiply(0.8));
        }
    }
}

fn power_iteration(cov: &[[f64; 5]; 5], mut vec: [f64; 5], iters: usize) -> [f64; 5] {
    for _ in 0..iters {
        let mut next = [0.0; 5];
        for r in 0..5 {
            for c in 0..5 {
                next[r] += cov[r][c] * vec[c];
            }
        }
        let mag = (next.iter().map(|v| v * v).sum::<f64>()).sqrt().max(1e-9);
        for j in 0..5 { vec[j] = next[j] / mag; }
    }
    vec
}

fn rayleigh_quotient(cov: &[[f64; 5]; 5], vec: &[f64; 5]) -> f64 {
    let mut av = [0.0; 5];
    for r in 0..5 {
        for c in 0..5 { av[r] += cov[r][c] * vec[c]; }
    }
    let mut num = 0.0;
    for j in 0..5 { num += vec[j] * av[j]; }
    num // Denominator is 1.0 since vec is normalized
}
