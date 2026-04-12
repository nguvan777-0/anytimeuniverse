{
    ui.add_space(GAP_XS);
    
    let cap_side = crate::ui::KEY_CAP_SIDE;
    let font_sz = 14.0;

    ui.horizontal(|ui| {
        // Camera movement rig (Keyboard block)
        ui.vertical(|ui| {
            ui.style_mut().spacing.item_spacing.y = GAP_XS;

            // Top Row: Dolly/Zoom, Vertical Pan, and Reset (Q W E V)
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = GAP_XS;
                ui.spacing_mut().interact_size.y = cap_side;

                if self.theme.provider().key_cap_small(ui, "Q", cap_side, font_sz, ui.input(|i| i.key_down(egui::Key::Q))).clicked() {
                    self.zoom /= 1.25; // Pull back (wider fov)
                    self.field_force_redraw = true;
                }
                if self.theme.provider().key_cap_small(ui, "W", cap_side, font_sz, ui.input(|i| i.key_down(egui::Key::W))).clicked() {
                    self.pan_y += 0.25 / self.zoom; // Pan up
                    self.field_force_redraw = true;
                }
                if self.theme.provider().key_cap_small(ui, "E", cap_side, font_sz, ui.input(|i| i.key_down(egui::Key::E))).clicked() {
                    self.zoom *= 1.25; // Push in (telephoto)
                    self.field_force_redraw = true;
                }
                
                if self.theme.provider().key_cap_small(ui, "V", cap_side, font_sz, ui.input(|i| i.key_down(egui::Key::V))).clicked() || ui.input(|i| i.key_pressed(egui::Key::V)) {
                    self.zoom = 1.0; // Reset zoom
                    self.pan_x = 0.0; // Reset pan X
                    self.pan_y = 0.0; // Reset pan Y
                    self.field_force_redraw = true;
                    // Reset 3D scanner/tracker orientation
                    self.strategy_engine.reset_view();
                }
            });

            // Bottom Row: Horizontal and Vertical Pan, and Fullscreen (A S D F)
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = GAP_XS;
                ui.spacing_mut().interact_size.y = cap_side;

                // Keyboard-like stagger offset
                ui.add_space(cap_side * 0.4);

                if self.theme.provider().key_cap_small(ui, "A", cap_side, font_sz, ui.input(|i| i.key_down(egui::Key::A))).clicked() {
                    self.pan_x -= 0.25 / self.zoom; // Pan left 
                    self.field_force_redraw = true;
                }
                if self.theme.provider().key_cap_small(ui, "S", cap_side, font_sz, ui.input(|i| i.key_down(egui::Key::S))).clicked() {
                    self.pan_y -= 0.25 / self.zoom; // Pan down
                    self.field_force_redraw = true;
                }
                if self.theme.provider().key_cap_small(ui, "D", cap_side, font_sz, ui.input(|i| i.key_down(egui::Key::D))).clicked() {
                    self.pan_x += 0.25 / self.zoom; // Pan right
                    self.field_force_redraw = true;
                }

                if self.theme.provider().key_cap_small(ui, "F", cap_side, font_sz, ui.input(|i| i.key_down(egui::Key::F))).clicked() || ui.input(|i| i.key_pressed(egui::Key::F)) {
                    self.pending_fullscreen_toggle = true;
                }
            });
        });

        ui.add_space(GAP_XS);

        // Telemetry text field box (Optics/Positioning)
        ui.vertical(|ui| {
            let label_font = egui::FontId::monospace(12.0);
            let text_color = self.theme.provider().palette().panel_text_color;
            let box_h = (cap_side * 2.0) + GAP_XS; // Match height of the two rows of keys

            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), box_h),
                egui::Sense::hover()
            );
            self.theme.provider().draw_sunken(ui.painter(), rect);

            let mut child_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect.shrink(6.0))
                    .layout(egui::Layout::top_down(egui::Align::Center))
            );
            child_ui.style_mut().spacing.item_spacing.y = 8.0;

            let z = (self.zoom * 100.0).round() / 100.0;
            child_ui.label(egui::RichText::new(format!("{}x", z)).font(label_font.clone()).color(text_color));

            let format_no_zero = |v: f32| -> String {
                let mut s = format!("{}", v);
                if s.starts_with("0.") {
                    s.remove(0);
                } else if s.starts_with("-0.") {
                    s.remove(1);
                }
                s
            };

            let px = (self.pan_x * 100.0).round() / 100.0;
            let py = (self.pan_y * 100.0).round() / 100.0;
            child_ui.label(egui::RichText::new(format!("({}, {})", format_no_zero(px), format_no_zero(py))).font(label_font).color(text_color));
        });
    });
}