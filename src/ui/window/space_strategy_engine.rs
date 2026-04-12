use egui::{Pos2, Rect, Color32, Stroke};

pub struct SpaceStrategyEngine {
    points: Vec<StrategyPoint>,
    is_scanning: bool,
    baseline_t: f64,
    rotation: [[f32; 3]; 3],
    ctrl: [f32; 2],       // joystick in unit disk: direction + magnitude = spin axis + speed
    ctrl_saved: [f32; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct StrategyPoint {
    pub branch: usize,
    #[allow(dead_code)]
    pub generation: u64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Default for SpaceStrategyEngine {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            is_scanning: false,
            baseline_t: 0.0,
            rotation: mat_mul(&rot_x(0.21), &rot_y(-0.42)),
            ctrl: [0.25, 0.0],
            ctrl_saved: [0.25, 0.0],
        }
    }
}

impl SpaceStrategyEngine {
    pub fn reset_view(&mut self) {
        self.rotation = mat_mul(&rot_x(0.21), &rot_y(-0.42));
        self.ctrl_saved = [0.25, 0.0];
        self.ctrl = [0.25, 0.0];
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty() && !self.is_scanning
    }

    pub fn scan(&mut self, env: &[f32; 24], noise: f64, start_t: f64, history_gens: u32) {
        self.is_scanning = true;
        self.baseline_t = start_t;
        self.points.clear();

        let mut raw_data = Vec::new();
        for w in 0..3 {
            let start_gn = crate::ui::ascii_render::get_gn_at_time(env, w, start_t, noise);
            for offset in 0..history_gens {
                if start_gn < offset as u64 { break; }
                let gn = start_gn - offset as u64;
                let params = crate::ui::ascii_render::get_params(env, w, gn);
                raw_data.push((w, gn, params));
            }
        }
        if raw_data.is_empty() { return; }

        let n_samples = raw_data.len() as f64;
        let mut means = [0.0; 5];
        for d in &raw_data { for j in 0..5 { means[j] += d.2[j]; } }
        for j in 0..5 { means[j] /= n_samples; }

        let mut cov = [[0.0f64; 5]; 5];
        for d in &raw_data {
            for r in 0..5 { for c in 0..5 {
                cov[r][c] += (d.2[r] - means[r]) * (d.2[c] - means[c]);
            }}
        }
        for r in 0..5 { for c in 0..5 { cov[r][c] /= n_samples - 1.0; } }

        let axis1 = power_iteration(&cov, [1.0, 0.0, 0.0, 0.0, 0.0], 50);
        let lambda1 = rayleigh_quotient(&cov, &axis1);
        let mut cov2 = cov;
        for r in 0..5 { for c in 0..5 { cov2[r][c] -= lambda1 * axis1[r] * axis1[c]; } }

        let axis2 = power_iteration(&cov2, [0.0, 1.0, 0.0, 0.0, 0.0], 50);
        let lambda2 = rayleigh_quotient(&cov2, &axis2);
        let mut cov3 = cov2;
        for r in 0..5 { for c in 0..5 { cov3[r][c] -= lambda2 * axis2[r] * axis2[c]; } }
        let axis3 = power_iteration(&cov3, [0.0, 0.0, 1.0, 0.0, 0.0], 50);

        for d in &raw_data {
            let mut dx = 0.0f64; let mut dy = 0.0f64; let mut dz = 0.0f64;
            for j in 0..5 {
                let c = d.2[j] - means[j];
                dx += c * axis1[j]; dy += c * axis2[j]; dz += c * axis3[j];
            }
            self.points.push(StrategyPoint {
                branch: d.0, generation: d.1,
                x: (dx * 0.2).clamp(-1.0, 1.0),
                y: (dy * 0.2).clamp(-1.0, 1.0),
                z: (dz * 0.2).clamp(-1.0, 1.0),
            });
        }
        self.is_scanning = false;
    }

    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        response: &egui::Response,
        wave_colors: &[Color32],
        _axis_color: Color32,
        theme: &dyn crate::ui::theme::ThemeProvider,
    ) {
        if self.points.is_empty() {
            ui.label("No scan data.");
            return;
        }

        let center = rect.center();

        // ── Trackball input ──────────────────────────────────────────────────
        // The whole plot is the control surface.
        // Dragging directly updates the rotational velocity array (throwing and grabbing).
        if response.dragged() {
            let delta = response.drag_delta();
            // Store the instantaneous drag velocity
            self.ctrl = [delta.x, delta.y];
        }

        // Click (no meaningful drag) — toggle pause / resume (catch the sphere)
        if response.clicked() {
            let stopped = vec2_len(self.ctrl) < 0.2;
            if stopped {
                self.ctrl = self.ctrl_saved;
            } else {
                self.ctrl_saved = self.ctrl;
                self.ctrl = [0.0, 0.0];
            }
        }

        // ── Apply rotation ───────────────────────────────────────────────────
        // Translate the current momentum into axis/angle rotation (trackball style)
        let sensitivity = 0.01; 
        let omega = [self.ctrl[1] * sensitivity, self.ctrl[0] * sensitivity, 0.0];
        let speed = vec_len(omega);
        if speed > 1e-4 {
            let axis = [omega[0]/speed, omega[1]/speed, omega[2]/speed];
            self.rotation = mat_mul(&axis_angle_mat(axis, speed), &self.rotation);
            ui.ctx().request_repaint();
        }

        // ── Point cloud ──────────────────────────────────────────────────────
        let painter = ui.painter();
        let scale = (rect.width().min(rect.height()) / 2.0) * 0.85;

        let project = |x: f32, y: f32, z: f32| -> (Pos2, f32) {
            let r = mat_vec(&self.rotation, [x, y, z]);
            (Pos2::new(center.x + r[0] * scale, center.y - r[1] * scale), r[2])
        };

        // Axis stubs - Classic analog RGB
        let axis_tips: [([f32; 3], Color32); 3] = [
            ([0.75, 0.0,  0.0], Color32::from_rgb(200, 80,  80)),
            ([0.0,  0.75, 0.0], Color32::from_rgb(60,  160, 60)),
            ([0.0,  0.0,  0.75], Color32::from_rgb(60, 100, 200)),
        ];
        let (origin, _) = project(0.0, 0.0, 0.0);
        for (tip, color) in &axis_tips {
            let (tp, _) = project(tip[0], tip[1], tip[2]);
            painter.line_segment([origin, tp], Stroke::new(1.0, *color));
        }

        // Sort back-to-front
        let mut projected: Vec<(usize, Pos2, f32)> = self.points.iter().enumerate().map(|(i, p)| {
            let (pos, depth) = project(p.x as f32, p.y as f32, p.z as f32);
            (i, pos, depth)
        }).collect();
        projected.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

        for (i, pos, depth) in projected {
            let p = &self.points[i];
            let color = wave_colors.get(p.branch).copied().unwrap_or(theme.palette().tracker_color);
            let alpha = ((depth + 1.5) / 3.0).clamp(0.4, 1.0);
            painter.circle_filled(pos, 2.5, color.linear_multiply(alpha));
        }

        // ── Reset Tracker — bottom-left ──────────────────────────────────────
        const IND_R: f32 = 14.0;
        let ind_center = Pos2::new(rect.left() + IND_R + 8.0, rect.bottom() - IND_R - 8.0);
        let text_color = theme.palette().tracker_color;
        let ind_color = text_color.linear_multiply(0.6);
        let hover_color = text_color;

        // Clickable area to reset orientation
        let center_dot_resp = ui.interact(
            egui::Rect::from_center_size(ind_center, egui::vec2(IND_R * 2.4, IND_R * 2.4)),
            ui.id().with("ind_reset"),
            egui::Sense::click(),
        );
        
        if center_dot_resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let bg_color = text_color.linear_multiply(0.05);

        let stroke = if center_dot_resp.hovered() { 
            if theme.palette().remove_tracker_border_on_hover {
                egui::Stroke::NONE
            } else {
                egui::Stroke::new(1.0, hover_color) 
            }
        } else { 
            egui::Stroke::new(1.0, ind_color) 
        };

        // Draw a subtle background for the tracker
        painter.circle_filled(ind_center, IND_R, bg_color);
        painter.circle_stroke(ind_center, IND_R, stroke);

        if center_dot_resp.clicked() {
            self.rotation = mat_mul(&rot_x(0.21), &rot_y(-0.42));
            self.ctrl_saved = [0.25, 0.0];
            self.ctrl = [0.25, 0.0];
        }

        // Space out the miniature trackball
        let mini_scale = IND_R * 0.85;
        let mini_project = |x: f32, y: f32, z: f32| -> (Pos2, f32) {
            let r = mat_vec(&self.rotation, [x, y, z]);
            (Pos2::new(ind_center.x + r[0] * mini_scale, ind_center.y - r[1] * mini_scale), r[2])
        };

        // We use (0, 0, 1) as our "marker pole" so the dot visibly orbits
        let (orbit_pos, depth) = mini_project(0.0, 0.0, 1.0);
        
        let alpha = ((depth + 1.0) / 2.0).clamp(0.2, 1.0);
        
        // Theme the dot dynamically: use the primary plot color or the theme's tracker text color
        let base_dot_color = wave_colors.first().copied().unwrap_or(theme.palette().tracker_color);
        let dot_color = base_dot_color.linear_multiply(alpha);
        
        let is_paused = vec2_len(self.ctrl) < 0.05;

        // Draw the orbiting trackball dot and center icon with proper depth sorting
        let dot_radius = 2.5 + (depth * 1.5);
        let center_icon_color = text_color.linear_multiply(0.7);

        if is_paused {
            painter.circle_filled(orbit_pos, dot_radius, dot_color);
            painter.line_segment([ind_center + egui::vec2(-1.5, -2.0), ind_center + egui::vec2(-1.5, 2.0)], Stroke::new(1.0, center_icon_color));
            painter.line_segment([ind_center + egui::vec2(1.5, -2.0), ind_center + egui::vec2(1.5, 2.0)], Stroke::new(1.0, center_icon_color));
        } else {
            if depth < 0.0 {
                // Orbiting dot is in the back; draw it first
                painter.circle_filled(orbit_pos, dot_radius, dot_color);
                painter.circle_filled(ind_center, 1.0, center_icon_color);
            } else {
                // Orbiting dot is in the front; draw inner dot first
                painter.circle_filled(ind_center, 1.0, center_icon_color);
                painter.circle_filled(orbit_pos, dot_radius, dot_color);
            }
        }
    }
}

// ── Math helpers ─────────────────────────────────────────────────────────────

fn mat_mul(a: &[[f32; 3]; 3], b: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let mut r = [[0f32; 3]; 3];
    for i in 0..3 { for j in 0..3 { for k in 0..3 {
        r[i][j] += a[i][k] * b[k][j];
    }}}
    r
}

fn mat_vec(m: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0]*v[0] + m[0][1]*v[1] + m[0][2]*v[2],
        m[1][0]*v[0] + m[1][1]*v[1] + m[1][2]*v[2],
        m[2][0]*v[0] + m[2][1]*v[1] + m[2][2]*v[2],
    ]
}

fn rot_y(a: f32) -> [[f32; 3]; 3] {
    let (s, c) = a.sin_cos();
    [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]]
}

fn rot_x(a: f32) -> [[f32; 3]; 3] {
    let (s, c) = a.sin_cos();
    [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]]
}

fn axis_angle_mat(axis: [f32; 3], angle: f32) -> [[f32; 3]; 3] {
    let (x, y, z) = (axis[0], axis[1], axis[2]);
    let (s, c) = angle.sin_cos();
    let t = 1.0 - c;
    [
        [t*x*x + c,   t*x*y - s*z, t*x*z + s*y],
        [t*x*y + s*z, t*y*y + c,   t*y*z - s*x],
        [t*x*z - s*y, t*y*z + s*x, t*z*z + c  ],
    ]
}

fn vec_len(v: [f32; 3]) -> f32 {
    (v[0]*v[0] + v[1]*v[1] + v[2]*v[2]).sqrt()
}

fn vec2_len(v: [f32; 2]) -> f32 {
    (v[0]*v[0] + v[1]*v[1]).sqrt()
}

fn power_iteration(cov: &[[f64; 5]; 5], mut vec: [f64; 5], iters: usize) -> [f64; 5] {
    for _ in 0..iters {
        let mut next = [0.0; 5];
        for r in 0..5 { for c in 0..5 { next[r] += cov[r][c] * vec[c]; } }
        let mag = (next.iter().map(|v| v * v).sum::<f64>()).sqrt().max(1e-9);
        for j in 0..5 { vec[j] = next[j] / mag; }
    }
    vec
}

fn rayleigh_quotient(cov: &[[f64; 5]; 5], vec: &[f64; 5]) -> f64 {
    let mut av = [0.0; 5];
    for r in 0..5 { for c in 0..5 { av[r] += cov[r][c] * vec[c]; } }
    let mut num = 0.0;
    for j in 0..5 { num += vec[j] * av[j]; }
    num
}
