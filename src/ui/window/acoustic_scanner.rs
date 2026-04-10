{
    ui.add_space(GAP_XS);
    ui.style_mut().spacing.item_spacing.y = 0.0;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = GAP_XS;
        // Mute Icon Button
        let icon = if self.is_muted || self.acoustic_volume <= 0.01 { "_" } else { "♪" };
        let cap_side = crate::ui::KEY_CAP_SIDE;
        ui.spacing_mut().interact_size.y = cap_side;

        if self.theme.provider().key_cap_small(ui, icon, cap_side, 20.0).clicked() {
            if self.is_muted || self.acoustic_volume <= 0.01 {
                self.is_muted = false;
                if self.acoustic_volume <= 0.01 {
                    self.acoustic_volume = 10.0;
                }
            } else {
                self.is_muted = true;
            }
        }

        // Native Volume Gauge
        let slider_rect = crate::ui::widgets::slider_fill_f32(
            self.theme.provider(),
            ui,
            &mut self.acoustic_volume,
            0.0..=100.0
        );

        // If the user drags or clicks the slider, automatically lock mute status
        if slider_rect.dragged() || slider_rect.clicked() {
            self.is_muted = self.acoustic_volume <= 0.01;
        }

        // Translate 0-100 GUI logic to 0.0-2.0 Engine logic
        let engine_limit = self.acoustic_volume * (2.0 / 100.0);
        let effective_volume = if self.is_muted { 0.0 } else { engine_limit };
        self.synth_engine.set_volume(effective_volume);
    });
}
