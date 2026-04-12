use egui::{Response, Ui};
use crate::ui::theme::ThemeProvider;

/// Maintains the selection cache for a text field's right-click context menu.
///
/// Call this AFTER `child.add(text_edit)` using the real `te_resp.id`.
///
/// Logic:
/// - Left-click on rect: clear cache (user repositioned cursor, no selection).
/// - Right-click on rect: freeze cache (preserve whatever selection was cached).
/// - Otherwise, if a non-empty selection exists this frame: update cache.
///
/// This avoids any ID-matching gymnastics and runs entirely on `te_resp.id`.
pub fn maintain_text_selection_cache(
    ctx: &egui::Context,
    te_resp: &Response,
    text: &str,
    rect: egui::Rect,
) {
    let cached_id = te_resp.id.with("_ctx_sel");
    
    let primary_interacted = te_resp.drag_started() || te_resp.clicked();
    let secondary_pressed = ctx.input(|i| {
        i.pointer.button_pressed(egui::PointerButton::Secondary)
            && i.pointer.interact_pos().is_some_and(|p| rect.contains(p))
    });

    if secondary_pressed {
        // Egui's TextEdit natively drops the selection mask on right-click. 
        // We forcefully restore it immediately so the highlight doesn't visually disappear!
        let (_, lo, hi) = ctx.data(|d| d.get_temp::<(String, usize, usize)>(cached_id).unwrap_or_default());
        if lo != hi
             && let Some(mut state) = egui::TextEdit::load_state(ctx, te_resp.id) {
                 state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                     egui::text::CCursor::new(lo),
                     egui::text::CCursor::new(hi),
                 )));
                 egui::TextEdit::store_state(ctx, te_resp.id, state);
             }
    } else if primary_interacted {
        ctx.data_mut(|d| d.insert_temp(cached_id, (String::new(), 0usize, 0usize)));
    } else {
        if let Some(range) = egui::TextEdit::load_state(ctx, te_resp.id)
            .and_then(|s| s.cursor.char_range())
        {
            let lo = range.primary.index.min(range.secondary.index);
            let hi = range.primary.index.max(range.secondary.index);
            if lo != hi {
                let sel: String = text.chars().skip(lo).take(hi - lo).collect();
                ctx.data_mut(|d| d.insert_temp(cached_id, (sel, lo, hi)));
            } else {
                ctx.data_mut(|d| d.insert_temp(cached_id, (String::new(), 0usize, 0usize)));
            }
        }
    }
}

/// Attaches a right-click context menu (Copy / Cut) to a text field.
///
/// Call `maintain_text_selection_cache` on the same `te_resp` every frame before
/// calling this, so the menu always has a fresh snapshot to read.
pub fn text_field_context_menu(theme: &dyn crate::ui::theme::ThemeProvider, te_resp: &Response, te_id: egui::Id, text: &mut String) -> bool {
    let cached_id = te_id.with("_ctx_sel");
    let mut text_changed = false;
    te_resp.context_menu(|ui| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        theme.edit_popup_visuals(ui.visuals_mut());
        theme.edit_popup_spacing(ui.spacing_mut());
        
        let (selected, lo, hi) = ui.ctx().data(|d| {
            d.get_temp::<(String, usize, usize)>(cached_id).unwrap_or_default()
        });
        let has_sel = !selected.is_empty();
        let sp = if theme.palette().is_terminal_style { " " } else { "  " };

        if ui.add_enabled(has_sel, egui::Button::new(format!("📋{sp}Copy"))).clicked() {
            ui.ctx().copy_text(selected.clone());
            ui.close();
        }
        if ui.add_enabled(has_sel, egui::Button::new(format!("✂{sp}Cut"))).clicked() {
            ui.ctx().copy_text(selected);
            if lo != hi {
                let mut chars: Vec<char> = text.chars().collect();
                chars.drain(lo..hi);
                *text = chars.into_iter().collect();
                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), te_id) {
                    let c = egui::text::CCursor::new(lo);
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(c, c)));
                    state.store(ui.ctx(), te_id);
                }
                text_changed = true;
            }
            ui.close();
        }
        if ui.button(format!("📥{sp}Paste")).clicked() {
            ui.memory_mut(|mem| mem.request_focus(te_id));
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestPaste);
            ui.close();
        }
    });
    text_changed
}
const SLIDER_MARGIN: f32 = 6.0;

/// Shared widget layout for collapsible section headers across all themes.
pub fn collapsible_header(theme: &dyn ThemeProvider, ui: &mut Ui, title: &str, _is_expanded: bool) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        let btn_resp = theme.section_toggle_btn(ui);
        // By chaining .interact(Sense::click()), we upgrade any inert label to be clickable
        let lbl_resp = theme.section_label(ui, title).interact(egui::Sense::click());
        
        if btn_resp.clicked() || lbl_resp.clicked() { 
            clicked = true; 
        }
    });
    clicked
}

pub struct SymlogSliderState {
    pub t: f32,
    pub anim_scale: f32,
    pub is_down: bool,
    pub is_hovered: bool,
}

pub fn interact_symlog_slider(
    ui: &mut egui::Ui,
    value: &mut f64,
    max_abs: f64,
    rect: egui::Rect,
    response: &mut egui::Response,
) -> SymlogSliderState {
    let scale = (max_abs + 1.0).ln();
    let val_to_t = |v: f64| -> f32 {
        let s = v.signum() * (v.abs() + 1.0).ln() / scale;
        ((s + 1.0) / 2.0).clamp(0.0, 1.0) as f32
    };
    let t_to_val = |t: f64| -> f64 {
        let s = 2.0 * t - 1.0;
        s.signum() * ((s.abs() * scale).exp() - 1.0)
    };

    let slider_width = rect.width() - 2.0 * SLIDER_MARGIN;
    let track_min_x = rect.min.x + SLIDER_MARGIN;

    if (response.dragged() || response.clicked())
        && let Some(pos) = response.interact_pointer_pos() {
            let x = pos.x - track_min_x;
            let t = (x / slider_width).clamp(0.0, 1.0) as f64;
            let new_val = t_to_val(t).clamp(-max_abs, max_abs);
            
            if (*value < 0.0 && new_val >= 0.0) || (*value > 0.0 && new_val <= 0.0) {
                let current_time = ui.input(|i| i.time);
                ui.ctx().data_mut(|d| d.insert_temp(response.id.with("z"), current_time));
            }
            
            *value = new_val;
            response.mark_changed();
        }

    let t = val_to_t(*value);
    let anim_start_time = ui.ctx().data(|d| d.get_temp::<f64>(response.id.with("z"))).unwrap_or(-10.0);
    let time_since = ui.input(|i| i.time) - anim_start_time;
    let target_scale = if time_since < 0.15 { 0.5 } else { 1.0 };
    let anim_scale = ui.ctx().animate_value_with_time(response.id.with("s"), target_scale, 0.1);
    
    if time_since < 0.3 || (anim_scale - target_scale).abs() > 0.01 {
        ui.ctx().request_repaint();
    }

    SymlogSliderState {
        t,
        anim_scale,
        is_down: response.dragged() || response.is_pointer_button_down_on(),
        is_hovered: response.hovered(),
    }
}

/// The generalized layout for symlog sliders. Themes provide the painting.
pub fn slider_symlog_f64(
    theme: &dyn ThemeProvider,
    ui: &mut egui::Ui,
    value: &mut f64,
    max_abs: f64,
    text: &str,
) -> egui::Response {
    let mut root_response = ui.allocate_response(egui::vec2(0.0, 0.0), egui::Sense::hover());
    ui.horizontal(|ui| {
        theme.paint_slider_text(ui, text);
        let slider_width = (ui.available_width() - 2.0 * SLIDER_MARGIN).max(60.0);
        let height = ui.spacing().interact_size.y;
        let (rect, mut s_resp) = ui.allocate_exact_size(egui::vec2(slider_width + 2.0 * SLIDER_MARGIN, height), egui::Sense::click_and_drag());
        
        let state = interact_symlog_slider(ui, value, max_abs, rect, &mut s_resp);
        
        if ui.is_rect_visible(rect) {
            let track_h = 4.0;
            let track_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(slider_width, track_h));
            let center_x = rect.min.x + rect.width() * 0.5;
            theme.paint_slider_track(ui, track_rect, center_x);
            
            let handle_x = (rect.min.x + SLIDER_MARGIN) + state.t * slider_width;
            let handle_w = 11.0 * state.anim_scale;
            let handle_h = height * 1.2 * state.anim_scale;
            let handle_rect = egui::Rect::from_center_size(egui::pos2(handle_x, rect.center().y), egui::vec2(handle_w, handle_h));
            theme.paint_slider_thumb(ui, handle_rect, state.is_down, state.is_hovered);
        }
        root_response = s_resp;
    });
    root_response
}

/// A standard linear slider mapped to the application's theme painter.
#[allow(dead_code)]
pub fn slider_f32(
    theme: &dyn ThemeProvider,
    ui: &mut egui::Ui,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    text: &str,
) -> egui::Response {
    let mut root_response = ui.allocate_response(egui::vec2(0.0, 0.0), egui::Sense::hover());
    ui.horizontal(|ui| {
        theme.paint_slider_text(ui, text);
        let slider_width = (ui.available_width() - 2.0 * SLIDER_MARGIN).max(60.0);
        let height = ui.spacing().interact_size.y;
        let (rect, mut s_resp) = ui.allocate_exact_size(egui::vec2(slider_width + 2.0 * SLIDER_MARGIN, height), egui::Sense::click_and_drag());
        
        // Linear mapping
        let min = *range.start();
        let max = *range.end();
        let span = max - min;
        
        let mut t = (*value - min) / span;

        if (s_resp.dragged() || s_resp.clicked()) && let Some(pos) = s_resp.interact_pointer_pos() {
            let x = pos.x - (rect.min.x + SLIDER_MARGIN);
            t = (x / slider_width).clamp(0.0, 1.0);
            *value = min + (t * span);
            s_resp.mark_changed();
        }

        let is_down = s_resp.dragged() || s_resp.is_pointer_button_down_on();
        let is_hovered = s_resp.hovered();

        let anim_target = if is_down { 0.8 } else if is_hovered { 1.2 } else { 1.0 };
        let anim_scale = ui.ctx().animate_value_with_time(s_resp.id.with("scale"), anim_target, 0.1);
        
        if ui.is_rect_visible(rect) {
            let track_h = 4.0;
            let track_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(slider_width, track_h));
            let center_x = rect.min.x + rect.width() * 0.5;
            theme.paint_slider_track(ui, track_rect, center_x);
            
            let handle_x = (rect.min.x + SLIDER_MARGIN) + t * slider_width;
            let handle_w = 11.0 * anim_scale;
            let handle_h = height * 1.2 * anim_scale;
            let handle_rect = egui::Rect::from_center_size(egui::pos2(handle_x, rect.center().y), egui::vec2(handle_w, handle_h));
            theme.paint_slider_thumb(ui, handle_rect, is_down, is_hovered);
        }
        root_response = s_resp;
    });
    root_response
}

/// A linear fill gauge slider (no discrete thumb, acts as a volume progress bar).
pub fn slider_fill_f32(
    theme: &dyn ThemeProvider,
    ui: &mut egui::Ui,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) -> egui::Response {
    let mut root_response = ui.allocate_response(egui::vec2(0.0, 0.0), egui::Sense::hover());
    ui.horizontal(|ui| {
        // Render exact volume metrics
        let galley = ui.painter().layout_no_wrap(
            format!("{:.0}", value),
            egui::FontId::monospace(13.0),
            theme.button_text_color(),
        );
        let text_w = galley.size().x;
        
        let slider_width = (ui.available_width() - text_w - 4.0).max(60.0);
        let height = ui.spacing().interact_size.y;
        let (rect, mut s_resp) = ui.allocate_exact_size(egui::vec2(slider_width, height), egui::Sense::click_and_drag());
        
        let min = *range.start();
        let max = *range.end();
        let span = max - min;
        
        let mut t = (*value - min) / span;

        if (s_resp.dragged() || s_resp.clicked()) && let Some(pos) = s_resp.interact_pointer_pos() {
            let x = pos.x - rect.min.x;
            t = (x / slider_width).clamp(0.0, 1.0);
            *value = min + (t * span);
            s_resp.mark_changed();
        }

        if ui.is_rect_visible(rect) {
            let track_h = 10.0;
            let bg_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(slider_width, track_h));
            
            let fill_w = t * slider_width;
            let fill_rect = egui::Rect::from_min_size(
                egui::pos2(rect.min.x, rect.center().y - track_h * 0.5),
                egui::vec2(fill_w, track_h)
            );
            
            let is_down = s_resp.dragged() || s_resp.is_pointer_button_down_on();
            theme.paint_slider_gauge(ui, bg_rect, fill_rect, is_down, s_resp.hovered());
        }
        
        let (text_rect, _) = ui.allocate_exact_size(egui::vec2(text_w, 14.0), egui::Sense::hover());
        let text_pos = egui::pos2(text_rect.min.x, text_rect.center().y - galley.size().y * 0.5);
        if let Some(shadow_color) = theme.gauge_label_shadow() {
            ui.painter().add(egui::Shape::Text(egui::epaint::TextShape {
                pos: text_pos + egui::vec2(1.0, 1.0),
                galley: galley.clone(),
                underline: egui::Stroke::NONE,
                fallback_color: shadow_color,
                override_text_color: Some(shadow_color),
                opacity_factor: 1.0,
                angle: 0.0,
            }));
        }
        let text_color = theme.gauge_label_text_color().unwrap_or(theme.button_text_color());
        ui.painter().add(egui::Shape::Text(egui::epaint::TextShape {
            pos: text_pos,
            galley,
            underline: egui::Stroke::NONE,
            fallback_color: text_color,
            override_text_color: Some(text_color),
            opacity_factor: 1.0,
            angle: 0.0,
        }));

        root_response = s_resp;
    });
    root_response
}
// ── Shared Button & KeyCap Engine ─────────────────────────────────────────────

#[allow(dead_code)]
pub fn button(theme: &dyn ThemeProvider, ui: &mut Ui, text: &str) -> Response {
    button_w(theme, ui, text, 0.0, false).0
}

pub fn button_w(theme: &dyn ThemeProvider, ui: &mut Ui, text: &str, min_w: f32, force_pressed: bool) -> (Response, f32) {
    let job = egui::text::LayoutJob::simple(
        text.to_string(),
        egui::FontId::monospace(13.0),
        theme.button_text_color(),
        10000.0,
    );
    button_job(theme, ui, job, min_w, force_pressed)
}

pub fn button_job(theme: &dyn ThemeProvider, ui: &mut Ui, job: egui::text::LayoutJob, min_w: f32, force_pressed: bool) -> (Response, f32) {
    let padding = ui.spacing().button_padding;
    let galley = ui.painter().layout_job(job);
    let w = (galley.size().x + padding.x * 2.0).max(min_w);
    
    // Lock the button height to the standard 13.0 monospace height so large text formatting doesn't warp the button grid
    let std_galley = ui.painter().layout_no_wrap("A".to_string(), egui::FontId::monospace(13.0), theme.button_text_color());
    let h = std_galley.size().y + padding.y * 2.0;

    let (rect, mut response) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());

    if response.clicked() {
        response.mark_changed();
    }

    let mut ret_shift = 0.0;
    if ui.is_rect_visible(rect) {
        let is_down = response.is_pointer_button_down_on() || force_pressed;
        let is_hovered = response.hovered();
        let shift_y = theme.paint_button(ui, rect, is_down, is_hovered);
        ret_shift = shift_y;
        
        let galley_pos = egui::pos2(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5 + shift_y,
        );
        ui.painter().galley(galley_pos, galley, theme.button_text_color());
    }
    
    (response, ret_shift)

}

