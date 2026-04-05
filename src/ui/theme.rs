use egui::{Color32, Context, Painter, Rect, Response, Ui};

pub trait ThemeProvider {
    fn apply_theme(&self, ctx: &Context);
    fn draw_sunken(&self, painter: &Painter, rect: Rect);
    fn section_toggle_btn(&self, ui: &mut Ui) -> Response;
    
    // Abstracted button drawing. Returns a Y-offset push applied to text if any (e.g. 1.0 down on press)
    fn paint_button(&self, ui: &mut Ui, rect: Rect, is_down: bool, is_hovered: bool) -> f32;
    fn button_text_color(&self) -> Color32;
    fn key_cap_text_color(&self) -> Color32;
    fn paint_key_cap(&self, p: &Painter, rect: Rect, is_down: bool, is_hovered: bool);
    fn key_cap_small(&self, ui: &mut Ui, text: &str, side: f32) -> Response;
    fn key_cap_small_rotated(&self, ui: &mut Ui, text: &str, angle: f32, side: f32) -> Response;
    fn collapsible_header(&self, ui: &mut Ui, text: &str, is_open: bool) -> bool;
    
    // The shared math layout lives in widgets.rs, but themes must paint it:
    fn paint_slider_track(&self, ui: &mut Ui, track_rect: Rect, center_x: f32);
    fn paint_slider_thumb(&self, ui: &mut Ui, handle_rect: Rect, is_down: bool, is_hovered: bool);
    fn paint_slider_text(&self, ui: &mut Ui, text: &str);

    fn section_label(&self, ui: &mut Ui, text: &str) -> Response;
    fn text_field_edit(&self, ui: &mut Ui, text: &mut String, font_size: f32, height: f32) -> Response;
}
