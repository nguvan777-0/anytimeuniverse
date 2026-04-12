//! Metallic theme
#![allow(dead_code)]

use egui::{Color32, Stroke, CornerRadius, Margin, Vec2, Context, Visuals, Rect, Painter, Response, Ui, FontId, Sense};
use crate::ui::ResponseExt;

// ── Metallic palette ──────────────────────────────────────────────────────────────
// Panel / window fill — Metallic background base
const PEARL:         Color32 = Color32::from_rgb(235, 235, 235);
// Metallic button faces
const METALLIC_BODY:      Color32 = Color32::from_rgba_premultiplied(130, 130, 130, 220);
#[allow(dead_code)]
const METALLIC_GLOW:      Color32 = Color32::from_rgba_premultiplied(240, 240, 240, 180); // bottom reflection
const METALLIC_PRESSED:   Color32 = Color32::from_rgba_premultiplied(160, 160, 160, 150);
const METALLIC_BORDER:    Color32 = Color32::from_rgba_premultiplied(100, 100, 100, 130);
// Inset field / track
const INSET_FILL:    Color32 = Color32::from_rgb(255, 255, 255);
const INSET_BORDER:  Color32 = Color32::from_rgb(140, 140, 140);
const INSET_SHADOW:  Color32 = Color32::from_rgb(180, 180, 180);
// Button rounding
const R_SM:   f32 = 4.0;
#[allow(dead_code)]
const R_BTN:  f32 = 8.0;

// ── Theme application ──────────────────────────────────────────────────────────

pub fn apply_theme(ctx: &Context) {
    let mut style = (*ctx.global_style()).clone();

    let mut visuals = Visuals::light();
    visuals.window_fill         = PEARL;
    visuals.panel_fill          = PEARL;
    visuals.extreme_bg_color    = INSET_FILL;
    visuals.faint_bg_color      = PEARL;
    visuals.code_bg_color       = INSET_FILL;
    visuals.text_cursor.stroke.color = Color32::BLACK;

    let r = CornerRadius::from(R_SM);
    let metallic_stroke  = Stroke::new(1.0, METALLIC_BORDER);
    let inset_stroke = Stroke::new(1.0, INSET_BORDER);

    visuals.window_corner_radius = CornerRadius::same(6);
    visuals.menu_corner_radius   = CornerRadius::same(6);

    visuals.widgets.noninteractive.bg_fill      = PEARL;
    visuals.widgets.noninteractive.weak_bg_fill  = PEARL;
    visuals.widgets.noninteractive.bg_stroke     = inset_stroke;
    visuals.widgets.noninteractive.fg_stroke     = Stroke::new(1.0, Color32::from_rgb(60, 60, 65));
    visuals.widgets.noninteractive.corner_radius      = r;

    visuals.widgets.inactive.bg_fill      = PEARL;
    visuals.widgets.inactive.weak_bg_fill  = PEARL;
    visuals.widgets.inactive.bg_stroke     = Stroke::new(1.0, INSET_BORDER);
    visuals.widgets.inactive.fg_stroke     = Stroke::new(1.0, Color32::BLACK);
    visuals.widgets.inactive.corner_radius      = r;

    visuals.widgets.hovered.bg_fill      = Color32::from_rgb(225, 225, 225);
    visuals.widgets.hovered.weak_bg_fill  = Color32::from_rgb(225, 225, 225);
    visuals.widgets.hovered.bg_stroke     = Stroke::new(1.5, METALLIC_BORDER);
    visuals.widgets.hovered.fg_stroke     = Stroke::new(1.0, Color32::BLACK);
    visuals.widgets.hovered.corner_radius      = r;
    visuals.widgets.hovered.expansion     = 0.0;

    visuals.widgets.active.bg_fill      = METALLIC_PRESSED;
    visuals.widgets.active.weak_bg_fill  = METALLIC_PRESSED;
    visuals.widgets.active.bg_stroke     = metallic_stroke;
    visuals.widgets.active.fg_stroke     = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.active.corner_radius      = r;
    visuals.widgets.active.expansion     = 0.0;

    visuals.selection.bg_fill = Color32::from_rgb(200, 200, 200);
    visuals.selection.stroke  = Stroke::new(1.0, Color32::BLACK);

    style.visuals = visuals;
    style.interaction.selectable_labels = false;

    style.spacing.item_spacing   = Vec2::new(6.0, 6.0);
    style.spacing.button_padding = Vec2::new(8.0, 4.0);
    style.spacing.window_margin  = Margin::same(8);
    style.spacing.slider_width   = 150.0;

    ctx.set_global_style(style);
}

// ── Primitive helpers ─────────────────────────────────────────────────────────

/// Metallic background: subtle grey horizontal stripes.
pub fn draw_stripes(painter: &Painter, rect: Rect) {
    let stripe = Color32::from_rgb(228, 228, 228);
    // Draw 1px thick horizontal lines every 2 pixels
    let mut y = rect.min.y;
    while y < rect.max.y {
        painter.line_segment(
            [egui::pos2(rect.min.x, y.floor() + 0.5), egui::pos2(rect.max.x, y.floor() + 0.5)],
            Stroke::new(1.0, stripe),
        );
        y += 2.0;
    }
}

/// Draw a Metallic-style inset border: used for text fields and slider tracks.
pub fn draw_inset(painter: &Painter, rect: Rect) {
    painter.rect_stroke(rect, R_SM, Stroke::new(1.0, INSET_BORDER), egui::StrokeKind::Outside);
    // Inner shadow at the top
    let r = R_SM;
    painter.line_segment(
        [rect.left_top() + egui::vec2(r, 1.0), rect.right_top() + egui::vec2(-r, 1.0)],
        Stroke::new(1.0, INSET_SHADOW),
    );
}

/// Draw a read-only Metallic text field showing `text`, right-aligned.
#[allow(dead_code)]
pub fn text_field_label(ui: &mut Ui, text: &str, font_size: f32) {
    let font = egui::FontId::proportional(font_size);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font.clone(), Color32::from_rgb(30, 30, 35));
    let padding = egui::vec2(6.0, 3.0);
    let field_w = ui.available_width();
    let field_h = galley.size().y + padding.y * 2.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(field_w, field_h), Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        // White fill
        p.rect_filled(rect, R_SM, INSET_FILL);
        // Inset border + shadow
        draw_inset(p, rect);
        // Text right-aligned inside
        let text_x = rect.max.x - padding.x - galley.size().x;
        let text_y = rect.center().y - galley.size().y / 2.0;
        p.galley(egui::pos2(text_x, text_y), galley, Color32::from_rgb(30, 30, 35));
    }
}

/// Editable Metallic text field. Returns true when the user commits a new value (Enter or focus lost).
pub fn text_field_edit(theme: &dyn crate::ui::theme::ThemeProvider, ui: &mut Ui, text: &mut String, font_size: f32, height: f32) -> Response {
    let field_w = ui.available_width();
    let padding = egui::vec2(6.0, 3.0);
    let field_h = if height > 0.0 { height } else { font_size + padding.y * 2.0 + 2.0 };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(field_w, field_h), Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        p.rect_filled(rect, R_SM, INSET_FILL);
        draw_inset(p, rect);
    }
    let inner_rect = egui::Rect::from_center_size(
        rect.center(),
        egui::vec2(rect.width() - 8.0, font_size + padding.y * 2.0 + 2.0),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner_rect).layout(*ui.layout()));
    // Force a lighter selection background for better text contrast
    child.visuals_mut().selection.bg_fill = Color32::from_rgb(200, 200, 200);
    child.visuals_mut().selection.stroke = Stroke::new(1.0, Color32::BLACK);

    let text_edit = egui::TextEdit::singleline(text)
        .font(egui::FontId::monospace(font_size))
        .horizontal_align(egui::Align::Center)
        .frame(egui::Frame::NONE)
        .text_color(Color32::from_rgb(30, 30, 35));
    let mut te_resp = child.add(text_edit);
    crate::ui::widgets::maintain_text_selection_cache(ui.ctx(), &te_resp, text, rect);
    if crate::ui::widgets::text_field_context_menu(theme, &te_resp, te_resp.id, text) { te_resp.mark_changed(); }
    te_resp
}

// ── Shared Metallic Button/Pill Rendering ──────────────────────────────────────────

/// Renders a perfect square Metallic translucent gumdrop / pill with physical squish animation.
/// Returns the Y offset applied (0.0 at rest, up to 1.5 at full press) so callers can shift
/// their text labels to travel with the button face.
pub fn draw_gumdrop(ui: &mut Ui, response: &Response, rect: Rect, force_pressed: bool) -> f32 {
    let p = ui.painter();
    let pressed = response.is_pointer_button_down_on() || force_pressed;
    let press_t = ui.ctx().animate_value_with_time(
        response.id.with("gd_press"),
        if pressed { 1.0 } else { 0.0 },
        0.05,
    );
    let hover_t = ui.ctx().animate_value_with_time(
        response.id.with("gd_hover"),
        if response.hovered() { 1.0 } else { 0.0 },
        0.12,
    );

    let push_y = press_t * 1.5;
    let draw_rect = rect.translate(egui::vec2(0.0, push_y));
    let r = rect.height() / 2.0;

    // Drop shadow stays put as button sinks into it, fading as it does
    let shadow_op = (60.0 * (1.0 - press_t)) as u8;
    if shadow_op > 0 {
        p.rect_filled(
            rect.translate(egui::vec2(0.0, 1.5)),
            r + 0.5,
            Color32::from_rgba_premultiplied(0, 0, 0, shadow_op),
        );
    }

    // Body — same darken formula as dew: press darkens, hover brightens
    let darken = 1.0 - press_t * 0.3 + hover_t * 0.12;
    let c = METALLIC_BODY;
    let v = (c.r() as f32 * darken).min(255.0) as u8;
    p.rect_filled(draw_rect, r, Color32::from_rgba_premultiplied(v, v, v, c.a()));

    // Bottom inner glow — body × 1.5, same as dew.
    // At rest body=130 → glow=195 (light grey, not white): three distinct tones = glass depth
    let lv = (v as f32 * 1.5).min(255.0) as u8;
    let glow_rect = Rect::from_min_max(
        egui::pos2(draw_rect.min.x + 1.5, draw_rect.center().y),
        draw_rect.max - egui::vec2(1.5, 1.5),
    );
    p.rect_filled(glow_rect, egui::epaint::CornerRadiusF32 { nw: 0.0, ne: 0.0, sw: r - 1.5, se: r - 1.5 },
        Color32::from_rgba_premultiplied(lv, lv, lv, 180));

    // Top specular — same as dew
    let hl_op = (220.0 * (1.0 - press_t * 0.5)) as u8;
    if hl_op > 0 {
        let hl_rect = Rect::from_min_size(
            draw_rect.min + egui::vec2(1.5, 1.0),
            egui::vec2(draw_rect.width() - 3.0, r * 0.8),
        );
        p.rect_filled(hl_rect,
            egui::epaint::CornerRadiusF32 { nw: r - 1.5, ne: r - 1.5, sw: r * 0.3, se: r * 0.3 },
            Color32::from_rgba_premultiplied(255, 255, 255, hl_op));
    }

    // Dark outline — same as dew
    p.rect_stroke(draw_rect, r, Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 0, 0, 130)), egui::StrokeKind::Outside);

    push_y
}

/// Plain section-header label that never flickers on click.
/// Paints text directly — bypasses egui's widget state machine so there is
/// no one-frame "active" style applied when the user clicks.
#[allow(dead_code)]
pub fn collapsible_header(ui: &mut Ui, title: &str, _is_expanded: bool) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        let btn_resp = section_toggle_btn(ui);
        let lbl_text = title;
        let lbl_resp = section_label(ui, lbl_text);

        if btn_resp.clicked() || lbl_resp.clicked() { clicked = true; }
    });
    clicked
}

pub fn section_label(ui: &mut Ui, text: &str) -> Response {
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        egui::FontId::proportional(12.0),
        Color32::from_rgb(60, 60, 65),
    );
    let (rect, resp) = ui.allocate_exact_size(galley.size(), Sense::click());
    if ui.is_rect_visible(rect) {
        ui.painter().galley(rect.min, galley, Color32::from_rgb(60, 60, 65));
    }
    resp
}

/// Animating Metallic dot button, encapsulating hover transparency,
/// squish-depth, and inner glyph rendering.
pub fn draw_dot_btn(
    ui: &mut Ui,
    resp: &Response,
    r: f32,
    base_color: Color32,
    symbol: &str,
    _group_hover_t: Option<f32>,
) {
    if !ui.is_rect_visible(resp.rect) {
        return;
    }

    let p = ui.painter();
    let pressed = resp.is_pointer_button_down_on();
    let press_t = ui.ctx().animate_value_with_time(
        resp.id.with("press"),
        if pressed { 1.0 } else { 0.0 },
        0.05,
    );

    let hover_t = ui.ctx().animate_value_with_time(
        resp.id.with("hover"),
        if resp.hovered() { 1.0 } else { 0.0 },
        0.12,
    );

    let center = resp.rect.center();
    let push_y = press_t * 1.5;
    let draw_center = center + egui::vec2(0.0, push_y);

    // shadow fades and stays put as button presses down into it
    let shadow_op = (60.0 * (1.0 - press_t)) as u8;
    if shadow_op > 0 {
        p.circle_filled(center + egui::vec2(0.0, 1.0), r+0.5, Color32::from_rgba_premultiplied(0, 0, 0, shadow_op));
    }

    // Body — same darken formula as dew
    let darken = 1.0 - press_t * 0.35 + hover_t * 0.25;
    let c_v = (base_color.r() as f32 * darken).min(255.0) as u8;
    p.circle_filled(draw_center, r, Color32::from_rgba_premultiplied(c_v, c_v, c_v, base_color.a()));

    // Bottom glow — body × 1.5, same as dew
    let lv = (c_v as f32 * 1.5).min(255.0) as u8;
    p.circle_filled(draw_center + egui::vec2(0.0, 1.5), r - 1.5, Color32::from_rgba_premultiplied(lv, lv, lv, 220));

    // Top specular — same as dew
    let hl_op = (220.0 * (1.0 - press_t * 0.4) * (1.0 + hover_t * 0.05)).min(255.0) as u8;
    let hl_rect = Rect::from_min_size(draw_center - egui::vec2(r-1.5, r-1.0), egui::vec2((r-1.5)*2.0, r*0.8));
    p.rect_filled(hl_rect, r, Color32::from_rgba_premultiplied(255, 255, 255, hl_op));
    // Symbols — drawn as text, sunken/embossed
    {
        let font = egui::FontId::proportional(16.0);
        let hl_col = Color32::from_rgba_premultiplied(255, 255, 255, 150);
        let ink_col = Color32::BLACK;

        let mut pos_offset = egui::vec2(0.0, 0.0);
        if symbol == "." {
            pos_offset.y -= 3.5; // Shift the period up from the baseline to center it visually
        }

        for &d in &[egui::vec2(0.0, 0.0), egui::vec2(0.5, 0.0), egui::vec2(0.0, 0.5), egui::vec2(0.5, 0.5)] {
            p.text(draw_center + pos_offset + egui::vec2(0.0, 1.0) + d, egui::Align2::CENTER_CENTER, symbol, font.clone(), hl_col);
            p.text(draw_center + pos_offset + d, egui::Align2::CENTER_CENTER, symbol, font.clone(), ink_col);
        }
    }
    // dark outline
    p.circle_stroke(draw_center, r, Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 0, 0, 130)));
}

/// Circle used as a section collapse/expand toggle.
/// Matches the title-bar minimize button visually (r = 6.0).
pub fn section_toggle_btn(ui: &mut Ui) -> Response {
    let r = 6.0;
    // ID based on cursor position provides unique ID for every panel button mapped
    let (_rect, resp) = ui.allocate_exact_size(egui::vec2(r * 2.0 + 2.0, r * 2.0 + 2.0), Sense::click());
    let color = METALLIC_BODY;
    draw_dot_btn(ui, &resp, r, color, ".", None);
    resp
}

// ── Button ─────────────────────────────────────────────────────────────────────

pub fn button(ui: &mut Ui, text: &str) -> Response {
    button_w(ui, text, 0.0)
}

pub fn button_w(ui: &mut Ui, text: &str, min_w: f32) -> Response {
    let mut padding = egui::vec2(16.0, 6.0); // 
    let text_color_normal = Color32::BLACK;
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        FontId::proportional(13.0),
        text_color_normal,
    );
    if min_w > 0.0 && galley.size().x + padding.x * 2.0 > min_w {
        padding.x = ((min_w - galley.size().x) / 2.0).max(4.0);
    }
    let w = (galley.size().x + padding.x * 2.0).max(min_w);
    let h = galley.size().y + padding.y * 2.0;
    let (rect, mut response) = ui.allocate_exact_size(egui::vec2(w, h), Sense::click());

    if response.clicked() { response.mark_changed(); }

    if ui.is_rect_visible(rect) {
        let shift_y = draw_gumdrop(ui, &response, rect, false);

        let text_pos = ui.layout().align_size_within_rect(galley.size(), rect.shrink(2.0)).min
            + egui::vec2(0.0, shift_y);
        let shadow_a = (120.0 * (1.0 - shift_y / 1.5)) as u8;
        if shadow_a > 0 {
            ui.painter().galley(text_pos + egui::vec2(0.0, 1.0),
                ui.painter().layout_no_wrap(text.to_string(), FontId::proportional(13.0),
                    Color32::from_rgba_premultiplied(255, 255, 255, shadow_a)),
                text_color_normal);
        }
        ui.painter().galley(text_pos, galley, text_color_normal);
    }
    response.hand()
}

// ── Key cap ────────────────────────────────────────────────────────────────────

/// A keycap badge — rounded, dew-style, used in the keyboard cheatsheet. Returns a clickable Response.
pub fn key_cap(ui: &mut Ui, text: &str, _is_pressed: bool) -> Response {
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        FontId::monospace(24.0),
        Color32::BLACK,
    );
    let padding = egui::vec2(5.0, 2.0);
    let size = galley.size() + padding * 2.0;
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let shift_y = draw_gumdrop(ui, &response, rect, false);

        let text_pos = ui.layout().align_size_within_rect(galley.size(), rect.shrink(2.0)).min
            + egui::vec2(0.0, shift_y);
        let shadow_a = (120.0 * (1.0 - shift_y / 1.5)) as u8;
        if shadow_a > 0 {
            ui.painter().galley(text_pos + egui::vec2(0.0, 1.0),
                ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(24.0),
                    Color32::from_rgba_premultiplied(255, 255, 255, shadow_a)),
                Color32::BLACK);
        }
        ui.painter().galley(text_pos, galley, Color32::BLACK);
    }
    response.hand()
}

pub fn key_cap_small(ui: &mut Ui, text: &str, min_side: f32, font_size: f32, is_pressed: bool) -> Response {
    let measure = ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(font_size), Color32::BLACK);
    let gw = measure.size().x;
    let gh = measure.size().y;
    let side = min_side.max(gw + 6.0); // at least 3px padding each side
    let (rect, response) = ui.allocate_exact_size(egui::vec2(side, side), Sense::click());
    if ui.is_rect_visible(rect) {
        let shift_y = draw_gumdrop(ui, &response, rect, is_pressed);
        // For unrotated text, pos is the top-left of the layout box.
        // We center it manually, applying the same optical vertical tweak (-1.5) as before.
        let c = rect.center();
        let pos = egui::pos2(c.x - gw / 2.0, c.y - gh / 2.0 - 2.5 + shift_y);
        let shadow_a = (120.0 * (1.0 - shift_y / 1.5)) as u8;
        let hi = Color32::from_rgba_premultiplied(255, 255, 255, shadow_a);
        for (off, color) in [
            (egui::vec2(0.0, 1.0), hi),             // depth shadow
            (egui::vec2(0.0, 0.0), Color32::BLACK), // main
        ] {
            ui.painter().add(egui::Shape::Text(egui::epaint::TextShape {
                pos: pos + off,
                galley: ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(font_size), color),
                underline: egui::Stroke::NONE,
                fallback_color: color,
                override_text_color: Some(color),
                opacity_factor: 1.0,
                angle: 0.0,
            }));
        }
    }
    response.hand()
}

/// Same as `key_cap_small` but the glyph is rotated by `angle` radians (±π/2 for left/right).
/// Use "↑" as the glyph — it renders thick and looks correct when rotated.
pub fn key_cap_small_rotated(ui: &mut Ui, text: &str, angle: f32, min_side: f32, font_size: f32, is_pressed: bool) -> Response {
    let measure = ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(font_size), Color32::BLACK);
    let gw = measure.size().x;
    let gh = measure.size().y;
    let side = min_side.max(gw + 6.0); // at least 3px padding each side
    let (rect, response) = ui.allocate_exact_size(egui::vec2(side, side), Sense::click());
    if ui.is_rect_visible(rect) {
        let shift_y = draw_gumdrop(ui, &response, rect, is_pressed);
        // s = +1 for → (CW, +π/2), -1 for ← (CCW, -π/2).
        // After rotation the galley optical center lands at rect.center():
        //   x offset = ±(gh/2 + 2.0)  — gh/2 is the mathematical center; +2.0 is an optical nudge outward
        //   y offset = ∓gw/2         — recenters the rotated glyph extent
        let s = angle.signum();
        let c = rect.center();
        let pos = egui::pos2(c.x + s * (gh / 2.0 + 2.0), c.y - s * gw / 2.0 + shift_y);
        let shadow_a = (120.0 * (1.0 - shift_y / 1.5)) as u8;
        let hi = Color32::from_rgba_premultiplied(255, 255, 255, shadow_a);
        for (off, color) in [
            (egui::vec2(0.0, 1.0), hi),             // depth shadow
            (egui::vec2(0.0, 0.0), Color32::BLACK), // main
        ] {
            ui.painter().add(egui::Shape::Text(egui::epaint::TextShape {
                pos: pos + off,
                galley: ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(font_size), color),
                underline: egui::Stroke::NONE,
                fallback_color: color,
                override_text_color: Some(color),
                opacity_factor: 1.0,
                angle,
            }));
        }
    }
    response.hand()
}

// ── Symmetric log slider ───────────────────────────────────────────────────────


pub fn slider_log_f64(ui: &mut Ui, value: &mut f64, range: std::ops::RangeInclusive<f64>, _text: &str, _fmt: fn(f64) -> String) -> Response {
    let min = *range.start();
    let max = *range.end();
    let l_min = if min <= 0.0 { 0.1f64.ln() } else { min.ln() };
    let l_max = max.ln();

    let mut root_response = ui.allocate_response(egui::vec2(0.0, 0.0), Sense::hover());

    ui.horizontal(|ui| {
        let slider_width = ui.spacing().slider_width;
        let height       = ui.spacing().interact_size.y;

        let (rect, mut s_resp) = ui.allocate_exact_size(egui::vec2(slider_width, height), Sense::click_and_drag());

        if (s_resp.dragged() || s_resp.clicked())
            && let Some(pos) = s_resp.interact_pointer_pos() {
                let x = pos.x - rect.min.x;
                let t = (x / slider_width).clamp(0.0, 1.0) as f64;
                let l_val = l_min + t * (l_max - l_min);
                *value = l_val.exp().clamp(min, max);
                s_resp.mark_changed();
            }

        if ui.is_rect_visible(rect) {
            let p = ui.painter();

            let track_h    = 4.0;
            let track_rect = Rect::from_center_size(rect.center(), egui::vec2(slider_width, track_h));
            p.rect_filled(track_rect, R_SM, INSET_FILL);
            draw_inset(p, track_rect);

            let l_val  = if *value <= 0.0 { 0.1f64.ln() } else { (*value).ln() };
            let t      = ((l_val - l_min) / (l_max - l_min)).clamp(0.0, 1.0) as f32;
            let hx     = rect.min.x + t * slider_width;
            let hw     = 13.0;
            let handle = Rect::from_center_size(egui::pos2(hx, rect.center().y), egui::vec2(hw, height * 1.15));

            p.rect_filled(handle, R_BTN, METALLIC_BODY);
            let hi = Rect::from_min_size(
                handle.min + egui::vec2(1.0, 1.0),
                egui::vec2(handle.width() - 2.0, handle.height() * 0.44),
            );
            p.rect_filled(hi, egui::epaint::CornerRadiusF32 { nw: R_BTN - 1.0, ne: R_BTN - 1.0, sw: 1.0, se: 1.0 },
                Color32::from_rgba_premultiplied(255, 255, 255, 60));
            p.rect_stroke(handle, R_BTN, Stroke::new(1.0, METALLIC_BORDER), egui::StrokeKind::Outside);

            let hcx = handle.center().x;
            let hcy = handle.center().y;
            for (dx, col) in [(-2.0f32, Color32::from_rgba_premultiplied(0,0,0,80)),
                              (-1.0f32, Color32::from_rgba_premultiplied(255,255,255,80)),
                              ( 1.0f32, Color32::from_rgba_premultiplied(0,0,0,80)),
                              ( 2.0f32, Color32::from_rgba_premultiplied(255,255,255,80))] {
                p.line_segment(
                    [egui::pos2(hcx + dx, hcy - 4.0), egui::pos2(hcx + dx, hcy + 4.0)],
                    Stroke::new(1.0, col),
                );
            }
        }

        root_response = s_resp;
    });

    root_response
}

// ── ThemeProvider impl ────────────────────────────────────────────────────────

pub struct Metallic;

impl crate::ui::theme::ThemeProvider for Metallic {
    fn palette(&self) -> crate::ui::theme::ThemePalette {
        crate::ui::theme::ThemePalette {
            is_terminal_style: false,
            panel_margin: 0.0,
            panel_text_color: egui::Color32::from_rgb(100, 100, 105),
            hash_stat_color: egui::Color32::from_rgb(30, 30, 35),
            hash_selection_color: egui::Color32::from_rgb(200, 200, 200),
            title_bar_text_color: egui::Color32::from_rgb(30, 30, 30),
            title_bar_button_color: METALLIC_BODY,
            tracker_color: self.button_text_color(),
            chart_axis_color: egui::Color32::from_white_alpha(30),
            remove_tracker_border_on_hover: false,
        }
    }

    fn apply_theme(&self, ctx: &Context) {
        ctx.set_fonts(egui::FontDefinitions::default());
        apply_theme(ctx);
    }
    
    fn draw_background_pattern(&self, painter: &Painter, rect: Rect) {
        draw_stripes(painter, rect);
    }
    
    fn edit_popup_visuals(&self, visuals: &mut egui::Visuals) {
        visuals.window_fill = PEARL;
        visuals.panel_fill = PEARL;
        visuals.window_stroke = Stroke::new(1.0, METALLIC_BORDER);
        visuals.popup_shadow = egui::Shadow::NONE;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::BLACK);

        visuals.widgets.hovered.bg_fill = Color32::from_rgb(200, 200, 200);
        visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(200, 200, 200);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::BLACK);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::BLACK);
    }

    fn paint_hash_bg(&self, p: &Painter, rect: Rect) {
        p.rect_filled(rect, R_SM, INSET_FILL);
        draw_inset(p, rect);
    }

    
    fn paint_title_bar_button(&self, ui: &mut Ui, resp: &Response, r: f32, base_color: Color32, symbol: &str, hover_t: f32) {
        draw_dot_btn(ui, resp, r, base_color, symbol, Some(hover_t));
    }
    
    fn paint_title_bar_text_bg(&self, ui: &mut Ui, _rect: Rect) {
        ui.visuals_mut().selection.bg_fill = Color32::from_rgb(200, 200, 200);
    }

    fn paint_title_bar_bg(&self, ui: &mut Ui, rect: Rect) {
        for i in 0..7 {
            let y = rect.min.y + 2.0 + i as f32 * 3.5;
            ui.painter().line_segment([egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)], Stroke::new(1.0, Color32::from_rgb(205, 210, 218)));
            ui.painter().line_segment([egui::pos2(rect.min.x, y+1.0), egui::pos2(rect.max.x, y+1.0)], Stroke::new(1.0, Color32::from_rgb(250, 255, 255)));
        }
    }

    fn draw_sunken(&self, painter: &Painter, rect: Rect) { draw_inset(painter, rect); }
    fn section_toggle_btn(&self, ui: &mut Ui) -> Response { section_toggle_btn(ui) }
    fn key_cap_small(&self, ui: &mut Ui, text: &str, side: f32, font_size: f32, is_pressed: bool) -> Response { key_cap_small(ui, text, side, font_size, is_pressed) }
    fn key_cap_small_rotated(&self, ui: &mut Ui, text: &str, angle: f32, side: f32, font_size: f32, is_pressed: bool) -> Response { key_cap_small_rotated(ui, text, angle, side, font_size, is_pressed) }
    fn paint_slider_track(&self, ui: &mut Ui, track_rect: Rect, center_x: f32) {
        let p = ui.painter();
        p.rect_filled(track_rect, 0.0, Color32::from_rgb(220, 220, 225));

        // Very simple tall, shifted brown rectangle for the door
        let cy = track_rect.center().y;
        let w = 3.0;
        let h = 7.0;
        let shift = 1.5; // Y-offset to make it look "shifted"

        use egui::Shape;
        p.add(Shape::convex_polygon(
            vec![
                egui::pos2(center_x - w, cy - h),                 // top left
                egui::pos2(center_x + w, cy - h + shift),         // top right (shifted down)
                egui::pos2(center_x + w, cy + h + shift),         // bottom right (shifted down)
                egui::pos2(center_x - w, cy + h),                 // bottom left
            ],
            Color32::from_rgb(139, 69, 19), // simple brown
            Stroke::new(1.0, Color32::from_rgb(100, 50, 10))
        ));
    }

    fn paint_slider_thumb(&self, ui: &mut Ui, handle_rect: Rect, is_down: bool, is_hov: bool) {
        let p = ui.painter();
        let r = handle_rect.width() / 2.0;

        let push_y = if is_down { 1.5 } else { 0.0 };
        let draw_rect = handle_rect.translate(egui::vec2(0.0, push_y));

        let shadow_op = if is_down { 0 } else { 60 };
        if shadow_op > 0 {
            p.rect_filled(
                handle_rect.translate(egui::vec2(0.0, 1.5)),
                r + 0.5,
                Color32::from_rgba_premultiplied(0, 0, 0, shadow_op),
            );
        }

        let darken = if is_down { 0.7 } else if is_hov { 1.1 } else { 1.0 };
        let c = METALLIC_BODY;
        let c_r = (c.r() as f32 * darken).clamp(0.0, 255.0) as u8;
        let c_g = (c.g() as f32 * darken).clamp(0.0, 255.0) as u8;
        let c_b = (c.b() as f32 * darken).clamp(0.0, 255.0) as u8;
        let active = Color32::from_rgba_premultiplied(c_r, c_g, c_b, c.a());

        p.rect_filled(draw_rect, r, active);

        let lr = (c_r as f32 * 1.5).min(255.0) as u8;
        let lg = (c_g as f32 * 1.5).min(255.0) as u8;
        let lb = (c_b as f32 * 1.5).min(255.0) as u8;
        let glow_rect = Rect::from_min_max(
            egui::pos2(draw_rect.min.x + 1.5, draw_rect.center().y),
            draw_rect.max - egui::vec2(1.5, 1.5),
        );
        p.rect_filled(glow_rect, egui::epaint::CornerRadiusF32 { nw: 0.0, ne: 0.0, sw: r - 1.5, se: r - 1.5 },
            Color32::from_rgba_premultiplied(lr, lg, lb, c.a()));

        let hl_op = if is_down { 110 } else { 220 };
        let hl_rect = Rect::from_min_size(
            draw_rect.min + egui::vec2(1.5, 1.0),
            egui::vec2(draw_rect.width() - 3.0, r * 0.8),
        );
        p.rect_filled(hl_rect,
            egui::epaint::CornerRadiusF32 { nw: r - 1.5, ne: r - 1.5, sw: r * 0.3, se: r * 0.3 },
            Color32::from_rgba_premultiplied(255, 255, 255, hl_op));

        p.rect_stroke(draw_rect, r, Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 0, 0, 130)), egui::StrokeKind::Outside);
    }

    fn paint_slider_text(&self, ui: &mut Ui, text: &str) {
        if !text.is_empty() {
            ui.vertical(|ui| {
                ui.style_mut().spacing.item_spacing.y = 0.0;
                let mut first = true;
                for line in text.split('\n') {
                    let rt = if first {
                        egui::RichText::new(line).monospace().size(18.0).color(Color32::from_rgb(100, 100, 105))
                    } else {
                        egui::RichText::new(line).monospace().size(7.0).color(Color32::from_rgb(100, 100, 105))
                    };
                    ui.label(rt);
                    first = false;
                }
            });
        }
    }

    fn paint_slider_gauge(&self, ui: &mut Ui, bg_rect: Rect, fill_rect: Rect, is_down: bool, is_hov: bool) {
        let p = ui.painter();
        
        p.rect_filled(bg_rect, R_SM, INSET_FILL);
        draw_inset(p, bg_rect);
        
        if fill_rect.width() > 0.0 {
            let r = R_SM;
            let darken = if is_down { 0.7 } else if is_hov { 1.1 } else { 1.0 };
            let c = METALLIC_BODY;
            let c_r = (c.r() as f32 * darken).clamp(0.0, 255.0) as u8;
            let c_g = (c.g() as f32 * darken).clamp(0.0, 255.0) as u8;
            let c_b = (c.b() as f32 * darken).clamp(0.0, 255.0) as u8;
            let active = Color32::from_rgba_premultiplied(c_r, c_g, c_b, c.a());

            p.rect_filled(fill_rect, r, active);

            let lr = (c_r as f32 * 1.5).min(255.0) as u8;
            let lg = (c_g as f32 * 1.5).min(255.0) as u8;
            let lb = (c_b as f32 * 1.5).min(255.0) as u8;
            
            // Only draw interior glows if the rect is wide enough
            if fill_rect.width() > 3.0 {
                let glow_rect = Rect::from_min_max(
                    egui::pos2(fill_rect.min.x + 1.5, fill_rect.center().y),
                    fill_rect.max - egui::vec2(1.5, 1.5),
                );
                p.rect_filled(glow_rect, egui::epaint::CornerRadiusF32 { nw: 0.0, ne: 0.0, sw: r - 1.5, se: r - 1.5 },
                    Color32::from_rgba_premultiplied(lr, lg, lb, c.a()));

                let hl_op = if is_down { 110 } else { 220 };
                let hl_rect = Rect::from_min_size(
                    fill_rect.min + egui::vec2(1.5, 1.0),
                    egui::vec2(fill_rect.width() - 3.0, r * 0.8),
                );
                p.rect_filled(hl_rect,
                    egui::epaint::CornerRadiusF32 { nw: r - 1.5, ne: r - 1.5, sw: r * 0.3, se: r * 0.3 },
                    Color32::from_rgba_premultiplied(255, 255, 255, hl_op));
            }

            p.rect_stroke(fill_rect, r, Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 0, 0, 130)), egui::StrokeKind::Outside);
        }
    }
    fn section_label(&self, ui: &mut Ui, text: &str) -> Response { section_label(ui, text) }
    fn text_field_edit(&self, ui: &mut Ui, text: &mut String, font_size: f32, height: f32) -> Response { text_field_edit(self, ui, text, font_size, height) }

    fn button_text_color(&self) -> egui::Color32 {
        egui::Color32::BLACK
    }

    fn paint_button(&self, ui: &mut egui::Ui, rect: egui::Rect, is_down: bool, is_hovered: bool) -> f32 {
        let id_hash = (rect.min.x as i32, rect.min.y as i32);
        let press_t = ui.ctx().animate_value_with_time(
            ui.id().with(id_hash).with("gd_press"),
            if is_down { 1.0 } else { 0.0 },
            0.05,
        );
        let hover_t = ui.ctx().animate_value_with_time(
            ui.id().with(id_hash).with("gd_hover"),
            if is_hovered { 1.0 } else { 0.0 },
            0.12,
        );

        let push_y = press_t * 1.5;
        let draw_rect = rect.translate(egui::vec2(0.0, push_y));
        let r = rect.height() / 2.0;
        let p = ui.painter();

        let shadow_op = (60.0 * (1.0 - press_t)) as u8;
        if shadow_op > 0 {
            p.rect_filled(
                rect.translate(egui::vec2(0.0, 1.5)),
                r + 0.5,
                Color32::from_rgba_premultiplied(0, 0, 0, shadow_op),
            );
        }

        let darken = 1.0 - press_t * 0.3 + hover_t * 0.12;
        let c = METALLIC_BODY;
        let active = Color32::from_rgba_premultiplied(
            (c.r() as f32 * darken) as u8,
            (c.g() as f32 * darken) as u8,
            (c.b() as f32 * darken) as u8, c.a(),
        );
        p.rect_filled(draw_rect, r, active);

        let lr = (c.r() as f32 * darken * 1.5).min(255.0) as u8;
        let lg = (c.g() as f32 * darken * 1.5).min(255.0) as u8;
        let lb = (c.b() as f32 * darken * 1.5).min(255.0) as u8;
        let glow_rect = Rect::from_min_max(
            egui::pos2(draw_rect.min.x + 1.5, draw_rect.center().y),
            draw_rect.max - egui::vec2(1.5, 1.5),
        );
        p.rect_filled(glow_rect, egui::epaint::CornerRadiusF32 { nw: 0.0, ne: 0.0, sw: r - 1.5, se: r - 1.5 },
            Color32::from_rgba_premultiplied(lr, lg, lb, c.a()));

        let hl_op = (220.0 * (1.0 - press_t * 0.5)) as u8;
        if hl_op > 0 {
            let hl_rect = Rect::from_min_size(
                draw_rect.min + egui::vec2(1.5, 1.0),
                egui::vec2(draw_rect.width() - 3.0, r * 0.8),
            );
            p.rect_filled(hl_rect,
                egui::epaint::CornerRadiusF32 { nw: r - 1.5, ne: r - 1.5, sw: r * 0.3, se: r * 0.3 },
                Color32::from_rgba_premultiplied(255, 255, 255, hl_op));
        }

        p.rect_stroke(draw_rect, r, Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 0, 0, 130)), egui::StrokeKind::Outside);

        push_y
    }



}
