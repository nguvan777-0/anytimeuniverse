{
if crate::ui::widgets::collapsible_header(self.theme.provider(), ui, "SYSTEM METRICS", self.show_metrics) {
    self.show_metrics = !self.show_metrics;
}

if self.show_metrics {
    ui.add_space(GAP_XS);

    let info = self.adapter.get_info();
    let dim = self.theme.provider().palette().panel_text_color;
    let sz = 10.5_f32;
    let font = egui::FontId::monospace(sz);
    let avail_w = ui.available_width();

    let gap = 2.5_f32;
    let line_h = ui.painter()
        .layout("A".to_string(), font.clone(), dim, avail_w)
        .size().y;
    let row_h = line_h + gap;

    // Deduplicate temps by first word, drop NAND
    let mut temp_map: std::collections::BTreeMap<String, f32> = std::collections::BTreeMap::new();
    for (label, temp) in &self.sys_temps {
        let first = label.split_whitespace().next().unwrap_or(label);
        if first.eq_ignore_ascii_case("NAND") { continue; }
        let abbr = first[..first.len().min(4)].to_string();
        temp_map.entry(abbr).and_modify(|t| *t = t.max(*temp)).or_insert(*temp);
    }

    let fixed_rows = 4usize; // GPU, API, CPU, FPS
    let temp_rows = temp_map.len();
    let total_rows = fixed_rows + temp_rows;
    let _content_h = row_h * total_rows as f32 + 8.0;
    let theme_picker_h = ui.spacing().interact_size.y + GAP_SM * 4.0;
    let max_h = (ui.available_height() - theme_picker_h).max(row_h * 3.0);
    let box_h = max_h;

    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(avail_w, box_h),
        egui::Sense::hover(),
    );
    self.theme.provider().draw_sunken(ui.painter(), rect);

    let mut child = ui.new_child(
        egui::UiBuilder::new().max_rect(rect.shrink(4.0)).layout(*ui.layout())
    );
    child.style_mut().spacing.item_spacing = egui::vec2(4.0, gap);

    let show_row = |ui: &mut egui::Ui, label: &str, val: String| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(label).monospace().size(sz).color(dim));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(val).monospace().size(sz));
            });
        });
    };

    egui::ScrollArea::vertical()
        .id_salt("sys_metrics_scroll")
        .max_height(box_h - 8.0)
        .show(&mut child, |ui| {
            ui.style_mut().spacing.item_spacing = egui::vec2(4.0, gap);
            ui.style_mut().spacing.interact_size.y = row_h;
            show_row(ui, "GPU", info.name.clone());
            show_row(ui, "API", format!("{:?}", info.backend));
            show_row(ui, "CPU", std::env::consts::ARCH.to_string());
            show_row(ui, "FPS", format!("{:.1}", self.fps));
            for (abbr, temp) in &temp_map {
                show_row(ui, abbr, format!("{:.0}°", temp));
            }
        });
}
}
