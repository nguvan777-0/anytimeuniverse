//! Future theme
#![allow(dead_code)]

use egui::{
    Color32, Stroke, CornerRadius, Margin, Vec2, Context, Visuals, Rect, Painter,
    Response, Ui, FontId, Sense, pos2, vec2,
};
use crate::ui::ResponseExt;
// ── Future palette ────────────────────────────────────────────────────────────
// Backgrounds
const BG:             Color32 = Color32::from_rgb( 14,  15,  21); // near-black gunmetal
const PANEL:          Color32 = Color32::from_rgb( 20,  22,  30); // panel surface
const INSET_FILL:     Color32 = Color32::from_rgb(  8,   9,  14); // deep inset field
const INSET_BORDER:   Color32 = Color32::from_rgb( 42,  46,  62); // inset rim

// Text
const TEXT:           Color32 = Color32::from_rgb(192, 202, 222); // cold blue-white

// Button body — polished gunmetal
const FUTURE_BODY:    Color32 = Color32::from_rgb( 88,  94, 112);
const FUTURE_PRESSED: Color32 = Color32::from_rgb( 52,  56,  70);
const FUTURE_GLOW:    Color32 = Color32::from_rgb(130, 148, 192); // blue future reflection
const FUTURE_BORDER:  Color32 = Color32::from_rgb( 20,  22,  36);

// ── Apply theme ───────────────────────────────────────────────────────────────

pub fn apply_theme(ctx: &Context) {
    let mut style = (*ctx.global_style()).clone();

    let mut visuals = Visuals::dark();
    visuals.window_fill        = BG;
    visuals.panel_fill         = PANEL;
    visuals.extreme_bg_color   = INSET_FILL;
    visuals.faint_bg_color     = Color32::from_rgb(18, 20, 28);
    visuals.code_bg_color      = INSET_FILL;
    visuals.override_text_color = Some(TEXT);
    visuals.text_cursor.stroke.color = TEXT;

    let r = CornerRadius::same(5);
    let border = Stroke::new(1.0, INSET_BORDER);
    let fg     = Stroke::new(1.0, TEXT);

    visuals.widgets.noninteractive.bg_fill      = PANEL;
    visuals.widgets.noninteractive.weak_bg_fill  = PANEL;
    visuals.widgets.noninteractive.bg_stroke     = border;
    visuals.widgets.noninteractive.fg_stroke     = fg;
    visuals.widgets.noninteractive.corner_radius      = r;
    visuals.widgets.noninteractive.expansion     = 0.0;

    visuals.widgets.inactive.bg_fill      = PANEL;
    visuals.widgets.inactive.weak_bg_fill  = PANEL;
    visuals.widgets.inactive.bg_stroke     = border;
    visuals.widgets.inactive.fg_stroke     = fg;
    visuals.widgets.inactive.corner_radius      = r;
    visuals.widgets.inactive.expansion     = 0.0;

    visuals.widgets.hovered.bg_fill      = Color32::from_rgb(30, 33, 44);
    visuals.widgets.hovered.weak_bg_fill  = Color32::from_rgb(30, 33, 44);
    visuals.widgets.hovered.bg_stroke     = Stroke::new(1.0, Color32::from_rgb(80, 90, 120));
    visuals.widgets.hovered.fg_stroke     = fg;
    visuals.widgets.hovered.corner_radius      = r;
    visuals.widgets.hovered.expansion     = 0.0;

    visuals.widgets.active.bg_fill      = Color32::from_rgb(50, 55, 75);
    visuals.widgets.active.weak_bg_fill  = Color32::from_rgb(50, 55, 75);
    visuals.widgets.active.bg_stroke     = Stroke::new(1.0, Color32::from_rgb(100, 120, 160));
    visuals.widgets.active.fg_stroke     = Stroke::new(1.0, TEXT);
    visuals.widgets.active.corner_radius      = r;
    visuals.widgets.active.expansion     = 0.0;

    // Highlight: match the shiny blue-shifted button color
    visuals.selection.bg_fill = FUTURE_GLOW;
    visuals.selection.stroke  = Stroke::new(1.0, Color32::BLACK);

    visuals.window_corner_radius = CornerRadius::same(6);
    visuals.menu_corner_radius   = CornerRadius::same(6);

    style.visuals = visuals;
    style.interaction.selectable_labels = false;
    style.spacing.item_spacing      = Vec2::new(6.0, 5.0);
    style.spacing.button_padding    = Vec2::new(10.0, 5.0);
    style.spacing.window_margin     = Margin::same(8);
    style.spacing.slider_width      = 150.0;
    style.spacing.interact_size.y   = 20.0;

    ctx.set_global_style(style);
}

// ── Background pattern ────────────────────────────────────────────────────────

/// Subtle horizontal scan lines — a CRT-future hybrid texture.
pub fn draw_scan_lines(painter: &Painter, rect: Rect) {
    let color = Color32::from_rgba_premultiplied(255, 255, 255, 6);
    let mut y = rect.min.y;
    while y < rect.max.y {
        painter.line_segment(
            [pos2(rect.min.x, y), pos2(rect.max.x, y)],
            Stroke::new(1.0, color),
        );
        y += 3.0;
    }
}

// ── Inset field ───────────────────────────────────────────────────────────────

pub fn draw_inset(painter: &Painter, rect: Rect) {
    painter.rect_filled(rect, 3.0, INSET_FILL);
    // Top-left darker inner shadow — bright edges for metal depth
    let hi = Color32::from_rgba_premultiplied(255, 255, 255, 18);
    let sh = Color32::from_rgba_premultiplied(0, 0, 0, 80);
    let r = rect;
    painter.line_segment([r.left_top(),  r.right_top()],   Stroke::new(1.0, sh));
    painter.line_segment([r.left_top(),  r.left_bottom()],  Stroke::new(1.0, sh));
    painter.line_segment([r.right_top(), r.right_bottom()], Stroke::new(1.0, hi));
    painter.line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(1.0, hi));
}

// ── Iridescent stripe helper ──────────────────────────────────────────────────

/// Teal → ice-blue → near-white → lavender → violet gradient.
fn irid_color(t: f32) -> Color32 {
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t.clamp(0.0, 1.0);
    let lerp3 = |a: (f32, f32, f32), b: (f32, f32, f32), t: f32| {
        (lerp(a.0, b.0, t), lerp(a.1, b.1, t), lerp(a.2, b.2, t))
    };
    let (r, g, b) = if t < 0.30 {
        lerp3((55.0, 195.0, 215.0), (170.0, 225.0, 255.0), t / 0.30)
    } else if t < 0.50 {
        lerp3((170.0, 225.0, 255.0), (238.0, 244.0, 255.0), (t - 0.30) / 0.20)
    } else if t < 0.70 {
        lerp3((238.0, 244.0, 255.0), (212.0, 185.0, 255.0), (t - 0.50) / 0.20)
    } else {
        lerp3((212.0, 185.0, 255.0), (155.0, 72.0, 228.0), (t - 0.70) / 0.30)
    };
    Color32::from_rgb(r as u8, g as u8, b as u8)
}

/// Draws the signature future iridescent stripe across the top edge of a rect.
fn draw_irid_stripe(painter: &Painter, draw_rect: Rect, r: f32, alpha_factor: f32) {
    if alpha_factor <= 0.0 {
        return;
    }
    let n: usize = 16;
    let stripe_h = 2.5f32;
    let sx = draw_rect.min.x + 2.0;
    let sw = draw_rect.width() - 4.0;
    if sw <= 0.0 { return; }
    let sy = draw_rect.min.y + 1.0;
    for i in 0..n {
        let t0 = i as f32 / n as f32;
        let t1 = (i + 1) as f32 / n as f32;
        let tc = (t0 + t1) * 0.5;
        let base = irid_color(tc);
        let alpha = (160.0 * alpha_factor) as u8;
        let col = Color32::from_rgba_premultiplied(
            ((base.r() as f32) * alpha_factor) as u8,
            ((base.g() as f32) * alpha_factor) as u8,
            ((base.b() as f32) * alpha_factor) as u8,
            alpha,
        );
        let x0 = sx + t0 * sw;
        let x1 = sx + t1 * sw;
        let seg = Rect::from_min_max(pos2(x0, sy), pos2(x1, sy + stripe_h));
        let rnd = egui::epaint::CornerRadiusF32 {
            nw: if i == 0     { r - 2.0 } else { 0.0 },
            ne: if i == n - 1 { r - 2.0 } else { 0.0 },
            sw: 0.0,
            se: 0.0,
        };
        painter.rect_filled(seg, rnd, col);
    }
}

// ── Holographic Hover Effect ──────────────────────────────────────────────────

pub fn draw_holographic_hover(_ctx: &Context, p: &Painter, draw_rect: Rect, r: f32, hover_t: f32) {
    let n: usize = 32;
    let shrink = 1.0;
    let sx = draw_rect.min.x + shrink;
    let sy = draw_rect.min.y + shrink;
    let sw = draw_rect.width() - shrink * 2.0;
    let sh = draw_rect.height() - shrink * 2.0;
    let r_inner = (r - shrink).max(0.0);

    if sw <= 0.0 || sh <= 0.0 { return; }

    // Use full-body holographic rainbow as the DEFAULT view!
    // Base brightness is 0.70, pushing up to 1.0 when actually hovered!
    let alpha_factor = 0.70 + (hover_t * 0.30);

    for i in 0..n {
        let t0 = i as f32 / n as f32;
        let t1 = (i + 1) as f32 / n as f32;
        let tc = (t0 + t1) * 0.5;
        let base = irid_color(tc);
        let alpha = (255.0 * alpha_factor) as u8;
        
        let col = Color32::from_rgba_premultiplied(
            ((base.r() as f32) * alpha_factor) as u8,
            ((base.g() as f32) * alpha_factor) as u8,
            ((base.b() as f32) * alpha_factor) as u8,
            alpha,
        );
        let x0 = sx + t0 * sw;
        let x1 = sx + t1 * sw;
        let seg = Rect::from_min_max(pos2(x0, sy), pos2(x1, sy + sh));
        let rnd = egui::epaint::CornerRadiusF32 {
            nw: if i == 0     { r_inner } else { 0.0 },
            sw: if i == 0     { r_inner } else { 0.0 },
            ne: if i == n - 1 { r_inner } else { 0.0 },
            se: if i == n - 1 { r_inner } else { 0.0 },
        };
        p.rect_filled(seg, rnd, col);
    }
}

// ── Future button primitive ───────────────────────────────────────────────────

pub fn draw_future_pill_base(ctx: &Context, p: &Painter, draw_rect: Rect, base_rect: Rect, press_t: f32, hover_t: f32) {
    let r = base_rect.height() / 2.0;

    // Drop shadow — stays at rest, fades as face descends into it (and fades on hover)
    let sh_op = (80.0 * (1.0 - press_t) * (1.0 - hover_t)).max(0.0) as u8;
    if sh_op > 0 {
        p.rect_filled(
            base_rect.translate(vec2(0.0, 1.5)),
            r + 0.5,
            Color32::from_rgba_premultiplied(0, 0, 0, sh_op),
        );
    }

    // Body — ALWAYS draw bright iridescent holographic rainbow
    draw_holographic_hover(ctx, p, draw_rect, r, 1.0);

    // Bottom future reflection (blue-shifted, lower half) - intensifies on hover
    let glow_op = (155.0 * (1.0 - press_t * 0.65) + hover_t * 80.0).clamp(0.0, 255.0) as u8;
    if glow_op > 0 {
        let glow = Rect::from_min_max(
            pos2(draw_rect.min.x + 1.5, draw_rect.center().y),
            draw_rect.max - vec2(1.5, 1.5),
        );
        p.rect_filled(glow,
            egui::epaint::CornerRadiusF32 { nw: 0.0, ne: 0.0, sw: r - 1.5, se: r - 1.5 },
            Color32::from_rgba_premultiplied(
                FUTURE_GLOW.r(), FUTURE_GLOW.g(), FUTURE_GLOW.b(), glow_op));
    }

    // Top specular cap — very bright, hallmark of polished future - intensifies on hover
    let hl_op = (215.0 * (1.0 - press_t * 0.55) + hover_t * 40.0).clamp(0.0, 255.0) as u8;
    if hl_op > 0 {
        let hl = Rect::from_min_size(
            draw_rect.min + vec2(1.5, 1.0),
            vec2(draw_rect.width() - 3.0, r * 0.72),
        );
        p.rect_filled(hl,
            egui::epaint::CornerRadiusF32 { nw: r - 1.5, ne: r - 1.5, sw: r * 0.25, se: r * 0.25 },
            Color32::from_rgba_premultiplied(215, 228, 255, hl_op));
    }

    // Full-body Holographic Iridescent Background (Default View!)
    draw_holographic_hover(ctx, p, draw_rect, r, hover_t);

    // Etched dark border
    p.rect_stroke(draw_rect, r, Stroke::new(1.0,
        Color32::from_rgba_premultiplied(FUTURE_BORDER.r(), FUTURE_BORDER.g(), FUTURE_BORDER.b(), 210)), egui::StrokeKind::Outside);
}

/// Polished future pill with squish physics.  Returns the Y push-down (0–1.5).
pub fn draw_future_pill(ui: &mut Ui, response: &Response, rect: Rect, is_pressed: bool) -> f32 {
    let pressed = response.is_pointer_button_down_on() || is_pressed;
    let press_t = ui.ctx().animate_value_with_time(
        response.id.with("ch_press"),
        if pressed { 1.0 } else { 0.0 },
        0.05,
    );
    let hover_t = ui.ctx().animate_value_with_time(
        response.id.with("ch_hover"),
        if response.hovered() { 1.0 } else { 0.0 },
        0.12,
    );

    let push_y = press_t * 1.5;
    let draw_rect = rect.translate(vec2(0.0, push_y));
    let p = ui.painter();

    draw_future_pill_base(ui.ctx(), p, draw_rect, rect, press_t, hover_t);

    push_y
}

// ── Section label ─────────────────────────────────────────────────────────────

/// Etched future section label — silver text with a subtle 1px shadow below.
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
        FontId::proportional(12.0),
        TEXT,
    );
    let (rect, resp) = ui.allocate_exact_size(galley.size(), Sense::click());
    if ui.is_rect_visible(rect) {
        // Draw a solid panel block behind the text to clear the scan line grid
        ui.painter().rect_filled(rect.expand(2.0), 0.0, Color32::BLACK);

        // Engraved shadow (1px below, even darker)
        let shadow_galley = ui.painter().layout_no_wrap(
            text.to_owned(),
            FontId::proportional(12.0),
            Color32::BLACK,
        );
        ui.painter().galley(rect.min + vec2(0.0, 1.0), shadow_galley, Color32::TRANSPARENT);
        ui.painter().galley(rect.min, galley, TEXT);
    }
    resp
}

// ── Future orb button ────────────────────────────

/// Same signature as `dew::draw_dot_btn`, rendered in Future's steel orb style.
pub fn draw_orb_btn(
    ui: &mut Ui,
    resp: &Response,
    r: f32,
    _base_color: Color32, // ignored — Future always uses FUTURE_BODY
    symbol: &str,
    _group_hover_t: Option<f32>,
) {
    if !ui.is_rect_visible(resp.rect) { return; }

    let pressed = resp.is_pointer_button_down_on();
    let press_t = ui.ctx().animate_value_with_time(
        resp.id.with("press"), if pressed { 1.0 } else { 0.0 }, 0.05,
    );
    // Ignore grouped hover; make each light up individually
    let _hover_t = ui.ctx().animate_value_with_time(
        resp.id.with("hover"), if resp.hovered() { 1.0 } else { 0.0 }, 0.1,
    );

    let center = resp.rect.center();
    let push_y = press_t * 1.5;
    let dc = center + vec2(0.0, push_y);
    let p = ui.painter();

    // Shadow
    let sh = (70.0 * (1.0 - press_t)) as u8;
    if sh > 0 {
        p.circle_filled(center + vec2(0.0, 1.2), r + 0.5,
            Color32::from_rgba_premultiplied(0, 0, 0, sh));
    }

    // Body
    let darken = 1.0 - press_t * 0.35 + _hover_t * 0.25;
    let bc = FUTURE_BODY;
    let body = Color32::from_rgb(
        ((bc.r() as f32 * darken).min(255.0)) as u8,
        ((bc.g() as f32 * darken).min(255.0)) as u8,
        ((bc.b() as f32 * darken).min(255.0)) as u8,
    );
    p.circle_filled(dc, r, body);
    
    // Bottom glow — blue-shifted, matches future pill
    let lr = (bc.r() as f32 * darken * 1.55).min(255.0) as u8;
    let lg = (bc.g() as f32 * darken * 1.65).min(255.0) as u8;
    let lb = (bc.b() as f32 * darken * 1.85).min(255.0) as u8;
    p.circle_filled(dc + vec2(0.0, 1.5), r - 1.5, Color32::from_rgb(lr, lg, lb));

    // Top specular — blue-tinted, matches future pill
    let hl_op = (210.0 * (1.0 - press_t * 0.5) * (1.0 + _hover_t * 0.05)).min(255.0) as u8;
    if hl_op > 0 {
        let hl_rect = Rect::from_min_size(dc - vec2(r - 1.5, r - 1.0),
            vec2((r - 1.5) * 2.0, r * 0.78));
        p.rect_filled(hl_rect, r,
            Color32::from_rgba_premultiplied(215, 228, 255, hl_op));
    }

    // Add digital static localized to the circle
    let draw_rect = Rect::from_center_size(dc, vec2(r * 2.0, r * 2.0));
    draw_holographic_hover(ui, p, draw_rect, r, _hover_t);

    // Symbols — drawn as text, sunken/embossed
    {
        let font = egui::FontId::proportional(16.0);
        let hl_col = Color32::from_rgba_premultiplied(255, 255, 255, 180);
        let ink_col = Color32::BLACK;

        let mut pos_offset = egui::vec2(0.0, 0.0);
        if symbol == "." {
            pos_offset.y -= 3.5; // Shift the period up from the baseline to center it visually
        }

        // Just one crisp highlight exactly 1px down for a clean glass emboss (no blurry 4x overlay veil)
        p.text(dc + pos_offset + egui::vec2(0.0, 1.0), egui::Align2::CENTER_CENTER, symbol, font.clone(), hl_col);
        p.text(dc + pos_offset, egui::Align2::CENTER_CENTER, symbol, font.clone(), ink_col);
    }

    // Outline
    p.circle_stroke(dc, r, egui::Stroke::new(1.0,
        Color32::from_rgba_premultiplied(0, 0, 0, 140)));
}

// ── Section toggle button ─────────────────────────────────────────────────────

/// Small future orb used as a section collapse / expand toggle.
pub fn section_toggle_btn(ui: &mut Ui) -> Response {
    let r = 6.0f32;
    let (rect, resp) = ui.allocate_exact_size(vec2(r * 2.0 + 2.0, r * 2.0 + 2.0), Sense::click());
    if !ui.is_rect_visible(rect) { return resp; }

    draw_orb_btn(ui, &resp, r, FUTURE_BODY, ".", None);
    resp
}

// ── Button ────────────────────────────────────────────────────────────────────

pub fn button(ui: &mut Ui, text: &str) -> Response {
    button_w(ui, text, 0.0)
}

pub fn button_w(ui: &mut Ui, text: &str, min_w: f32) -> Response {
    let mut padding = vec2(16.0, 6.0);
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        FontId::proportional(13.0),
        Color32::BLACK,
    );
    if min_w > 0.0 && galley.size().x + padding.x * 2.0 > min_w {
        padding.x = ((min_w - galley.size().x) / 2.0).max(4.0);
    }
    let w = (galley.size().x + padding.x * 2.0).max(min_w);
    let h = galley.size().y + padding.y * 2.0;
    let (rect, mut response) = ui.allocate_exact_size(vec2(w, h), Sense::click());
    if response.clicked() { response.mark_changed(); }

    if ui.is_rect_visible(rect) {
        let shift_y = draw_future_pill(ui, &response, rect, false);

        let text_pos = ui.layout().align_size_within_rect(galley.size(), rect.shrink(2.0)).min
            + vec2(0.0, shift_y);
        // White shadow text fades as button descends
        let shadow_a = (110.0 * (1.0 - shift_y / 1.5)) as u8;
        if shadow_a > 0 {
            ui.painter().galley(text_pos + vec2(0.0, 1.0),
                ui.painter().layout_no_wrap(text.to_string(), FontId::proportional(13.0),
                    Color32::WHITE),
                Color32::BLACK);
        }
        ui.painter().galley(text_pos, galley, Color32::BLACK);
    }
    response.hand()
}

// ── Key cap ───────────────────────────────────────────────────────────────────

pub fn key_cap(ui: &mut Ui, text: &str) -> Response {
    let galley = ui.painter().layout_no_wrap(
        text.to_string(), FontId::monospace(24.0), Color32::BLACK,
    );
    let padding = vec2(5.0, 2.0);
    let size = galley.size() + padding * 2.0;
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let shift_y = draw_future_pill(ui, &response, rect, false);
        let text_pos = ui.layout().align_size_within_rect(galley.size(), rect.shrink(2.0)).min
            + vec2(0.0, shift_y);
        let hover_t = ui.ctx().animate_value_with_time(
            response.id.with("ch_hover"),
            if response.hovered() { 1.0 } else { 0.0 },
            0.12,
        );
        let shadow_a = (110.0 * (1.0 - shift_y / 1.5) * (1.0 - hover_t)).max(0.0) as u8;
        if shadow_a > 0 {
            ui.painter().galley(text_pos + vec2(0.0, 1.0),
                ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(24.0),
                    Color32::from_white_alpha(shadow_a)),
                Color32::from_white_alpha(shadow_a));
        }
        ui.painter().galley(text_pos, galley, Color32::BLACK);
    }
    response.hand()
}

pub fn key_cap_small(ui: &mut Ui, text: &str, min_side: f32, font_size: f32, is_pressed: bool) -> Response {
    let measure = ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(font_size), Color32::BLACK);
    let gw = measure.size().x;
    let gh = measure.size().y;
    let side = min_side.max(gw + 6.0);
    let (rect, response) = ui.allocate_exact_size(vec2(side, side), Sense::click());
    if ui.is_rect_visible(rect) {
        let shift_y = draw_future_pill(ui, &response, rect, is_pressed);
        let c = rect.center();
        let pos = pos2(c.x - gw / 2.0, c.y - gh / 2.0 - 2.5 + shift_y);
        let hover_t = ui.ctx().animate_value_with_time(
            response.id.with("ch_hover"),
            if response.hovered() { 1.0 } else { 0.0 },
            0.12,
        );
        let shadow_a = (110.0 * (1.0 - shift_y / 1.5) * (1.0 - hover_t)).max(0.0) as u8;
        if shadow_a > 0 {
            ui.painter().add(egui::Shape::Text(egui::epaint::TextShape {
                pos: pos + vec2(0.0, 1.0),
                galley: ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(font_size), Color32::from_white_alpha(shadow_a)),
                underline: egui::Stroke::NONE,
                fallback_color: Color32::from_white_alpha(shadow_a),
                override_text_color: Some(Color32::from_white_alpha(shadow_a)),
                opacity_factor: 1.0,
                angle: 0.0,
            }));
        }
        ui.painter().add(egui::Shape::Text(egui::epaint::TextShape {
            pos,
            galley: ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(font_size), Color32::BLACK),
            underline: egui::Stroke::NONE,
            fallback_color: Color32::BLACK,
            override_text_color: Some(Color32::BLACK),
            opacity_factor: 1.0,
            angle: 0.0,
        }));
    }
    response.hand()
}

pub fn key_cap_small_rotated(ui: &mut Ui, text: &str, angle: f32, min_side: f32, font_size: f32, is_pressed: bool) -> Response {
    let measure = ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(font_size), Color32::BLACK);
    let gw = measure.size().x;
    let gh = measure.size().y;
    let side = min_side.max(gw + 6.0);
    let (rect, response) = ui.allocate_exact_size(vec2(side, side), Sense::click());
    if ui.is_rect_visible(rect) {
        let shift_y = draw_future_pill(ui, &response, rect, is_pressed);
        let s = angle.signum();
        let c = rect.center();
        let pos = pos2(c.x + s * (gh / 2.0 + 2.0), c.y - s * gw / 2.0 + shift_y);
        let hover_t = ui.ctx().animate_value_with_time(
            response.id.with("ch_hover"),
            if response.hovered() { 1.0 } else { 0.0 },
            0.12,
        );
        let shadow_a = (110.0 * (1.0 - shift_y / 1.5) * (1.0 - hover_t)).max(0.0) as u8;
        if shadow_a > 0 {
            ui.painter().add(egui::Shape::Text(egui::epaint::TextShape {
                pos: pos + vec2(0.0, 1.0),
                galley: ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(font_size), Color32::from_white_alpha(shadow_a)),
                underline: egui::Stroke::NONE,
                fallback_color: Color32::from_white_alpha(shadow_a),
                override_text_color: Some(Color32::from_white_alpha(shadow_a)),
                opacity_factor: 1.0,
                angle,
            }));
        }
        ui.painter().add(egui::Shape::Text(egui::epaint::TextShape {
            pos,
            galley: ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(24.0), Color32::BLACK),
            underline: egui::Stroke::NONE,
            fallback_color: Color32::BLACK,
            override_text_color: Some(Color32::BLACK),
            opacity_factor: 1.0,
            angle,
        }));
    }
    response.hand()
}

// ── Symmetric log slider ──────────────────────────────────────────────────────


// ── ThemeProvider impl ────────────────────────────────────────────────────────

pub struct Future;

impl crate::ui::theme::ThemeProvider for Future {
    fn palette(&self) -> crate::ui::theme::ThemePalette {
        crate::ui::theme::ThemePalette {
            is_terminal_style: false,
            panel_margin: 0.0,
            panel_text_color: egui::Color32::from_rgb(192, 202, 222),
            hash_stat_color: crate::ui::future::TEXT,
            hash_selection_color: crate::ui::future::FUTURE_GLOW,
            title_bar_text_color: egui::Color32::WHITE,
            title_bar_button_color: crate::ui::future::FUTURE_BODY,
            tracker_color: crate::ui::future::TEXT,
            chart_axis_color: egui::Color32::from_white_alpha(30),
            remove_tracker_border_on_hover: false,
        }
    }

    fn apply_theme(&self, ctx: &Context) {
        ctx.set_fonts(egui::FontDefinitions::default());
        apply_theme(ctx);
    }
    

    fn draw_background_pattern(&self, painter: &Painter, rect: Rect) {
        draw_scan_lines(painter, rect);
    }
    
    fn edit_popup_visuals(&self, visuals: &mut egui::Visuals) {
        visuals.window_fill = BG;
        visuals.panel_fill = PANEL;
        visuals.window_stroke = Stroke::new(1.0, INSET_BORDER);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, TEXT);

        visuals.widgets.hovered.bg_fill = Color32::from_rgb(130, 148, 192); // FUTURE_GLOW
        visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(130, 148, 192);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::BLACK);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    }

    fn paint_hash_bg(&self, p: &Painter, rect: Rect) {
        p.rect_filled(rect, egui::CornerRadius::same(3), INSET_FILL);
        crate::ui::dew::draw_inset(p, rect);
    }

    fn paint_title_bar_text_bg(&self, ui: &mut Ui, rect: Rect) {
        ui.painter().rect_filled(
            rect.expand2(egui::vec2(4.0, -1.0)),
            2.0,
            Color32::BLACK,
        );
    }

    
    fn paint_title_bar_button(&self, ui: &mut Ui, resp: &Response, r: f32, base_color: Color32, symbol: &str, hover_t: f32) {
        draw_orb_btn(ui, resp, r, base_color, symbol, Some(hover_t));
    }

    fn draw_sunken(&self, painter: &Painter, rect: Rect) { draw_inset(painter, rect); }

    fn draw_space_strategy_bg(&self, ui: &mut Ui, rect: Rect) {
        draw_inset(ui.painter(), rect);
        
        // Faded scan lines for the Space Strategy chart background (~10% of typical brightness)
        // Egui's `from_rgba_premultiplied` physically adds the RGB values. We must lower them to fade.
        let color = Color32::from_rgba_premultiplied(8, 8, 8, 1);
        let mut y = rect.min.y;
        while y < rect.max.y {
            ui.painter().line_segment(
                [pos2(rect.min.x, y), pos2(rect.max.x, y)],
                Stroke::new(1.0, color),
            );
            y += 3.0;
        }
    }

    fn section_toggle_btn(&self, ui: &mut Ui) -> Response { section_toggle_btn(ui) }
    fn key_cap_small(&self, ui: &mut Ui, text: &str, side: f32, font_size: f32, is_pressed: bool) -> Response { key_cap_small(ui, text, side, font_size, is_pressed) }
    fn key_cap_small_rotated(&self, ui: &mut Ui, text: &str, angle: f32, side: f32, font_size: f32, is_pressed: bool) -> Response { key_cap_small_rotated(ui, text, angle, side, font_size, is_pressed) }
    fn paint_slider_track(&self, ui: &mut Ui, track_rect: Rect, center_x: f32) {
        let p = ui.painter();
        p.rect_filled(track_rect, 0.0, Color32::from_rgb(4, 5, 8));

        // A little window into a starfield!
        let cy = track_rect.center().y;
        let window_rect = Rect::from_center_size(
            egui::pos2(center_x, cy),
            egui::vec2(6.0, 12.0) // 33% smaller
        );

        // Deep space background
        p.rect_filled(window_rect, 0.0, Color32::from_rgb(2, 2, 8));

        // Some stars! (deterministic static positions inside our window)
        let stars = [
            egui::vec2(-2.0, -4.5),
            egui::vec2(1.5, -3.0),
            egui::vec2(-1.0, -1.0),
            egui::vec2(1.0, 1.0),
            egui::vec2(-1.5, 3.5),
            egui::vec2(2.0, 4.5),
        ];

        let star_colors = [
            Color32::from_rgb(255, 255, 255),
            Color32::from_rgb(200, 220, 255), // bluish
            Color32::from_rgb(255, 255, 200), // yellowish
            Color32::from_rgb(255, 255, 255),
            Color32::from_rgb(200, 255, 255), // cyan-ish
            Color32::from_rgb(255, 200, 200), // reddish
        ];

        for (i, offset) in stars.iter().enumerate() {
            let radius = if i % 3 == 0 { 0.6 } else { 0.3 }; // some big, some small
            p.circle_filled(
                egui::pos2(center_x + offset.x, cy + offset.y),
                radius,
                star_colors[i % star_colors.len()]
            );
        }
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

        let darken = if is_down { 0.7 } else if is_hov { 1.2 } else { 1.0 };
        let c = FUTURE_BODY;
        let c_r = (c.r() as f32 * darken).clamp(0.0, 255.0) as u8;
        let c_g = (c.g() as f32 * darken).clamp(0.0, 255.0) as u8;
        let c_b = (c.b() as f32 * darken).clamp(0.0, 255.0) as u8;
        let active = Color32::from_rgb(c_r, c_g, c_b);

        p.rect_filled(draw_rect, r, active);

        let glow_op = if is_down { 100 } else if is_hov { 200 } else { 180 };
        let lr = (c_r as f32 * 1.5).min(255.0) as u8;
        let lg = (c_g as f32 * 1.5).min(255.0) as u8;
        let lb = (c_b as f32 * 1.5).min(255.0) as u8;
        let glow_rect = Rect::from_min_max(
            egui::pos2(draw_rect.min.x + 1.5, draw_rect.center().y),
            draw_rect.max - egui::vec2(1.5, 1.5),
        );
        p.rect_filled(glow_rect, egui::epaint::CornerRadiusF32 { nw: 0.0, ne: 0.0, sw: r - 1.5, se: r - 1.5 },
            Color32::from_rgba_premultiplied(lr, lg, lb, glow_op));

        let hl_op = if is_down { 110 } else { 220 };
        let hl_rect = Rect::from_min_size(
            draw_rect.min + egui::vec2(1.5, 1.0),
            egui::vec2(draw_rect.width() - 3.0, r * 0.8),
        );
        p.rect_filled(hl_rect,
            egui::epaint::CornerRadiusF32 { nw: r - 1.5, ne: r - 1.5, sw: r * 0.3, se: r * 0.3 },
            Color32::from_rgba_premultiplied(255, 255, 255, hl_op));

        let hover_t = ui.ctx().animate_value_with_time(
            ui.id().with("thumb_hover").with(handle_rect.center().x as i32),
            if is_hov { 1.0 } else { 0.0 },
            0.12,
        );
        draw_holographic_hover(ui.ctx(), p, draw_rect, r, hover_t);

        p.rect_stroke(draw_rect, r, Stroke::new(1.0,
            Color32::from_rgba_premultiplied(FUTURE_BORDER.r(), FUTURE_BORDER.g(), FUTURE_BORDER.b(), 210)), egui::StrokeKind::Outside);
    }

    fn paint_slider_text(&self, ui: &mut Ui, text: &str) {
        if !text.is_empty() {
            egui::Frame::NONE.fill(PANEL).show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.style_mut().spacing.item_spacing.y = 0.0;
                    let mut first = true;
                    for line in text.split('\n') {
                        let rt = if first {
                            egui::RichText::new(line).monospace().size(18.0).color(TEXT)
                        } else {
                            egui::RichText::new(line).monospace().size(7.0).color(TEXT)
                        };
                        ui.label(rt);
                        first = false;
                    }
                });
            });
        }
    }
    fn section_label(&self, ui: &mut Ui, text: &str) -> Response { section_label(ui, text) }

    fn paint_slider_gauge(&self, ui: &mut Ui, bg_rect: Rect, fill_rect: Rect, is_down: bool, is_hovered: bool) {
        let p = ui.painter();
        p.rect_filled(bg_rect, 2.0, Color32::from_rgb(4, 5, 8)); // Matches future track background
        
        if fill_rect.width() > 0.0 {
            let darken = if is_down { 0.7 } else if is_hovered { 1.2 } else { 1.0 };
            let c = FUTURE_GLOW;
            let c_r = (c.r() as f32 * darken).clamp(0.0, 255.0) as u8;
            let c_g = (c.g() as f32 * darken).clamp(0.0, 255.0) as u8;
            let c_b = (c.b() as f32 * darken).clamp(0.0, 255.0) as u8;
            let active = Color32::from_rgb(c_r, c_g, c_b);

            p.rect_filled(fill_rect, 2.0, active);
            
            // Specular metallic highlight
            let hl_op = if is_down { 30 } else { 60 };
            let hl_rect = Rect::from_min_size(
                fill_rect.min + egui::vec2(0.0, 1.0),
                egui::vec2(fill_rect.width(), fill_rect.height() * 0.4),
            );
            p.rect_filled(hl_rect, 2.0,
                Color32::from_rgba_premultiplied(255, 255, 255, hl_op));

            let target_active = if is_hovered || is_down { 1.0 } else { 0.0 };
            let active_t = ui.ctx().animate_value_with_time(
                ui.id().with(fill_rect.min.x as i32).with("gauge_active"),
                target_active,
                0.12,
            );
            draw_digital_static_grid(ui.ctx(), p, fill_rect, 2.0, active_t);
        }
    }
    fn text_field_edit(&self, ui: &mut Ui, text: &mut String, font_size: f32, height: f32) -> Response {
        let field_w = ui.available_width();
        let padding = egui::vec2(6.0, 3.0);
        let field_h = if height > 0.0 { height } else { font_size + padding.y * 2.0 + 2.0 };
        let (rect, _) = ui.allocate_exact_size(egui::vec2(field_w, field_h), Sense::hover());
        if ui.is_rect_visible(rect) {
            let p = ui.painter();
            // Solid background to hide grid for this text area
            p.rect_filled(rect, CornerRadius::same(3), INSET_FILL);
            draw_inset(p, rect);
        }
        let inner_rect = egui::Rect::from_center_size(
            rect.center(),
            egui::vec2(rect.width() - 8.0, font_size + padding.y * 2.0 + 2.0),
        );
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner_rect).layout(*ui.layout()));
        child.visuals_mut().selection.bg_fill = FUTURE_GLOW;
        child.visuals_mut().selection.stroke = Stroke::new(1.0, Color32::BLACK);

        let text_edit = egui::TextEdit::singleline(text)
            .font(egui::FontId::monospace(font_size))
            .horizontal_align(egui::Align::Center)
            .frame(egui::Frame::NONE)
            .text_color(TEXT);

        let mut te_resp = child.add(text_edit);
        crate::ui::widgets::maintain_text_selection_cache(ui.ctx(), &te_resp, text, rect);
        if crate::ui::widgets::text_field_context_menu(self, &te_resp, te_resp.id, text) { te_resp.mark_changed(); }
        te_resp
    }

    fn button_text_color(&self) -> egui::Color32 {
        egui::Color32::BLACK
    }

    fn gauge_label_shadow(&self) -> Option<egui::Color32> {
        Some(egui::Color32::from_black_alpha(220))
    }

    fn gauge_label_text_color(&self) -> Option<egui::Color32> {
        Some(egui::Color32::WHITE)
    }

    fn paint_button(&self, ui: &mut egui::Ui, rect: egui::Rect, is_down: bool, is_hovered: bool) -> f32 {
        let id_hash = (rect.min.x as i32, rect.min.y as i32);
        let press_t = ui.ctx().animate_value_with_time(
            ui.id().with(id_hash).with("ch_press"),
            if is_down { 1.0 } else { 0.0 },
            0.05,
        );
        let hover_t = ui.ctx().animate_value_with_time(
            ui.id().with(id_hash).with("ch_hover"),
            if is_hovered { 1.0 } else { 0.0 },
            0.12,
        );

        let push_y = press_t * 1.5;
        let draw_rect = rect.translate(egui::vec2(0.0, push_y));
        let p = ui.painter();

        draw_future_pill_base(ui.ctx(), p, draw_rect, rect, press_t, hover_t);

        push_y
    }




}

// ── Digital Static Grid (Volume Slider) ───────────────────────────────────────

pub fn draw_digital_static_grid(ctx: &egui::Context, p: &egui::Painter, draw_rect: egui::Rect, r: f32, hover_t: f32) {
    if hover_t <= 0.01 { return; }
    ctx.request_repaint();
    let t = (ctx.input(|i| i.time) * 12.0) as u32; // 12 FPS grid step
    let grid_size: f32 = 3.0; // 3x3 blocks
    let min_x = draw_rect.min.x + 1.0;
    let max_x = draw_rect.max.x - 1.0;
    let min_y = draw_rect.min.y + 1.0;
    let max_y = draw_rect.max.y - 1.0;
    let is_horizontal = draw_rect.width() >= draw_rect.height();
    let cx1 = if is_horizontal { draw_rect.min.x + r } else { draw_rect.center().x };
    let cx2 = if is_horizontal { draw_rect.max.x - r } else { draw_rect.center().x };
    let cy1 = if !is_horizontal { draw_rect.min.y + r } else { draw_rect.center().y };
    let cy2 = if !is_horizontal { draw_rect.max.y - r } else { draw_rect.center().y };
    let r_sq = (r - 1.0) * (r - 1.0);
    let alpha_mult = hover_t * 0.8;
    let mut y = min_y;
    while y < max_y {
        let mut x = min_x;
        while x < max_x {
            let cell_cx = x + grid_size * 0.5;
            let cell_cy = y + grid_size * 0.5;
            let mut inside = true;
            if is_horizontal {
                if cell_cx < cx1 {
                    let d = (cell_cx - cx1) * (cell_cx - cx1) + (cell_cy - cy1) * (cell_cy - cy1);
                    if d > r_sq { inside = false; }
                } else if cell_cx > cx2 {
                    let d = (cell_cx - cx2) * (cell_cx - cx2) + (cell_cy - cy1) * (cell_cy - cy1);
                    if d > r_sq { inside = false; }
                }
            } else {
                if cell_cy < cy1 {
                    let d = (cell_cx - cx1) * (cell_cx - cx1) + (cell_cy - cy1) * (cell_cy - cy1);
                    if d > r_sq { inside = false; }
                } else if cell_cy > cy2 {
                    let d = (cell_cx - cx1) * (cell_cx - cx1) + (cell_cy - cy2) * (cell_cy - cy2);
                    if d > r_sq { inside = false; }
                }
            }
            if inside {
                let ix = (x * 13.0) as u32;
                let iy = (y * 17.0) as u32;
                let mut seed = ix.wrapping_mul(0x9E3779B9).wrapping_add(iy.wrapping_mul(0xC2B2AE35)).wrapping_add(t.wrapping_mul(0x85EBCA6B));
                seed ^= seed >> 13;
                seed = seed.wrapping_mul(0xC2B2AE35);
                seed ^= seed >> 16;
                // Elegant sparse static distribution
                if seed % 10 < 5 {
                    let tc = ((x - draw_rect.min.x) / draw_rect.width().max(1.0)).clamp(0.0, 1.0);
                    let col = match (seed / 10) % 5 {
                        0..=2 => irid_color(tc),
                        3 => egui::Color32::WHITE,
                        _ => egui::Color32::from_gray(140),
                    };
                    let col_alpha = egui::Color32::from_rgba_premultiplied((col.r() as f32 * alpha_mult) as u8, (col.g() as f32 * alpha_mult) as u8, (col.b() as f32 * alpha_mult) as u8, (255.0 * alpha_mult) as u8);
                    let cell = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(grid_size.min(max_x - x), grid_size.min(max_y - y)));
                    p.rect_filled(cell, 0.0, col_alpha);
                }
            }
            x += grid_size;
        }
        y += grid_size;
    }
}

