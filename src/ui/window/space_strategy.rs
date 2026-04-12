{
if crate::ui::widgets::collapsible_header(self.theme.provider(), ui, "SPACE STRATEGY", self.show_strategy) {
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
    let (rect, response) = ui.allocate_exact_size(egui::vec2(plot_w, 200.0), egui::Sense::click_and_drag());

    self.theme.provider().draw_space_strategy_bg(ui, rect);

    // Draw the parameter space plot
    self.strategy_engine.draw(ui, rect, &response, &self.wave_colors, self.theme.provider().palette().chart_axis_color, self.theme.provider());
}
}
