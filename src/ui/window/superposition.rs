{
if crate::ui::widgets::collapsible_header(self.theme.provider(), ui, "SUPERPOSITION", self.show_branch_metrics) {
    self.show_branch_metrics = !self.show_branch_metrics;
}

if self.show_branch_metrics {
    ui.add_space(GAP_XS);

    let env_data = make_env_data(&self.seed);
    let current_t = self.t_epoch as f64 * PERIOD_SL + self.t_residual;
    let metrics = crate::ui::ascii_render::get_summary_metrics(&env_data, current_t, self.background_noise as f64);

    let dim = self.theme.provider().palette().panel_text_color;

    let font    = egui::FontId::monospace(11.0);
    let pad     = 6.0;
    let col_gap = 10.0;
    let row_gap = 5.0;

    // measure text width at our font
    let mw = |s: &str| -> f32 {
        ui.painter().layout_no_wrap(s.to_string(), font.clone(), dim).size().x.ceil()
    };
    let row_h = ui.painter().layout_no_wrap("W".to_string(), font.clone(), dim).size().y.ceil();

    // fixed columns sized to their widest possible content
    let w_wave = mw("wave0");
    let w_resn = mw("RESN").max(mw("0.00"));
    let w_pwr  = mw("PWR").max(mw("0.7766"));
    let w_enrg = mw("ENERGY").max(mw("+1.000"));

    // GN gets all remaining space so large generations stay as integers longer
    let w_gn = (ui.available_width() - pad * 2.0 - w_wave - w_resn - w_pwr - w_enrg - 4.0 * col_gap)
        .max(mw("GN"));

    // how many digit characters fit → sets the integer threshold
    let digit_w    = mw("0");
    let max_digits = (w_gn / digit_w).floor().max(1.0) as u32;
    let gn_threshold: u64 = 10u64.saturating_pow(max_digits);

    // fixed overhead for e notation: "X.e" (mantissa digit + dot + e)
    let e_prefix_w = mw("0.e");

    let fmt_gn = |gn: u64| -> String {
        if gn < gn_threshold {
            format!("{}", gn)
        } else {
            let e        = (gn as f64).log10().floor() as i32;
            // monospace: every char is digit_w wide, so exponent width = num digits * digit_w
            let exp_w    = digit_w * format!("{}", e).len() as f32;
            let decimals = ((w_gn - e_prefix_w - exp_w) / digit_w).floor().max(0.0) as usize;
            format!("{:.prec$}e{}", gn as f64 / 10f64.powi(e), e, prec = decimals)
        }
    };

    let num_rows = 4; // header + 3 data rows
    let table_h = pad * 2.0 + num_rows as f32 * row_h + (num_rows - 1) as f32 * row_gap;

    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), table_h),
        egui::Sense::hover(),
    );
    self.theme.provider().draw_sunken(ui.painter(), rect);

    let p = ui.painter().with_clip_rect(rect);

    // left edge of each column
    let x_wave = rect.min.x + pad;
    let x_gn   = x_wave + w_wave + col_gap;
    let x_resn = x_gn   + w_gn   + col_gap;
    let x_pwr  = x_resn + w_resn + col_gap;
    let x_enrg = x_pwr  + w_pwr  + col_gap;

    // paint helpers: left-aligned and right-aligned within a column
    let left  = |p: &egui::Painter, x: f32, y: f32, s: &str, c: egui::Color32| {
        p.text(egui::pos2(x, y), egui::Align2::LEFT_CENTER, s, font.clone(), c);
    };
    let right = |p: &egui::Painter, x: f32, w: f32, y: f32, s: &str, c: egui::Color32| {
        p.text(egui::pos2(x + w, y), egui::Align2::RIGHT_CENTER, s, font.clone(), c);
    };

    // header row — headers right-aligned to match their data columns
    let y = rect.min.y + pad + row_h * 0.5;
    left (&p, x_wave, y, "WAVE",  dim);
    right(&p, x_gn,   w_gn,   y, "GN",     dim);
    right(&p, x_resn, w_resn, y, "RESN",   dim);
    right(&p, x_pwr,  w_pwr,  y, "PWR",    dim);
    right(&p, x_enrg, w_enrg, y, "ENERGY", dim);

    // data rows
    for i in 0..3 {
        let y = rect.min.y + pad + (i + 1) as f32 * (row_h + row_gap) + row_h * 0.5;
        let (gn, resonance, power, energy) = metrics[i];

        let wave_color = {
            let r = self.wave_colors.get(i).map(|c| c.r()).unwrap_or(255);
            let g = self.wave_colors.get(i).map(|c| c.g()).unwrap_or(255);
            let b = self.wave_colors.get(i).map(|c| c.b()).unwrap_or(255);
            egui::Color32::from_rgb(r, g, b)
        };

        let zeroed = energy <= 0.0;
        const PHI: f64 = std::f64::consts::GOLDEN_RATIO;
        const RES_TARGET: f64 = PHI - 1.0;
        let res_color = if (resonance - RES_TARGET).abs() < 0.05 && !zeroed {
            egui::Color32::YELLOW
        } else {
            dim
        };

        let res_str  = format!("{:.2}", resonance);
        let enrg_str = if zeroed { "zeroed".to_string() } else { format!("{:+.3}", energy) };

        left (&p, x_wave, y, &format!("wave{}", i), wave_color);
        right(&p, x_gn,   w_gn,   y, &fmt_gn(gn),       dim);
        right(&p, x_resn, w_resn, y, &res_str,           res_color);
        right(&p, x_pwr,  w_pwr,  y, &format!("{:.4}", power), dim);
        right(&p, x_enrg, w_enrg, y, &enrg_str,         dim);
    }
}
}
