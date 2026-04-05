//! Color projection math.
//!
//! Stores the 12 first_wave anchor colors and their original 14D weights.
//! Forks compute their color by taking the delta of their current weights
//! vs their first_wave's weights, and applying that as an RGB shift from the anchor.

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ColorData {
    pub data: [[f32; 4]; 60], // Flat representation to guarantee strict WGSL memory layout matching (12 anchors + 48 vecs of weights)
}
unsafe impl bytemuck::Pod for ColorData {}
unsafe impl bytemuck::Zeroable for ColorData {}

pub fn build(
    first_wave_weights: &[[f32; 14]; 12],
    espresso_colors: &[[f32; 3]; 12],
) -> ColorData {
    let mut flat = [[0.0f32; 4]; 60];

    // Indices 0..11: anchors
    for i in 0..12 {
        flat[i][0] = espresso_colors[i][0];
        flat[i][1] = espresso_colors[i][1];
        flat[i][2] = espresso_colors[i][2];
        flat[i][3] = 0.0;
    }

    // Indices 12..59: weights (4 vecs per first_wave)
    for i in 0..12 {
        for j in 0..14 {
            flat[12 + i * 4 + j / 4][j % 4] = first_wave_weights[i][j];
        }
    }

    ColorData { data: flat }
}

pub fn apply(cd: &ColorData, weights: &[f32; 14], first_wave_id: usize) -> [f32; 3] {
    let id = first_wave_id % 12;
    let anchor = cd.data[id];

    // Hash constants mapped to RGB shifts
    let hash_r = [0.13, -0.07, 0.42, -0.11, 0.28, -0.34, 0.15, -0.09, 0.22, -0.18, 0.05, 0.31, -0.27, 0.19];
    let hash_g = [-0.21, 0.14, -0.33, 0.08, -0.19, 0.25, -0.12, 0.41, -0.06, 0.17, -0.29, 0.03, 0.35, -0.14];
    let hash_b = [0.08, -0.16, 0.22, -0.35, 0.11, -0.04, 0.29, -0.18, 0.31, -0.09, 0.15, -0.24, 0.07, 0.13];

    let mut dr = 0.0;
    let mut dg = 0.0;
    let mut db = 0.0;

    for j in 0..14 {
        let fw_val = cd.data[12 + id * 4 + j / 4][j % 4];
        let delta = (weights[j] - fw_val) * 0.01;
        dr += delta * hash_r[j];
        dg += delta * hash_g[j];
        db += delta * hash_b[j];
    }

    [
        (anchor[0] + dr).clamp(0.0, 1.0),
        (anchor[1] + dg).clamp(0.0, 1.0),
        (anchor[2] + db).clamp(0.0, 1.0)
    ]
}
