use egui::{Color32, Context, Painter, Rect, Response, Ui};

#[derive(Clone, Copy, Debug)]
pub struct ThemePalette {
    pub is_terminal_style: bool,
    pub panel_margin: f32,
    pub panel_text_color: egui::Color32,
    pub hash_stat_color: egui::Color32,
    pub hash_selection_color: egui::Color32,
    pub title_bar_text_color: egui::Color32,
    pub title_bar_button_color: egui::Color32,
    pub tracker_color: egui::Color32,
    pub chart_axis_color: egui::Color32,
    pub remove_tracker_border_on_hover: bool,
}

pub trait ThemeProvider {
    fn palette(&self) -> ThemePalette;
    fn apply_theme(&self, ctx: &Context);
    fn draw_sunken(&self, painter: &Painter, rect: Rect);
    
    /// Optional specific background element for Space Strategy chart (used to draw the digital grid in Future theme)
    fn draw_space_strategy_bg(&self, ui: &mut Ui, rect: Rect) {
        self.draw_sunken(ui.painter(), rect);
    }

    fn section_toggle_btn(&self, ui: &mut Ui) -> Response;

    /// Per-frame early setup (useful if visual overrides need to be enforced every frame)
    fn setup_frame(&self, _ctx: &Context) {}

    /// Extra border around the central simulation area (used by Rect theme)
    fn paint_sim_area_border(&self, _ui: &mut Ui, _sim_rect: Rect) {}

    /// Optional extra padding outside the sim field to push it away from side panels
    fn sim_area_padding(&self) -> f32 { 0.0 }

    // ── Application UI Configuration & Hooks ─────────────────────────────────

    


    /// Draw the generic full-panel background texture (e.g. scan lines, stripes)
    fn draw_background_pattern(&self, _painter: &Painter, _rect: Rect) {}

    /// Mutate popup & dropdown visuals if the theme needs custom hover states
    fn edit_popup_visuals(&self, _visuals: &mut egui::Visuals) {}
    
    /// Mutate popup spacing if the theme needs tighter paddings (e.g. Rect theme menus)
    fn edit_popup_spacing(&self, _spacing: &mut egui::Spacing) {}

    /// Moment hash bar background box paint
    fn paint_hash_bg(&self, p: &Painter, rect: Rect) {
        p.rect_filled(rect, rect.height() / 2.0, Color32::from_rgba_premultiplied(0, 0, 0, 12));
    }
    
    /// Paint the copy button inside the hash bar
    fn paint_hash_copy_btn(&self, ui: &mut Ui, btn_rect: Rect, is_down: bool, is_hovered: bool) -> f32 {
        let clipped_btn_rect = btn_rect.shrink2(egui::vec2(1.0, 1.0));
        self.paint_button(ui, clipped_btn_rect, is_down, is_hovered)
    }

    /// Custom title bar background painting (like Dew stripes)
    fn paint_title_bar_bg(&self, _ui: &mut Ui, _rect: Rect) {}
    
    /// Custom title text background (like Future black inset)
    fn paint_title_bar_text_bg(&self, _ui: &mut Ui, _rect: Rect) {}
    


    /// Hover-reactive paint call for window buttons
    fn paint_title_bar_button(&self, _ui: &mut Ui, _resp: &Response, _r: f32, _base_color: Color32, _symbol: &str, _hover_t: f32) {}

    // ── Shared Widgets ───────────────────────────────────────────────────────
    
    // Abstracted button drawing. Returns a Y-offset push applied to text if any (e.g. 1.0 down on press)
    fn paint_button(&self, ui: &mut Ui, rect: Rect, is_down: bool, is_hovered: bool) -> f32;
    fn button_text_color(&self) -> Color32;
    fn key_cap_small(&self, ui: &mut Ui, text: &str, side: f32, font_size: f32, is_pressed: bool) -> Response;
    fn key_cap_small_rotated(&self, ui: &mut Ui, text: &str, angle: f32, side: f32, font_size: f32, is_pressed: bool) -> Response;
    
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




}
