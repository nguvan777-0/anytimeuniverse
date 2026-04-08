impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

        // Use LogicalSize so the 260px panel is always 260 logical px regardless of DPI,
        // and DISPLAY_H controls the zoom level.
        let window = Arc::new(
            event_loop
                .create_window(
                    winit::window::WindowAttributes::default()
                        .with_title("anytimeuniverse")
                        .with_inner_size(winit::dpi::LogicalSize::new(DISPLAY_H + 260 + 260, DISPLAY_H))
                        .with_min_inner_size(winit::dpi::LogicalSize::new(320u32, 240u32)),
                )
                .expect("failed to create window"),
        );

        let surface: wgpu::Surface<'static> = self
            .instance
            .create_surface(Arc::clone(&window))
            .expect("failed to create surface");

        let caps = surface.get_capabilities(&self.adapter);
        let format = caps.formats[0];

        let inner = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format,
            width: inner.width.max(1),
            height: inner.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&self.device, &config);

        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("render"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../render.wgsl").into()),
            });

        // Tick uniform: [tick, noise, epoch, pan_x, pan_y, pad, pad, pad] = 32 bytes (std140)
        let tick_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tick-uniform"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // EnvUniform: 3 waves × 8 f32 = 96 bytes
        // Wave layout: amp, freq, phase, dir_x, dir_y, _p0, _p1, _p2
        let env_data = make_env_data(&self.seed);
        let env_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("env-uniform"),
            size: (env_data.len() * 4) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&env_buf, 0, bytemuck::cast_slice(&env_data));

        // WaveColors uniform: 3 × vec4<f32> = 48 bytes
        let wc_data: [f32; 12] = {
            let wc = &self.wave_colors;
            [
                wc[0].r() as f32 / 255.0, wc[0].g() as f32 / 255.0, wc[0].b() as f32 / 255.0, 0.0,
                wc[1].r() as f32 / 255.0, wc[1].g() as f32 / 255.0, wc[1].b() as f32 / 255.0, 0.0,
                wc[2].r() as f32 / 255.0, wc[2].g() as f32 / 255.0, wc[2].b() as f32 / 255.0, 0.0,
            ]
        };
        let color_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wave-colors-uniform"),
            size: (wc_data.len() * 4) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&color_buf, 0, bytemuck::cast_slice(&wc_data));

        let bgl = self
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    // binding 0 — sim uniform (wave time T)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // binding 1 — env uniform (3 waves) — kept for buffer continuity
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // binding 2 — wave colors (3 seed-derived colors)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // binding 3 — wave cache (6 × WaveData, 384 bytes)
                    // Pre-baked per-frame results of fhash/memory/gen_acc etc.
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let wave_cache_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label:              Some("wave-cache"),
            size:               std::mem::size_of::<[crate::engine::wave_cache::WaveData; 6]>() as u64,
            usage:              wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render-bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: tick_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: env_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: color_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: wave_cache_buf.as_entire_binding() },
            ],
        });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(&bgl)],
                immediate_size: 0,
            });

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("render"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        let egui_ctx = egui::Context::default();
        Theme::Dew.provider().apply_theme(&egui_ctx);

        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(&self.device, format, egui_wgpu::RendererOptions {
            msaa_samples: 1,
            depth_stencil_format: None,
            dithering: false,
            ..Default::default()
        });

        // ── Offscreen field texture ───────────────────────────────────────────
        // The expensive field shader renders into this once per T-change.
        // A cheap blit copies it to the swap-chain surface every frame.
        let field_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label:                 Some("field-texture"),
            size:                  wgpu::Extent3d { width: inner.width.max(1), height: inner.height.max(1), depth_or_array_layers: 1 },
            mip_level_count:       1,
            sample_count:          1,
            dimension:             wgpu::TextureDimension::D2,
            format,
            usage:                 wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats:          &[],
        });
        let field_view = field_texture.create_view(&Default::default());

        // ── Blit pipeline ─────────────────────────────────────────────────────
        let blit_shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label:  Some("blit"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../blit.wgsl").into()),
        });
        let blit_bgl = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding:    0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty:         wgpu::BindingType::Texture {
                        sample_type:    wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled:   false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding:    1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty:         wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count:      None,
                },
            ],
        });
        let blit_sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let blit_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("blit-bg"),
            layout:  &blit_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&field_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&blit_sampler) },
            ],
        });
        let blit_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label:                Some("blit-layout"),
            bind_group_layouts:   &[Some(&blit_bgl)],
            immediate_size:       0,
        });
        let blit_pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label:    Some("blit"),
            layout:   Some(&blit_layout),
            vertex:   wgpu::VertexState { module: &blit_shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState {
                module:               &blit_shader,
                entry_point:          Some("fs_main"),
                targets:              &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options:  Default::default(),
            }),
            primitive:       wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleStrip, ..Default::default() },
            depth_stencil:   None,
            multisample:     wgpu::MultisampleState::default(),
            multiview_mask:  None,
            cache:           None,
        });

        self.state = Some(RenderState {
            window,
            surface,
            config,
            pipeline,
            bg,
            tick_buf,
            env_buf,
            color_buf,
            egui_ctx,
            egui_state,
            egui_renderer,
            field_texture,
            field_view,
            blit_pipeline,
            blit_bg,
            blit_bgl,
            blit_sampler,
            wave_cache_buf,
        });
        self.field_force_redraw = true;
        self.state.as_ref().unwrap().window.focus_window();
        self.state.as_ref().unwrap().window.request_redraw();
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {
        let mut got_stats = false;
        
        while self.sim_handle.stats_buffer.update() {
            let stats = self.sim_handle.stats_buffer.read().clone();
            if let Some(prev) = &self.last_stats
                && stats.tick < prev.tick {
                    self.branch_density_latest = None;
                    self.branch_density_dirty = false;
                    self.last_projection_tick = 0;
                    self.last_bounds_instant = None;
                    self.last_sent_bounds = [-15.0, 15.0, -15.0, 15.0];
                    self.circle_axes = ([0.0; 14], [0.0; 14], [0.0; 14]);
                    self.history.clear();
                }
            // color_counts is left empty by the sim thread — skip empty pushes.
            if !stats.color_counts.is_empty() {
                self.history.push_back((stats.color_counts.clone(), self.wave_colors.clone()));
                if self.history.len() > 240 {
                    self.history.pop_front();
                }
            }
            if let Some(density) = stats.branch_density.clone() {
                self.branch_density_latest = Some(density);
                self.branch_density_dirty = true;
            }
            self.last_stats = Some(stats);
            got_stats = true;
        }

        if got_stats {
            // Stats updated — mark dirty but don't request redraw.
            // The vsync render loop runs continuously when unpaused and will
            // pick up the latest stats on the next frame automatically.
            let _ = got_stats; // suppress unused warning
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // Handle pan keys before egui gets a chance to consume them.
        if let WindowEvent::KeyboardInput { event: ref key_event, .. } = event {
            if key_event.state == winit::event::ElementState::Pressed {
                if let winit::keyboard::PhysicalKey::Code(code) = key_event.physical_key {
                    let panned = match code {
                        winit::keyboard::KeyCode::KeyW => { self.pan_y -= 0.25; true }
                        winit::keyboard::KeyCode::KeyA => { self.pan_x -= 0.25; true }
                        winit::keyboard::KeyCode::KeyS => { self.pan_y += 0.25; true }
                        winit::keyboard::KeyCode::KeyD => { self.pan_x += 0.25; true }
                        _ => false,
                    };
                    if panned {
                        self.field_force_redraw = true;
                        if let Some(state) = &self.state {
                            state.window.request_redraw();
                        }
                    }
                }
            }
        }

        if let Some(state) = &mut self.state {
            let res = state.egui_state.on_window_event(&state.window, &event);
            if res.repaint {
                state.window.request_redraw();
            }
            if res.consumed {
                return;
            }
        }

        match event {
            WindowEvent::Resized(physical_size) => {
                if let Some(state) = &mut self.state
                    && physical_size.width > 0 && physical_size.height > 0 {
                        state.config.width = physical_size.width;
                        state.config.height = physical_size.height;
                        state.surface.configure(&self.device, &state.config);
                        // Recreate field texture at new dimensions
                        let new_tex = self.device.create_texture(&wgpu::TextureDescriptor {
                            label:              Some("field-texture"),
                            size:               wgpu::Extent3d { width: physical_size.width, height: physical_size.height, depth_or_array_layers: 1 },
                            mip_level_count:    1,
                            sample_count:       1,
                            dimension:          wgpu::TextureDimension::D2,
                            format:             state.config.format,
                            usage:              wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                            view_formats:       &[],
                        });
                        let new_view = new_tex.create_view(&Default::default());
                        let new_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label:   Some("blit-bg"),
                            layout:  &state.blit_bgl,
                            entries: &[
                                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&new_view) },
                                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&state.blit_sampler) },
                            ],
                        });
                        state.field_texture = new_tex;
                        state.field_view    = new_view;
                        state.blit_bg       = new_bg;
                        self.field_force_redraw = true;
                        state.window.request_redraw();
                    }
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == winit::event::ElementState::Pressed
                    && let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
                        match code {
                            winit::keyboard::KeyCode::KeyF => {
                                self.pending_fullscreen_toggle = true;
                            }
                            winit::keyboard::KeyCode::F12 => {
                                self.take_screenshot = true;
                            }
                            winit::keyboard::KeyCode::KeyW
                            | winit::keyboard::KeyCode::KeyA
                            | winit::keyboard::KeyCode::KeyS
                            | winit::keyboard::KeyCode::KeyD => {
                                // Handled before egui above so panning always works.
                            }
                            winit::keyboard::KeyCode::KeyR => {
                                // Rewind: zero T, speed=1, resume, same seed
                                self.t_epoch = 0;
                                self.t_residual = 0.0;
                                self.wave_speed = 1.0;
                                self.custom_speed = 1.0;
                                self.is_paused = false;
                                let _ = self.sim_handle.cmd_tx.send(Command::Resume);
                                self.reset_simulation(false);
                            }
                            winit::keyboard::KeyCode::KeyC => {
                                // New seed: zero T, speed=1, resume, new seed
                                self.t_epoch = 0;
                                self.t_residual = 0.0;
                                self.wave_speed = 1.0;
                                self.custom_speed = 1.0;
                                self.is_paused = false;
                                let _ = self.sim_handle.cmd_tx.send(Command::Resume);
                                self.reset_simulation(true);
                            }
                            winit::keyboard::KeyCode::Space => {
                                self.is_paused = !self.is_paused;
                                if self.is_paused {
                                    let _ = self.sim_handle.cmd_tx.send(Command::Pause);
                                    println!("[ world ] pause");
                                } else {
                                    let _ = self.sim_handle.cmd_tx.send(Command::Resume);
                                    println!("[ world ] resume");
                                }
                                if let Some(state) = &self.state {
                                    state.window.request_redraw();
                                }
                            }
                            winit::keyboard::KeyCode::Digit0
                            | winit::keyboard::KeyCode::Digit1
                            | winit::keyboard::KeyCode::Digit2
                            | winit::keyboard::KeyCode::Digit3
                            | winit::keyboard::KeyCode::Digit4
                            | winit::keyboard::KeyCode::Digit5
                            | winit::keyboard::KeyCode::Numpad0
                            | winit::keyboard::KeyCode::Numpad1
                            | winit::keyboard::KeyCode::Numpad2
                            | winit::keyboard::KeyCode::Numpad3
                            | winit::keyboard::KeyCode::Numpad4
                            | winit::keyboard::KeyCode::Numpad5 => {
                                let s = match code {
                                    winit::keyboard::KeyCode::Digit0
                                    | winit::keyboard::KeyCode::Numpad0 => 0u8,
                                    winit::keyboard::KeyCode::Digit1
                                    | winit::keyboard::KeyCode::Numpad1 => 1,
                                    winit::keyboard::KeyCode::Digit2
                                    | winit::keyboard::KeyCode::Numpad2 => 2,
                                    winit::keyboard::KeyCode::Digit3
                                    | winit::keyboard::KeyCode::Numpad3 => 3,
                                    winit::keyboard::KeyCode::Digit4
                                    | winit::keyboard::KeyCode::Numpad4 => 4,
                                    _ => 5,
                                };
                                self.speed = s;
                                self.wave_speed = match s {
                                    0 => 0.25,
                                    1 => 1.0,
                                    2 => 10.0,
                                    3 => 100.0,
                                    4 => 1_000.0,
                                    _ => 1_000_000.0,
                                };
                                self.custom_speed = self.wave_speed as f64; // keep slider in sync
                                let _ = self
                                    .sim_handle
                                    .cmd_tx
                                    .send(Command::SetSpeed(speed_duration(s)));
                            }
                            winit::keyboard::KeyCode::ArrowLeft
                            | winit::keyboard::KeyCode::ArrowRight => {
                                const FREQ_MIN: f64 = 0.1;
                                const PERIOD: f64 = std::f64::consts::TAU / FREQ_MIN;
                                let current_t = self.t_epoch as f64 * PERIOD + self.t_residual;
                                let jump = if current_t.abs() < 1.0 {
                                    PERIOD
                                } else {
                                    let magnitude = 10f64.powi(current_t.abs().log10().floor() as i32);
                                    (current_t.abs() / magnitude).floor() * magnitude
                                };
                                if code == winit::keyboard::KeyCode::ArrowLeft {
                                    self.t_residual -= jump;
                                } else {
                                    self.t_residual += jump;
                                }
                                // Normalise residual into [0, PERIOD)
                                if self.t_residual >= PERIOD {
                                    let extra = (self.t_residual / PERIOD).floor() as i64;
                                    self.t_epoch = self.t_epoch.saturating_add(extra);
                                    self.t_residual -= extra as f64 * PERIOD;
                                } else if self.t_residual < 0.0 {
                                    let borrow = (-self.t_residual / PERIOD).ceil() as i64;
                                    self.t_epoch = self.t_epoch.saturating_sub(borrow);
                                    self.t_residual += borrow as f64 * PERIOD;
                                }
                                self.is_paused = true;
                                let _ = self.sim_handle.cmd_tx.send(Command::Pause);
                                if let Some(state) = &self.state {
                                    state.window.request_redraw();
                                }
                            }
                            winit::keyboard::KeyCode::ArrowUp
                            | winit::keyboard::KeyCode::ArrowDown => {
                                // Step through symmetric speed ladder crossing zero (pause)
                                const LADDER: &[f64] = &[
                                    -1e12, -1e11, -1e10, -1e9, -1e8, -1e7,
                                    -1_000_000.0, -1_000.0, -100.0, -10.0, -1.0, -0.25,
                                    0.0, // pause
                                    0.25, 1.0, 10.0, 100.0, 1_000.0, 1_000_000.0,
                                    1e7, 1e8, 1e9, 1e10, 1e11, 1e12,
                                ];
                                let cur = self.wave_speed as f64;
                                // Find closest ladder index
                                let idx = LADDER
                                    .iter()
                                    .enumerate()
                                    .min_by(|(_, a), (_, b)| {
                                        ((**a - cur).abs()).partial_cmp(&((**b - cur).abs())).unwrap()
                                    })
                                    .map(|(i, _)| i)
                                    .unwrap_or(7); // default to 1.0
                                let new_idx = if code == winit::keyboard::KeyCode::ArrowUp {
                                    (idx + 1).min(LADDER.len() - 1)
                                } else {
                                    idx.saturating_sub(1)
                                };
                                let new_speed = LADDER[new_idx];
                                if new_speed == 0.0 {
                                    self.is_paused = true;
                                    let _ = self.sim_handle.cmd_tx.send(Command::Pause);
                                } else {
                                    self.is_paused = false;
                                    let _ = self.sim_handle.cmd_tx.send(Command::Resume);
                                }
                                self.wave_speed = new_speed as f32;
                                self.custom_speed = new_speed;
                                if let Some(state) = &self.state {
                                    state.window.request_redraw();
                                }
                            }
                            _ => {}
                        }
                    }
            }
            WindowEvent::RedrawRequested => {
                let mut pending_reset = None;
                let mut pending_title = None;
                let state = self.state.as_mut().unwrap();

                let dt = self.last_frame.elapsed().as_secs_f32();
                let _dt = dt; // clear warning
                self.last_frame = std::time::Instant::now();

                if let Some(_stats) = &self.last_stats {
                    let elapsed = self.last_tps_update.elapsed().as_secs_f64();
                    if elapsed >= 0.5 {
                        const PERIOD: f64 = std::f64::consts::TAU / 0.1;
                        let cur = self.t_epoch as f64 * PERIOD + self.t_residual;
                        let prev = self.last_tps_t_epoch as f64 * PERIOD + self.last_tps_t_residual;
                        self.t_per_sec = (cur - prev) / elapsed;
                        self.last_tps_t_epoch = self.t_epoch;
                        self.last_tps_t_residual = self.t_residual;
                        self.last_tps_update = std::time::Instant::now();
                    }
                }

                let monitor_hz = state
                    .window
                    .current_monitor()
                    .and_then(|m| m.refresh_rate_millihertz())
                    .map(|mhz| (mhz as f32 / 1000.0).round())
                    .unwrap_or(60.0);

                let raw_input = state.egui_state.take_egui_input(&state.window);
                let mut rewind_req = false;
                let mut reroll_req = false;
                let mut pause_req = false;
                let speed_req: Option<u8> = None;
                let mut arrow_up_req = false;
                let mut arrow_down_req = false;
                let mut arrow_left_req = false;
                let mut arrow_right_req = false;
                let mut exit_req = false;
                let mut minimize_req = false;
                let mut fullscreen_req = false;


                let full_output = state.egui_ctx.run_ui(raw_input, |ui| {
                    let _navy_blue = egui::Color32::from_rgb(0, 0, 128);
                    if matches!(self.theme, Theme::Rect) {
                        let term_bg   = egui::Color32::BLACK;
                        let term_green = egui::Color32::from_rgb(0, 230, 65);
                        let term_dim  = term_green;
                        let mut visuals = egui::Visuals::dark();
                        visuals.panel_fill = term_bg;
                        visuals.window_fill = term_bg;
                        visuals.selection.bg_fill = term_dim;
                        visuals.selection.stroke = egui::Stroke::new(1.0, term_bg);
                        visuals.widgets.noninteractive.bg_fill = term_bg;
                        visuals.widgets.noninteractive.weak_bg_fill = term_bg;
                        visuals.widgets.inactive.bg_fill = term_bg;
                        visuals.widgets.inactive.weak_bg_fill = term_bg;
                        visuals.widgets.hovered.bg_fill = term_dim;
                        visuals.widgets.hovered.weak_bg_fill = term_dim;
                        visuals.widgets.active.bg_fill = term_dim;
                        visuals.widgets.active.weak_bg_fill = term_dim;
                        visuals.override_text_color = Some(term_green);
                        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, term_dim);
                        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, term_green);
                        visuals.widgets.active.bg_stroke  = egui::Stroke::new(1.0, term_green);
                        visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::ZERO;
                        visuals.widgets.inactive.corner_radius = egui::CornerRadius::ZERO;
                        visuals.widgets.hovered.corner_radius  = egui::CornerRadius::ZERO;
                        visuals.widgets.active.corner_radius   = egui::CornerRadius::ZERO;
                        visuals.window_corner_radius = egui::CornerRadius::ZERO;
                        visuals.menu_corner_radius   = egui::CornerRadius::ZERO;
                        ui.ctx().set_visuals(visuals);
                        let mut fonts = egui::FontDefinitions::default();
                        if let Some(mono) = fonts.families.get(&egui::FontFamily::Monospace).cloned() {
                            fonts.families.insert(egui::FontFamily::Proportional, mono);
                        }
                        ui.ctx().set_fonts(fonts);
                    }

                    let term_title_bar = |ui: &mut egui::Ui, title_text: &mut String, real_title: &mut String, pending_title: &mut Option<String>, subtitle: Option<&str>, sub_suffix: Option<&str>| -> (bool, bool, bool) {
                        let height = 18.0;
                        let term_bar_bg    = egui::Color32::BLACK;
                        let term_bar_green = egui::Color32::from_rgb(0, 230, 65);
                        // removed dim
                        let (rect, _resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::hover());
                        ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, term_bar_bg);
                        ui.painter().rect_stroke(rect, egui::CornerRadius::ZERO, egui::Stroke::new(1.0, term_bar_green), egui::StrokeKind::Outside);

                        let mut right_edge = rect.max.x - 2.0;

                        let draw_term_btn = |ui: &mut egui::Ui, x: f32, id: &str, text: &str, offset_y: f32| -> bool {
                            let btn_rect = egui::Rect::from_min_size(egui::pos2(x, rect.min.y + 2.0), egui::vec2(14.0, 14.0));
                            let resp = ui.interact(btn_rect, egui::Id::new(id), egui::Sense::click());

                            let is_down = resp.is_pointer_button_down_on();
                            let is_hov  = resp.hovered();
                            let bg = term_bar_bg;
                            let fg = term_bar_green;
                            ui.painter().rect_filled(btn_rect, egui::CornerRadius::ZERO, bg);
                            ui.painter().rect_stroke(btn_rect, egui::CornerRadius::ZERO, if is_down || is_hov { egui::Stroke::NONE } else { egui::Stroke::new(1.0, fg) }, egui::StrokeKind::Outside);

                            let text_pos = btn_rect.center() + egui::vec2(0.0, offset_y);
                            ui.painter().text(
                                text_pos,
                                egui::Align2::CENTER_CENTER,
                                text,
                                egui::FontId::monospace(11.0),
                                fg,
                            );

                            resp.clicked()
                        };

                        right_edge -= 14.0;
                        let max_clicked = draw_term_btn(ui, right_edge, "c_btn_m", "~", 0.0);

                        right_edge -= 16.0;
                        let min_clicked = draw_term_btn(ui, right_edge, "c_btn_n", ".", 0.0);

                        right_edge -= 16.0;
                        let exit_clicked = draw_term_btn(ui, right_edge, "c_btn_x", "*", 0.0);

                        right_edge -= 6.0;

                        if let Some(sub) = subtitle {
                            if let Some(suffix) = sub_suffix {
                                let suffix_rect = ui.painter().text(
                                    egui::pos2(right_edge, rect.min.y + 6.0),
                                    egui::Align2::RIGHT_TOP,
                                    suffix,
                                    egui::FontId::monospace(11.0),
                                    term_bar_green,
                                );
                                right_edge = suffix_rect.min.x - 1.0;
                            }

                            ui.painter().text(
                                egui::pos2(right_edge, rect.min.y + 4.0),
                                egui::Align2::RIGHT_TOP,
                                sub,
                                egui::FontId::monospace(11.0),
                                term_bar_green,
                            );
                        }
                        
                        let text_rect = egui::Rect::from_min_max(rect.min + egui::vec2(4.0, 2.0), egui::pos2(right_edge, rect.max.y));
                        let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(text_rect).layout(egui::Layout::left_to_right(egui::Align::TOP)));
                        child_ui.visuals_mut().extreme_bg_color = egui::Color32::TRANSPARENT;
                        child_ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                        child_ui.visuals_mut().widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                        
                        let edit = egui::TextEdit::singleline(title_text)
                            .frame(egui::Frame::NONE)
                            .font(egui::FontId::monospace(13.0))
                            .text_color(term_bar_green)
                            .margin(egui::vec2(0.0, -1.0));
                        let action = child_ui.add(edit);
                        
                        if action.gained_focus()
                            && let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), action.id) {
                                state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                                    egui::text::CCursor::new(0),
                                    egui::text::CCursor::new(title_text.chars().count()),
                                )));
                                egui::TextEdit::store_state(ui.ctx(), action.id, state);
                            }
                        
                        if (action.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || action.lost_focus() {
                            if *title_text != *real_title {
                                if title_text.trim().is_empty() {
                                    *title_text = real_title.clone();
                                } else {
                                    *real_title = title_text.clone();
                                    *pending_title = Some(real_title.clone());
                                }
                            }
                        } else if !action.has_focus() {
                            *title_text = real_title.clone();
                        }

                        ui.add_space(GAP_XS);

                        (exit_clicked, min_clicked, max_clicked)
                    };

                    let mut frame = egui::Frame::side_top_panel(&ui.ctx().global_style());
                    frame.shadow = egui::epaint::Shadow::NONE;
                    frame.corner_radius = egui::CornerRadius::ZERO;

                    let panel_margin = if matches!(self.theme, Theme::Rect) { GAP_SM as i8 } else { 0 };

                    let mut left_frame = frame;
                    left_frame.stroke = egui::Stroke::NONE;
                    left_frame.inner_margin = egui::Margin::same(panel_margin);

                    let mut right_frame = frame;
                    right_frame.stroke = egui::Stroke::NONE;
                    right_frame.inner_margin = egui::Margin::same(panel_margin);

                    // SidePanel fills the height of the window; since LogicalSize results in
                    // no pillarboxing, this matches the sim grid height.

                    // --- Left panel: Space Strategy ---
                    let left_panel_res = egui::Panel::left("wight_strategy").show_separator_line(false)
                        .resizable(false)
                        .exact_size(260.0)
                        .frame(left_frame)
                        .show_inside(ui, |ui| {
                            match self.theme {
                                Theme::Dew => crate::ui::dew::draw_pinstripes(ui.painter(), ui.max_rect()),
                                Theme::Future => crate::ui::future::draw_scan_lines(ui.painter(), ui.max_rect()),
                                _ => {}
                            }
                            ui.style_mut().spacing.item_spacing.y = 0.0;
                            include!("space_strategy.rs");

                            ui.add_space(GAP_SM);
                            ui.style_mut().spacing.item_spacing.y = 0.0;
                            include!("color_river.rs");

                            ui.add_space(GAP_SM);
                            ui.style_mut().spacing.item_spacing.y = 0.0;
                            include!("system_metrics.rs");

                            // Pin theme selector to the very bottom of the left panel
                            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                                ui.add_space(GAP_SM);
                                let mut ds = self.theme;
                                let btn_text = match ds {
                                    Theme::Rect   => "Rect   ^",
                                    Theme::Dew    => "Dew    ^",
                                    Theme::Future => "Future ^",
                                };
                                let resp = crate::ui::widgets::button_w(self.theme.provider(), ui, btn_text, 0.0);
                                let popup_id = ui.make_persistent_id("theme_popup");
                                if matches!(self.theme, Theme::Rect) {
                                    if resp.clicked() {
                                        egui::Popup::toggle_id(ui.ctx(), popup_id);
                                    }
                                    let is_open = egui::Popup::is_id_open(ui.ctx(), popup_id);
                                    if is_open {
                                        let term_bg    = egui::Color32::BLACK;
                                        let term_green = egui::Color32::from_rgb(0, 230, 65);
                                        let z = egui::CornerRadius::ZERO;
                                        let area_resp = egui::Area::new(popup_id)
                                            .order(egui::Order::Foreground)
                                            .kind(egui::UiKind::Popup)
                                            .fixed_pos(resp.rect.left_top())
                                            .pivot(egui::Align2::LEFT_BOTTOM)
                                            .constrain_to(ui.ctx().content_rect())
                                            .show(ui.ctx(), |ui| {
                                                egui::Frame::NONE
                                                    .fill(term_bg)
                                                    .stroke(egui::Stroke::new(1.0, term_green))
                                                    .inner_margin(egui::Margin::same(6))
                                                    .show(ui, |ui| {
                                                        ui.visuals_mut().override_text_color = Some(term_green);
                                                        // Keep bg_stroke=NONE on all states so the frame budget
                                                        // (inner_margin = button_padding + expansion - stroke_width)
                                                        // is never disturbed and items never shift position.
                                                        // The green border is painted separately via the Painter.
                                                        ui.visuals_mut().widgets.inactive.bg_fill      = egui::Color32::TRANSPARENT;
                                                        ui.visuals_mut().widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                                                        ui.visuals_mut().widgets.inactive.bg_stroke    = egui::Stroke::NONE;
                                                        ui.visuals_mut().widgets.inactive.corner_radius = z;
                                                        ui.visuals_mut().widgets.hovered.bg_fill       = term_bg;
                                                        ui.visuals_mut().widgets.hovered.weak_bg_fill  = term_bg;
                                                        ui.visuals_mut().widgets.hovered.bg_stroke     = egui::Stroke::NONE;
                                                        ui.visuals_mut().widgets.hovered.corner_radius  = z;
                                                        ui.visuals_mut().widgets.active.bg_fill        = term_bg;
                                                        ui.visuals_mut().widgets.active.weak_bg_fill   = term_bg;
                                                        ui.visuals_mut().widgets.active.bg_stroke      = egui::Stroke::NONE;
                                                        ui.visuals_mut().widgets.active.corner_radius   = z;
                                                        ui.visuals_mut().selection.bg_fill = term_bg;
                                                        ui.visuals_mut().selection.stroke  = egui::Stroke::new(1.0, term_green);
                                                        ui.set_min_width(resp.rect.width());
                                                        let border = egui::Stroke::new(1.0, term_green);
                                                        let mut add_btn = |ui: &mut egui::Ui, t: Theme, label: &str| {
                                                            let text = egui::RichText::new(label).monospace().size(13.0);
                                                            let is_selected = ds == t;
                                                            let btn_resp = ui.selectable_value(&mut ds, t, text);
                                                            if btn_resp.hovered() != is_selected {
                                                                ui.painter().rect_stroke(btn_resp.rect, z, border, egui::StrokeKind::Outside);
                                                            }
                                                            if btn_resp.clicked() {
                                                                egui::Popup::close_id(ui.ctx(), popup_id);
                                                            }
                                                        };
                                                        add_btn(ui, Theme::Rect, "Rect    ");
                                                        add_btn(ui, Theme::Dew, "Dew     ");
                                                        add_btn(ui, Theme::Future, "Future  ");
                                                    });
                                            });
                                        let close = ui.input(|i| i.pointer.any_pressed())
                                            && ui.input(|i| i.pointer.interact_pos())
                                                .is_some_and(|p| !area_resp.response.rect.contains(p));
                                        if close {
                                            egui::Popup::close_id(ui.ctx(), popup_id);
                                        } else {
                                            egui::Popup::open_id(ui.ctx(), popup_id);
                                        }
                                    }
                                } else {
                                    // Make the entire popup use smaller margins/padding
                                    let prev_style = (*ui.ctx().global_style()).clone();
                                    let mut popup_style = prev_style.clone();
                                    popup_style.spacing.window_margin = egui::Margin::same(4);
                                    popup_style.spacing.button_padding = egui::vec2(6.0, 4.0);
                                    popup_style.spacing.item_spacing = egui::vec2(4.0, 2.0);
                                    ui.ctx().set_global_style(popup_style);

                                    egui::Popup::menu(&resp).id(popup_id).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui: &mut egui::Ui| {
                                        ui.visuals_mut().widgets.hovered.expansion = 0.0;
                                        ui.visuals_mut().widgets.active.expansion  = 0.0;
                                        ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::NONE;
                                        ui.visuals_mut().widgets.hovered.bg_stroke = egui::Stroke::NONE;
                                        ui.visuals_mut().widgets.active.bg_stroke  = egui::Stroke::NONE;
                                        
                                        // Sync hover highlights with text selection colors across themes
                                        if matches!(self.theme, Theme::Dew) {
                                            ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::from_rgb(180, 210, 255);
                                            ui.visuals_mut().widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(180, 210, 255);
                                        } else if matches!(self.theme, Theme::Future) {
                                            ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::from_rgb(130, 148, 192); // FUTURE_GLOW matches the button reflection
                                            ui.visuals_mut().widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(130, 148, 192);
                                            // Make text dark when hovering over the bright accent
                                            ui.visuals_mut().widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::BLACK);
                                        }

                                        ui.set_min_width(resp.rect.width());
                                        let mut add_btn = |ui: &mut egui::Ui, t: Theme, label: &str| {
                                            let text = egui::RichText::new(label).monospace().size(13.0);
                                            if ui.selectable_value(&mut ds, t, text).clicked() {
                                                egui::Popup::close_id(ui.ctx(), popup_id);
                                            }
                                        };
                                        add_btn(ui, Theme::Rect, "Rect    ");
                                        add_btn(ui, Theme::Dew, "Dew     ");
                                        add_btn(ui, Theme::Future, "Future  ");
                                    });
                                    
                                    ui.ctx().set_global_style(prev_style);
                                }
                                if ds != self.theme {
                                    self.theme = ds;
                                    self.theme.provider().apply_theme(ui.ctx());
                                }
                            });

                        });
                        
                    let right_panel_res = egui::Panel::right("wight_control").show_separator_line(false)
                        .resizable(false)
                        .exact_size(260.0)
                        .frame(right_frame)
                        .show_inside(ui, |ui| {
                            match self.theme {
                                Theme::Dew => crate::ui::dew::draw_pinstripes(ui.painter(), ui.max_rect()),
                                Theme::Future => crate::ui::future::draw_scan_lines(ui.painter(), ui.max_rect()),
                                _ => {}
                            }

                            match self.theme {
                                Theme::Rect => {
                                    let (c_exit, c_min, c_max) = term_title_bar(ui, &mut self.title_text, &mut self.title, &mut pending_title, None, None);
                                    if c_exit { exit_req = true; }
                                    if c_min { minimize_req = true; }
                                    if c_max { fullscreen_req = true; }
                                }
                                Theme::Dew | Theme::Future => {
                                    let height = 26.0;
                                    let (rect, _resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::hover());
                                    let left = rect.min.x;
                                    let right = rect.max.x;

                                    if matches!(self.theme, Theme::Dew) {
                                        // 7 lines for square OS X pinstripe titlebar layout
                                        for i in 0..7 {
                                            let y = rect.min.y + 2.0 + i as f32 * 3.5;
                                            ui.painter().line_segment([egui::pos2(left, y), egui::pos2(right, y)], egui::Stroke::new(1.0, egui::Color32::from_rgb(205, 210, 218)));
                                            ui.painter().line_segment([egui::pos2(left, y+1.0), egui::pos2(right, y+1.0)], egui::Stroke::new(1.0, egui::Color32::from_rgb(250, 255, 255)));
                                        }
                                    }
                                    
                                    let r = 6.0;
                                    let cy = rect.center().y;
                                    
                                    // macOS behaviour: hover ANY of the 3 buttons, and ALL 3 show their icons.
                                    let group_rect = egui::Rect::from_min_max(
                                        egui::pos2(right - 50.0 - r - 2.0, cy - r - 2.0),
                                        egui::pos2(right - 14.0 + r + 2.0, cy + r + 2.0),
                                    );
                                    let group_hovered = ui.rect_contains_pointer(group_rect);
                                    let hover_t = ui.ctx().animate_value_with_time(
                                        egui::Id::new("tb_group_hover"),
                                        if group_hovered { 1.0 } else { 0.0 },
                                        0.1,
                                    );
                                    
                                    let btn_color = if matches!(self.theme, Theme::Future) {
                                        egui::Color32::from_rgb(88, 94, 112) // FUTURE_BODY — match future big buttons
                                    } else {
                                        egui::Color32::from_rgb(50, 130, 240) // GEL_BODY — match dew big buttons
                                    };

                                    let draw_anim_gumdrop = |ui: &mut egui::Ui, id: &str, cx: f32, base_color: egui::Color32, symbol: &str| -> bool {
                                        let center = egui::pos2(right - cx, cy);
                                        let btn_size = egui::vec2(r * 2.0 + 2.0, r * 2.0 + 2.0);
                                        let resp = ui.interact(egui::Rect::from_center_size(center, btn_size), egui::Id::new(id), egui::Sense::click());
                                        if matches!(self.theme, Theme::Future) {
                                            crate::ui::future::draw_mac_traffic_light(ui, &resp, r, base_color, symbol, Some(hover_t));
                                        } else {
                                            crate::ui::dew::draw_mac_traffic_light(ui, &resp, r, base_color, symbol, Some(hover_t));
                                        }
                                        resp.clicked()
                                    };
                                    
                                    if draw_anim_gumdrop(ui, "tb_red", 50.0, btn_color, "*") { exit_req = true; }
                                    if draw_anim_gumdrop(ui, "tb_yellow", 32.0, btn_color, ".") { minimize_req = true; }
                                    if draw_anim_gumdrop(ui, "tb_green", 14.0, btn_color, "~") { fullscreen_req = true; }
                                    
                                    let title_color = if matches!(self.theme, Theme::Future) {
                                        egui::Color32::WHITE
                                    } else {
                                        egui::Color32::from_rgb(30, 30, 30)
                                    };
                                    
                                    let font_id = egui::FontId::proportional(14.0);
                                    let galley = ui.painter().layout_no_wrap(self.title_text.clone(), font_id.clone(), title_color);
                                    let text_pos = egui::pos2(left + 8.0, cy - galley.size().y / 2.0);
                                    
                                    let text_rect = egui::Rect::from_min_size(text_pos, galley.size());
                                    
                                    if matches!(self.theme, Theme::Future) {
                                        ui.painter().rect_filled(
                                            text_rect.expand2(egui::vec2(4.0, -1.0)),
                                            2.0,
                                            egui::Color32::BLACK,
                                        );
                                    }
                                    
                                    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(text_rect).layout(egui::Layout::left_to_right(egui::Align::TOP)));
                                    child_ui.visuals_mut().extreme_bg_color = egui::Color32::TRANSPARENT;
                                    child_ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                                    child_ui.visuals_mut().widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                                    
                                    let edit = egui::TextEdit::singleline(&mut self.title_text)
                                        .frame(egui::Frame::NONE)
                                        .font(font_id)
                                        .text_color(title_color)
                                        .margin(egui::vec2(0.0, 0.0))
                                        .desired_width(150.0);
                                        
                                    let action = child_ui.add(edit);
                                    
                                    if action.gained_focus()
                                        && let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), action.id) {
                                            state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                                                egui::text::CCursor::new(0),
                                                egui::text::CCursor::new(self.title_text.chars().count()),
                                            )));
                                            egui::TextEdit::store_state(ui.ctx(), action.id, state);
                                        }
                                    
                                    if (action.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || action.lost_focus() {
                                        if self.title_text != self.title {
                                            if self.title_text.trim().is_empty() {
                                                self.title_text = self.title.clone();
                                            } else {
                                                self.title = self.title_text.clone();
                                                pending_title = Some(self.title.clone());
                                            }
                                        }
                                    } else if !action.has_focus() {
                                        self.title_text = self.title.clone();
                                    }
                                    
                                }
                            }

                            egui::ScrollArea::vertical().scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
                                ui.style_mut().spacing.item_spacing.y = 0.0;
                                // Performance stats row — Dew inset pill / Editable seed
                                {
                                    let noise_str = format!("noise:{:.2}", self.background_noise);
                                    let full_text = format!("{}  ·  {}  ·  {:.0}fps", self.seed_text, noise_str, self.fps);
                                    let stat_color = if matches!(self.theme, Theme::Rect) {
                                        egui::Color32::from_rgb(0, 210, 60)
                                    } else if matches!(self.theme, Theme::Future) {
                                        egui::Color32::WHITE
                                    } else {
                                        egui::Color32::from_rgb(120, 120, 130)
                                    };

                                    let galley = ui.painter().layout_no_wrap(
                                        full_text,
                                        egui::FontId::monospace(11.0),
                                        stat_color,
                                    );
                                    let padding = egui::vec2(8.0, 4.0);
                                    let size = egui::vec2(ui.available_width(), galley.size().y + padding.y * 2.0);
                                    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                                    
                                    if ui.is_rect_visible(rect) {
                                        let p = ui.painter();
                                        if matches!(self.theme, Theme::Rect) {
                                            p.rect_filled(rect, egui::CornerRadius::ZERO, egui::Color32::BLACK);
                                            p.rect_stroke(rect, egui::CornerRadius::ZERO, egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 230, 65)), egui::StrokeKind::Outside);
                                        } else {
                                            let bg = if matches!(self.theme, Theme::Future) { egui::Color32::BLACK } else { egui::Color32::from_rgba_premultiplied(0, 0, 0, 18) };
                                            p.rect_filled(rect, rect.height() / 2.0, bg);
                                            crate::ui::dew::draw_inset(p, rect);
                                        }
                                        
                                        let inner_rect = egui::Rect::from_center_size(rect.center(), galley.size());
                                        let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(inner_rect).layout(*ui.layout()));
                                        child_ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |child_ui| {
                                            child_ui.spacing_mut().item_spacing.x = 0.0;
                                            
                                            let font_id = egui::FontId::monospace(11.0);
                                            let seed_w = child_ui.painter().layout_no_wrap(self.seed_text.clone(), font_id.clone(), stat_color).size().x;
                                            
                                            // Invisible editable text
                                            child_ui.visuals_mut().extreme_bg_color = egui::Color32::TRANSPARENT;
                                            child_ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                                            child_ui.visuals_mut().widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                                            
                                            let edit = egui::TextEdit::singleline(&mut self.seed_text)
                                                .frame(egui::Frame::NONE)
                                                .font(font_id.clone())
                                                .text_color(stat_color)
                                                .desired_width(seed_w.max(5.0)); // Prevent total collapse of width
                                                
                                            let seed_action = child_ui.add(edit);
                                            
                                            child_ui.label(egui::RichText::new(format!("  ·  {}  ·  {:.0}fps", noise_str, self.fps))
                                                .font(font_id)
                                                .color(stat_color));
                                                
                                            if seed_action.gained_focus()
                                                && let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), seed_action.id) {
                                                    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                                                        egui::text::CCursor::new(0),
                                                        egui::text::CCursor::new(self.seed_text.chars().count()),
                                                    )));
                                                    egui::TextEdit::store_state(ui.ctx(), seed_action.id, state);
                                                }
                                            
                                            if (seed_action.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || seed_action.lost_focus() {
                                                if self.seed_text != self.seed {
                                                    if self.seed_text.trim().is_empty() {
                                                        self.seed_text = self.seed.clone(); // Revert if blanked
                                                    } else {
                                                        self.seed = self.seed_text.clone();
                                                        pending_reset = Some(false);
                                                    }
                                                }
                                            } else if !seed_action.has_focus() {
                                                self.seed_text = self.seed.clone();
                                            }
                                        });
                                    }
                                }
                                ui.add_space(GAP_MD);

                                // Transport row: 3 equal-width buttons filling the panel
                                {
                                    let avail = ui.available_width();
                                    let btn_gap = 4.0_f32;
                                    let btn_w = (avail - 2.0 * btn_gap) / 3.0;
                                    ui.horizontal(|ui| {
                                        ui.style_mut().spacing.item_spacing.x = btn_gap;
                                        if crate::ui::widgets::button_w(self.theme.provider(), ui, "<< R", btn_w).clicked() { rewind_req = true; }
                                        if crate::ui::widgets::button_w(self.theme.provider(), ui, "⟳ C", btn_w).clicked() { reroll_req = true; }
                                        let label = if self.is_paused { "▶ Space" } else { "⏸ Space" };
                                        if crate::ui::widgets::button_w(self.theme.provider(), ui, label, btn_w).clicked() { pause_req = true; }
                                    });
                                    ui.add_space(GAP_SM);
                                    ui.style_mut().spacing.item_spacing.y = 0.0;
                                    if crate::ui::widgets::button_w(self.theme.provider(), ui, "⛶ Screenshot", avail).clicked() {
                                        self.take_screenshot = true;
                                    }
                                    ui.add_space(GAP_MD);
                                }

                                // Speed slider: left = reverse, center = 0, right = forward
                                let speed_resp = ui.vertical(|ui| {
                                    ui.style_mut().spacing.item_spacing.y = GAP_XS;
                                    let lbl_galley = ui.painter().layout_no_wrap("TIME TRAVEL".to_string(), egui::FontId::monospace(8.0), egui::Color32::BLACK);
                                    let label_h = lbl_galley.size().y;
                                    let btn_side = KEY_CAP_SIDE;

                                    let field_h = btn_side + GAP_SM + label_h;
                                    ui.horizontal(|ui| {
                                        ui.style_mut().spacing.item_spacing.x = GAP_SM;
                                        {
                                            ui.vertical(|ui| {
                                                ui.style_mut().spacing.item_spacing.y = GAP_SM;
                                                let label_top_y = ui.cursor().min.y;
                                                ui.add_space(label_h);
                                                let btn_row = ui.horizontal(|ui| {
                                                    ui.style_mut().spacing.item_spacing.x = GAP_SM;
                                                    let r = self.theme.provider().key_cap_small(ui, "↓", btn_side, 26.0);
                                                    if r.clicked() { arrow_down_req = true; }
                                                    let r = self.theme.provider().key_cap_small(ui, "↑", btn_side, 26.0);
                                                    if r.clicked() { arrow_up_req = true; }
                                                });
                                                let center_x = btn_row.response.rect.center().x;
                                                let color = if matches!(self.theme, Theme::Rect) { egui::Color32::from_rgb(0, 230, 65) } else if matches!(self.theme, Theme::Future) { egui::Color32::WHITE } else { egui::Color32::from_rgb(100, 100, 110) };
                                                let font = egui::FontId::monospace(8.0);
                                                let text = "SLOW  FAST";
                                                let galley = ui.painter().layout_no_wrap(text.to_string(), font, color);
                                                let text_rect = egui::Align2::CENTER_TOP.anchor_size(egui::pos2(center_x, label_top_y), galley.size());
                                                
                                                if matches!(self.theme, Theme::Future) {
                                                    ui.painter().rect_filled(
                                                        text_rect.expand2(egui::vec2(2.0, -1.0)),
                                                        2.0,
                                                        egui::Color32::BLACK,
                                                    );
                                                }
                                                ui.painter().galley(text_rect.min, galley, color);
                                            });
                                        }
                                        let speed_action = self.theme.provider().text_field_edit(ui, &mut self.speed_text, 16.0, field_h);
                                        if speed_action.gained_focus()
                                            && let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), speed_action.id) {
                                                state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                                                    egui::text::CCursor::new(0),
                                                    egui::text::CCursor::new(self.speed_text.chars().count()),
                                                )));
                                                egui::TextEdit::store_state(ui.ctx(), speed_action.id, state);
                                            }
                                        if speed_action.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) || speed_action.lost_focus() {
                                            if let Ok(v) = self.speed_text.trim().trim_end_matches("T/s").trim().parse::<f64>() {
                                                self.custom_speed = v.clamp(-1e12, 1e12);
                                                self.wave_speed = self.custom_speed as f32;
                                            }
                                            let av2 = self.custom_speed.abs();
                                            let sign2 = if self.custom_speed < 0.0 { "-" } else { "" };
                                            self.speed_text = if av2 < 0.01 { "0.00 T/s".to_string() }
                                                else if av2 < 10.0 { format!("{}{:.2} T/s", sign2, av2) }
                                                else { format!("{}{:.0} T/s", sign2, av2) };
                                        } else if !speed_action.has_focus() {
                                            let av = self.custom_speed.abs();
                                            let sign = if self.custom_speed < 0.0 { "-" } else { "" };
                                            self.speed_text = if av < 0.01 { "0.00 T/s".to_string() }
                                                else if av < 10.0 { format!("{}{:.2} T/s", sign, av) }
                                                else { format!("{}{:.0} T/s", sign, av) };
                                        }
                                    });
                                    ui.add_space(GAP_XS);
                                    let r = crate::ui::widgets::slider_symlog_f64(self.theme.provider(), 
                                        ui,
                                        &mut self.custom_speed,
                                        1e12f64,
                                        "",
                                        |v| {
                                            let av = v.abs();
                                            let sign = if v < 0.0 { "-" } else { "" };
                                            if av < 0.01 { "0.00 T/s".to_string() }
                                            else if av < 10.0 { format!("{}{:.2} T/s", sign, av) }
                                            else { format!("{}{:.0} T/s", sign, av) }
                                        }
                                    );
                                    let av = self.custom_speed.abs();
                                    let sign = if self.custom_speed < 0.0 { "-" } else { "" };
                                    let val_str = if av < 0.01 { "0.00 T/s".to_string() }
                                        else if av < 10.0 { format!("{}{:.2} T/s", sign, av) }
                                        else { format!("{}{:.0} T/s", sign, av) };
                                    if r.changed() { self.speed_text = val_str; }
                                    r
                                }).inner;
                                if speed_resp.changed() {
                                    self.wave_speed = self.custom_speed as f32;
                                }

                                ui.add_space(GAP_MD);
                                // Time slider: center = T=0, left = past, right = future
                                // Display value is full T = epoch*P + residual, max ±i64::MAX×PERIOD
                                const PERIOD_SL: f64 = std::f64::consts::TAU / 0.1;
                                let t_display_max = (i64::MAX as f64) * PERIOD_SL;
                                let mut t_display = (self.t_epoch as f64 * PERIOD_SL + self.t_residual).clamp(-t_display_max, t_display_max);
                                let time_resp = ui.vertical(|ui| {
                                    ui.style_mut().spacing.item_spacing.y = GAP_XS;
                                    let lbl_galley = ui.painter().layout_no_wrap("TIME TRAVEL".to_string(), egui::FontId::monospace(8.0), egui::Color32::BLACK);
                                    let label_h = lbl_galley.size().y;
                                    let btn_side = KEY_CAP_SIDE;
                                    
                                    let field_h = btn_side + GAP_SM + label_h;
                                    let av = t_display.abs();
                                    let sign = if t_display < 0.0 { "-" } else { "" };
                                    ui.horizontal(|ui| {
                                        ui.style_mut().spacing.item_spacing.x = GAP_SM;
                                        {
                                            ui.vertical(|ui| {
                                                ui.style_mut().spacing.item_spacing.y = GAP_SM;
                                                let label_top_y = ui.cursor().min.y;
                                                ui.add_space(label_h);
                                                let btn_row = ui.horizontal(|ui| {
                                                    ui.style_mut().spacing.item_spacing.x = GAP_SM;
                                                    let r = self.theme.provider().key_cap_small_rotated(ui, "↑", -std::f32::consts::FRAC_PI_2, btn_side, 26.0);
                                                    if r.clicked() { arrow_left_req = true; }
                                                    let r = self.theme.provider().key_cap_small_rotated(ui, "↑", std::f32::consts::FRAC_PI_2, btn_side, 26.0);
                                                    if r.clicked() { arrow_right_req = true; }
                                                });
                                                let center_x = btn_row.response.rect.center().x;
                                                let color = if matches!(self.theme, Theme::Rect) { egui::Color32::from_rgb(0, 230, 65) } else if matches!(self.theme, Theme::Future) { egui::Color32::WHITE } else { egui::Color32::from_rgb(100, 100, 110) };
                                                let font = egui::FontId::monospace(8.0);
                                                let text = "TIME TRAVEL";
                                                let galley = ui.painter().layout_no_wrap(text.to_string(), font, color);
                                                let text_rect = egui::Align2::CENTER_TOP.anchor_size(egui::pos2(center_x, label_top_y), galley.size());
                                                
                                                if matches!(self.theme, Theme::Future) {
                                                    ui.painter().rect_filled(
                                                        text_rect.expand2(egui::vec2(2.0, -1.0)),
                                                        2.0,
                                                        egui::Color32::BLACK,
                                                    );
                                                }
                                                ui.painter().galley(text_rect.min, galley, color);
                                            });
                                        }
                                        let time_action = self.theme.provider().text_field_edit(ui, &mut self.time_text, 16.0, field_h);
                                        if time_action.gained_focus()
                                            && let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), time_action.id) {
                                                state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                                                    egui::text::CCursor::new(0),
                                                    egui::text::CCursor::new(self.time_text.chars().count()),
                                                )));
                                                egui::TextEdit::store_state(ui.ctx(), time_action.id, state);
                                            }
                                        if time_action.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) || time_action.lost_focus() {
                                            if let Ok(v) = self.time_text.trim().trim_end_matches('T').trim().parse::<f64>() {
                                                let new_t = v.clamp(-t_display_max, t_display_max);
                                                let new_residual = new_t.rem_euclid(PERIOD_SL);
                                                let new_epoch = ((new_t - new_residual) / PERIOD_SL).round() as i64;
                                                self.t_epoch = new_epoch;
                                                self.t_residual = new_residual;
                                                self.is_paused = true;
                                                let _ = self.sim_handle.cmd_tx.send(Command::Pause);
                                                let av2 = new_t.abs();
                                                let sign2 = if new_t < 0.0 { "-" } else { "" };
                                                self.time_text = if av2 < 0.01 { "0.0 T".to_string() }
                                                    else if av2 < 1e5 { format!("{}{:.1} T", sign2, av2) }
                                                    else { let e = av2.log10().floor() as i32; format!("{}{:.2}e{} T", sign2, av2 / 10f64.powi(e), e) };
                                            } else {
                                                self.time_text = if av < 0.01 { "0.0 T".to_string() }
                                                    else if av < 1e5 { format!("{}{:.1} T", sign, av) }
                                                    else { let e = av.log10().floor() as i32; format!("{}{:.2}e{} T", sign, av / 10f64.powi(e), e) };
                                            }
                                        } else if !time_action.has_focus() {
                                            self.time_text = if av < 0.01 { "0.0 T".to_string() }
                                                else if av < 1e5 { format!("{}{:.1} T", sign, av) }
                                                else { let e = av.log10().floor() as i32; format!("{}{:.2}e{} T", sign, av / 10f64.powi(e), e) };
                                        }
                                    });
                                    ui.add_space(GAP_XS);
                                    let r = crate::ui::widgets::slider_symlog_f64(self.theme.provider(), 
                                        ui,
                                        &mut t_display,
                                        t_display_max,
                                        "",
                                        |v| {
                                            let av = v.abs();
                                            let sign = if v < 0.0 { "-" } else { "" };
                                            if av < 0.01 { "0.0 T".to_string() }
                                            else if av < 1e5 { format!("{}{:.1} T", sign, av) }
                                            else {
                                                let e = av.log10().floor() as i32;
                                                format!("{}{:.2}e{} T", sign, av / 10f64.powi(e), e)
                                            }
                                        }
                                    );
                                    let val_str = if av < 0.01 { "0.0 T".to_string() }
                                        else if av < 1e5 { format!("{}{:.1} T", sign, av) }
                                        else { let e = av.log10().floor() as i32; format!("{}{:.2}e{} T", sign, av / 10f64.powi(e), e) };
                                    if r.changed() { self.time_text = val_str; }
                                    r
                                }).inner;
                                
                                if time_resp.drag_started() {
                                    let was_playing = !self.is_paused;
                                    ui.memory_mut(|mem| mem.data.insert_temp(time_resp.id.with("was_playing"), was_playing));
                                }

                                if time_resp.changed() {
                                    let new_residual = t_display.rem_euclid(PERIOD_SL);
                                    let new_epoch = ((t_display - new_residual) / PERIOD_SL).round() as i64;
                                    self.t_epoch = new_epoch;
                                    self.t_residual = new_residual;
                                    self.is_paused = true;
                                    
                                    // Recompute COLOR RIVER as 240 evenly-spaced T samples ending at current T.
                                    // Always in sync: rewind, jump, reverse — the river instantly shows that epoch.
                                    const PERIOD: f64 = std::f64::consts::TAU / 0.1;
                                    let t_now = self.t_epoch as f64 * PERIOD + self.t_residual;
                                    let window = (10.0 * std::f64::consts::TAU / 0.005)
                                        .max(self.wave_speed.abs() as f64 * 4.0);
                                    const SAMPLES: usize = 240;
                                    self.history.clear();
                                    for i in 0..SAMPLES {
                                        let frac = i as f64 / (SAMPLES - 1) as f64;
                                        let t_sample = t_now - window * (1.0 - frac);
                                        let prom = wave_prominence_at(&self.env_data, t_sample, self.background_noise);
                                        // Compute the correct colors at this historical t_sample.
                                        let colors: Vec<egui::Color32> = (0..3).map(|w| {
                                            let gn = crate::ui::ascii_render::get_gn_at_time(&self.env_data, w, t_sample, self.background_noise as f64);
                                            let p = crate::ui::ascii_render::get_params(&self.env_data, w, gn);
                                            crate::ui::espresso_walk::params_to_color(self.wave_lch[w], p, self.wave_params0[w])
                                        }).collect();
                                        self.history.push_back((prom.iter().map(|&f| (f * 1000.0) as u32).collect(), colors));
                                    }

                                    ui.ctx().request_repaint();
                                    let _ = self.sim_handle.cmd_tx.send(Command::Pause);
                                }

                                if time_resp.drag_stopped() {
                                    let was_playing = ui.memory_mut(|mem| mem.data.get_temp(time_resp.id.with("was_playing")).unwrap_or(false));
                                    if was_playing {
                                        self.is_paused = false;
                                        let _ = self.sim_handle.cmd_tx.send(Command::Resume);
                                    }
                                }
                                
                            ui.add_space(GAP_MD);
                            ui.style_mut().spacing.item_spacing.y = 0.0;
                            include!("acoustic_scanner.rs");
                                
                            ui.add_space(GAP_MD);
                            ui.style_mut().spacing.item_spacing.y = 0.0;
                            include!("superposition.rs");
                                ui.add_space(GAP_SM);
                            }); // scroll area
                        }); // side panel
                        
                    if matches!(self.theme, Theme::Rect) {
                        let left_x = left_panel_res.response.rect.right();
                        let right_x = right_panel_res.response.rect.left();
                        let top_y = 0.0;
                        let bottom_y = ui.ctx().input(|i| i.content_rect()).height();
                        
                        let sim_rect = egui::Rect::from_min_max(
                            egui::pos2(left_x, top_y),
                            egui::pos2(right_x, bottom_y),
                        );
                        
                        ui.ctx().layer_painter(egui::LayerId::background()).rect_stroke(
                            sim_rect,
                            egui::CornerRadius::ZERO,
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 230, 65)),
                            egui::StrokeKind::Outside,
                        );
                    }
                });

                if exit_req { event_loop.exit(); }
                if minimize_req { state.window.set_minimized(true); }
                if rewind_req {                    self.t_epoch = 0;
                    self.t_residual = 0.0;
                    self.wave_speed = 1.0;
                    self.custom_speed = 1.0;
                    self.is_paused = false;
                    let _ = self.sim_handle.cmd_tx.send(Command::Resume);
                    pending_reset = Some(false);
                    state.window.request_redraw();
                }
                if reroll_req {
                    self.t_epoch = 0;
                    self.t_residual = 0.0;
                    self.wave_speed = 1.0;
                    self.custom_speed = 1.0;
                    self.is_paused = false;
                    let _ = self.sim_handle.cmd_tx.send(Command::Resume);
                    pending_reset = Some(true);
                    state.window.request_redraw();
                }
                if pause_req {
                    self.is_paused = !self.is_paused;
                    if self.is_paused {
                        let _ = self.sim_handle.cmd_tx.send(Command::Pause);
                    } else {
                        let _ = self.sim_handle.cmd_tx.send(Command::Resume);
                    }
                    state.window.request_redraw();
                }
                if let Some(s) = speed_req {
                    self.speed = s;
                    self.wave_speed = match s {
                        0 => 0.25,
                        1 => 1.0,
                        2 => 10.0,
                        3 => 100.0,
                        4 => 1_000.0,
                        _ => 1_000_000.0,
                    };
                    self.custom_speed = self.wave_speed as f64; // keep slider in sync
                    let _ = self
                        .sim_handle
                        .cmd_tx
                        .send(Command::SetSpeed(speed_duration(s)));
                }
                if arrow_up_req || arrow_down_req {
                    const LADDER: &[f64] = &[
                        -1e12, -1e11, -1e10, -1e9, -1e8, -1e7,
                        -1_000_000.0, -1_000.0, -100.0, -10.0, -1.0, -0.25,
                        0.0,
                        0.25, 1.0, 10.0, 100.0, 1_000.0, 1_000_000.0,
                        1e7, 1e8, 1e9, 1e10, 1e11, 1e12,
                    ];
                    let cur = self.wave_speed as f64;
                    let idx = LADDER.iter().enumerate()
                        .min_by(|(_, a), (_, b)| ((**a - cur).abs()).partial_cmp(&((**b - cur).abs())).unwrap())
                        .map(|(i, _)| i).unwrap_or(7);
                    let new_idx = if arrow_up_req { (idx + 1).min(LADDER.len() - 1) } else { idx.saturating_sub(1) };
                    let new_speed = LADDER[new_idx];
                    if new_speed == 0.0 {
                        self.is_paused = true;
                        let _ = self.sim_handle.cmd_tx.send(Command::Pause);
                    } else {
                        self.is_paused = false;
                        let _ = self.sim_handle.cmd_tx.send(Command::Resume);
                    }
                    self.wave_speed = new_speed as f32;
                    self.custom_speed = new_speed;
                    let av = new_speed.abs();
                    let sign = if new_speed < 0.0 { "-" } else { "" };
                    self.speed_text = if av < 0.01 { "0.00 T/s".to_string() }
                        else if av < 10.0 { format!("{}{:.2} T/s", sign, av) }
                        else { format!("{}{:.0} T/s", sign, av) };
                    state.window.request_redraw();
                }
                if arrow_left_req || arrow_right_req {
                    const FREQ_MIN: f64 = 0.1;
                    const PERIOD: f64 = std::f64::consts::TAU / FREQ_MIN;
                    let current_t = self.t_epoch as f64 * PERIOD + self.t_residual;
                    let jump = if current_t.abs() < 1.0 {
                        PERIOD
                    } else {
                        let magnitude = 10f64.powi(current_t.abs().log10().floor() as i32);
                        (current_t.abs() / magnitude).floor() * magnitude
                    };
                    if arrow_left_req { self.t_residual -= jump; } else { self.t_residual += jump; }
                    if self.t_residual >= PERIOD {
                        let extra = (self.t_residual / PERIOD).floor() as i64;
                        self.t_epoch = self.t_epoch.saturating_add(extra);
                        self.t_residual -= extra as f64 * PERIOD;
                    } else if self.t_residual < 0.0 {
                        let borrow = (-self.t_residual / PERIOD).ceil() as i64;
                        self.t_epoch = self.t_epoch.saturating_sub(borrow);
                        self.t_residual += borrow as f64 * PERIOD;
                    }
                    self.is_paused = true;
                    let _ = self.sim_handle.cmd_tx.send(Command::Pause);
                    let new_t = self.t_epoch as f64 * PERIOD + self.t_residual;
                    let av_t = new_t.abs();
                    let sign_t = if new_t < 0.0 { "-" } else { "" };
                    self.time_text = if av_t < 0.01 { "0.0 T".to_string() }
                        else if av_t < 1e5 { format!("{}{:.1} T", sign_t, av_t) }
                        else { let e = av_t.log10().floor() as i32; format!("{}{:.2}e{} T", sign_t, av_t / 10f64.powi(e), e) };
                    state.window.request_redraw();
                }
                state
                    .egui_state
                    .handle_platform_output(&state.window, full_output.platform_output);
                let tris = state
                    .egui_ctx
                    .tessellate(full_output.shapes, state.window.scale_factor() as f32);
                for (id, image_delta) in &full_output.textures_delta.set {
                    state
                        .egui_renderer
                        .update_texture(&self.device, &self.queue, *id, image_delta);
                }

                let frame = match state.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
                    _ => return,
                };
                let view = frame.texture.create_view(&Default::default());

                let mut encoder = self.device.create_command_encoder(&Default::default());
                
                // FINAL ATTEMPT AT "ENGINE CAPABILITY FPS":
                // By starting the stopwatch *after* get_current_texture(), we completely 
                // bypass macOS/VSync swapchain waiting (which is what crashed the FPS 
                // to 30 when you moused around and filled the queue!). We now only 
                // measure CPU encoding time.
                let _render_start = std::time::Instant::now();

                let screen_descriptor = egui_wgpu::ScreenDescriptor {
                    size_in_pixels: [
                        state.window.inner_size().width,
                        state.window.inner_size().height,
                    ],
                    pixels_per_point: state.window.scale_factor() as f32,
                };
                state.egui_renderer.update_buffers(
                    &self.device,
                    &self.queue,
                    &mut encoder,
                    &tris,
                    &screen_descriptor,
                );

                // Wave time: advance this frame using epoch+residual split.
                // residual stays in [0, PERIOD) and overflows cleanly into epoch.
                const FREQ_MIN: f64 = 0.1;
                const PERIOD: f64 = std::f64::consts::TAU / FREQ_MIN; // ≈ 62.83
                if !self.is_paused {
                    let frame_dt = 1.0 / self.fps.max(1.0) as f64;
                    self.t_residual += frame_dt * self.wave_speed as f64;
                    // Normalise: residual must stay in [0, PERIOD)
                    if self.t_residual >= PERIOD {
                        let extra = (self.t_residual / PERIOD).floor() as i64;
                        self.t_epoch = self.t_epoch.saturating_add(extra);
                        self.t_residual -= extra as f64 * PERIOD;
                    } else if self.t_residual < 0.0 {
                        let borrow = (-self.t_residual / PERIOD).ceil() as i64;
                        self.t_epoch = self.t_epoch.saturating_sub(borrow);
                        self.t_residual += borrow as f64 * PERIOD;
                    }

                    // Advance COLOR RIVER: push one real prominence snapshot per frame,
                    // bundled with the current color palette so past columns stay accurate.
                    let t_now = self.t_epoch as f64 * PERIOD + self.t_residual;
                    let prominence = wave_prominence_at(&self.env_data, t_now, self.background_noise)
                        .iter().map(|&f| (f * 1000.0) as u32).collect::<Vec<_>>();
                    
                    if self.wave_speed >= 0.0 {
                        self.history.push_back((prominence, self.wave_colors.clone()));
                        if self.history.len() > 240 {
                            self.history.pop_front();
                        }
                    } else {
                        self.history.push_front((prominence, self.wave_colors.clone()));
                        if self.history.len() > 240 {
                            self.history.pop_back();
                        }
                    }
                }

                {
                    // DYAMIC VISUAL COLOR BINDING
                    // Every frame, we extract the entity's literal physics at the current Time T,
                    // and use their physical layout (Angle, Shape, Amp) to dictate their visual color.
                    let current_t = self.t_epoch as f64 * PERIOD + self.t_residual;
                    let mut audio_params = [0.0f32; 15];
                    for w in 0..3 {
                        let gn = crate::ui::ascii_render::get_gn_at_time(&self.env_data, w, current_t, self.background_noise as f64);
                        let params = crate::ui::ascii_render::get_params(&self.env_data, w, gn);
                        
                        // Audio Pipeline Injection
                        let base = w * 5;
                        audio_params[base] = params[0] as f32;     // amp
                        audio_params[base + 1] = params[1] as f32; // freq
                        audio_params[base + 2] = params[2] as f32; // angle
                        audio_params[base + 3] = params[3] as f32; // shape
                        audio_params[base + 4] = params[4] as f32; // warp

                        self.wave_colors[w] = crate::ui::espresso_walk::params_to_color(self.wave_lch[w], params, self.wave_params0[w]);
                    }
                    self.synth_engine.set_params(audio_params);

                    // Instantly sync the new Visual colors to the GPU Compute Shader so the universe
                    // crosses over identically on screen!
                    let wc = &self.wave_colors;
                    let wc_data: [f32; 12] = [
                        wc[0].r() as f32 / 255.0, wc[0].g() as f32 / 255.0, wc[0].b() as f32 / 255.0, 0.0,
                        wc[1].r() as f32 / 255.0, wc[1].g() as f32 / 255.0, wc[1].b() as f32 / 255.0, 0.0,
                        wc[2].r() as f32 / 255.0, wc[2].g() as f32 / 255.0, wc[2].b() as f32 / 255.0, 0.0,
                    ];
                    let color_buf = &state.color_buf;
                    self.queue.write_buffer(color_buf, 0, bytemuck::cast_slice(&wc_data));
                }


                // Egui rendering boilerplate:
                // tick = residual (precise f32 phase within period)
                // t_epoch = full periods elapsed (reconstructed in shader as t_epoch * PERIOD + tick)
                let t_wrapped = self.t_residual as f32;
                let t_epoch_f = self.t_epoch as f32;
                self.queue.write_buffer(
                    &state.tick_buf,
                    0,
                    bytemuck::cast_slice(&[
                        t_wrapped, 
                        self.background_noise, 
                        t_epoch_f, 
                        self.pan_x,
                        self.pan_y,
                        0.0f32,
                        0.0f32,
                        0.0f32
                    ]),
                );

                let w = screen_descriptor.size_in_pixels[0] as f32;
                let h = screen_descriptor.size_in_pixels[1] as f32;

                let padding_logical = if matches!(self.theme, Theme::Rect) { 4.0 } else { 0.0 };
                let padding_px = padding_logical * screen_descriptor.pixels_per_point;

                // Pillarbox: keep the sim square within the area between both panels
                let panel_physical = 260.0 * screen_descriptor.pixels_per_point;
                let available_w = (w - panel_physical * 2.0 - padding_px * 2.0).max(1.0);
                let side = available_w.min(h - padding_px * 2.0);
                
                let vx = panel_physical + padding_px + (available_w - side) / 2.0;
                let vy = padding_px + (h - padding_px * 2.0 - side) / 2.0;

                // ── Pass 1: field shader → offscreen texture (only when T changed) ──
                let field_dirty = self.field_force_redraw
                    || self.t_epoch    != self.last_rendered_epoch
                    || (self.t_residual - self.last_rendered_residual).abs() > 1e-12;

                if field_dirty {
                    // Upload pre-baked wave cache (replaces ~1200 per-pixel trig calls
                    // with ~30, moving all position-independent math to the CPU).
                    let cache = crate::engine::wave_cache::compute(
                        &self.env_data,
                        self.t_epoch,
                        self.t_residual,
                        self.background_noise,
                    );
                    self.queue.write_buffer(
                        &state.wave_cache_buf,
                        0,
                        bytemuck::cast_slice(&cache),
                    );
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("field-pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view:           &state.field_view, // offscreen
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load:  wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes:         None,
                        occlusion_query_set:      None,
                        multiview_mask:           None,
                    });
                    pass.set_pipeline(&state.pipeline);
                    pass.set_viewport(vx, vy, side, side, 0.0, 1.0);
                    pass.set_bind_group(0, &state.bg, &[]);
                    pass.draw(0..4, 0..1);

                    self.last_rendered_epoch    = self.t_epoch;
                    self.last_rendered_residual = self.t_residual;
                    self.field_force_redraw     = false;
                }

                // ── Pass 2: blit cached field + egui → swap-chain surface (every frame) ──
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("compose-pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view:           &view, // swap-chain surface
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load:  wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes:         None,
                        occlusion_query_set:      None,
                        multiview_mask:           None,
                    });
                    // Blit cached field (cheap — one texture lookup per pixel)
                    pass.set_pipeline(&state.blit_pipeline);
                    pass.set_bind_group(0, &state.blit_bg, &[]);
                    pass.draw(0..4, 0..1);

                    // egui UI overlay
                    pass.set_viewport(0.0, 0.0, w, h, 0.0, 1.0);
                    state.egui_renderer.render(
                        &mut pass.forget_lifetime(),
                        &tris,
                        &screen_descriptor,
                    );
                }
                self.queue.submit([encoder.finish()]);

                // TRUE FPS calculation:
                // We use the exact physical time elapsed since the very start of the last frame (dt).
                // If the user doesn't move the mouse and the simulation is at 4 TPS, this naturally
                // reads 4 FPS. When they interact with the UI, it instantly jumps up to monitor_hz.
                let inst_fps = if dt > 0.0 {
                    1.0 / dt
                } else {
                    monitor_hz
                };
                
                // Clamp to monitor refresh rate so a 1ms frame (1000 FPS) doesn't swing the average wildly.
                let inst_fps = inst_fps.min(monitor_hz); 
                self.fps = self.fps * 0.9 + inst_fps * 0.1;

                if self.take_screenshot {
                    self.take_screenshot = false;

                    let w = state.config.width;
                    let h = state.config.height;

                    // Align row bytes to 256 for WGPU
                    let bytes_per_pixel = 4;
                    let unpadded_bytes_per_row = w * bytes_per_pixel;
                    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
                    let padded_bytes_per_row = (unpadded_bytes_per_row + align - 1) & !(align - 1);
                    let buffer_size = (padded_bytes_per_row * h) as wgpu::BufferAddress;

                    let capture_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("screenshot_buffer"),
                        size: buffer_size,
                        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                        mapped_at_creation: false,
                    });

                    let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("screenshot_encoder"),
                    });

                    encoder.copy_texture_to_buffer(
                        wgpu::TexelCopyTextureInfo {
                            texture: &frame.texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyBufferInfo {
                            buffer: &capture_buffer,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(padded_bytes_per_row),
                                rows_per_image: Some(h),
                            },
                        },
                        wgpu::Extent3d {
                            width: w,
                            height: h,
                            depth_or_array_layers: 1,
                        },
                    );

                    let submission = self.queue.submit(Some(encoder.finish()));

                    let capture_slice = capture_buffer.slice(..);
                    let (tx, rx) = std::sync::mpsc::channel();
                    capture_slice.map_async(wgpu::MapMode::Read, move |result| {
                        tx.send(result).unwrap();
                    });

                    let _ = self.device.poll(wgpu::PollType::Wait {
                        submission_index: Some(submission),
                        timeout: None,
                    });

                    if let Ok(Ok(())) = rx.recv() {
                        let data = capture_slice.get_mapped_range();
                        let raw_data = data.to_vec(); // Copy from GPU to CPU RAM (instant)
                        drop(data); // Release the mapping lock
                        capture_buffer.unmap(); // Immediately free the buffer

                        let format = state.config.format;
                        let current_seed = self.seed.clone();
                        let current_time = self.t_epoch as f64 * (std::f64::consts::TAU / 0.1) + self.t_residual;
                        // Spawn a background thread so PNG compression doesn't freeze the UI
                        std::thread::spawn(move || {
                            let mut img = image::ImageBuffer::<image::Rgba<u8>, _>::new(w, h);
                            for y in 0..h {
                                for x in 0..w {
                                    let offset = (y * padded_bytes_per_row + x * bytes_per_pixel) as usize;
                                    let r = raw_data[offset];
                                    let g = raw_data[offset + 1];
                                    let b = raw_data[offset + 2];
                                    let a = raw_data[offset + 3];

                                    match format {
                                        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
                                            img.put_pixel(x, y, image::Rgba([b, g, r, a]));
                                        }
                                        _ => {
                                            img.put_pixel(x, y, image::Rgba([r, g, b, a]));
                                        }
                                    }
                                }
                            }

                            let time_str = format!("{:.1}", current_time).replace('.', "_");
                            let filename = format!("anytimeuniverse-{}-{}.png", current_seed, time_str);
                            if let Err(e) = img.save(&filename) {
                                println!("[ ui   ] failed to save screenshot: {}", e);
                            } else {
                                println!("[ ui   ] saved {}", filename);
                            }
                        });
                    } else {
                        capture_buffer.unmap();
                    }
                }

                frame.present();

                for id in &full_output.textures_delta.free {
                    state.egui_renderer.free_texture(id);
                }

                let repaint_delay = full_output
                    .viewport_output
                    .get(&egui::ViewportId::ROOT)
                    .map(|v| v.repaint_delay)
                    .unwrap_or(std::time::Duration::MAX);

                // When paused and egui has no pending animations (repaint_delay is large),
                // go fully event-driven: no redraws until the user interacts. GPU usage → ~0%.
                // When running, always request the next frame — VSync in frame.present() caps
                // the rate to the monitor refresh rate without CPU spinning.
                let truly_idle = self.is_paused
                    && repaint_delay >= std::time::Duration::from_millis(200);

                if !truly_idle {
                    state.window.request_redraw();
                    self.sim_handle.ui_requested_frame.store(true, std::sync::atomic::Ordering::Relaxed);
                }

                if truly_idle {
                    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
                } else if repaint_delay < std::time::Duration::from_secs(10) {
                    let target = std::time::Instant::now() + repaint_delay;
                    event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(target));
                } else {
                    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
                }

                // Apply fullscreen AFTER present() so no SurfaceTexture is alive
                // when macOS fires the synchronous Resized event that reconfigures the surface.
                if fullscreen_req || self.pending_fullscreen_toggle {
                    self.pending_fullscreen_toggle = false;
                    let is_fs = state.window.fullscreen().is_some();
                    state.window.set_fullscreen(if is_fs { None } else { Some(winit::window::Fullscreen::Borderless(None)) });
                    // Reset cursor: macOS briefly shows a native resize cursor during the
                    // window-bounds change; override it back to the default arrow.
                    state.window.set_cursor(winit::window::CursorIcon::Default);
                }

                if let Some(new_title) = pending_title {
                    state.window.set_title(&new_title);
                }

                if let Some(reset) = pending_reset {
                    self.reset_simulation(reset);
                }
            }
            _ => {}
        }
    }
}
