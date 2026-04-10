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

/// Binary-search the maximum chroma that stays within the sRGB gamut at the
/// given lightness and hue. The LAB gamut is wildly non-cylindrical — cyan
/// and blue have far lower max chroma than red or yellow at the same lightness.
fn max_chroma_in_gamut(l: f64, h_deg: f64) -> f64 {
    let h_rad = h_deg.to_radians();
    let in_gamut = |c: f64| {
        let a = c * h_rad.cos();
        let b = c * h_rad.sin();
        let fy = (l + 16.0) / 116.0;
        let fx = a / 500.0 + fy;
        let fz = fy - b / 200.0;
        let f = |t: f64| if t.powi(3) > 0.008856 { t.powi(3) } else { (t - 16.0/116.0) / 7.787 };
        let (x, y, z) = (0.95047 * f(fx), f(fy), 1.08883 * f(fz));
        let r = x *  3.2406 + y * -1.5372 + z * -0.4986;
        let g = x * -0.9689 + y *  1.8758 + z *  0.0415;
        let b = x *  0.0557 + y * -0.2040 + z *  1.0570;
        (0.0..=1.0).contains(&r) && (0.0..=1.0).contains(&g) && (0.0..=1.0).contains(&b)
    };
    let (mut lo, mut hi) = (0.0f64, 200.0f64);
    for _ in 0..24 { // 24 iterations → precision < 0.01
        let mid = (lo + hi) / 2.0;
        if in_gamut(mid) { lo = mid; } else { hi = mid; }
    }
    lo
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
    let clamp = |v: f64| (v.clamp(0.0, 1.0) * 255.0) as u8;
    (clamp(gamma(rl)), clamp(gamma(gl)), clamp(gamma(bl)))
}

/// Returns (L, C, H) per wave for a seed — the palette identity used as the
/// base for `params_to_color`. Extracted so callers can store it without
/// going through Color32 (which would lose precision).
pub fn seed_lch(world_seed: &str, n: usize) -> Vec<(f64, f64, f64)> {
    let seed = world_seed.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x00000100000001b3)
    });
    let mut rng = Xorshift64(seed | 1);

    // 1. Randomly pick a starting color
    let mut l = 10.0 + rng.next_f64() * 80.0; // 10..90 Lightness
    let mut c = 10.0 + rng.next_f64() * 90.0; // 10..100 Chroma
    let mut h = rng.next_f64() * 360.0;       // 0..360 Hue

    // 2. Pick a random direction / velocity
    // This defines how much the color walks per wave in the sequence
    let mut dir_l = (rng.next_f64() - 0.5) * 30.0; 
    let mut dir_c = (rng.next_f64() - 0.5) * 30.0;
    let dir_h = (rng.next_f64() - 0.5) * 120.0; 

    // 3. Take the random walk
    (0..n).map(|_| {
        // Yield the current color, ensuring it fits inside the physical limits of the screen
        let max_c = max_chroma_in_gamut(l, h);
        let result = (l, c.min(max_c), h);

        // Step forward in the chosen direction with a small organic jitter
        l += dir_l + (rng.next_f64() - 0.5) * 5.0;
        c += dir_c + (rng.next_f64() - 0.5) * 5.0;
        h = (h + dir_h + (rng.next_f64() - 0.5) * 10.0).rem_euclid(360.0);

        // Bounce if we hit the top/bottom boundaries so the walk doesn't get stuck at pure white/black
        if !(5.0..=95.0).contains(&l) {
            dir_l *= -1.0; // Reverse direction
            l = l.clamp(5.0, 95.0);
        }
        if !(0.0..=120.0).contains(&c) {
            dir_c *= -1.0;
            c = c.clamp(0.0, 120.0);
        }

        result
    }).collect()
}

/// Wave color from params.
/// `base` is the seed's LCH palette identity.
/// `params0` is the params at gn=0 — subtracting it centers the drift so t=0
/// shows the seed's color. As gn evolves the diff covers ±1 → full gamut.
pub fn params_to_color(base: (f64, f64, f64), params: [f64; 5], params0: [f64; 5]) -> egui::Color32 {
    let (base_l, base_c, base_h) = base;
    let h = (base_h + (params[2] - params0[2]).to_degrees()).rem_euclid(360.0);
    let c = (base_c + (params[3] - params0[3]) * 90.0).clamp(0.0, 90.0);
    let l = (base_l + (params[4] - params0[4]) * 90.0).clamp(5.0, 95.0);
    let h_rad = h.to_radians();
    let a = c * h_rad.cos();
    let b = c * h_rad.sin();
    let (r, g, bv) = lab_to_rgb(l, a, b);
    egui::Color32::from_rgb(r, g, bv)
}

#[allow(dead_code)]
pub fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 { (c, x, 0.0) }
    else if h < 120.0 { (x, c, 0.0) }
    else if h < 180.0 { (0.0, c, x) }
    else if h < 240.0 { (0.0, x, c) }
    else if h < 300.0 { (x, 0.0, c) }
    else { (c, 0.0, x) };

    (((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8)
}

/// Which color algorithm `generate` should use.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Palette {
    /// Anchored at espresso-brown, 3-D orbital walk (warm, muted).
    #[allow(dead_code)]
    Espresso,
    /// Full-gamut vivid hues evenly spaced around the CIELAB hue circle at high
    /// chroma, with a seed-derived starting angle.
    Wide,
}

/// Generate `n` perceptually-spread colors, deterministic for a given `world_seed`.
pub fn generate(n: usize, world_seed: &str, palette: Palette) -> Vec<egui::Color32> {
    // FNV-1a hash of the seed string for a stable, well-mixed u64
    let seed = world_seed.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x00000100000001b3)
    });
    let mut rng = Xorshift64(seed | 1);

    if palette == Palette::Wide {
        return seed_lch(world_seed, n)
            .into_iter()
            .map(|(l, c, h)| {
                let h_rad = h.to_radians();
                let a = c * h_rad.cos();
                let b = c * h_rad.sin();
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
