{
if self.theme.provider().collapsible_header(ui, "SYSTEM METRICS", self.show_metrics) {
    self.show_metrics = !self.show_metrics;
}

if self.show_metrics {
    ui.add_space(GAP_XS);
    ui.style_mut().spacing.item_spacing.y = 0.0;
    
    let info = self.adapter.get_info();
    let dim = if matches!(self.theme, Theme::Rect) {
        egui::Color32::from_rgb(0, 230, 65) /* TERM_GREEN */
    } else if matches!(self.theme, Theme::Future) {
        egui::Color32::from_rgb(192, 202, 222) /* FUTURE TEXT */
    } else {
        egui::Color32::from_rgb(100, 100, 105)
    };

    let wrap_w = ui.available_width() - 40.0;
    let gpu_galley = ui.painter().layout(info.name.clone(), egui::FontId::monospace(11.0), dim, wrap_w);
    // The horizontal layouts enforce `interact_size.y` due to alignment!
    let base_h = ui.spacing().interact_size.y.max(
        ui.painter().layout("A".to_string(), egui::FontId::monospace(11.0), dim, wrap_w).size().y
    );
    let row_h = gpu_galley.size().y.max(base_h);
    let total_h = row_h + (base_h * 3.0) + (3.0 * 3.0) + 8.0; // 8.0 padding total

    let (metrics_rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), total_h),
        egui::Sense::hover()
    );
    self.theme.provider().draw_sunken(ui.painter(), metrics_rect);

    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(metrics_rect.shrink(4.0)).layout(*ui.layout()));
    child_ui.style_mut().spacing.item_spacing = egui::vec2(4.0, 3.0);

    let rows: &[(&str, egui::RichText)] = &[
        ("GPU",   egui::RichText::new(info.name.clone()).monospace().size(11.0)),
        ("API",   egui::RichText::new(format!("{:?}", info.backend)).monospace().size(11.0)),
        ("CPU",   egui::RichText::new(std::env::consts::ARCH).monospace().size(11.0)),
        ("Frame", egui::RichText::new(format!("{:.1} ms", if self.fps > 0.0 { 1000.0 / self.fps } else { 0.0 })).monospace().size(11.0)),
    ];
    for (label, val) in rows {
        child_ui.horizontal(|ui| {
            ui.label(egui::RichText::new(*label).monospace().size(11.0).color(dim));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(val.clone());
            });
        });
    }
}
}
