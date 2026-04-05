//! Future theme

use egui::{
    Color32, Stroke, Rounding, Margin, Vec2, Context, Visuals, Rect, Painter,
    Response, Ui, FontId, Sense, pos2, vec2,
};
use crate::ui::ResponseExt;
use crate::ui::dew;
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

    let r = Rounding::same(5);
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

    visuals.window_corner_radius = Rounding::same(6);
    visuals.menu_corner_radius   = Rounding::same(6);

    style.visuals = visuals;
    style.interaction.selectable_labels = false;
    style.spacing.item_spacing      = Vec2::new(6.0, 5.0);
    style.spacing.button_padding    = Vec2::new(10.0, 5.0);
    style.spacing.window_margin     = Margin::same(8);
    style.spacing.slider_width      = 150.0;
    style.spacing.interact_size.y   = 20.0;

    ctx.set_style(style);
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

// ── Future button primitive ───────────────────────────────────────────────────

/// Polished future pill with squish physics.  Returns the Y push-down (0–1.5).
pub fn draw_future_pill(ui: &mut Ui, response: &Response, rect: Rect) -> f32 {
    let pressed = response.is_pointer_button_down_on();
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
    let r = rect.height() / 2.0;
    let p = ui.painter();

    // Drop shadow — stays at rest, fades as face descends into it
    let sh_op = (80.0 * (1.0 - press_t)) as u8;
    if sh_op > 0 {
        p.rect_filled(
            rect.translate(vec2(0.0, 1.5)),
            r + 0.5,
            Color32::from_rgba_premultiplied(0, 0, 0, sh_op),
        );
    }

    // Body — gunmetal, darkens on press, brightens on hover
    let darken = 1.0 - press_t * 0.35 + hover_t * 0.12;
    let c = FUTURE_BODY;
    let body = Color32::from_rgb(
        (c.r() as f32 * darken) as u8,
        (c.g() as f32 * darken) as u8,
        (c.b() as f32 * darken) as u8,
    );
    p.rect_filled(draw_rect, r, body);

    // Bottom future reflection (blue-shifted, lower half)
    let glow_op = (155.0 * (1.0 - press_t * 0.65)) as u8;
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

    // Top specular cap — very bright, hallmark of polished future
    let hl_op = (215.0 * (1.0 - press_t * 0.55)) as u8;
    if hl_op > 0 {
        let hl = Rect::from_min_size(
            draw_rect.min + vec2(1.5, 1.0),
            vec2(draw_rect.width() - 3.0, r * 0.72),
        );
        p.rect_filled(hl,
            egui::epaint::CornerRadiusF32 { nw: r - 1.5, ne: r - 1.5, sw: r * 0.25, se: r * 0.25 },
            Color32::from_rgba_premultiplied(215, 228, 255, hl_op));
    }

    // Iridescent rainbow stripe — the signature future sheen
    draw_irid_stripe(p, draw_rect, r, 1.0 - press_t * 0.65);

    // Etched dark border
    p.rect_stroke(draw_rect, r, Stroke::new(1.0,
        Color32::from_rgba_premultiplied(FUTURE_BORDER.r(), FUTURE_BORDER.g(), FUTURE_BORDER.b(), 210)), egui::StrokeKind::Outside);

    push_y
}

// ── Section label ─────────────────────────────────────────────────────────────

/// Etched future section label — silver text with a subtle 1px shadow below.
pub fn collapsible_header(ui: &mut Ui, title: &str, is_expanded: bool) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        let btn_resp = section_toggle_btn(ui);
        let lbl_text = title;
        let lbl_resp = section_label(ui, &lbl_text);

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

// ── Mac-style traffic-light orb (future variant) ────────────────────────────

/// Same signature as `dew::draw_mac_traffic_light`, rendered in Future's steel orb style.
pub fn draw_mac_traffic_light(
    ui: &mut Ui,
    resp: &Response,
    r: f32,
    _base_color: Color32, // ignored — Future always uses FUTURE_BODY
    symbol: &str,
    group_hover_t: Option<f32>,
) {
    if !ui.is_rect_visible(resp.rect) { return; }

    let pressed = resp.is_pointer_button_down_on();
    let press_t = ui.ctx().animate_value_with_time(
        resp.id.with("press"), if pressed { 1.0 } else { 0.0 }, 0.05,
    );
    let hover_t = group_hover_t.unwrap_or_else(|| {
        ui.ctx().animate_value_with_time(
            resp.id.with("hover"), if resp.hovered() { 1.0 } else { 0.0 }, 0.1,
        )
    });

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
    let darken = 1.0 - press_t * 0.35;
    let bc = FUTURE_BODY;
    let body = Color32::from_rgb(
        (bc.r() as f32 * darken) as u8,
        (bc.g() as f32 * darken) as u8,
        (bc.b() as f32 * darken) as u8,
    );
    p.circle_filled(dc, r, body);

    // Bottom glow — blue-shifted, matches future pill
    let lr = (bc.r() as f32 * darken * 1.55).min(255.0) as u8;
    let lg = (bc.g() as f32 * darken * 1.65).min(255.0) as u8;
    let lb = (bc.b() as f32 * darken * 1.85).min(255.0) as u8;
    p.circle_filled(dc + vec2(0.0, 1.5), r - 1.5, Color32::from_rgb(lr, lg, lb));

    // Top specular — blue-tinted, matches future pill
    let hl = (210.0 * (1.0 - press_t * 0.5)) as u8;
    if hl > 0 {
        let hl_rect = Rect::from_min_size(dc - vec2(r - 1.5, r - 1.0),
            vec2((r - 1.5) * 2.0, r * 0.78));
        p.rect_filled(hl_rect, r,
            Color32::from_rgba_premultiplied(215, 228, 255, hl));
    }

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
            p.text(dc + pos_offset + egui::vec2(0.0, 1.0) + d, egui::Align2::CENTER_CENTER, symbol, font.clone(), hl_col);
            p.text(dc + pos_offset + d, egui::Align2::CENTER_CENTER, symbol, font.clone(), ink_col);
        }
    }

    // Outline
    p.circle_stroke(dc, r, egui::Stroke::new(1.0,
        Color32::from_rgba_premultiplied(0, 0, 0, 140)));
}

// ── Section toggle button ─────────────────────────────────────────────────────

/// Small future orb used as a section collapse / expand toggle.
pub fn section_toggle_btn(ui: &mut Ui) -> Response {
    let r = 6.0f32;
    let (rect, mut resp) = ui.allocate_exact_size(vec2(r * 2.0 + 2.0, r * 2.0 + 2.0), Sense::click());
    if !ui.is_rect_visible(rect) { return resp; }

    draw_mac_traffic_light(ui, &resp, r, FUTURE_BODY, ".", None);
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
        let shift_y = draw_future_pill(ui, &response, rect);

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
        text.to_string(), FontId::monospace(16.0), Color32::BLACK,
    );
    let padding = vec2(5.0, 2.0);
    let size = galley.size() + padding * 2.0;
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let shift_y = draw_future_pill(ui, &response, rect);
        let text_pos = ui.layout().align_size_within_rect(galley.size(), rect.shrink(2.0)).min
            + vec2(0.0, shift_y);
        let shadow_a = (110.0 * (1.0 - shift_y / 1.5)) as u8;
        if shadow_a > 0 {
            ui.painter().galley(text_pos + vec2(0.0, 1.0),
                ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(16.0),
                    Color32::WHITE),
                Color32::BLACK);
        }
        ui.painter().galley(text_pos, galley, Color32::BLACK);
    }
    response.hand()
}

pub fn key_cap_small(ui: &mut Ui, text: &str, min_side: f32) -> Response {
    let measure = ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(16.0), Color32::BLACK);
    let gw = measure.size().x;
    let gh = measure.size().y;
    let side = min_side.max(gw + 6.0);
    let (rect, response) = ui.allocate_exact_size(vec2(side, side), Sense::click());
    if ui.is_rect_visible(rect) {
        let shift_y = draw_future_pill(ui, &response, rect);
        let c = rect.center();
        let pos = pos2(c.x - gw / 2.0, c.y - gh / 2.0 - 1.5 + shift_y);
        let shadow_a = (110.0 * (1.0 - shift_y / 1.5)) as u8;
        if shadow_a > 0 {
            ui.painter().add(egui::Shape::Text(egui::epaint::TextShape {
                pos: pos + vec2(0.0, 1.0),
                galley: ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(16.0), Color32::WHITE),
                underline: egui::Stroke::NONE,
                fallback_color: Color32::WHITE,
                override_text_color: Some(Color32::WHITE),
                opacity_factor: 1.0,
                angle: 0.0,
            }));
        }
        ui.painter().add(egui::Shape::Text(egui::epaint::TextShape {
            pos: pos,
            galley: ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(16.0), Color32::BLACK),
            underline: egui::Stroke::NONE,
            fallback_color: Color32::BLACK,
            override_text_color: Some(Color32::BLACK),
            opacity_factor: 1.0,
            angle: 0.0,
        }));
    }
    response.hand()
}

pub fn key_cap_small_rotated(ui: &mut Ui, text: &str, angle: f32, min_side: f32) -> Response {
    let measure = ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(16.0), Color32::BLACK);
    let gw = measure.size().x;
    let gh = measure.size().y;
    let side = min_side.max(gw + 6.0);
    let (rect, response) = ui.allocate_exact_size(vec2(side, side), Sense::click());
    if ui.is_rect_visible(rect) {
        let shift_y = draw_future_pill(ui, &response, rect);
        let s = angle.signum();
        let c = rect.center();
        let pos = pos2(c.x + s * (gh * 0.42 + 3.0), c.y - s * gw / 2.0 + shift_y);
        let shadow_a = (110.0 * (1.0 - shift_y / 1.5)) as u8;
        if shadow_a > 0 {
            ui.painter().add(egui::Shape::Text(egui::epaint::TextShape {
                pos: pos + vec2(0.0, 1.0),
                galley: ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(16.0), Color32::WHITE),
                underline: egui::Stroke::NONE,
                fallback_color: Color32::WHITE,
                override_text_color: Some(Color32::WHITE),
                opacity_factor: 1.0,
                angle,
            }));
        }
        ui.painter().add(egui::Shape::Text(egui::epaint::TextShape {
            pos: pos,
            galley: ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(16.0), Color32::BLACK),
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
    fn apply_theme(&self, ctx: &Context) { apply_theme(ctx); }
    fn draw_sunken(&self, painter: &Painter, rect: Rect) { draw_inset(painter, rect); }
    fn section_toggle_btn(&self, ui: &mut Ui) -> Response { section_toggle_btn(ui) }
    fn key_cap_small(&self, ui: &mut Ui, text: &str, side: f32) -> Response { key_cap_small(ui, text, side) }
    fn key_cap_small_rotated(&self, ui: &mut Ui, text: &str, angle: f32, side: f32) -> Response { key_cap_small_rotated(ui, text, angle, side) }
    fn collapsible_header(&self, ui: &mut Ui, text: &str, is_open: bool) -> bool { crate::ui::widgets::collapsible_header(self, ui, text, is_open) }
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

        let lr = (c_r as f32 * 1.5).min(255.0) as u8;
        let lg = (c_g as f32 * 1.5).min(255.0) as u8;
        let lb = (c_b as f32 * 1.5).min(255.0) as u8;
        let glow_rect = Rect::from_min_max(
            egui::pos2(draw_rect.min.x + 1.5, draw_rect.center().y),
            draw_rect.max - egui::vec2(1.5, 1.5),
        );
        p.rect_filled(glow_rect, egui::epaint::CornerRadiusF32 { nw: 0.0, ne: 0.0, sw: r - 1.5, se: r - 1.5 },
            Color32::from_rgba_premultiplied(lr, lg, lb, 180));

        let hl_op = if is_down { 110 } else { 220 };
        let hl_rect = Rect::from_min_size(
            draw_rect.min + egui::vec2(1.5, 1.0),
            egui::vec2(draw_rect.width() - 3.0, r * 0.8),
        );
        p.rect_filled(hl_rect,
            egui::epaint::CornerRadiusF32 { nw: r - 1.5, ne: r - 1.5, sw: r * 0.3, se: r * 0.3 },
            Color32::from_rgba_premultiplied(255, 255, 255, hl_op));

        p.rect_stroke(draw_rect, r, Stroke::new(1.0,
            Color32::from_rgba_premultiplied(FUTURE_BORDER.r(), FUTURE_BORDER.g(), FUTURE_BORDER.b(), 210)), egui::StrokeKind::Outside);
    }

    fn paint_slider_text(&self, ui: &mut Ui, text: &str) {
        if !text.is_empty() {
            egui::Frame::none().fill(PANEL).show(ui, |ui| {
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
    fn text_field_edit(&self, ui: &mut Ui, text: &mut String, font_size: f32, height: f32) -> Response {
        let field_w = ui.available_width();
        let padding = egui::vec2(6.0, 3.0);
        let field_h = if height > 0.0 { height } else { font_size + padding.y * 2.0 + 2.0 };
        let (rect, _) = ui.allocate_exact_size(egui::vec2(field_w, field_h), Sense::hover());
        if ui.is_rect_visible(rect) {
            let p = ui.painter();
            // Solid background to hide grid for this text area
            p.rect_filled(rect, Rounding::same(3), INSET_FILL);
            draw_inset(p, rect);
        }
        let inner_rect = egui::Rect::from_center_size(
            rect.center(),
            egui::vec2(rect.width() - 8.0, font_size + padding.y * 2.0 + 2.0),
        );
        let mut child = ui.child_ui(inner_rect, *ui.layout(), None);
        child.visuals_mut().selection.bg_fill = FUTURE_GLOW;
        child.visuals_mut().selection.stroke = Stroke::new(1.0, Color32::BLACK);

        let text_edit = egui::TextEdit::singleline(text)
            .font(egui::FontId::monospace(font_size))
            .horizontal_align(egui::Align::Center)
            .frame(egui::Frame::NONE)
            .text_color(TEXT);

        child.add(text_edit)
    }

    fn button_text_color(&self) -> egui::Color32 {
        egui::Color32::BLACK
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
        let r = rect.height() / 2.0;
        let p = ui.painter();

        let sh_op = (80.0 * (1.0 - press_t)) as u8;
        if sh_op > 0 {
            p.rect_filled(
                rect.translate(egui::vec2(0.0, 1.5)),
                r + 0.5,
                Color32::from_rgba_premultiplied(0, 0, 0, sh_op),
            );
        }

        let darken = 1.0 - press_t * 0.35 + hover_t * 0.12;
        let c = FUTURE_BODY;
        let body = Color32::from_rgb(
            (c.r() as f32 * darken) as u8,
            (c.g() as f32 * darken) as u8,
            (c.b() as f32 * darken) as u8,
        );
        p.rect_filled(draw_rect, r, body);

        let glow_op = (155.0 * (1.0 - press_t * 0.65)) as u8;
        if glow_op > 0 {
            let glow = Rect::from_min_max(
                egui::pos2(draw_rect.min.x + 1.5, draw_rect.center().y),
                draw_rect.max - egui::vec2(1.5, 1.5),
            );
            p.rect_filled(glow,
                egui::epaint::CornerRadiusF32 { nw: 0.0, ne: 0.0, sw: r - 1.5, se: r - 1.5 },
                Color32::from_rgba_premultiplied(
                    FUTURE_GLOW.r(), FUTURE_GLOW.g(), FUTURE_GLOW.b(), glow_op));
        }

        let hl_op = (215.0 * (1.0 - press_t * 0.55)) as u8;
        if hl_op > 0 {
            let hl = Rect::from_min_size(
                draw_rect.min + egui::vec2(1.5, 1.0),
                egui::vec2(draw_rect.width() - 3.0, r * 0.72),
            );
            p.rect_filled(hl,
                egui::epaint::CornerRadiusF32 { nw: r - 1.5, ne: r - 1.5, sw: r * 0.25, se: r * 0.25 },
                Color32::from_rgba_premultiplied(215, 228, 255, hl_op));
        }

        draw_irid_stripe(p, draw_rect, r, 1.0 - press_t * 0.65);

        p.rect_stroke(draw_rect, r, Stroke::new(1.0,
            Color32::from_rgba_premultiplied(FUTURE_BORDER.r(), FUTURE_BORDER.g(), FUTURE_BORDER.b(), 210)), egui::StrokeKind::Outside);

        push_y
    }

    fn key_cap_text_color(&self) -> egui::Color32 {
        egui::Color32::BLACK
    }

    fn paint_key_cap(&self, p: &egui::Painter, rect: egui::Rect, is_down: bool, is_hovered: bool) {
        let r = rect.height() / 2.0;
        if is_down {
            p.rect_filled(rect, r, Color32::from_rgb(180, 180, 180));
        } else if !is_hovered {
            p.rect_stroke(rect, r, Stroke::new(1.0,
                Color32::from_rgba_premultiplied(FUTURE_BORDER.r(), FUTURE_BORDER.g(), FUTURE_BORDER.b(), 210)), egui::StrokeKind::Outside);
        }
    }
}
