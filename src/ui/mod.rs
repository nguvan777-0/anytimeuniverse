pub mod dew;
pub mod future;
pub mod rect;

/// Shared spacing scale used everywhere via `ui.add_space(SP_*)`.
/// Prefer these over raw literals to keep the layout vocabulary consistent.
pub const GAP_XS:  f32 =  2.0;  // tight gap (between paired widgets / post-transport-btn)
pub const GAP_SM:  f32 =  4.0;  // section separator (between left-panel include blocks)
pub const GAP_MD:  f32 =  8.0;  // medium gap (between major sections / legend rows)
#[allow(dead_code)]
pub const GAP_LG:  f32 = 10.0;  // large gap (e.g. transport → speed slider)

/// Standard square side length for key cap buttons (mute, speed arrows, etc.).
/// Sized to comfortably contain a monospace-24 glyph with padding.
pub const KEY_CAP_SIDE: f32 = 26.0;

/// Extension trait for egui::Response. All interactive widgets in this app MUST
/// finalize their return value with `.hand()` so the pointer cursor is always
/// a hand over clickable elements — this is the single place the policy lives.
pub trait ResponseExt {
    fn hand(self) -> Self;
}

impl ResponseExt for egui::Response {
    #[inline]
    fn hand(self) -> Self {
        self
    }
}

pub mod espresso_walk;
pub mod ascii_render;
pub mod controls;
mod window;

pub use window::run;
pub mod theme;
pub mod widgets;
pub mod metallic;
