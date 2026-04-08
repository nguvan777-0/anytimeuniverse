// Minimal blit shader — copies the cached field texture onto the swap-chain surface.
// No math, just a texture lookup. Runs every frame; the expensive field shader
// (render.wgsl fs_main) only runs when T changes.

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0)       uv:  vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VOut {
    var positions = array<vec2<f32>, 4>(
        vec2(-1.0,  1.0), vec2(-1.0, -1.0),
        vec2( 1.0,  1.0), vec2( 1.0, -1.0),
    );
    var uvs = array<vec2<f32>, 4>(
        vec2(0.0, 0.0), vec2(0.0, 1.0),
        vec2(1.0, 0.0), vec2(1.0, 1.0),
    );
    var out: VOut;
    out.pos = vec4(positions[vi], 0.0, 1.0);
    out.uv  = uvs[vi];
    return out;
}

@group(0) @binding(0) var t_field: texture_2d<f32>;
@group(0) @binding(1) var s_field: sampler;

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    return textureSample(t_field, s_field, in.uv);
}
