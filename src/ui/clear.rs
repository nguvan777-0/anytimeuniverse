//! Clear theme — "Crystal" glassmorphism aesthetic.
#![allow(dead_code)]

use egui::{
    Color32, Stroke, CornerRadius, Margin, Vec2, Context, Visuals, Rect, Painter,
    Response, Ui, FontId, Sense, pos2, vec2,
};
use crate::ui::ResponseExt;

// ── Clear palette ────────────────────────────────────────────────────────────
const WHITE:         Color32 = Color32::from_rgb(255, 255, 255);
const GLASS_BG:      Color32 = Color32::from_rgb(245, 246, 250);
const PANEL:         Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 200);
const INSET_FILL:    Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 5);
const INSET_BORDER:  Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 30);

const ACCENT_BLUE:   Color32 = Color32::from_rgb(100, 180, 255);
const ACCENT_GLOW:   Color32 = Color32::from_rgb(200, 230, 255);

// ── Apply theme ───────────────────────────────────────────────────────────────

pub fn apply_theme(ctx: &Context) {
    let mut style = (*ctx.global_style()).clone();

    let mut visuals = Visuals::light();
    visuals.window_fill         = PANEL;
    visuals.panel_fill          = PANEL;
    visuals.extreme_bg_color    = INSET_FILL;
    visuals.text_cursor.stroke.color = Color32::BLACK;

    let r = CornerRadius::same(6);
    let border = Stroke::new(1.0, INSET_BORDER);

    visuals.widgets.noninteractive.bg_fill      = Color32::TRANSPARENT;
    visuals.widgets.noninteractive.bg_stroke     = border;
    visuals.widgets.noninteractive.corner_radius      = r;

    visuals.widgets.inactive.bg_fill            = Color32::WHITE;
    visuals.widgets.inactive.bg_stroke          = border;
    visuals.widgets.inactive.corner_radius           = r;

    visuals.widgets.hovered.bg_fill             = Color32::from_rgb(250, 252, 255);
    visuals.widgets.hovered.bg_stroke           = Stroke::new(1.0, ACCENT_BLUE);
    visuals.widgets.hovered.corner_radius            = r;

    visuals.widgets.active.bg_fill              = Color32::from_rgb(240, 245, 255);
    visuals.widgets.active.bg_stroke            = Stroke::new(1.5, ACCENT_BLUE);
    visuals.widgets.active.corner_radius             = r;

    visuals.selection.bg_fill = Color32::from_rgba_premultiplied(100, 180, 255, 120);
    visuals.selection.stroke  = Stroke::new(1.0, Color32::BLACK);

    style.visuals = visuals;
    style.spacing.item_spacing   = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(10.0, 5.0);
    style.spacing.window_margin  = Margin::same(10);

    ctx.set_global_style(style);
}

// ── Clear Button Primitive ────────────────────────────────────────────────────

pub fn draw_clear_pill(ui: &mut Ui, response: &Response, rect: Rect) -> f32 {
    let p = ui.painter();
    let pressed = response.is_pointer_button_down_on();
    let press_t = ui.ctx().animate_value_with_time(
        response.id.with("clr_press"),
        if pressed { 1.0 } else { 0.0 },
        0.05,
    );
    let hover_t = ui.ctx().animate_value_with_time(
        response.id.with("clr_hover"),
        if response.hovered() { 1.0 } else { 0.0 },
        0.15,
    );

    let push_y = press_t * 1.5;
    let draw_rect = rect.translate(vec2(0.0, push_y));
    let r = rect.height() / 2.0;

    // Outer "bloom" glow when hovered
    if hover_t > 0.0 {
        let bloom_op = (30.0 * hover_t) as u8;
        p.rect_filled(draw_rect.expand(2.0 + hover_t * 2.0), r + 2.0, Color32::from_rgba_premultiplied(100, 180, 255, bloom_op));
    }

    // Shadow
    let shadow_op = (40.0 * (1.0 - press_t)) as u8;
    if shadow_op > 0 {
        p.rect_filled(rect.translate(vec2(0.0, 2.0)), r + 1.0, Color32::from_rgba_premultiplied(0, 0, 0, shadow_op));
    }

    // Body — Pure white
    p.rect_filled(draw_rect, r, WHITE);

    // Subtle inner bevel
    p.rect_stroke(draw_rect, r, Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 0, 0, 15)), egui::StrokeKind::Inside);
    
    // Glossy top shine
    let hl_op = (180.0 * (1.0 - press_t * 0.4)) as u8;
    let hl_rect = Rect::from_min_size(
        draw_rect.min + vec2(1.5, 1.0),
        vec2(draw_rect.width() - 3.0, r * 0.8),
    );
    p.rect_filled(hl_rect, CornerRadius::from(r), Color32::from_rgba_premultiplied(255, 255, 255, hl_op));

    push_y
}

// ── ThemeProvider Implementation ──────────────────────────────────────────────

pub struct Clear;

impl crate::ui::theme::ThemeProvider for Clear {
    fn apply_theme(&self, ctx: &Context) { apply_theme(ctx); }
    fn draw_sunken(&self, painter: &Painter, rect: Rect) {
        painter.rect_filled(rect, 4.0, INSET_FILL);
        painter.rect_stroke(rect, 4.0, Stroke::new(1.0, INSET_BORDER), egui::StrokeKind::Outside);
    }
    fn section_toggle_btn(&self, ui: &mut Ui) -> Response {
        let r = 7.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(r * 2.0 + 4.0, r * 2.0 + 4.0), Sense::click());
        if ui.is_rect_visible(rect) {
            let shift_y = draw_clear_pill(ui, &resp, rect);
            let dc = rect.center() + vec2(0.0, shift_y);
            ui.painter().circle_filled(dc, 1.0, Color32::from_gray(100));
        }
        resp.hand()
    }

    fn paint_button(&self, ui: &mut Ui, rect: Rect, _is_down: bool, _is_hovered: bool) -> f32 {
        let id_hash = (rect.min.x as i32, rect.min.y as i32);
        let resp = ui.interact(rect, ui.id().with(id_hash), Sense::click());
        draw_clear_pill(ui, &resp, rect)
    }

    fn button_text_color(&self) -> Color32 { Color32::from_rgb(60, 70, 90) }
    fn key_cap_text_color(&self) -> Color32 { Color32::from_rgb(60, 70, 90) }

    fn paint_key_cap(&self, p: &Painter, rect: Rect, is_down: bool, is_hovered: bool) {
        let r = rect.height() / 2.0;
        p.rect_filled(rect, r, WHITE);
        let stroke_col = if is_hovered { ACCENT_BLUE } else { INSET_BORDER.into() };
        p.rect_stroke(rect, r, Stroke::new(1.0, stroke_col), egui::StrokeKind::Outside);
        if is_down {
            p.rect_filled(rect, r, Color32::from_rgba_premultiplied(0, 0, 0, 10));
        }
    }

    fn key_cap_small(&self, ui: &mut Ui, text: &str, side: f32, font_size: f32) -> Response {
        let (rect, resp) = ui.allocate_exact_size(vec2(side, side), Sense::click());
        if ui.is_rect_visible(rect) {
            let shift_y = draw_clear_pill(ui, &resp, rect);
            let galley = ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(font_size), self.key_cap_text_color());
            ui.painter().galley(rect.center() - galley.size() / 2.0 + vec2(0.0, shift_y), galley, self.key_cap_text_color());
        }
        resp.hand()
    }

    fn key_cap_small_rotated(&self, ui: &mut Ui, text: &str, angle: f32, side: f32, font_size: f32) -> Response {
        let (rect, resp) = ui.allocate_exact_size(vec2(side, side), Sense::click());
        if ui.is_rect_visible(rect) {
            let shift_y = draw_clear_pill(ui, &resp, rect);
            let galley = ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(font_size), self.key_cap_text_color());
            ui.painter().add(egui::Shape::Text(egui::epaint::TextShape {
                pos: rect.center() + vec2(0.0, shift_y),
                galley,
                underline: Stroke::NONE,
                fallback_color: self.key_cap_text_color(),
                override_text_color: Some(self.key_cap_text_color()),
                opacity_factor: 1.0,
                angle,
            }));
        }
        resp.hand()
    }

    fn collapsible_header(&self, ui: &mut Ui, text: &str, is_open: bool) -> bool {
        crate::ui::widgets::collapsible_header(self, ui, text, is_open)
    }

    fn paint_slider_track(&self, ui: &mut Ui, track_rect: Rect, _center_x: f32) {
        ui.painter().rect_filled(track_rect, 2.0, INSET_FILL);
        ui.painter().rect_stroke(track_rect, 2.0, Stroke::new(1.0, INSET_BORDER), egui::StrokeKind::Outside);
    }

    fn paint_slider_thumb(&self, ui: &mut Ui, handle_rect: Rect, _is_down: bool, _is_hov: bool) {
        let resp = ui.interact(handle_rect, ui.id().with(handle_rect.center().x as i32), Sense::click());
        draw_clear_pill(ui, &resp, handle_rect);
    }

    fn paint_slider_text(&self, ui: &mut Ui, text: &str) {
        ui.label(egui::RichText::new(text).color(self.button_text_color()).monospace());
    }

    fn paint_slider_gauge(&self, ui: &mut Ui, bg_rect: Rect, fill_rect: Rect, is_down: bool, is_hovered: bool) {
        let p = ui.painter();
        p.rect_filled(bg_rect, 3.0, INSET_FILL);
        p.rect_stroke(bg_rect, 3.0, Stroke::new(1.0, INSET_BORDER), egui::StrokeKind::Outside);
        
        if fill_rect.width() > 0.0 {
            let color = if is_down { ACCENT_BLUE.linear_multiply(0.8) } else if is_hovered { ACCENT_GLOW } else { ACCENT_BLUE };
            p.rect_filled(fill_rect, 3.0, color);
            // Shine on the gauge
            let shine = Rect::from_min_size(fill_rect.min + vec2(0.0, 1.0), vec2(fill_rect.width(), 2.0));
            p.rect_filled(shine, 1.0, Color32::from_rgba_premultiplied(255, 255, 255, 100));
        }
    }

    fn section_label(&self, ui: &mut Ui, text: &str) -> Response {
        let galley = ui.painter().layout_no_wrap(text.to_owned(), FontId::proportional(13.0), self.button_text_color());
        let (rect, resp) = ui.allocate_exact_size(galley.size(), Sense::click());
        ui.painter().galley(rect.min, galley, self.button_text_color());
        resp
    }

    fn text_field_edit(&self, ui: &mut Ui, text: &mut String, font_size: f32, height: f32) -> Response {
        let field_w = ui.available_width();
        let padding = vec2(6.0, 3.0);
        let h = if height > 0.0 { height } else { font_size + padding.y * 2.0 + 2.0 };
        let (rect, _) = ui.allocate_exact_size(vec2(field_w, h), Sense::hover());
        self.draw_sunken(ui.painter(), rect);
        
        let inner_rect = rect.shrink(4.0);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner_rect).layout(*ui.layout()));
        let text_edit = egui::TextEdit::singleline(text)
            .font(FontId::monospace(font_size))
            .text_color(self.button_text_color())
            .frame(egui::Frame::NONE);
        child.add(text_edit)
    }

    fn chart_bg(&self) -> Color32 { Color32::from_rgb(255, 255, 255) }
    fn chart_axis_color(&self) -> Color32 { Color32::from_gray(220) }
}
