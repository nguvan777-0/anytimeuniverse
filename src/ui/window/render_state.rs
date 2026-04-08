struct RenderState {
    window:         Arc<Window>,
    surface:        wgpu::Surface<'static>,
    config:         wgpu::SurfaceConfiguration,
    pipeline:       wgpu::RenderPipeline,
    bg:             wgpu::BindGroup,
    tick_buf:       wgpu::Buffer,
    env_buf:        wgpu::Buffer,
    color_buf:      wgpu::Buffer,
    egui_ctx:       egui::Context,
    egui_state:     egui_winit::State,
    egui_renderer:  egui_wgpu::Renderer,
    // Cached offscreen field texture — the expensive fragment shader renders
    // into this; a cheap blit copies it to the swap-chain surface every frame.
    field_texture:  wgpu::Texture,
    field_view:     wgpu::TextureView,
    blit_pipeline:  wgpu::RenderPipeline,
    blit_bg:        wgpu::BindGroup,
    blit_bgl:       wgpu::BindGroupLayout, // kept for resize recreation
    blit_sampler:   wgpu::Sampler,         // kept for resize recreation
    // Pre-baked per-frame wave parameters uploaded to binding 3.
    // Contains all results of fhash/memory/gen_acc/wave_energy/center/velocity
    // so the fragment shader only does position-dependent math (~30 trig vs ~1200).
    wave_cache_buf: wgpu::Buffer,
}
