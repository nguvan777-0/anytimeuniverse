{
if self.theme.provider().collapsible_header(ui, "SPACE STRATEGY", self.show_strategy) {
    self.show_strategy = !self.show_strategy;
}

if self.show_strategy {
    ui.add_space(GAP_XS);
    ui.style_mut().spacing.item_spacing.y = 0.0;

    const PERIOD: f64 = std::f64::consts::TAU / 0.1;
    let current_t = self.t_epoch as f64 * PERIOD + self.t_residual;

    // Live continuous scanning — only when T is advancing (paused → skip, T is frozen)
    if !self.is_paused {
        self.strategy_engine.scan(&self.env_data, self.background_noise as f64, current_t, 1000);
    }

    
    ui.add_space(GAP_XS);

    let plot_w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(plot_w, 200.0), egui::Sense::hover());
    
    // Background
    ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, self.theme.provider().chart_bg());
    self.theme.provider().draw_sunken(ui.painter(), rect);

    // Draw the parameter space plot
    self.strategy_engine.draw(ui, rect, &self.wave_colors, self.theme.provider().chart_axis_color());
}
}
