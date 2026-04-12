{
    const VOLUME_EPSILON: f32 = 0.01;
    const DEFAULT_RECOVERY_VOLUME: f32 = 10.0;
    const VOLUME_MAX: f32 = 100.0;
    const ENGINE_VOLUME_MAX: f32 = 2.0;

    ui.add_space(GAP_XS);
    ui.style_mut().spacing.item_spacing.y = 0.0;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = GAP_XS;
        // Mute Icon Button
        let is_effectively_muted = self.is_muted || self.acoustic_volume <= VOLUME_EPSILON;
        let icon = if is_effectively_muted { "_" } else { "♪" };
        let cap_side = crate::ui::KEY_CAP_SIDE;
        ui.spacing_mut().interact_size.y = cap_side;

        let is_m_down = ui.input(|i| i.key_down(egui::Key::M));
        let m_pressed = ui.input(|i| i.key_pressed(egui::Key::M));
        let m_clicked = self.theme.provider().key_cap_small(ui, icon, cap_side, 20.0, is_m_down).clicked();

        if m_clicked || m_pressed {
            if is_effectively_muted {
                self.is_muted = false;
                if self.acoustic_volume <= VOLUME_EPSILON {
                    self.acoustic_volume = DEFAULT_RECOVERY_VOLUME;
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
            0.0..=VOLUME_MAX
        );

        // If the user drags or clicks the slider, automatically lock mute status
        if slider_rect.dragged() || slider_rect.clicked() {
            self.is_muted = self.acoustic_volume <= VOLUME_EPSILON;
        }

        // Translate 0-100 GUI logic to 0.0-2.0 Engine logic
        let engine_limit = self.acoustic_volume * (ENGINE_VOLUME_MAX / VOLUME_MAX);
        let effective_volume = if self.is_muted { 0.0 } else { engine_limit };
        self.synth_engine.set_volume(effective_volume);
    });
}
