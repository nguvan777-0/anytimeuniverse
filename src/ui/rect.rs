//! Rectangle theme
#![allow(dead_code)]

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

pub fn text_field_edit(theme: &dyn crate::ui::theme::ThemeProvider, ui: &mut Ui, text: &mut String, font_size: f32, height: f32) -> Response {
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
    let mut te_resp = child.add(text_edit);
    crate::ui::widgets::maintain_text_selection_cache(ui.ctx(), &te_resp, text, rect);
    if crate::ui::widgets::text_field_context_menu(theme, &te_resp, te_resp.id, text) { te_resp.mark_changed(); }
    te_resp
}

pub fn section_toggle_btn(ui: &mut Ui) -> Response {
    let r = 6.0f32;
    let (btn_rect, btn_resp) = ui.allocate_exact_size(egui::vec2(r * 2.0 + 2.0, r * 2.0 + 2.0), egui::Sense::click());
    if ui.is_rect_visible(btn_rect) {
        let is_down = btn_resp.is_pointer_button_down_on();
        let is_hov  = btn_resp.hovered();
        let fg = TERM_GREEN;

        // Border only at rest - use Inside stroke so it doesn't expand into the label's space
        if !is_hov && !is_down {
            ui.painter().rect_stroke(btn_rect, egui::CornerRadius::ZERO, egui::Stroke::new(1.0, fg), egui::StrokeKind::Inside);
        }
        
        // No fill on hover (requested "no fill")

        let offset = if is_down { 1.0 } else { 0.0 };
        let dot_color = fg;
        let text_pos = btn_rect.center() + egui::vec2(offset, offset - 2.0); // nudge up optically
        ui.painter().text(text_pos, egui::Align2::CENTER_CENTER, ".", FontId::monospace(11.0), dot_color);
    }
    btn_resp
}

pub fn section_label(ui: &mut Ui, text: &str) -> Response {
    ui.label(egui::RichText::new(text).strong().color(TERM_GREEN))
}

pub fn collapsible_header(ui: &mut Ui, title: &str, _is_expanded: bool) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        let btn_resp = section_toggle_btn(ui).hand();
        let lbl_resp = section_label(ui, title)
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
pub fn key_cap(ui: &mut Ui, text: &str, min_side: f32, font_size: f32, is_pressed: bool) -> Response {
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
        let is_down = response.is_pointer_button_down_on() || is_pressed;
        let is_hov  = response.hovered();
        let fg = TERM_GREEN;
        if !is_down && !is_hov {
            draw_outset(p, rect);
        }
        let push = if is_down { egui::vec2(1.0, 1.0) } else { egui::Vec2::ZERO };
        let text_pos = rect.center() - galley.size() * 0.5 - egui::vec2(0.0, 2.5) + push;
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

pub fn key_cap_rotated(ui: &mut egui::Ui, text: &str, angle: f32, min_side: f32, font_size: f32, is_pressed: bool) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        egui::FontId::monospace(font_size),
        TERM_GREEN,
    );
    let size = egui::vec2(min_side, min_side);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        let is_down = response.is_pointer_button_down_on() || is_pressed;
        let is_hov  = response.hovered();
        let _bg = TERM_BG;
        let fg = TERM_GREEN;
        // Rect theme key caps are hollow
        if !(is_down || is_hov) { crate::ui::rect::draw_outset(p, rect); }

        let c = rect.center();
        let gw = galley.size().x;
        let gh = galley.size().y;
        let s = angle.signum();

        let push = if is_down { egui::vec2(1.0, 1.0) } else { egui::Vec2::ZERO };
        let pos = egui::pos2(c.x + s * (gh / 2.0 + 2.0), c.y - s * gw / 2.0) + push;

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
    fn palette(&self) -> crate::ui::theme::ThemePalette {
        crate::ui::theme::ThemePalette {
            is_terminal_style: true,
            panel_margin: 6.0,
            panel_text_color: egui::Color32::from_rgb(20, 240, 50),
            hash_stat_color: egui::Color32::from_rgb(20, 240, 50),
            hash_selection_color: TERM_GREEN,
            title_bar_text_color: egui::Color32::from_rgb(20, 240, 50),
            title_bar_button_color: egui::Color32::from_rgb(20, 240, 50),
            tracker_color: egui::Color32::from_rgb(20, 240, 50),
            chart_axis_color: egui::Color32::from_rgb(0, 100, 0),
            remove_tracker_border_on_hover: true,
        }
    }

    fn apply_theme(&self, ctx: &Context) {
        apply_theme(ctx);
        let term_bg   = egui::Color32::BLACK;
        let term_green = egui::Color32::from_rgb(0, 230, 65);
        let term_dim  = term_green;
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = term_bg;
        visuals.window_fill = term_bg;
        visuals.selection.bg_fill = term_dim;
        visuals.selection.stroke = egui::Stroke::new(1.0, term_bg);
        visuals.widgets.noninteractive.bg_fill = term_bg;
        visuals.widgets.noninteractive.weak_bg_fill = term_bg;
        visuals.widgets.inactive.bg_fill = term_bg;
        visuals.widgets.inactive.weak_bg_fill = term_bg;
        visuals.widgets.hovered.bg_fill = term_dim;
        visuals.widgets.hovered.weak_bg_fill = term_dim;
        visuals.widgets.active.bg_fill = term_dim;
        visuals.widgets.active.weak_bg_fill = term_dim;
        visuals.override_text_color = Some(term_green);
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, term_dim);
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, term_green);
        visuals.widgets.active.bg_stroke  = egui::Stroke::new(1.0, term_green);
        visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::ZERO;
        visuals.widgets.inactive.corner_radius = egui::CornerRadius::ZERO;
        visuals.widgets.hovered.corner_radius  = egui::CornerRadius::ZERO;
        visuals.widgets.active.corner_radius   = egui::CornerRadius::ZERO;
        visuals.window_corner_radius = egui::CornerRadius::ZERO;
        visuals.menu_corner_radius   = egui::CornerRadius::ZERO;
        visuals.window_stroke = egui::Stroke::new(1.0, term_green);
        visuals.popup_shadow = egui::Shadow::NONE;
        ctx.set_visuals(visuals);
        let mut fonts = egui::FontDefinitions::default();
        if let Some(mono) = fonts.families.get(&egui::FontFamily::Monospace).cloned() {
            fonts.families.insert(egui::FontFamily::Proportional, mono);
        }
        ctx.set_fonts(fonts);
    }

    fn edit_popup_visuals(&self, visuals: &mut egui::Visuals) {
        visuals.window_fill = TERM_BG;
        visuals.panel_fill = TERM_BG;
        visuals.window_stroke = egui::Stroke::new(1.0, TERM_GREEN);
        visuals.popup_shadow = egui::Shadow::NONE;
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, TERM_GREEN);

        visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
        visuals.widgets.hovered.weak_bg_fill = egui::Color32::TRANSPARENT;
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, TERM_GREEN);
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, TERM_GREEN);
        
        visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
        visuals.widgets.active.weak_bg_fill = egui::Color32::TRANSPARENT;
        visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, TERM_GREEN);
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, TERM_GREEN);
    }

    fn edit_popup_spacing(&self, spacing: &mut egui::Spacing) {
        spacing.button_padding = egui::vec2(4.0, 2.0);
        spacing.item_spacing = egui::vec2(8.0, 0.0);
        spacing.window_margin = egui::Margin::same(2);
        spacing.menu_margin = egui::Margin::same(2);
        spacing.icon_spacing = 8.0;
    }

    fn setup_frame(&self, _ctx: &egui::Context) {}

    fn paint_sim_area_border(&self, ui: &mut egui::Ui, sim_rect: egui::Rect) {
        ui.ctx().layer_painter(egui::LayerId::background()).rect_stroke(
            sim_rect,
            egui::CornerRadius::ZERO,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 230, 65)),
            egui::StrokeKind::Outside,
        );
    }

    fn sim_area_padding(&self) -> f32 { 4.0 }

    
    fn paint_hash_bg(&self, p: &egui::Painter, rect: egui::Rect) {
        p.rect_filled(rect, egui::CornerRadius::ZERO, TERM_BG);
        p.rect_stroke(rect, egui::CornerRadius::ZERO, egui::Stroke::new(1.0, TERM_GREEN), egui::StrokeKind::Outside);
    }
    fn paint_hash_copy_btn(&self, ui: &mut egui::Ui, btn_rect: egui::Rect, is_down: bool, is_hovered: bool) -> f32 {
        let bg = TERM_BG;
        let fg = TERM_GREEN;
        ui.painter().rect_filled(btn_rect, egui::CornerRadius::ZERO, bg);
        if !is_down && !is_hovered {
            ui.painter().line_segment([btn_rect.left_top(), btn_rect.left_bottom()], egui::Stroke::new(1.0, fg));
        }
        if is_down { 1.0 } else { 0.0 }
    }

    fn draw_sunken(&self, painter: &Painter, rect: egui::Rect) { draw_sunken(painter, rect); }

    fn draw_space_strategy_bg(&self, ui: &mut Ui, rect: egui::Rect) {
        self.draw_sunken(ui.painter(), rect);
        
        let painter = ui.painter();
        let grid_step = 20.0;
        let line_color = Color32::from_rgba_premultiplied(0, 10, 0, 10); // very faint CRT terminal green
        
        let cx = rect.center().x;
        let mut i = 0.0;
        while cx + i <= rect.max.x || cx - i >= rect.min.x {
            if cx + i <= rect.max.x {
                painter.line_segment([egui::pos2(cx + i, rect.min.y), egui::pos2(cx + i, rect.max.y)], Stroke::new(1.0, line_color));
            }
            if i > 0.0 && cx - i >= rect.min.x {
                painter.line_segment([egui::pos2(cx - i, rect.min.y), egui::pos2(cx - i, rect.max.y)], Stroke::new(1.0, line_color));
            }
            i += grid_step;
        }
        
        let cy = rect.center().y;
        let mut j = 0.0;
        while cy + j <= rect.max.y || cy - j >= rect.min.y {
            if cy + j <= rect.max.y {
                painter.line_segment([egui::pos2(rect.min.x, cy + j), egui::pos2(rect.max.x, cy + j)], Stroke::new(1.0, line_color));
            }
            if j > 0.0 && cy - j >= rect.min.y {
                painter.line_segment([egui::pos2(rect.min.x, cy - j), egui::pos2(rect.max.x, cy - j)], Stroke::new(1.0, line_color));
            }
            j += grid_step;
        }
    }

    fn section_toggle_btn(&self, ui: &mut Ui) -> Response { section_toggle_btn(ui) }
    fn key_cap_small(&self, ui: &mut Ui, text: &str, side: f32, font_size: f32, is_pressed: bool) -> Response { key_cap(ui, text, side, font_size, is_pressed) }
    fn key_cap_small_rotated(&self, ui: &mut Ui, text: &str, angle: f32, side: f32, font_size: f32, is_pressed: bool) -> Response { key_cap_rotated(ui, text, angle, side, font_size, is_pressed) }
    fn paint_slider_track(&self, ui: &mut Ui, track_rect: egui::Rect, center_x: f32) {
        let p = ui.painter();
        draw_sunken(p, track_rect);

        let cy = track_rect.center().y;
        
        // A vertical "door" with a retro grid pattern on it, shifted for an isometric popup look
        // Same size as the door marker on Dew theme
        let w = 3.0; // Same as Dew's w = 3.0
        let h = 7.0; // Same as Dew's h = 7.0
        let shift = 1.5; // Same as Dew's shift = 1.5

        let tl = egui::pos2(center_x - w, cy - h);
        let tr = egui::pos2(center_x + w, cy - h + shift);
        let br = egui::pos2(center_x + w, cy + h + shift);
        let bl = egui::pos2(center_x - w, cy + h);

        use egui::Shape;
        
        // Solid dark background to block the track, and glowing outline
        p.add(Shape::convex_polygon(
            vec![tl, tr, br, bl],
            TERM_BG.linear_multiply(0.8),
            Stroke::new(1.0, TERM_GREEN)
        ));
        
        // Draw the inner grid lines on the "door"
        let line_color = TERM_GREEN.linear_multiply(0.5);
        
        // Vertical center line
        p.line_segment(
            [egui::pos2(center_x, cy - h + shift * 0.5), egui::pos2(center_x, cy + h + shift * 0.5)],
            Stroke::new(1.0, line_color),
        );
        
        // Horizontal center line (perfectly bisecting the isometric angles)
        p.line_segment(
            [egui::pos2(center_x - w, cy), egui::pos2(center_x + w, cy + shift)],
            Stroke::new(1.0, line_color),
        );
    }

    fn paint_slider_thumb(&self, ui: &mut Ui, handle_rect: egui::Rect, is_down: bool, is_hov: bool) {
        let p = ui.painter();
        if is_hov || is_down {
            p.rect_filled(handle_rect, 0.0, TERM_GREEN);
        } else {
            p.rect_filled(handle_rect, 0.0, TERM_BG);
            p.rect_stroke(handle_rect, CornerRadius::ZERO, Stroke::new(1.0, TERM_GREEN), egui::StrokeKind::Outside);
        }
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
    fn section_label(&self, ui: &mut Ui, text: &str) -> Response { section_label(ui, text) }
    fn text_field_edit(&self, ui: &mut Ui, text: &mut String, font_size: f32, height: f32) -> Response { text_field_edit(self, ui, text, font_size, height) }

    fn button_text_color(&self) -> egui::Color32 {
        TERM_GREEN
    }

    fn paint_button(&self, ui: &mut egui::Ui, rect: egui::Rect, is_down: bool, is_hovered: bool) -> f32 {
        let p = ui.painter();
        if !is_down && !is_hovered {
            draw_outset(p, rect);
        }
        if is_down { 1.0 } else { 0.0 }
    }





}
