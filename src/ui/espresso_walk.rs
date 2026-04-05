//! Espresso-walk: a 3D orbital walk through CIELAB color space (returns a rainbow)

struct Xorshift64(u64);
impl Xorshift64 {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn next_f64(&mut self) -> f64 {
        (self.next() >> 11) as f64 / ((1u64 << 53) as f64)
    }
    /// Box-Muller normal variate
    fn next_normal(&mut self) -> f64 {
        let u1 = self.next_f64().max(1e-10);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

fn unit_vec(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let len = (x * x + y * y + z * z).sqrt();
    if len == 0.0 {
        return (1.0, 0.0, 0.0);
    }
    (x / len, y / len, z / len)
}

fn rotate3d(
    (cx, cy, cz): (f64, f64, f64),
    (px, py, pz): (f64, f64, f64),
    (nx, ny, nz): (f64, f64, f64),
    angle_deg: f64,
) -> (f64, f64, f64) {
    let (px, py, pz) = (px - cx, py - cy, pz - cz);
    let (nx, ny, nz) = unit_vec(nx, ny, nz);
    let rad = angle_deg * std::f64::consts::PI / 180.0;
    let (cos, sin) = (rad.cos(), rad.sin());
    let dot = px * nx + py * ny + pz * nz;
    let (cross_x, cross_y, cross_z) = (ny * pz - nz * py, nz * px - nx * pz, nx * py - ny * px);
    (
        px * cos + cross_x * sin + nx * dot * (1.0 - cos) + cx,
        py * cos + cross_y * sin + ny * dot * (1.0 - cos) + cy,
        pz * cos + cross_z * sin + nz * dot * (1.0 - cos) + cz,
    )
}

fn lab_to_rgb(l: f64, a: f64, b: f64) -> (u8, u8, u8) {
    let fy = (l + 16.0) / 116.0;
    let fx = a / 500.0 + fy;
    let fz = fy - b / 200.0;
    let f = |t: f64| {
        if t.powi(3) > 0.008856 {
            t.powi(3)
        } else {
            (t - 16.0 / 116.0) / 7.787
        }
    };
    let (x, y, z) = (0.95047 * f(fx), 1.00000 * f(fy), 1.08883 * f(fz));
    let rl = x * 3.2406 + y * -1.5372 + z * -0.4986;
    let gl = x * -0.9689 + y * 1.8758 + z * 0.0415;
    let bl = x * 0.0557 + y * -0.2040 + z * 1.0570;
    let gamma = |c: f64| {
        if c > 0.0031308 {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        } else {
            12.92 * c
        }
    };
    let clamp = |v: f64| (v.max(0.0).min(1.0) * 255.0) as u8;
    (clamp(gamma(rl)), clamp(gamma(gl)), clamp(gamma(bl)))
}

/// Which color algorithm `generate` should use.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Palette {
    /// Anchored at espresso-brown, 3-D orbital walk (warm, muted).
    Espresso,
    /// Full-gamut vivid hues evenly spaced around the CIELAB hue circle at high
    /// chroma, with a seed-derived starting angle.
    Bright,
}

/// Generate `n` perceptually-spread colors, deterministic for a given `world_seed`.
pub fn generate(n: usize, world_seed: &str, palette: Palette) -> Vec<egui::Color32> {
    // FNV-1a hash of the seed string for a stable, well-mixed u64
    let seed = world_seed.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x00000100000001b3)
    });
    let mut rng = Xorshift64(seed | 1);

    if palette == Palette::Bright {
        // Flat orbit in the CIELAB a,b plane at full chroma.
        // L varies gently around 65 so colours stay vivid but not identical in brightness.
        // Start at a random hue based on the world seed, then walk the wheel.
        let hue_start = rng.next_f64() * 360.0;
        let chroma    = 58.0 + rng.next_f64() * 10.0; // [58, 68]
        let l_center  = 62.0 + rng.next_f64() * 8.0;  // [62, 70]
        let step = 360.0 / n as f64;
        return (0..n)
            .map(|i| {
                let hue_deg = hue_start + i as f64 * step;
                let hue_rad = hue_deg * std::f64::consts::PI / 180.0;
                let a = chroma * hue_rad.cos();
                let b = chroma * hue_rad.sin();
                // Gentle L wobble so adjacent colours don't look flat
                let l = l_center + 6.0 * (hue_rad * 1.5).sin();
                let (r, g, bv) = lab_to_rgb(l, a, b);
                egui::Color32::from_rgb(r, g, bv)
            })
            .collect();
    }

    let target = (
        45.0 + rng.next_normal() * 5.0,
        12.0 + rng.next_normal() * 3.0,
        18.0 + rng.next_normal() * 4.0,
    );
    let center = (65.0_f64, 0.0_f64, 0.0_f64);
    let axis = (rng.next_normal(), rng.next_normal(), rng.next_normal());

    let anchor_idx = (rng.next() as usize) % n;
    let degrees_per_step = 360.0 / n as f64;

    let start_angle = -(anchor_idx as f64) * degrees_per_step;
    let start = rotate3d(center, target, axis, start_angle);

    (0..n)
        .map(|i| {
            let angle = i as f64 * degrees_per_step;
            let (l, a, b) = rotate3d(center, start, axis, angle);
            let (r, g, bv) = lab_to_rgb(l, a, b);
            egui::Color32::from_rgb(r, g, bv)
        })
        .collect()
}
