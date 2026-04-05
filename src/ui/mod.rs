pub mod dew;
pub mod future;
pub mod rect;

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
