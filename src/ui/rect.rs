//! Rectangle theme

use egui::{Color32, Stroke, CornerRadius, Margin, Vec2, Context, Visuals, Painter, Response, Ui, FontId, Sense};
use crate::ui::{GAP_MD, GAP_SM, GAP_XS, ResponseExt};

const TERM_BG:     Color32 = Color32::BLACK;
const TERM_GREEN:  Color32 = Color32::from_rgb(0,   230, 65);

pub fn apply_theme(ctx: &Context) {
    let mut style = (*ctx.global_style()).clone();
    let z = CornerRadius::ZERO;

    let mut visuals = Visuals::dark();
    visuals.window_fill        = TERM_BG;
    visuals.panel_fill         = TERM_BG;
    visuals.window_stroke      = Stroke::new(1.0, TERM_GREEN);
    visuals.popup_shadow       = egui::Shadow::NONE;
    visuals.window_shadow      = egui::Shadow::NONE;
    visuals.extreme_bg_color   = Color32::from_rgb(5, 5, 5);
    visuals.faint_bg_color     = Color32::from_rgb(20, 20, 20);
    visuals.code_bg_color      = Color32::from_rgb(5, 5, 5);
    visuals.text_cursor.stroke = Stroke::new(2.0, TERM_GREEN);
    visuals.window_corner_radius = z;
    visuals.menu_corner_radius   = z;

    visuals.widgets.noninteractive.bg_fill      = egui::Color32::TRANSPARENT;
    visuals.widgets.noninteractive.weak_bg_fill  = TERM_BG;
    visuals.widgets.noninteractive.bg_stroke     = Stroke::new(1.0, TERM_GREEN);
    visuals.widgets.noninteractive.fg_stroke     = Stroke::new(1.0, TERM_GREEN);
    visuals.widgets.noninteractive.corner_radius = z;

    visuals.widgets.inactive.bg_fill      = egui::Color32::TRANSPARENT;
    visuals.widgets.inactive.weak_bg_fill  = TERM_BG;
    visuals.widgets.inactive.bg_stroke     = Stroke::new(1.0, TERM_GREEN);
    visuals.widgets.inactive.fg_stroke     = Stroke::new(1.0, TERM_GREEN);
    visuals.widgets.inactive.corner_radius = z;

    visuals.widgets.hovered.bg_fill       = TERM_GREEN;
    visuals.widgets.hovered.weak_bg_fill  = TERM_GREEN;
    visuals.widgets.hovered.bg_stroke     = Stroke::new(1.0, TERM_GREEN);
    visuals.widgets.hovered.fg_stroke     = Stroke::new(1.0, TERM_GREEN);
    visuals.widgets.hovered.corner_radius = z;
    visuals.widgets.hovered.expansion     = 0.0;

    visuals.widgets.active.bg_fill      = TERM_GREEN;
    visuals.widgets.active.weak_bg_fill  = TERM_GREEN;
    visuals.widgets.active.bg_stroke     = Stroke::new(1.0, TERM_GREEN);
    visuals.widgets.active.fg_stroke     = Stroke::new(1.0, TERM_GREEN);
    visuals.widgets.active.corner_radius = z;
    visuals.widgets.active.expansion     = 0.0;

    visuals.widgets.open.bg_fill      = TERM_GREEN;
    visuals.widgets.open.weak_bg_fill  = TERM_GREEN;
    visuals.widgets.open.bg_stroke     = Stroke::new(1.0, TERM_GREEN);
    visuals.widgets.open.fg_stroke     = Stroke::new(1.0, TERM_GREEN);
    visuals.widgets.open.corner_radius = z;
    visuals.widgets.open.expansion     = 0.0;

    visuals.selection.bg_fill = TERM_GREEN;
    visuals.selection.stroke  = Stroke::new(1.0, TERM_BG); // Black/dark text when selected

    style.visuals = visuals;
    style.interaction.selectable_labels = false;

    style.spacing.item_spacing   = Vec2::new(GAP_MD, GAP_MD);
    style.spacing.button_padding = Vec2::new(GAP_SM, GAP_SM);
    style.spacing.window_margin  = Margin::same(GAP_SM as i8);
    style.spacing.slider_width   = 150.0;

    ctx.set_global_style(style);
}

pub fn text_field_edit(ui: &mut Ui, text: &mut String, font_size: f32, height: f32) -> Response {
    let field_w = ui.available_width();
    let padding = egui::vec2(GAP_MD, GAP_SM);
    let field_h = if height > 0.0 { height } else { font_size + padding.y * 2.0 + 2.0 };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(field_w, field_h), Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        p.rect_filled(rect, CornerRadius::ZERO, TERM_BG);
        p.rect_stroke(rect, CornerRadius::ZERO, Stroke::new(1.0, TERM_GREEN), egui::StrokeKind::Outside);
    }
    let inner_rect = egui::Rect::from_center_size(
        rect.center(),
        egui::vec2(rect.width() - GAP_MD, font_size + padding.y * 2.0 + 2.0),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner_rect).layout(*ui.layout()));
    child.visuals_mut().extreme_bg_color = TERM_BG;
    child.visuals_mut().widgets.hovered.bg_fill      = TERM_BG;
    child.visuals_mut().widgets.hovered.weak_bg_fill = TERM_BG;
    child.visuals_mut().widgets.active.bg_fill       = TERM_BG;
    child.visuals_mut().widgets.active.weak_bg_fill  = TERM_BG;
    child.visuals_mut().selection.bg_fill = TERM_GREEN;
    child.visuals_mut().selection.stroke = Stroke::new(1.0, TERM_BG);
    let text_edit = egui::TextEdit::singleline(text)
        .font(egui::FontId::monospace(font_size))
        .horizontal_align(egui::Align::Center)
        .frame(egui::Frame::NONE);
    child.add(text_edit)
}

pub fn section_toggle_btn(ui: &mut Ui) -> Response {
    let (btn_rect, btn_resp) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::click());
    if ui.is_rect_visible(btn_rect) {
        let is_down = btn_resp.is_pointer_button_down_on();
        let is_hov  = btn_resp.hovered();
        let bg = TERM_BG;
        let fg = TERM_GREEN;

        ui.painter().rect_filled(btn_rect, egui::CornerRadius::ZERO, bg);
        ui.painter().rect_stroke(btn_rect, egui::CornerRadius::ZERO, if is_down || is_hov { Stroke::NONE } else { Stroke::new(1.0, fg) }, egui::StrokeKind::Outside);

        let offset = if is_down { 1.0 } else { 0.0 };
        let text_pos = btn_rect.center() + egui::vec2(offset, offset);
        ui.painter().text(text_pos, egui::Align2::CENTER_CENTER, ".", FontId::monospace(11.0), fg);
    }
    btn_resp
}

pub fn collapsible_header(ui: &mut Ui, title: &str, _is_expanded: bool) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        let btn_resp = section_toggle_btn(ui).hand();
        let lbl_resp = ui.label(egui::RichText::new(title).strong().color(TERM_GREEN))
            .hand().interact(Sense::click());

        if btn_resp.clicked() || lbl_resp.clicked() { clicked = true; }
    });
    clicked
}

pub fn draw_outset(painter: &Painter, rect: egui::Rect) {
    painter.rect_stroke(rect, CornerRadius::ZERO, Stroke::new(1.0, TERM_GREEN), egui::StrokeKind::Outside);
}

pub fn draw_sunken(painter: &Painter, rect: egui::Rect) {
    painter.rect_stroke(rect, CornerRadius::ZERO, Stroke::new(1.0, TERM_GREEN), egui::StrokeKind::Outside);
}

pub fn button(ui: &mut Ui, text: &str) -> Response {
    button_w(ui, text, 0.0)
}

pub fn button_w(ui: &mut Ui, text: &str, min_w: f32) -> Response {
    let padding = egui::vec2(GAP_MD, GAP_SM);
    let is_hovered_pre = false; // computed after allocation
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        FontId::monospace(13.0),
        TERM_GREEN,
    );
    let w = (galley.size().x + padding.x * 2.0).max(min_w);
    let h = galley.size().y + padding.y * 2.0;
    let (rect, mut response) = ui.allocate_exact_size(egui::vec2(w, h), Sense::click());
    let _ = is_hovered_pre;

    if response.clicked() {
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let is_down = response.is_pointer_button_down_on();
        let is_hov  = response.hovered();
        let p = ui.painter();
        let bg = TERM_BG;
        let fg = TERM_GREEN;
        p.rect_filled(rect, 0.0, bg);
        p.rect_stroke(rect, CornerRadius::ZERO, if is_down || is_hov { Stroke::NONE } else { Stroke::new(1.0, fg) }, egui::StrokeKind::Outside);
        let offset = if is_down { egui::vec2(1.0, 1.0) } else { egui::vec2(0.0, 0.0) };
        let text_pos = ui.layout().align_size_within_rect(galley.size(), rect.shrink(GAP_XS)).min + offset;
        p.galley(text_pos, galley, fg);
    }
    response.hand()
}

/// A keycap badge — sunken border, monospace label, compact. Returns a clickable Response.
/// Used in the keyboard cheatsheet.
pub fn key_cap(ui: &mut Ui, text: &str, min_side: f32, font_size: f32) -> Response {
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        FontId::monospace(font_size),
        TERM_GREEN,
    );
    let side = min_side.max(galley.size().x + 6.0);
    let size = egui::vec2(side, side);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        let is_down = response.is_pointer_button_down_on();
        let is_hov  = response.hovered();
        let fg = TERM_GREEN;
        if is_down {
            p.rect_filled(rect, 0.0, TERM_GREEN.linear_multiply(0.2));
        } else if !is_hov {
            draw_outset(p, rect);
        }
        let text_pos = rect.center() - galley.size() * 0.5;
        p.galley(text_pos, galley.clone(), fg);
        p.galley(text_pos + egui::vec2(1.0, 0.0), galley, fg);
    }
    response.hand()
}

/// Symmetric log slider: center (t=0.5) = 0, left = negative, right = positive.
/// Range should be symmetric: e.g. -max..=+max or 0..=max (still works).
/// Formula: val = sign(s) * (exp(|s| * ln(max_abs+1)) - 1), where s = 2t-1 ∈ [-1,1]


pub fn slider_log_f64(ui: &mut Ui, value: &mut f64, range: std::ops::RangeInclusive<f64>, text: &str, fmt: fn(f64)->String) -> Response {
    let min = *range.start();
    let max = *range.end();

    let l_min = if min <= 0.0 { 0.1f64.ln() } else { min.ln() };
    let l_max = max.ln();

    let mut root_response = ui.allocate_response(egui::vec2(0.0, 0.0), Sense::hover());

    ui.horizontal(|ui| {
        let slider_width = ui.available_width().max(60.0);
        let height = ui.spacing().interact_size.y;

        let (rect, mut s_resp) = ui.allocate_exact_size(egui::vec2(slider_width, height), Sense::click_and_drag());

        if s_resp.dragged() || s_resp.clicked() {
            let x = s_resp.interact_pointer_pos().unwrap().x - rect.min.x;
            let t = (x / slider_width).clamp(0.0, 1.0) as f64;
            let l_val = l_min + t * (l_max - l_min);
            *value = l_val.exp().clamp(min, max);
            s_resp.mark_changed();
        }

        if ui.is_rect_visible(rect) {
            let p = ui.painter();

            let track_h = GAP_SM;
            let track_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(slider_width, track_h));
            draw_sunken(p, track_rect);

            let l_val = if *value <= 0.0 { 0.1f64.ln() } else { (*value).ln() };
            let t = ((l_val - l_min) / (l_max - l_min)).clamp(0.0, 1.0) as f32;
            let handle_x = rect.min.x + t * slider_width;

            let handle_w = 11.0;
            let handle_rect = egui::Rect::from_center_size(egui::pos2(handle_x, rect.center().y), egui::vec2(handle_w, height * 1.2));

            let _is_down = s_resp.dragged() || s_resp.is_pointer_button_down_on();
            let _is_hov  = s_resp.hovered();
            let bg = TERM_BG;
        let fg = TERM_GREEN;

            p.rect_filled(handle_rect, 0.0, bg);
            p.rect_stroke(handle_rect, CornerRadius::ZERO, Stroke::new(1.0, fg), egui::StrokeKind::Outside);
        }

        ui.add_space(GAP_MD);
        let label = format!("{} {}", fmt(*value), text);
        ui.label(label);

        root_response = s_resp;
    });

    root_response
}

pub fn key_cap_rotated(ui: &mut egui::Ui, text: &str, angle: f32, min_side: f32, font_size: f32) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        egui::FontId::monospace(font_size),
        TERM_GREEN,
    );
    let size = egui::vec2(min_side, min_side);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        let is_down = response.is_pointer_button_down_on();
        let is_hov  = response.hovered();
        let bg = TERM_BG;
        let fg = TERM_GREEN;
        p.rect_filled(rect, 0.0, bg);
        if !(is_down || is_hov) { crate::ui::rect::draw_outset(p, rect); }

        let c = rect.center();
        let gw = galley.size().x;
        let gh = galley.size().y;
        let s = angle.signum();

        let pos = egui::pos2(c.x + s * (gh * 0.42 + 3.0), c.y - s * gw / 2.0);

        let make_shape = |galley: std::sync::Arc<egui::Galley>, offset: egui::Vec2| egui::Shape::Text(egui::epaint::TextShape {
            pos: pos + offset,
            galley,
            underline: egui::Stroke::NONE,
            fallback_color: fg,
            override_text_color: Some(fg),
            opacity_factor: 1.0,
            angle,
        });
        ui.painter().add(make_shape(galley.clone(), egui::Vec2::ZERO));
        ui.painter().add(make_shape(galley, egui::vec2(1.0, 0.0)));
    }
    response.hand()
}

// ── ThemeProvider impl ────────────────────────────────────────────────────────

pub struct Rect;

impl crate::ui::theme::ThemeProvider for Rect {
    fn apply_theme(&self, ctx: &Context) { apply_theme(ctx); }
    fn draw_sunken(&self, painter: &Painter, rect: egui::Rect) { draw_sunken(painter, rect); }
    fn section_toggle_btn(&self, ui: &mut Ui) -> Response { section_toggle_btn(ui) }
    fn key_cap_small(&self, ui: &mut Ui, text: &str, side: f32, font_size: f32) -> Response { key_cap(ui, text, side, font_size) }
    fn key_cap_small_rotated(&self, ui: &mut Ui, text: &str, angle: f32, side: f32, font_size: f32) -> Response { key_cap_rotated(ui, text, angle, side, font_size) }
    fn collapsible_header(&self, ui: &mut Ui, text: &str, is_open: bool) -> bool { crate::ui::widgets::collapsible_header(self, ui, text, is_open) }
    fn paint_slider_track(&self, ui: &mut Ui, track_rect: egui::Rect, center_x: f32) {
        let p = ui.painter();
        draw_sunken(p, track_rect);

        p.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(center_x - 1.0, track_rect.min.y),
                egui::pos2(center_x + 1.0, track_rect.max.y)
            ),
            0.0,
            TERM_BG,
        );

        p.line_segment(
            [egui::pos2(center_x - 1.0, track_rect.min.y - GAP_SM), egui::pos2(center_x - 1.0, track_rect.max.y + GAP_SM)],
            Stroke::new(1.0, TERM_GREEN),
        );
        p.line_segment(
            [egui::pos2(center_x, track_rect.min.y - GAP_SM), egui::pos2(center_x, track_rect.max.y + GAP_SM)],
            Stroke::new(1.0, TERM_GREEN),
        );
        p.line_segment(
            [egui::pos2(center_x + 1.0, track_rect.min.y - GAP_SM), egui::pos2(center_x + 1.0, track_rect.max.y + GAP_SM)],
            Stroke::new(1.0, TERM_GREEN),
        );
    }

    fn paint_slider_thumb(&self, ui: &mut Ui, handle_rect: egui::Rect, _is_down: bool, _is_hov: bool) {
        let p = ui.painter();
        p.rect_filled(handle_rect, 0.0, TERM_BG);
        p.rect_stroke(handle_rect, CornerRadius::ZERO, Stroke::new(1.0, TERM_GREEN), egui::StrokeKind::Outside);
    }

    fn paint_slider_text(&self, _ui: &mut Ui, _text: &str) {
        // Rect theme doesn't inline slider text drawing typically, or if it does, we can leave it empty / generic
    }
    
    fn paint_slider_gauge(&self, ui: &mut Ui, bg_rect: egui::Rect, fill_rect: egui::Rect, is_down: bool, is_hovered: bool) {
        let p = ui.painter();
        draw_sunken(p, bg_rect);
        
        if fill_rect.width() > 0.0 {
            let fill_color = if is_down {
                TERM_GREEN
            } else if is_hovered {
                TERM_GREEN.linear_multiply(0.8)
            } else {
                TERM_GREEN.linear_multiply(0.6)
            };
            p.rect_filled(fill_rect, 0.0, fill_color);
        }
    }
    fn section_label(&self, ui: &mut Ui, text: &str) -> Response { ui.label(text) }
    fn text_field_edit(&self, ui: &mut Ui, text: &mut String, font_size: f32, height: f32) -> Response { text_field_edit(ui, text, font_size, height) }

    fn button_text_color(&self) -> egui::Color32 {
        TERM_GREEN
    }

    fn paint_button(&self, ui: &mut egui::Ui, rect: egui::Rect, is_down: bool, is_hovered: bool) -> f32 {
        let p = ui.painter();
        if is_down {
            return 1.0;
        } else if !is_hovered {
            draw_outset(p, rect);
            return 0.0;
        }
        0.0
    }

    fn key_cap_text_color(&self) -> egui::Color32 {
        TERM_GREEN
    }

    fn paint_key_cap(&self, p: &egui::Painter, rect: egui::Rect, is_down: bool, is_hovered: bool) {
        if !is_down && !is_hovered {
            draw_outset(p, rect);
        }
    }

    fn chart_bg(&self) -> egui::Color32 {
        egui::Color32::BLACK
    }

    fn chart_axis_color(&self) -> egui::Color32 {
        TERM_GREEN.linear_multiply(0.6)
    }
}
