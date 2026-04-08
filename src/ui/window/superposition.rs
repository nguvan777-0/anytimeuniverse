{
if self.theme.provider().collapsible_header(ui, "SUPERPOSITION", self.show_branch_metrics) {
    self.show_branch_metrics = !self.show_branch_metrics;
}

if self.show_branch_metrics {
    ui.add_space(GAP_XS);
    ui.style_mut().spacing.item_spacing.y = 0.0;
    let env_data = make_env_data(&self.seed);
    let current_t = self.t_epoch as f64 * PERIOD_SL + self.t_residual;
    let metrics = crate::ui::ascii_render::get_summary_metrics(&env_data, current_t, self.background_noise as f64);

    let dim = if matches!(self.theme, Theme::Rect) {
        egui::Color32::from_rgb(0, 230, 65) /* TERM_GREEN */
    } else if matches!(self.theme, Theme::Future) {
        egui::Color32::from_rgb(192, 202, 222) /* FUTURE TEXT */
    } else {
        egui::Color32::from_rgb(100, 100, 105)
    };
    let sample_galley = ui.painter().layout_no_wrap(
        "Wg".to_string(),
        egui::FontId::monospace(11.0),
        dim,
    );
    let row_h = ui.spacing().interact_size.y.max(sample_galley.size().y);
    let row_spacing = 6.0;
    let num_rows = 4usize; // header + 3 waves
    let total_h = num_rows as f32 * row_h + (num_rows - 1) as f32 * row_spacing + 8.0; // 8 = 4px shrink top+bottom

    let (table_rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), total_h),
        egui::Sense::hover(),
    );
    self.theme.provider().draw_sunken(ui.painter(), table_rect);

    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(table_rect.shrink(4.0)).layout(*ui.layout()));
    child_ui.style_mut().spacing.item_spacing.y = 0.0;

    let inner_w = child_ui.available_width();
    let col_width = ((inner_w - 40.0) / 5.0).floor().max(20.0);
    egui::Grid::new("summary_metrics_grid")
        .num_columns(5)
        .spacing([10.0, row_spacing])
        .min_col_width(col_width)
        .show(&mut child_ui, |ui| {
            ui.label(egui::RichText::new("WAVE").monospace().size(11.0).color(dim).strong());
            ui.label(egui::RichText::new("GN").monospace().size(11.0).color(dim).strong());
            ui.label(egui::RichText::new("RESN").monospace().size(11.0).color(dim).strong());
            ui.label(egui::RichText::new("PWR").monospace().size(11.0).color(dim).strong());
            ui.label(egui::RichText::new("ENERGY").monospace().size(11.0).color(dim).strong());
            ui.end_row();

            for i in 0..3 {
                let (gn, resonance, power, energy) = metrics[i];
                let r = self.wave_colors.get(i).map(|c| c.r()).unwrap_or(255);
                let g = self.wave_colors.get(i).map(|c| c.g()).unwrap_or(255);
                let b = self.wave_colors.get(i).map(|c| c.b()).unwrap_or(255);
                let color = egui::Color32::from_rgb(r, g, b);
                let zeroed = energy <= 0.0;
                const PHI: f64 = std::f64::consts::GOLDEN_RATIO;
                const RES_TARGET: f64 = PHI - 1.0;
                let res_color = if (resonance - RES_TARGET).abs() < 0.05 && !zeroed { egui::Color32::YELLOW } else { dim };

                ui.label(egui::RichText::new(format!("W{}", i)).monospace().size(11.0).color(color).strong());
                ui.label(egui::RichText::new(format!("{:>4}", gn)).monospace().size(11.0).color(dim));
                let format_res = format!("{:.2}", resonance);
                let res_stripped = format_res.strip_prefix("0").unwrap_or(&format_res);
                ui.label(egui::RichText::new(format!("{:>4}", res_stripped)).monospace().size(11.0).color(res_color));
                ui.label(egui::RichText::new(format!("{:.4}", power)).monospace().size(11.0).color(dim));
                let e_text = if zeroed { "zeroed".to_string() } else { format!("{energy:>+.3}") };
                ui.label(egui::RichText::new(e_text).monospace().size(11.0).color(dim));
                ui.end_row();
            }
        });
}
}
