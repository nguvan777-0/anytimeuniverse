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
    fn key_cap_small(&self, ui: &mut Ui, text: &str, side: f32, font_size: f32) -> Response;
    fn key_cap_small_rotated(&self, ui: &mut Ui, text: &str, angle: f32, side: f32, font_size: f32) -> Response;
    fn collapsible_header(&self, ui: &mut Ui, text: &str, is_open: bool) -> bool;
    
    // The shared math layout lives in widgets.rs, but themes must paint it:
    fn paint_slider_track(&self, ui: &mut Ui, track_rect: Rect, center_x: f32);
    fn paint_slider_thumb(&self, ui: &mut Ui, handle_rect: Rect, is_down: bool, is_hovered: bool);
    fn paint_slider_text(&self, ui: &mut Ui, text: &str);
    fn paint_slider_gauge(&self, ui: &mut Ui, bg_rect: Rect, fill_rect: Rect, is_down: bool, is_hovered: bool);

    /// Shadow color painted behind gauge labels to keep them readable over fill bars.
    /// Return None to skip the shadow (default).
    fn gauge_label_shadow(&self) -> Option<Color32> { None }
    /// Override text color for gauge labels. None = use button_text_color.
    fn gauge_label_text_color(&self) -> Option<Color32> { None }

    fn section_label(&self, ui: &mut Ui, text: &str) -> Response;
    fn text_field_edit(&self, ui: &mut Ui, text: &mut String, font_size: f32, height: f32) -> Response;

    /// Preferred background color for charts and data displays (e.g. Space Strategy).
    fn chart_bg(&self) -> Color32;

    /// Color used for axis lines in charts (e.g. Space Strategy crosshairs).
    fn chart_axis_color(&self) -> Color32 { Color32::from_white_alpha(30) }
}
