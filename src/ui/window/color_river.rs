{
if crate::ui::widgets::collapsible_header(self.theme.provider(), ui, "COLOR RIVER", self.show_branch) {
    self.show_branch = !self.show_branch;
}

if self.show_branch {
    ui.add_space(GAP_XS);
    ui.style_mut().spacing.item_spacing.y = 0.0;

    // Chart — always visible, fills from history
    let chart_height = 160.0;
    let chart_w = ui.available_width();
    let (rect, _response) = ui.allocate_exact_size(
        egui::vec2(chart_w, chart_height),
        egui::Sense::hover(),
    );
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::ZERO,
        egui::Color32::from_rgb(10, 10, 10),
    );
    self.theme.provider().draw_sunken(ui.painter(), rect);

    // 2. Render Live History - full width
    if !self.history.is_empty() {
        let n = self.history.len() as f32;
        let dx = rect.width() / n;
        let x0 = rect.min.x;
        for (i, (slice, colors)) in self.history.iter().enumerate() {
            let total: f32 = slice.iter().map(|&c| c as f32).sum::<f32>().max(1.0);
            let mut current_y = rect.max.y;
            let x = x0 + (i as f32) * dx;
            for (generation, &count) in slice.iter().enumerate() {
                let h = (count as f32 / total) * chart_height;
                if h > 0.0 {
                    let color = colors[generation % colors.len()];
                    let top = (current_y - h).floor();
                    ui.painter().rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x.floor(), top),
                            egui::pos2((x + dx).ceil(), current_y.ceil()),
                        ),
                        egui::CornerRadius::ZERO,
                        color,
                    );
                    current_y -= h;
                }
            }
        }
    }

// Legend — top colors
if let Some(stats) = &self.last_stats {
    let mut active_colors: Vec<_> = stats.color_counts.iter().enumerate().filter(|(_, c)| **c > 0).collect();
    if !active_colors.is_empty() {
        ui.horizontal_wrapped(|ui| {
            active_colors.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
            for (generation, &count) in active_colors.into_iter().take(20) {
                ui.horizontal(|ui| {
                let color = self.wave_colors[generation % self.wave_colors.len()];
                let (lbl_rect, _) = ui.allocate_exact_size(
                    egui::vec2(8.0, 8.0),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(
                    lbl_rect,
                    egui::CornerRadius::ZERO,
                    color,
                );
                ui.label(format!("{count}"));
                ui.add_space(GAP_MD);
            });
        }
    });
    }
}
}
}
