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
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Theme { Rect, Metallic, Dew, Future }

impl Theme {
    pub fn provider(self) -> &'static dyn crate::ui::theme::ThemeProvider {
        match self {
            Theme::Rect   => &crate::ui::rect::Rect,
            Theme::Dew    => &crate::ui::dew::Dew,
            Theme::Future => &crate::ui::future::Future,
            Theme::Metallic => &crate::ui::metallic::Metallic,
        }
    }
}