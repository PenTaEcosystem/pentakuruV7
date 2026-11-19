//! Notes workspace and compact Spotlight-style search/AI palettes.
use crate::{
    PentaKuru, ai, search, system,
    ui_theme::{self as theme, Glyph},
};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2, ViewportCommand, ViewportId};

fn palette_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(20)
        .inner_margin(12)
}

fn dismissed(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.viewport().close_requested())
        || ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
}

impl PentaKuru {
    pub(super) fn notes_window(&mut self, ctx: &egui::Context) {
        if !self.notes_open {
            return;
        }
        let mut open = true;
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("notes"),
            egui::ViewportBuilder::default()
                .with_title("PentaKuru · Ghi chú")
                .with_inner_size(if self.ui_preview.as_deref() == Some("notes-small") {
                    [660.0, 460.0]
                } else {
                    [480.0, 560.0]
                })
                .with_min_inner_size([380.0, 460.0])
                .with_resizable(true)
                .with_decorations(false)
                .with_transparent(true)
                .with_always_on_top()
                .with_active(!self.smoke_test),
            |viewport_ui, _| {
                system::apply_window_dark_and_transparency("PentaKuru · Ghi chú");
                open = self.notes_ui(viewport_ui);
            },
        );
        self.notes_open = open;
    }

    pub(super) fn palettes(&mut self, ctx: &egui::Context) {
        let root_rect = ctx
            .input(|i| i.viewport().outer_rect)
            .unwrap_or(Rect::from_min_size(
                Pos2::new(100.0, 100.0),
                Vec2::new(160.0, 190.0),
            ));
        let screen_rect = ctx
            .input(|i| i.viewport().monitor_size)
            .map(|sz| Rect::from_min_size(Pos2::ZERO, sz))
            .unwrap_or(Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0)));

        let calc_pos = |w: f32, h: f32| -> [f32; 2] {
            let mut x = root_rect.center().x - w * 0.5;
            x = x.clamp(12.0, (screen_rect.max.x - w - 12.0).max(12.0));

            let y_below = root_rect.max.y + 8.0;
            let y = if y_below + h > screen_rect.max.y - 40.0 && root_rect.min.y - h - 8.0 >= 10.0 {
                root_rect.min.y - h - 8.0
            } else {
                y_below.min(screen_rect.max.y - h - 12.0)
            };
            [x, y]
        };

        if !self.search_open {
            self.search_focus = 10;
        }
        if !self.ai_open {
            self.ai_focus = 10;
        }
        if self.search_open {
            self.search_state.update_search();
            let height = search_height(&self.search_state.query, self.search_state.results.len());
            let position = calc_pos(540.0, height);
            let mut open = true;
            ctx.show_viewport_immediate(
                ViewportId::from_hash_of("search_dock"),
                egui::ViewportBuilder::default()
                    .with_title("PentaKuru · Tìm kiếm")
                    .with_position(position)
                    .with_inner_size([540.0, height])
                    .with_resizable(false)
                    .with_decorations(false)
                    .with_transparent(true)
                    .with_always_on_top()
                    .with_taskbar(false)
                    .with_active(!self.smoke_test),
                |viewport_ui, _| {
                    system::apply_tool_window("PentaKuru · Tìm kiếm");
                    open = self.search_ui(viewport_ui);
                },
            );
            self.search_open = open;
            if !open {
                self.search_focus = 10;
            }
        }
        let mut ai_bottom_edge = root_rect.max.y;
        if self.ai_open {
            let expanded = self.ai_history_expanded;
            let width = if expanded { 540.0_f32 } else { 480.0_f32 }
                .min((screen_rect.width() - 24.0).max(240.0));
            let lines = prompt_rows(&self.ai_prompt, (width - 104.0).max(1.0));
            let has_strip = !expanded
                && (self.notes_mgr.notes.iter().any(|note| note.ai_shared)
                    || !self.ai_attached_files.is_empty()
                    || crate::ai::note_command(&self.ai_prompt).is_some());
            let height = (ai_height(expanded, lines) + if has_strip { 20.0 } else { 0.0 })
                .max(if self.ai_menu_open { 240.0 } else { 0.0 })
                .min((screen_rect.height() - 64.0).max(160.0));
            let position = calc_pos(width, height);
            ai_bottom_edge = position[1] + height;
            let mut open = true;
            ctx.show_viewport_immediate(
                ViewportId::from_hash_of("ai_dock"),
                egui::ViewportBuilder::default()
                    .with_title("PentaKuru · AI")
                    .with_position(position)
                    .with_inner_size([width, height])
                    .with_resizable(false)
                    .with_decorations(false)
                    .with_transparent(true)
                    .with_always_on_top()
                    .with_taskbar(false)
                    .with_active(!self.smoke_test),
                |viewport_ui, _| {
                    system::apply_tool_window("PentaKuru · AI");
                    open = self.ai_ui(viewport_ui);
                },
            );
            self.ai_open = open;
            if !open {
                self.ai_focus = 10;
            }
        }
        if self.ai_reply_bubble_visible() {
            let bubble_w = 420.0_f32.min((screen_rect.width() - 24.0).max(240.0));
            let bubble_h = self
                .ai_bubble_height
                .clamp(60.0, 180.0)
                .max(if self.ai_bubble_menu_open { 220.0 } else { 0.0 });
            let bubble_x = (root_rect.center().x - bubble_w * 0.5).clamp(
                screen_rect.min.x + 12.0,
                (screen_rect.max.x - bubble_w - 12.0).max(12.0),
            );
            let y_above = root_rect.min.y - bubble_h - 8.0;
            let bubble_y = if y_above >= screen_rect.min.y + 8.0 {
                y_above
            } else {
                (ai_bottom_edge + 8.0).min((screen_rect.max.y - bubble_h - 12.0).max(12.0))
            };
            ctx.show_viewport_immediate(
                ViewportId::from_hash_of("ai_reply_bubble"),
                egui::ViewportBuilder::default()
                    .with_title("PentaKuru · Trả lời AI")
                    .with_position([bubble_x, bubble_y])
                    .with_inner_size([bubble_w, bubble_h])
                    .with_resizable(false)
                    .with_decorations(false)
                    .with_transparent(true)
                    .with_always_on_top()
                    .with_taskbar(false)
                    .with_active(false),
                |viewport_ui, _| {
                    system::apply_tool_window("PentaKuru · Trả lời AI");
                    self.ai_reply_bubble_ui(viewport_ui);
                },
            );
        }
        self.mini_music_window(ctx, screen_rect);
    }

    pub(super) fn mini_music_window(&mut self, ctx: &egui::Context, screen_rect: Rect) {
        if !self.mini_music.open {
            return;
        }
        const MINI_DISC_SIZE: f32 = 64.0;
        const AUTO_COLLAPSE: std::time::Duration = std::time::Duration::from_secs(5);
        let card_w = 368.0_f32.min((screen_rect.width() - 24.0).max(260.0));
        let card_h = if self.mini_music.queue_open {
            268.0
        } else {
            122.0
        };

        if !self.smoke_test
            && !self.mini_music.collapsed_disc
            && !self.mini_music.queue_open
            && !self.mini_music.tracks.is_empty()
        {
            let elapsed = self.mini_music.last_interaction.elapsed();
            if elapsed >= AUTO_COLLAPSE {
                self.mini_music.collapsed_disc = true;
            } else {
                ctx.request_repaint_after(
                    (AUTO_COLLAPSE - elapsed).max(std::time::Duration::from_millis(200)),
                );
            }
        }

        let default_base = [
            (screen_rect.max.x - card_w - 20.0).max(screen_rect.min.x + 12.0),
            (screen_rect.max.y - 122.0 - 56.0).max(screen_rect.min.y + 12.0),
        ];
        let base_pos = self.mini_music_pos.unwrap_or(default_base);
        let (w, h, pos) = if self.mini_music.collapsed_disc {
            let dx = (base_pos[0] + card_w - MINI_DISC_SIZE).clamp(
                screen_rect.min.x + 8.0,
                (screen_rect.max.x - MINI_DISC_SIZE - 8.0).max(8.0),
            );
            let dy = (base_pos[1] + 122.0 - MINI_DISC_SIZE).clamp(
                screen_rect.min.y + 8.0,
                (screen_rect.max.y - MINI_DISC_SIZE - 48.0).max(8.0),
            );
            (MINI_DISC_SIZE, MINI_DISC_SIZE, [dx, dy])
        } else {
            let expand_up = card_h - 122.0;
            let px = base_pos[0].clamp(
                screen_rect.min.x + 8.0,
                (screen_rect.max.x - card_w - 8.0).max(8.0),
            );
            let py = (base_pos[1] - expand_up).clamp(
                screen_rect.min.y + 8.0,
                (screen_rect.max.y - card_h - 48.0).max(8.0),
            );
            (card_w, card_h, [px, py])
        };
        let mut open = true;
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("mini_music_player"),
            egui::ViewportBuilder::default()
                .with_title("PentaKuru · Mini Music")
                .with_position(pos)
                .with_inner_size([w, h])
                .with_min_inner_size([MINI_DISC_SIZE, MINI_DISC_SIZE])
                .with_resizable(false)
                .with_decorations(false)
                .with_transparent(true)
                .with_always_on_top()
                .with_taskbar(false)
                .with_active(false),
            |viewport_ui, _| {
                system::apply_tool_window("PentaKuru · Mini Music");
                if let Some(outer) = viewport_ui.ctx().input(|i| i.viewport().outer_rect) {
                    if !self.mini_music.collapsed_disc
                        && outer.width() > 200.0
                        && (outer.height() - card_h).abs() < 6.0
                    {
                        self.mini_music_pos = Some([outer.min.x, outer.min.y + (card_h - 122.0)]);
                    } else if self.mini_music.collapsed_disc
                        && outer.width() <= 80.0
                        && ((outer.min.x - pos[0]).abs() > 1.5
                            || (outer.min.y - pos[1]).abs() > 1.5)
                    {
                        self.mini_music_pos = Some([
                            outer.min.x + MINI_DISC_SIZE - card_w,
                            outer.min.y + MINI_DISC_SIZE - 122.0,
                        ]);
                    }
                }
                open = self.mini_music_ui(viewport_ui);
            },
        );
        if !open {
            self.mini_music.close();
        }
    }

    pub(super) fn mini_music_ui(&mut self, viewport_ui: &mut egui::Ui) -> bool {
        let mut open = true;
        let ctx = viewport_ui.ctx().clone();
        if ctx.input(|i| i.viewport().close_requested()) {
            open = false;
        }
        let had_activity = ctx.input(|i| {
            i.pointer.any_down()
                || i.pointer.any_released()
                || i.pointer.delta() != Vec2::ZERO
                || i.smooth_scroll_delta != Vec2::ZERO
                || i.events.iter().any(|e| {
                    matches!(
                        e,
                        egui::Event::PointerMoved(_)
                            | egui::Event::PointerButton { .. }
                            | egui::Event::Text(_)
                            | egui::Event::Key { .. }
                    )
                })
        });
        if !self.mini_music.collapsed_disc && had_activity {
            self.mini_music.touch();
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(viewport_ui, |ui| {
                let open_inner = if self.mini_music.collapsed_disc {
                    self.mini_music_disc_contents(ui)
                } else {
                    self.mini_music_contents(ui)
                };
                if !open_inner {
                    open = false;
                }
            });
        open
    }

    fn mini_music_disc_contents(&mut self, ui: &mut egui::Ui) -> bool {
        let ctx = ui.ctx().clone();
        let snap = self.mini_music.engine.snapshot();
        let is_playing = matches!(
            snap.state,
            crate::mcp_media::PlaybackState::Playing | crate::mcp_media::PlaybackState::Opening
        );
        let track_opt = self.mini_music.current_track().cloned();
        let emerald = Color32::from_rgb(16, 185, 129);
        let emerald_bright = Color32::from_rgb(52, 211, 153);

        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(62.0), Sense::click_and_drag());
        let center = rect.center();

        // Dark vinyl outer body + grooves
        {
            let p = ui.painter();
            p.circle_filled(center, 29.0, Color32::from_rgb(15, 18, 22));
            p.circle_stroke(
                center,
                26.0,
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 18)),
            );
            p.circle_stroke(
                center,
                23.0,
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 14)),
            );
        }

        let tex = track_opt
            .as_ref()
            .and_then(|t| self.mini_music.thumbs.get(&t.video_id))
            .and_then(|o| o.as_ref());
        if let Some(tex) = tex {
            let art_rect = Rect::from_center_size(center, Vec2::splat(42.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(art_rect), |ui| {
                ui.add(
                    egui::Image::new(tex)
                        .fit_to_exact_size(Vec2::splat(42.0))
                        .corner_radius(21.0),
                );
            });
        } else {
            let p = ui.painter();
            p.circle_filled(center, 20.0, Color32::from_rgb(24, 32, 38));
            p.text(
                center + Vec2::new(-1.0, 0.0),
                egui::Align2::CENTER_CENTER,
                "♫",
                egui::FontId::proportional(18.0),
                emerald_bright,
            );
        }

        let t = ui.input(|i| i.time as f32);
        {
            let p = ui.painter();
            // Vinyl center spindle hole
            p.circle_filled(center, 5.2, Color32::from_rgb(15, 18, 22));
            p.circle_stroke(
                center,
                5.2,
                Stroke::new(
                    1.2,
                    if is_playing {
                        emerald_bright
                    } else {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 90)
                    },
                ),
            );

            // Outer border + animated spin marker & playback progress arc
            p.circle_stroke(
                center,
                29.0,
                Stroke::new(
                    1.4,
                    Color32::from_rgba_unmultiplied(
                        255,
                        255,
                        255,
                        if is_playing { 42 } else { 30 },
                    ),
                ),
            );
            if is_playing {
                let angle = t * 1.8;
                let notch = center + Vec2::new(angle.cos(), angle.sin()) * 24.5;
                p.circle_filled(notch, 2.2, emerald_bright);
            }
            let frac = if snap.duration_secs > 0.5 {
                (snap.position_secs / snap.duration_secs).clamp(0.0, 1.0) as f32
            } else {
                0.0
            };
            if frac > 0.02 || is_playing {
                let sweep = if frac > 0.02 { frac } else { 0.32 } * std::f32::consts::TAU;
                let start = if frac > 0.02 {
                    -std::f32::consts::FRAC_PI_2
                } else {
                    t * 1.8
                };
                let steps = 28;
                let pts: Vec<Pos2> = (0..=steps)
                    .map(|s| {
                        let a = start + sweep * (s as f32 / steps as f32);
                        center + Vec2::new(a.cos(), a.sin()) * 29.0
                    })
                    .collect();
                p.add(egui::Shape::line(pts, Stroke::new(2.0, emerald)));
            }
        }

        if resp.drag_started() {
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
        }
        let tip = track_opt
            .as_ref()
            .map(|t| format!("{} — {} (Rê chuột/Bấm để mở)", t.title, t.artist))
            .unwrap_or_else(|| "Đĩa nhạc PentaKuru (Bấm để mở)".into());
        let resp = resp.on_hover_text(tip);

        let outer_compact = ctx
            .input(|i| i.viewport().outer_rect)
            .is_some_and(|r| r.width() <= 80.0);
        let pointer_entered = ctx.input(|i| {
            i.pointer.delta() != Vec2::ZERO
                || i.events
                    .iter()
                    .any(|e| matches!(e, egui::Event::PointerMoved(_)))
        });
        if !self.smoke_test
            && outer_compact
            && !resp.dragged()
            && !resp.is_pointer_button_down_on()
            && (resp.clicked() || (resp.hovered() && pointer_entered))
        {
            self.mini_music.collapsed_disc = false;
            self.mini_music.touch();
        }
        true
    }

    pub(super) fn mini_music_contents(&mut self, ui: &mut egui::Ui) -> bool {
        let ctx = ui.ctx().clone();
        let mut open = true;
        let snap = self.mini_music.engine.snapshot();
        let is_playing = matches!(
            snap.state,
            crate::mcp_media::PlaybackState::Playing | crate::mcp_media::PlaybackState::Opening
        );
        let emerald = Color32::from_rgb(16, 185, 129);
        let emerald_bright = Color32::from_rgb(52, 211, 153);

        egui::Frame::new()
            .fill(Color32::from_rgb(17, 21, 25))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(255, 255, 255, 32),
            ))
            .corner_radius(16)
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.set_width((ui.available_width()).max(220.0));
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);

                let track_opt = self.mini_music.current_track().cloned();
                if self.mini_music.queue_open {
                    let mcp_ready = !self.prefs.integrations.resolved_music_key().is_empty();
                    let mcp_busy = self.music_mcp_job.is_some();
                    let mut submit_search = false;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.spacing_mut().button_padding = Vec2::new(6.0, 2.5);
                        let search_resp = ui.add(
                            egui::TextEdit::singleline(&mut self.mini_music.search_query)
                                .id_salt("mini-music-search")
                                .desired_width(104.0)
                                .font(egui::FontId::proportional(11.5))
                                .hint_text("Tìm bài hát…"),
                        );
                        if search_resp.clicked() {
                            ctx.send_viewport_cmd(ViewportCommand::Focus);
                            search_resp.request_focus();
                        }
                        if search_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            submit_search = true;
                        }
                        let has_q = !self.mini_music.search_query.trim().is_empty();
                        if ui
                            .add_enabled(
                                has_q && mcp_ready && !mcp_busy,
                                egui::Button::new("Tìm").small(),
                            )
                            .clicked()
                        {
                            submit_search = true;
                        }
                        if ui
                            .add_enabled(
                                mcp_ready && !mcp_busy,
                                egui::Button::new("Hợp gu").small(),
                            )
                            .on_hover_text("Chọn nhạc hợp gu từ Penta Music")
                            .clicked()
                        {
                            self.run_music_mcp_picker_tool(
                                &ctx,
                                "blend_history_and_taste",
                                serde_json::json!({ "limit": 10, "remote_play": false }),
                                false,
                            );
                        }
                        if ui
                            .add_enabled(
                                mcp_ready && !mcp_busy,
                                egui::Button::new("Bài mới").small(),
                            )
                            .on_hover_text("AI tự tìm các bài hát mới cùng gu")
                            .clicked()
                        {
                            let q = if self.mini_music.search_query.trim().is_empty() {
                                self.mini_music.derive_discovery_query()
                            } else {
                                self.mini_music.search_query.trim().to_owned()
                            };
                            self.run_music_mcp_discover_append(&ctx, q);
                        }
                        if ui
                            .small_button("Web")
                            .on_hover_text("Mở Penta Music trong WebView")
                            .clicked()
                            && let Err(err) = crate::integrations::music_open()
                        {
                            self.mini_music.status = Some(err);
                        }
                    });
                    if submit_search {
                        let q = self.mini_music.search_query.trim().to_owned();
                        if !q.is_empty() && mcp_ready && !mcp_busy {
                            self.run_music_mcp_picker_tool(
                                &ctx,
                                "search_and_stream_music",
                                serde_json::json!({
                                    "query": q,
                                    "limit": 10,
                                    "remote_play": false
                                }),
                                true,
                            );
                        }
                    }

                    if mcp_busy {
                        ui.horizontal(|ui| {
                            ui.add(egui::Spinner::new().size(12.0));
                            theme::caption(ui, "Đang tải danh sách chọn nhạc…");
                        });
                    } else if self.mini_music.tracks.is_empty() {
                        theme::caption(
                            ui,
                            if mcp_ready {
                                "Bấm Hợp gu / Bài mới hoặc nhập tên bài để chọn nhạc."
                            } else {
                                "Nhập Music Key ở Alt+Q hoặc bấm Web để mở WebView."
                            },
                        );
                    } else {
                        let mut pick_idx = None;
                        egui::ScrollArea::vertical()
                            .id_salt("mini-music-queue")
                            .max_height(88.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                for (idx, t) in self.mini_music.tracks.iter().enumerate() {
                                    let active = track_opt.as_ref().is_some_and(|cur| {
                                        !cur.video_id.is_empty() && cur.video_id == t.video_id
                                    }) || (self.mini_music.active_track.is_none()
                                        && idx == self.mini_music.current_index);
                                    let label = format!("{}. {} — {}", idx + 1, t.title, t.artist);
                                    let resp = ui.add(
                                        egui::Button::new(
                                            egui::RichText::new(label).size(11.5).color(
                                                if active { emerald_bright } else { theme::TEXT },
                                            ),
                                        )
                                        .fill(if active {
                                            Color32::from_rgba_unmultiplied(16, 185, 129, 28)
                                        } else {
                                            Color32::TRANSPARENT
                                        })
                                        .frame(false)
                                        .truncate(),
                                    );
                                    if resp.clicked() {
                                        pick_idx = Some(idx);
                                    }
                                }
                            });
                        if let Some(idx) = pick_idx {
                            self.mini_music.play_index(idx, &ctx);
                        }
                    }
                    ui.separator();
                }

                let header = ui
                    .horizontal(|ui| {
                        let (art_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(46.0), Sense::hover());
                        let tex = track_opt
                            .as_ref()
                            .and_then(|t| self.mini_music.thumbs.get(&t.video_id))
                            .and_then(|o| o.as_ref());
                        if let Some(tex) = tex {
                            ui.scope_builder(egui::UiBuilder::new().max_rect(art_rect), |ui| {
                                ui.add(
                                    egui::Image::new(tex)
                                        .fit_to_exact_size(Vec2::splat(46.0))
                                        .corner_radius(23.0),
                                );
                            });
                        } else {
                            ui.painter().circle_filled(
                                art_rect.center(),
                                23.0,
                                Color32::from_rgb(28, 34, 42),
                            );
                            ui.painter().text(
                                art_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "♪",
                                egui::FontId::proportional(20.0),
                                emerald_bright,
                            );
                        }
                        ui.painter().circle_stroke(
                            art_rect.center(),
                            22.5,
                            Stroke::new(
                                1.5,
                                if is_playing {
                                    emerald
                                } else {
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 40)
                                },
                            ),
                        );
                        if is_playing {
                            let badge = Rect::from_min_size(
                                art_rect.max - Vec2::new(17.0, 13.0),
                                Vec2::new(16.0, 12.0),
                            );
                            ui.painter().rect_filled(
                                badge,
                                4,
                                Color32::from_rgba_unmultiplied(15, 18, 22, 225),
                            );
                            let base = badge.left_bottom() + Vec2::new(3.0, -2.0);
                            let t = ui.input(|i| i.time as f32);
                            for idx in 0..3 {
                                let h =
                                    3.0 + 5.0 * ((t * 6.0 + idx as f32 * 1.4).sin() * 0.5 + 0.5);
                                let x = base.x + idx as f32 * 4.5;
                                ui.painter().line_segment(
                                    [Pos2::new(x, base.y), Pos2::new(x, base.y - h)],
                                    Stroke::new(2.0, emerald_bright),
                                );
                            }
                        }

                        ui.vertical(|ui| {
                            ui.set_width(ui.available_width());
                            let title = track_opt
                                .as_ref()
                                .map(|t| t.title.as_str())
                                .unwrap_or("Chưa có bài hát");
                            let artist = track_opt
                                .as_ref()
                                .map(|t| t.artist.as_str())
                                .unwrap_or("Penta Music MCP");
                            let drag_info = ui
                                .vertical(|ui| {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(title)
                                                .size(13.5)
                                                .strong()
                                                .color(theme::TEXT),
                                        )
                                        .truncate(),
                                    );
                                    let time_label = if snap.duration_secs > 0.5 {
                                        format!(
                                            "{} · {} / {}",
                                            artist,
                                            crate::mcp_media::format_time(snap.position_secs),
                                            crate::mcp_media::format_time(snap.duration_secs)
                                        )
                                    } else if let Some(t) = &track_opt
                                        && !t.duration_formatted.is_empty()
                                    {
                                        format!("{artist} · {}", t.duration_formatted)
                                    } else {
                                        artist.to_owned()
                                    };
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(time_label)
                                                .size(11.5)
                                                .color(theme::MUTED),
                                        )
                                        .truncate(),
                                    );
                                })
                                .response
                                .interact(Sense::drag());
                            if drag_info.drag_started() {
                                ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                            }

                            let (bar_rect, bar_resp) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width().max(40.0), 8.0),
                                Sense::click_and_drag(),
                            );
                            let track_line = Rect::from_center_size(
                                bar_rect.center(),
                                Vec2::new(bar_rect.width(), 3.0),
                            );
                            ui.painter()
                                .rect_filled(track_line, 2, Color32::from_rgb(38, 44, 52));
                            let frac = if snap.duration_secs > 0.5 {
                                (snap.position_secs / snap.duration_secs).clamp(0.0, 1.0) as f32
                            } else {
                                0.0
                            };
                            if frac > 0.0 {
                                let fill_rect = Rect::from_min_size(
                                    track_line.min,
                                    Vec2::new(track_line.width() * frac, track_line.height()),
                                );
                                ui.painter().rect_filled(fill_rect, 2, emerald);
                            }
                            if (bar_resp.clicked() || bar_resp.dragged())
                                && snap.duration_secs > 0.5
                                && let Some(pos) = bar_resp.interact_pointer_pos()
                            {
                                let seek_frac = ((pos.x - track_line.min.x) / track_line.width())
                                    .clamp(0.0, 1.0)
                                    as f64;
                                self.mini_music.engine.seek(snap.duration_secs * seek_frac);
                            }
                        });
                    })
                    .response;
                header.context_menu(|ui| {
                    compact_menu(ui);
                    if ui.button("Thu gọn thành đĩa tròn").clicked() {
                        self.mini_music.queue_open = false;
                        self.mini_music.collapsed_disc = true;
                        ui.close();
                    }
                    if !self.prefs.integrations.resolved_music_key().is_empty()
                        && ui.button("Tìm bài mới (AI)").clicked()
                    {
                        let q = self.mini_music.derive_discovery_query();
                        self.run_music_mcp_discover_append(&ctx, q);
                        ui.close();
                    }
                    if ui.button("Đóng trình phát").clicked() {
                        open = false;
                        ui.close();
                    }
                });

                let div_rect = ui.allocate_space(Vec2::new(ui.available_width(), 1.0)).1;
                ui.painter().line_segment(
                    [div_rect.left_center(), div_rect.right_center()],
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 20)),
                );

                // Evenly balanced control row: playback left, utilities right
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if mini_media_btn(
                        ui,
                        MiniBtn::Shuffle,
                        if self.mini_music.shuffle {
                            "Phát ngẫu nhiên: Bật"
                        } else {
                            "Phát ngẫu nhiên: Tắt"
                        },
                        self.mini_music.shuffle,
                    )
                    .clicked()
                    {
                        self.mini_music.shuffle = !self.mini_music.shuffle;
                    }
                    if mini_media_btn(ui, MiniBtn::Prev, "Bài trước", false).clicked() {
                        self.mini_music.prev(&ctx);
                    }
                    let (pp_rect, pp_resp) =
                        ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
                    let pp_fill = if pp_resp.hovered() {
                        emerald_bright
                    } else {
                        emerald
                    };
                    ui.painter().circle_filled(pp_rect.center(), 16.0, pp_fill);
                    let c = pp_rect.center();
                    if is_playing {
                        ui.painter().rect_filled(
                            Rect::from_center_size(c - Vec2::new(3.0, 0.0), Vec2::new(3.0, 10.0)),
                            1.0,
                            Color32::WHITE,
                        );
                        ui.painter().rect_filled(
                            Rect::from_center_size(c + Vec2::new(3.0, 0.0), Vec2::new(3.0, 10.0)),
                            1.0,
                            Color32::WHITE,
                        );
                    } else {
                        ui.painter().add(egui::Shape::convex_polygon(
                            vec![
                                c + Vec2::new(-3.5, -5.5),
                                c + Vec2::new(6.0, 0.0),
                                c + Vec2::new(-3.5, 5.5),
                            ],
                            Color32::WHITE,
                            Stroke::NONE,
                        ));
                    }
                    if pp_resp
                        .on_hover_text(if is_playing { "Tạm dừng" } else { "Phát" })
                        .clicked()
                    {
                        self.mini_music.toggle_play_pause(&ctx);
                    }

                    let can_discover = !self.smoke_test
                        && !self.prefs.integrations.resolved_music_key().is_empty();
                    if mini_media_btn(ui, MiniBtn::Next, "Bài tiếp / tìm bài mới", false).clicked()
                    {
                        self.mini_music.next_or_discover(can_discover, &ctx);
                    }
                    if mini_media_btn(
                        ui,
                        MiniBtn::Repeat,
                        if self.mini_music.repeat {
                            "Lặp lại bài: Bật"
                        } else {
                            "Lặp lại bài: Tắt"
                        },
                        self.mini_music.repeat,
                    )
                    .clicked()
                    {
                        self.mini_music.repeat = !self.mini_music.repeat;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        if mini_media_btn(ui, MiniBtn::Close, "Đóng trình phát", false).clicked()
                        {
                            open = false;
                        }
                        if mini_media_btn(
                            ui,
                            MiniBtn::Queue,
                            "Chọn nhạc / Danh sách phát",
                            self.mini_music.queue_open,
                        )
                        .clicked()
                        {
                            self.mini_music.queue_open = !self.mini_music.queue_open;
                        }
                        let vol_pct = (self.mini_music.volume * 100.0).round() as u8;
                        let vol_tip = if vol_pct == 0 {
                            "Đang tắt tiếng (bấm bật 85%)".to_owned()
                        } else {
                            format!("Âm lượng {vol_pct}% (bấm đổi 45% / Tắt)")
                        };
                        if mini_media_btn(ui, MiniBtn::Volume(vol_pct), &vol_tip, false).clicked() {
                            self.mini_music.volume = if self.mini_music.volume > 0.6 {
                                0.45
                            } else if self.mini_music.volume > 0.05 {
                                0.0
                            } else {
                                0.85
                            };
                            self.mini_music.engine.set_volume(self.mini_music.volume);
                        }
                        if mini_media_btn(
                            ui,
                            MiniBtn::Download,
                            "Tải bài hát về Downloads",
                            self.music_download_job.is_some(),
                        )
                        .clicked()
                            && self.music_download_job.is_none()
                            && let Some(t) = &track_opt
                        {
                            let target = if t.download_url.is_empty() {
                                t.stream_url.clone()
                            } else {
                                t.download_url.clone()
                            };
                            let title = t.title.clone();
                            self.mini_music.status = Some("Đang tải bài hát về Downloads…".into());
                            self.music_download_job = Some(system::Job::spawn(&ctx, move || {
                                crate::mcp_media::download_track_file(&target, &title)
                            }));
                        }
                    });
                });

                if let Some(status_msg) = &self.mini_music.status {
                    let color = if status_msg.starts_with("Đã tải")
                        || status_msg.contains("đang tìm")
                    {
                        emerald_bright
                    } else {
                        Color32::from_rgb(248, 113, 113)
                    };
                    ui.add(
                        egui::Label::new(egui::RichText::new(status_msg).size(11.0).color(color))
                            .wrap(),
                    );
                }
            });

        open
    }

    pub(super) fn notes_ui(&mut self, viewport_ui: &mut egui::Ui) -> bool {
        self.poll_workspace();
        let mut open = true;

        let ctx = viewport_ui.ctx().clone();
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        if dismissed(&ctx) {
            open = false;
        }
        let dropped: Vec<_> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| p.is_file())
                .collect()
        });
        if !dropped.is_empty() {
            self.notes_mgr.upload_files_to_selected(&dropped);
        }

        egui::Panel::top("notes-titlebar")
            .frame(egui::Frame::new().fill(theme::PANEL).inner_margin(10))
            .show(viewport_ui, |ui| {
                let avail_w = ui.available_width();
                let titlebar_rect = Rect::from_min_size(ui.cursor().min, Vec2::new(avail_w, 36.0));
                let titlebar_response = ui.interact(
                    titlebar_rect,
                    ui.id().with("notes_titlebar_drag"),
                    Sense::click_and_drag(),
                );
                if titlebar_response.dragged() {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                if titlebar_response.double_clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                }

                ui.horizontal(|ui| {
                    theme::icon(ui, Glyph::File, "Ghi chú", true);
                    if ui
                        .selectable_label(self.workspace.library_open, "Thư viện")
                        .clicked()
                    {
                        self.workspace.library_open = !self.workspace.library_open;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::icon(ui, Glyph::Close, "Đóng · Esc", true).clicked() {
                            open = false;
                        }
                        if theme::icon(
                            ui,
                            if maximized {
                                Glyph::Restore
                            } else {
                                Glyph::Maximize
                            },
                            if maximized {
                                "Khôi phục kích thước"
                            } else {
                                "Phóng to cửa sổ"
                            },
                            true,
                        )
                        .clicked()
                        {
                            ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                        }
                        if theme::icon(ui, Glyph::Minimize, "Thu xuống thanh tác vụ", true)
                            .clicked()
                        {
                            ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                        }
                        if theme::toolbar_icon(
                            ui,
                            Glyph::Plus,
                            &format!(
                                "Tạo note mới trong thư mục «{}»",
                                self.notes_mgr.selected_folder
                            ),
                            true,
                            false,
                        )
                        .clicked()
                        {
                            let target_folder = self.notes_mgr.selected_folder.clone();
                            self.notes_mgr
                                .create_note_in_folder(&target_folder, "Ghi chú mới", "");
                            self.workspace.board_open = false;
                            self.workspace.profile_open = false;
                        }
                        let folder_btn = theme::icon(
                            ui,
                            Glyph::Folder,
                            "Hiện / ẩn danh sách thư mục kế bên ghi chú (Chuột phải: mở thư mục trên máy)",
                            true,
                        );
                        if folder_btn.clicked() {
                            self.workspace.library_open = !self.workspace.library_open;
                            if self.workspace.library_open {
                                self.workspace.board_open = false;
                                self.workspace.profile_open = false;
                            }
                        }
                        folder_btn.context_menu(|ui| {
                            compact_menu(ui);
                            if ui.button("Mở thư mục lưu trên máy").clicked() {
                                let _ = system::open(&self.notes_mgr.root_dir.to_string_lossy());
                                ui.close();
                            }
                        });
                    });
                });
            });

        if self.workspace.library_open && !self.workspace.board_open {
            egui::Panel::left("notes-library")
                .exact_size(if viewport_ui.available_width() < 600.0 {
                    175.0
                } else {
                    220.0
                })
                .resizable(false)
                .frame(
                    egui::Frame::new()
                        .fill(theme::SIDEBAR)
                        .inner_margin(egui::Margin::symmetric(10, 10)),
                )
                .show(viewport_ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                    ui.horizontal(|ui| {
                        let search_w = (ui.available_width() - 28.0).max(80.0);
                        ui.add_sized(
                            [search_w, 26.0],
                            egui::TextEdit::singleline(&mut self.notes_mgr.search_query)
                                .font(egui::FontId::proportional(11.5))
                                .hint_text("Tìm ghi chú…")
                                .margin(egui::vec2(8.0, 4.0)),
                        );
                        if theme::mini_icon(
                            ui,
                            Glyph::Plus,
                            "Tạo thư mục mới để chứa ghi chú",
                            true,
                            self.folder_form,
                        )
                        .clicked()
                        {
                            self.folder_form = !self.folder_form;
                        }
                    });
                    if self.folder_form {
                        ui.horizontal(|ui| {
                            let input_w = (ui.available_width() - 42.0).max(70.0);
                            let resp = ui.add_sized(
                                [input_w, 26.0],
                                egui::TextEdit::singleline(&mut self.new_folder_input)
                                    .font(egui::FontId::proportional(11.5))
                                    .hint_text("Tên thư mục · Cha/Con"),
                            );
                            if (ui.small_button("Tạo").clicked()
                                || (resp.has_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                                && !self.new_folder_input.trim().is_empty()
                            {
                                self.notes_mgr.add_folder(&self.new_folder_input);
                                self.new_folder_input.clear();
                                self.folder_form = false;
                            }
                        });
                    }
                    ui.add_space(4.0);

                    let folders_snapshot = self.notes_mgr.folders.clone();
                    let query = system::fold(self.notes_mgr.search_query.trim());
                    let mut folder_to_toggle_ai: Option<String> = None;
                    let mut folder_to_create_note: Option<String> = None;
                    let mut folder_to_toggle_expand: Option<String> = None;
                    let mut folder_to_toggle_branch: Option<String> = None;
                    let mut subfolder_parent: Option<String> = None;
                    let mut note_to_select: Option<(String, String)> = None;
                    let mut note_to_move: Option<(String, String)> = None;
                    let mut note_to_toggle_ai: Option<String> = None;
                    // One pass over all notes instead of one per folder per frame.
                    let by_folder: std::collections::HashMap<String, Vec<(String, String, bool)>> =
                        self.notes_mgr
                            .notes_by_folder(&query)
                            .into_iter()
                            .map(|(folder, ids)| {
                                let rows = ids
                                    .into_iter()
                                    .map(|i| {
                                        let n = &self.notes_mgr.notes[i];
                                        (n.id.clone(), n.title.clone(), n.ai_shared)
                                    })
                                    .collect();
                                (folder.to_owned(), rows)
                            })
                            .collect();

                    egui::ScrollArea::vertical()
                        .id_salt("note-list-scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            if !query.is_empty() && by_folder.is_empty() {
                                theme::caption(ui, "Không có ghi chú khớp.");
                            }
                            for folder in &folders_snapshot {
                                let folder_notes =
                                    by_folder.get(folder).cloned().unwrap_or_default();
                                // Searching shows a flat list of matching folders by full path.
                                if !query.is_empty() && folder_notes.is_empty()
                                    || query.is_empty()
                                        && self.notes_mgr.is_hidden_by_parent(folder)
                                {
                                    continue;
                                }
                                let depth = if query.is_empty() {
                                    folder.matches('/').count()
                                } else {
                                    0
                                };
                                let indent = depth as f32 * 12.0;
                                let label = if query.is_empty() {
                                    folder.rsplit('/').next().unwrap_or(folder)
                                } else {
                                    folder.as_str()
                                };
                                let child_prefix = format!("{folder}/");
                                let has_children =
                                    folders_snapshot.iter().any(|f| f.starts_with(&child_prefix));
                                let collapsed = query.is_empty()
                                    && self.notes_mgr.collapsed_folders.contains(folder);

                                let is_selected_folder = &self.notes_mgr.selected_folder == folder;
                                let is_folder_ai = self.notes_mgr.is_folder_ai_shared(folder);
                                let is_expanded = self.notes_mgr.expanded_folders.contains(folder)
                                    || !query.is_empty();

                                ui.push_id(("folder-row", folder), |ui| {
                                    ui.horizontal(
                                        |ui| {
                                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 2.0);
                                            ui.add_space(indent);
                                            // Branch chevron: ▸ folded, ▾ open.
                                            let (chev_rect, chev_resp) = ui.allocate_exact_size(
                                                Vec2::new(10.0, 16.0),
                                                Sense::click(),
                                            );
                                            if query.is_empty()
                                                && (has_children || !folder_notes.is_empty())
                                            {
                                                let c = chev_rect.center();
                                                let points = if collapsed {
                                                    vec![
                                                        c + Vec2::new(-2.0, -4.0),
                                                        c + Vec2::new(3.0, 0.0),
                                                        c + Vec2::new(-2.0, 4.0),
                                                    ]
                                                } else {
                                                    vec![
                                                        c + Vec2::new(-4.0, -2.0),
                                                        c + Vec2::new(4.0, -2.0),
                                                        c + Vec2::new(0.0, 3.0),
                                                    ]
                                                };
                                                ui.painter().add(egui::Shape::convex_polygon(
                                                    points,
                                                    if chev_resp.hovered() {
                                                        Color32::WHITE
                                                    } else {
                                                        theme::MUTED
                                                    },
                                                    Stroke::NONE,
                                                ));
                                                if chev_resp.clicked() {
                                                    folder_to_toggle_branch = Some(folder.clone());
                                                }
                                            }
                                            let (icon_rect, icon_resp) = ui.allocate_exact_size(
                                                Vec2::splat(16.0),
                                                Sense::click(),
                                            );
                                            theme::draw_icon(
                                                ui.painter(),
                                                icon_rect.center(),
                                                Glyph::Folder,
                                                if is_folder_ai {
                                                    Color32::from_rgb(100, 220, 175)
                                                } else if is_selected_folder {
                                                    Color32::WHITE
                                                } else {
                                                    theme::MUTED
                                                },
                                            );
                                            let name_w = (ui.available_width() - 52.0).max(50.0);
                                            let name_resp = ui
                                                .allocate_ui_with_layout(
                                                    Vec2::new(name_w, 22.0),
                                                    egui::Layout::left_to_right(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        ui.add(
                                                            egui::Label::new(
                                                                egui::RichText::new(
                                                                    if folder_notes.is_empty() {
                                                                        label.to_owned()
                                                                    } else {
                                                                        format!(
                                                                            "{label}  {}",
                                                                            folder_notes.len()
                                                                        )
                                                                    },
                                                                )
                                                                    .size(12.5)
                                                                    .color(if is_selected_folder {
                                                                        Color32::WHITE
                                                                    } else {
                                                                        Color32::from_gray(175)
                                                                    }),
                                                            )
                                                            .truncate()
                                                            .sense(Sense::click()),
                                                        )
                                                    },
                                                )
                                                .inner;
                                            if icon_resp.clicked() || name_resp.clicked() {
                                                self.notes_mgr.selected_folder = folder.clone();
                                                self.notes_mgr.collapsed_folders.remove(folder);
                                            }
                                            name_resp.context_menu(|ui| {
                                                compact_menu(ui);
                                                if ui.button("Tạo thư mục con").clicked() {
                                                    subfolder_parent = Some(folder.clone());
                                                    ui.close();
                                                }
                                                if ui.button("Mở thư mục trên máy").clicked() {
                                                    let _ = system::open(
                                                        &self
                                                            .notes_mgr
                                                            .root_dir
                                                            .join(folder)
                                                            .to_string_lossy(),
                                                    );
                                                    ui.close();
                                                }
                                            });
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.spacing_mut().item_spacing =
                                                        Vec2::new(2.0, 2.0);
                                                    if theme::mini_icon(
                                                        ui,
                                                        Glyph::Plus,
                                                        &format!(
                                                            "Tạo ghi chú mới trong thư mục «{folder}»"
                                                        ),
                                                        true,
                                                        false,
                                                    )
                                                    .clicked()
                                                    {
                                                        folder_to_create_note =
                                                            Some(folder.clone());
                                                    }
                                                    if theme::mini_icon(
                                                        ui,
                                                        Glyph::AiMemory,
                                                        if is_folder_ai {
                                                            "Đang bật AI cho thư mục này (tự động cập nhật dữ liệu mỗi khi có note)"
                                                        } else {
                                                            "Bật AI cho thư mục này (tự động cập nhật dữ liệu mỗi khi có note bên trong)"
                                                        },
                                                        true,
                                                        is_folder_ai,
                                                    )
                                                    .clicked()
                                                    {
                                                        folder_to_toggle_ai = Some(folder.clone());
                                                    }
                                                },
                                            );
                                        },
                                    );

                                    let total_notes = folder_notes.len();
                                    let visible_limit = if collapsed {
                                        0
                                    } else if is_expanded {
                                        total_notes
                                    } else {
                                        6
                                    };
                                    for (note_id, note_title, note_ai) in
                                        folder_notes.into_iter().take(visible_limit)
                                    {
                                        let selected = self.notes_mgr.selected_note_id.as_ref()
                                            == Some(&note_id);
                                        ui.horizontal(|ui| {
                                            ui.add_space(10.0 + indent);
                                            let row_w = ui.available_width();
                                            let (rect, resp) = ui.allocate_exact_size(
                                                Vec2::new(row_w, 26.0),
                                                Sense::click(),
                                            );
                                            if selected || resp.hovered() {
                                                ui.painter().rect_filled(
                                                    rect,
                                                    7.0,
                                                    if selected {
                                                        Color32::from_rgb(48, 50, 56)
                                                    } else {
                                                        Color32::from_white_alpha(12)
                                                    },
                                                );
                                            }
                                            let text_color = if selected {
                                                Color32::WHITE
                                            } else {
                                                Color32::from_gray(208)
                                            };
                                            let max_text_w = (rect.width()
                                                - if note_ai { 24.0 } else { 14.0 })
                                            .max(40.0);
                                            let mut job = egui::text::LayoutJob::single_section(
                                                note_title.clone(),
                                                egui::TextFormat {
                                                    font_id: egui::FontId::proportional(12.0),
                                                    color: text_color,
                                                    ..Default::default()
                                                },
                                            );
                                            job.wrap = egui::text::TextWrapping {
                                                max_width: max_text_w,
                                                max_rows: 1,
                                                break_anywhere: true,
                                                overflow_character: Some('…'),
                                            };
                                            let galley = ui.fonts_mut(|f| f.layout_job(job));
                                            ui.painter().galley(
                                                Pos2::new(
                                                    rect.left() + 8.0,
                                                    rect.center().y - galley.size().y * 0.5,
                                                ),
                                                galley,
                                                text_color,
                                            );
                                            if note_ai {
                                                ui.painter().circle_filled(
                                                    Pos2::new(rect.right() - 7.0, rect.center().y),
                                                    3.0,
                                                    Color32::from_rgb(100, 220, 175),
                                                );
                                            }
                                            if resp.clicked() {
                                                note_to_select = Some((note_id.clone(), folder.clone()));
                                            }
                                            resp.context_menu(|ui| {
                                                compact_menu(ui);
                                                if ui
                                                    .button(if note_ai {
                                                        "Tắt AI cho ghi chú này"
                                                    } else {
                                                        "Bật AI cho ghi chú này"
                                                    })
                                                    .clicked()
                                                {
                                                    note_to_toggle_ai = Some(note_id.clone());
                                                    ui.close();
                                                }
                                                ui.separator();
                                                ui.label(
                                                    egui::RichText::new("Chuyển sang thư mục")
                                                        .strong(),
                                                );
                                                for target_f in &folders_snapshot {
                                                    if ui
                                                        .selectable_label(
                                                            target_f == folder,
                                                            target_f,
                                                        )
                                                        .clicked()
                                                    {
                                                        note_to_move = Some((
                                                            note_id.clone(),
                                                            target_f.clone(),
                                                        ));
                                                        ui.close();
                                                    }
                                                }
                                            });
                                        });
                                    }

                                    if total_notes > 6 && query.is_empty() && !collapsed {
                                        ui.horizontal(|ui| {
                                            ui.add_space(18.0 + indent);
                                            let see_label = if is_expanded {
                                                "Thu gọn".to_owned()
                                            } else {
                                                format!("See all ({total_notes})")
                                            };
                                            if ui
                                                .add(
                                                    egui::Label::new(
                                                        egui::RichText::new(see_label)
                                                            .size(11.5)
                                                            .color(theme::MUTED),
                                                    )
                                                    .sense(Sense::click()),
                                                )
                                                .clicked()
                                            {
                                                folder_to_toggle_expand = Some(folder.clone());
                                            }
                                        });
                                    }
                                    ui.add_space(if collapsed { 2.0 } else { 6.0 });
                                });
                            }
                        });

                    if let Some(f) = folder_to_toggle_branch
                        && !self.notes_mgr.collapsed_folders.remove(&f)
                    {
                        self.notes_mgr.collapsed_folders.insert(f);
                    }
                    if let Some(parent) = subfolder_parent {
                        self.new_folder_input = format!("{parent}/");
                        self.folder_form = true;
                    }
                    if let Some(f) = folder_to_toggle_ai {
                        let enabled = self.notes_mgr.toggle_folder_ai(&f);
                        self.workspace.status = if enabled {
                            format!(
                                "Đã bật AI cho thư mục «{f}» · mỗi khi có note bên trong AI sẽ tự cập nhật dữ liệu."
                            )
                        } else {
                            format!("Đã tắt tự động cập nhật AI cho thư mục «{f}».")
                        };
                    }
                    if let Some(f) = folder_to_create_note {
                        self.notes_mgr.create_note_in_folder(&f, "Ghi chú mới", "");
                        self.pending_note_delete = false;
                    }
                    if let Some(f) = folder_to_toggle_expand {
                        if self.notes_mgr.expanded_folders.contains(&f) {
                            self.notes_mgr.expanded_folders.remove(&f);
                        } else {
                            self.notes_mgr.expanded_folders.insert(f);
                        }
                    }
                    if let Some((nid, f)) = note_to_select {
                        self.notes_mgr.selected_note_id = Some(nid);
                        self.notes_mgr.selected_folder = f;
                        self.pending_note_delete = false;
                    }
                    if let Some(nid) = note_to_toggle_ai
                        && let Some(pos) = self.notes_mgr.notes.iter().position(|n| n.id == nid)
                    {
                        self.notes_mgr.notes[pos].ai_shared = !self.notes_mgr.notes[pos].ai_shared;
                        let prev = self.notes_mgr.selected_note_id.clone();
                        self.notes_mgr.selected_note_id = Some(nid);
                        self.notes_mgr.save_selected_note();
                        self.notes_mgr.selected_note_id = prev;
                    }
                    if let Some((nid, target_f)) = note_to_move {
                        self.notes_mgr.selected_note_id = Some(nid);
                        self.notes_mgr.move_selected_note_to_folder(&target_f);
                        self.notes_mgr.selected_folder = target_f;
                    }
                });
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::PANEL).inner_margin(10))
            .show(viewport_ui, |ui| {
                let index = self
                    .notes_mgr
                    .selected_note_id
                    .as_ref()
                    .and_then(|id| self.notes_mgr.notes.iter().position(|n| &n.id == id));

                // Thanh icon mini: một hàng, tự xuống dòng khi cửa sổ hẹp để không bị cắt mất nút
                let mut trigger_format: Option<&'static str> = None;
                let mut trigger_sync_kanban = false;
                let mut trigger_pick_files = false;

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(2.5, 2.0);
                    if theme::mini_icon(
                        ui,
                        Glyph::File,
                        "Ghi chú (Note)",
                        true,
                        !self.workspace.board_open,
                    )
                    .clicked()
                    {
                        self.workspace.board_open = false;
                        self.workspace.profile_open = false;
                    }
                    if theme::mini_icon(
                        ui,
                        Glyph::Kanban,
                        "Bảng công việc Kanban",
                        true,
                        self.workspace.board_open,
                    )
                    .clicked()
                    {
                        self.workspace.board_open = true;
                        self.workspace.profile_open = false;
                    }

                    if !self.workspace.board_open
                        && let Some(idx) = index
                    {
                        ui.separator();
                        let note_folder = self.notes_mgr.notes[idx].folder.clone();
                        let note_icon = self.notes_mgr.notes[idx].icon.clone();
                        let shared = self.notes_mgr.notes[idx].ai_shared;
                        let attachments_len = self.notes_mgr.notes[idx].attachments.len();
                        let linked = self
                            .notes_mgr
                            .notes[idx]
                            .task_card_id
                            .as_ref()
                            .is_some_and(|id| {
                                self.workspace.board.cards.iter().any(|card| &card.id == id)
                            });

                        let icon_btn = theme::mini_icon(
                            ui,
                            theme::glyph_from_name(&note_icon),
                            &format!(
                                "Biểu tượng & Thư mục ({note_folder}) · Chuột phải/trái để đổi"
                            ),
                            true,
                            false,
                        );
                        if icon_btn.clicked() {
                            let cur_pos = theme::NOTE_ICONS
                                .iter()
                                .position(|(k, _, _)| *k == note_icon)
                                .unwrap_or(0);
                            let next_key =
                                theme::NOTE_ICONS[(cur_pos + 1) % theme::NOTE_ICONS.len()].0;
                            self.notes_mgr.set_selected_note_icon(next_key);
                        }
                        icon_btn.context_menu(|ui| {
                            compact_menu(ui);
                            ui.label(egui::RichText::new("Biểu tượng").strong());
                            let mut next_icon = None;
                            for (key, _, label) in theme::NOTE_ICONS {
                                if ui.selectable_label(note_icon == *key, *label).clicked() {
                                    next_icon = Some((*key).to_string());
                                    ui.close();
                                }
                            }
                            if let Some(chosen) = next_icon {
                                self.notes_mgr.set_selected_note_icon(&chosen);
                            }
                            ui.separator();
                            ui.label(egui::RichText::new("Chuyển thư mục").strong());
                            let mut next_folder = None;
                            for f in &self.notes_mgr.folders {
                                if ui.selectable_label(note_folder == *f, f).clicked() {
                                    next_folder = Some(f.clone());
                                    ui.close();
                                }
                            }
                            if let Some(chosen) = next_folder {
                                self.notes_mgr.move_selected_note_to_folder(&chosen);
                            }
                        });

                        if theme::mini_icon(
                            ui,
                            Glyph::AiMemory,
                            if shared {
                                "Tắt liên kết & ghi nhớ AI cho ghi chú này"
                            } else {
                                "Bật liên kết & ghi nhớ AI cho ghi chú này"
                            },
                            true,
                            shared,
                        )
                        .clicked()
                        {
                            self.notes_mgr.notes[idx].ai_shared = !shared;
                            self.notes_mgr.save_selected_note();
                        }

                        if theme::mini_icon(
                            ui,
                            Glyph::Check,
                            if linked {
                                "Đồng bộ các công việc trong ghi chú sang Kanban"
                            } else {
                                "Tạo công việc Kanban từ ghi chú này"
                            },
                            self.workspace.storage_ready,
                            linked,
                        )
                        .clicked()
                        {
                            trigger_sync_kanban = true;
                        }

                        ui.separator();

                        for (glyph, marker, help) in [
                            (Glyph::Heading1, "# ", "Tiêu đề lớn (#)"),
                            (Glyph::Heading2, "## ", "Tiêu đề nhỏ (##)"),
                            (Glyph::Bold, "**", "In đậm (**)"),
                            (Glyph::Italic, "*", "In nghiêng (*)"),
                        ] {
                            if theme::mini_icon(ui, glyph, help, !self.workspace.preview, false)
                                .clicked()
                            {
                                trigger_format = Some(marker);
                            }
                        }

                        if theme::mini_icon(
                            ui,
                            if self.workspace.preview {
                                Glyph::Edit
                            } else {
                                Glyph::Eye
                            },
                            if self.workspace.preview {
                                "Chuyển sang Sửa Markdown"
                            } else {
                                "Chuyển sang Xem định dạng"
                            },
                            true,
                            self.workspace.preview,
                        )
                        .clicked()
                        {
                            self.workspace.preview = !self.workspace.preview;
                        }

                        let attach_btn = theme::mini_icon(
                            ui,
                            Glyph::Attach,
                            &format!(
                                "Đính kèm tệp ({attachments_len}) · Chuột trái thêm, chuột phải mở tệp"
                            ),
                            true,
                            attachments_len > 0,
                        );
                        if attach_btn.clicked() {
                            trigger_pick_files = true;
                        }
                        let attachments_snapshot = self.notes_mgr.notes[idx].attachments.clone();
                        attach_btn.context_menu(|ui| {
                            compact_menu(ui);
                            if ui.button("Thêm tệp đính kèm…").clicked() {
                                let paths = system::pick_files();
                                self.notes_mgr.upload_files_to_selected(&paths);
                                ui.close();
                            }
                            if !attachments_snapshot.is_empty() {
                                ui.separator();
                                let mut removed = None;
                                for att in &attachments_snapshot {
                                    ui.horizontal(|ui| {
                                        if ui.small_button(&att.name).clicked() {
                                            let path =
                                                self.notes_mgr.root_dir.join(&att.relative_path);
                                            let _ = system::open(&path.to_string_lossy());
                                            ui.close();
                                        }
                                        if ui.small_button("×").clicked() {
                                            let path =
                                                self.notes_mgr.root_dir.join(&att.relative_path);
                                            removed = Some((att.relative_path.clone(), path));
                                        }
                                    });
                                }
                                if let Some((relative, path)) = removed {
                                    self.notes_mgr.notes[idx]
                                        .attachments
                                        .retain(|a| a.relative_path != relative);
                                    let _ = std::fs::remove_file(path);
                                }
                            }
                        });

                        if theme::mini_icon(
                            ui,
                            Glyph::Trash,
                            "Xóa ghi chú đang mở",
                            true,
                            self.pending_note_delete,
                        )
                        .clicked()
                        {
                            self.pending_note_delete = !self.pending_note_delete;
                        }
                    }
                });

                if self.workspace.board_open {
                    self.kanban_ui(ui);
                    return;
                }

                let Some(index) = index else {
                    ui.add_space(40.0);
                    ui.heading("Một nơi cho ý tưởng của anh");
                    theme::caption(ui, "Tạo ghi chú mới để bắt đầu.");
                    return;
                };

                let note_id = self.notes_mgr.notes[index].id.clone();
                let note_folder = self.notes_mgr.notes[index].folder.clone();
                let note_title = self.notes_mgr.notes[index].title.clone();
                let note_updated = self.notes_mgr.notes[index].updated_at.clone();
                let editor_id = ui.make_persistent_id(("note-content", &note_id));

                if let Some(marker) = trigger_format {
                    let mut state =
                        egui::text_edit::TextEditState::load(&ctx, editor_id).unwrap_or_default();
                    let range = state.cursor.char_range();
                    let start = range.map_or(0, |range| usize::from(range.primary.index));
                    let end = range.map_or(start, |range| usize::from(range.secondary.index));
                    let previous_length = self.notes_mgr.notes[index].content.chars().count();
                    crate::workspace::format_selection(
                        &mut self.notes_mgr.notes[index].content,
                        start,
                        end,
                        marker,
                    );
                    let current_length = self.notes_mgr.notes[index].content.chars().count();
                    let cursor = start
                        .max(end)
                        .saturating_add(current_length)
                        .saturating_sub(previous_length)
                        .min(current_length);
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
                        egui::text::CCursor::new(cursor),
                    )));
                    state.store(&ctx, editor_id);
                    self.notes_mgr.save_selected_note();
                    ui.memory_mut(|memory| memory.request_focus(editor_id));
                }
                if trigger_sync_kanban {
                    self.sync_note_to_kanban(index);
                }
                if trigger_pick_files {
                    let paths = system::pick_files();
                    self.notes_mgr.upload_files_to_selected(&paths);
                }

                if self.pending_note_delete {
                    let mut remove = false;
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Xóa ghi chú này?").size(11.5));
                        remove = ui.small_button("Xóa").clicked();
                        if ui.small_button("Hủy").clicked() {
                            self.pending_note_delete = false;
                        }
                    });
                    if remove {
                        self.notes_mgr.delete_selected_note();
                        self.pending_note_delete = false;
                        return;
                    }
                }

                // Khung điền ghi chú kéo dài toàn bộ chiều ngang & chiều dọc, phong cách LIST (Ảnh 3)
                let avail_w = ui.available_width();
                let editor_height = (ui.available_height() - 4.0).max(140.0);
                let mut changed = false;

                egui::Frame::new()
                    .fill(Color32::from_rgb(24, 24, 26))
                    .stroke(Stroke::new(1.0, theme::BORDER))
                    .corner_radius(10)
                    .inner_margin(egui::Margin {
                        left: 30,
                        right: 12,
                        top: 8,
                        bottom: 8,
                    })
                    .show(ui, |ui| {
                        ui.set_width((avail_w - 42.0).max(120.0));
                        ui.set_min_height(editor_height - 16.0);
                        let pad_rect = Rect::from_min_size(
                            ui.max_rect().min - Vec2::new(30.0, 8.0),
                            Vec2::new(avail_w, editor_height),
                        );
                        theme::draw_lined_notepad_bg(ui.painter(), pad_rect, 26.0);

                        // Tiêu đề LIST tối giản căn giữa phía trên kèm tên thư mục
                        ui.vertical_centered(|ui| {
                            let header_text = format!(
                                "L I S T  ·  {} / {}",
                                note_folder.to_uppercase(),
                                note_title.chars().take(28).collect::<String>().to_uppercase()
                            );
                            ui.label(
                                egui::RichText::new(header_text)
                                    .size(10.0)
                                    .color(Color32::from_white_alpha(130)),
                            );
                        });
                        ui.add_space(4.0);

                        egui::ScrollArea::vertical()
                            .id_salt("note-editor-scroll")
                            .auto_shrink([false, false])
                            .max_height((editor_height - 40.0).max(80.0))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.set_min_width(ui.available_width());
                                if self.workspace.preview {
                                    theme::render_markdown(
                                        ui,
                                        &self.notes_mgr.notes[index].content,
                                    );
                                } else {
                                    changed = ui
                                        .add_sized(
                                            [
                                                ui.available_width(),
                                                (editor_height - 46.0).max(120.0),
                                            ],
                                            egui::TextEdit::multiline(
                                                &mut self.notes_mgr.notes[index].content,
                                            )
                                            .id(editor_id)
                                            .frame(egui::Frame::NONE)
                                            .font(egui::FontId::proportional(13.5))
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(10)
                                            .hint_text(
                                                "Viết ghi chú hoặc gõ @note để AI tạo nội dung & danh sách công việc…",
                                            ),
                                        )
                                        .changed();
                                }

                                // Hiển thị thông tin cập nhật & trạng thái gọn nhẹ ở cuối trang
                                ui.add_space(6.0);
                                let mut meta_line = format!("Đã lưu · {note_updated}");
                                if !self.workspace.status.is_empty() {
                                    meta_line.push_str("  ·  ");
                                    meta_line.push_str(&self.workspace.status);
                                }
                                if let Some(status) = &self.notes_mgr.status_message {
                                    meta_line.push_str("  ·  ");
                                    meta_line.push_str(status);
                                }
                                ui.label(
                                    egui::RichText::new(meta_line)
                                        .size(9.5)
                                        .color(Color32::from_white_alpha(90)),
                                );
                            });
                    });
                if changed {
                    self.notes_mgr.save_selected_note();
                }
            });

        if !maximized {
            // Window inner_rect uses desktop coordinates; UI hit targets must
            // use viewport-local coordinates even after moving the window.
            let rect = ctx.viewport_rect();
            let handle = Rect::from_min_max(rect.max - Vec2::splat(22.0), rect.max);
            let response = viewport_ui
                .interact(handle, viewport_ui.id().with("notes_resize"), Sense::drag())
                .on_hover_cursor(egui::CursorIcon::ResizeNwSe)
                .on_hover_text("Kéo góc để phóng to hoặc thu nhỏ");
            for offset in [5.0, 10.0, 15.0] {
                viewport_ui.painter().line_segment(
                    [
                        handle.max - Vec2::new(offset, 3.0),
                        handle.max - Vec2::new(3.0, offset),
                    ],
                    Stroke::new(1.5, theme::MUTED),
                );
            }
            if response.is_pointer_button_down_on()
                && ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary))
            {
                ctx.send_viewport_cmd(ViewportCommand::BeginResize(
                    egui::ResizeDirection::SouthEast,
                ));
            }
        }
        open
    }

    pub(super) fn search_ui(&mut self, viewport_ui: &mut egui::Ui) -> bool {
        let mut open = true;

        self.search_state.update_search();

        let ctx = viewport_ui.ctx().clone();
        let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.is_none());
        if dismissed(&ctx) {
            open = false;
        }

        let show_results =
            !self.search_state.query.trim().is_empty() && !self.search_state.results.is_empty();
        let pill_bg = Color32::from_rgb(26, 26, 26);
        let pill_border = Stroke::new(1.0, Color32::from_white_alpha(35));

        egui::CentralPanel::default()
            .frame(if show_results {
                palette_frame()
            } else {
                egui::Frame::NONE
            })
            .show(viewport_ui, |ui| {
                // Thanh pill tìm kiếm cố định (Giữ focus 100%, không bị dãn ra khi chưa có kết quả)
                egui::Frame::new()
                    .fill(pill_bg)
                    .stroke(pill_border)
                    .corner_radius(24)
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Nút tròn kính lúp bên trái
                            let (icon_rect, icon_resp) =
                                ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
                            let icon_hover = icon_resp.hovered();
                            ui.painter().circle_filled(
                                icon_rect.center(),
                                14.0,
                                if icon_hover {
                                    Color32::from_gray(55)
                                } else {
                                    Color32::from_gray(38)
                                },
                            );
                            let c = icon_rect.center() - Vec2::new(1.0, 1.0);
                            let lens_color = Color32::from_gray(if icon_hover { 255 } else { 200 });
                            ui.painter()
                                .circle_stroke(c, 5.0, Stroke::new(1.5, lens_color));
                            ui.painter().line_segment(
                                [c + Vec2::new(3.5, 3.5), c + Vec2::new(7.0, 7.0)],
                                Stroke::new(1.5, lens_color),
                            );

                            // Ô nhập tìm kiếm (Duy nhất 1 TextEdit, luôn giữ focus)
                            let edit = ui.add_sized(
                                [ui.available_width() - 36.0, 32.0],
                                egui::TextEdit::singleline(&mut self.search_state.query)
                                    .id_salt("spotlight-input")
                                    .frame(egui::Frame::NONE)
                                    .margin(4)
                                    .font(egui::FontId::proportional(14.5))
                                    .hint_text("Tìm kiếm tệp, ứng dụng..."),
                            );

                            if self.search_focus > 0 {
                                system::focus_window_by_title("PentaKuru · Tìm kiếm");
                                ui.ctx().send_viewport_cmd(ViewportCommand::Focus);
                                edit.request_focus();
                                if edit.has_focus() {
                                    self.search_focus = 0;
                                } else {
                                    self.search_focus = self.search_focus.saturating_sub(1);
                                }
                            }
                            if edit.changed() {
                                self.search_state.update_search();
                                self.search_selection = 0;
                            }

                            // Nút tròn bên phải (Xóa tìm kiếm nếu có chữ, hoặc Đóng Esc nếu rỗng)
                            let (btn_rect, btn_resp) =
                                ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
                            let bh = btn_resp.hovered();
                            let bc = btn_rect.center();
                            ui.painter().circle_filled(
                                bc,
                                14.0,
                                if bh {
                                    Color32::from_gray(55)
                                } else {
                                    Color32::from_gray(38)
                                },
                            );
                            let bs =
                                Stroke::new(1.4, Color32::from_gray(if bh { 240 } else { 180 }));
                            ui.painter()
                                .line_segment([bc - Vec2::splat(3.5), bc + Vec2::splat(3.5)], bs);
                            ui.painter().line_segment(
                                [bc + Vec2::new(-3.5, 3.5), bc + Vec2::new(3.5, -3.5)],
                                bs,
                            );

                            if !self.search_state.query.is_empty() {
                                if btn_resp.on_hover_text("Xóa tìm kiếm").clicked() {
                                    self.search_state.query.clear();
                                    self.search_state.update_search();
                                }
                            } else {
                                if btn_resp.on_hover_text("Đóng · Esc").clicked() {
                                    open = false;
                                }
                            }
                        });
                    });

                // Danh sách kết quả (Chỉ hiển thị khi có kết quả tìm kiếm)
                if show_results {
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(2.0);

                    // Phím mũi tên lên / xuống
                    let results_len = self.search_state.results.len();
                    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown))
                    {
                        self.search_selection =
                            (self.search_selection + 1).min(results_len.min(5).saturating_sub(1));
                    }
                    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
                        self.search_selection = self.search_selection.saturating_sub(1);
                    }

                    let results: Vec<_> =
                        self.search_state.results.iter().take(5).cloned().collect();
                    for (index, item) in results.iter().enumerate() {
                        let is_selected = self.search_selection == index;
                        let response = egui::Frame::new()
                            .fill(if is_selected {
                                theme::SURFACE
                            } else {
                                theme::PANEL
                            })
                            .corner_radius(10)
                            .inner_margin(8)
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    let (tag_text, tag_color) = if item.size == "[UI]"
                                        || item.name.starts_with("Thực thi UI:")
                                    {
                                        ("[UI]", Color32::from_rgb(110, 205, 255))
                                    } else if item.kind == search::SearchKind::Folder {
                                        ("[Thư mục]", Color32::from_rgb(220, 190, 100))
                                    } else if item.kind == search::SearchKind::WebLink {
                                        ("[Web]", Color32::from_rgb(100, 200, 220))
                                    } else {
                                        ("[Tệp]", theme::MUTED)
                                    };

                                    ui.label(
                                        egui::RichText::new(tag_text)
                                            .small()
                                            .strong()
                                            .color(tag_color),
                                    );

                                    let has_snip = item.snippet.is_some();
                                    let text_width = (ui.available_width() - 44.0).max(100.0);
                                    let text_height = if has_snip { 42.0 } else { 34.0 };
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(text_width, text_height),
                                        egui::Layout::top_down(egui::Align::Min),
                                        |ui| {
                                            ui.set_min_width(text_width);
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&item.name).strong(),
                                                )
                                                .truncate(),
                                            );
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(format!(
                                                        "{}  {}",
                                                        item.path, item.size
                                                    ))
                                                    .size(11.0)
                                                    .color(theme::MUTED),
                                                )
                                                .truncate(),
                                            );
                                            if let Some(snip) = &item.snippet {
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(format!("↳ {snip}"))
                                                            .size(11.0)
                                                            .color(theme::TEXT),
                                                    )
                                                    .truncate(),
                                                );
                                            }
                                        },
                                    );
                                    if ui
                                        .small_button("…")
                                        .on_hover_text("Mở thư mục chứa tệp")
                                        .clicked()
                                    {
                                        let _ = self.search_state.reveal_item(item);
                                    }
                                });
                            })
                            .response
                            .interact(Sense::click());

                        if response.clicked() {
                            let q_trim = self.search_state.query.trim();
                            if ai::note_command(q_trim).is_some() {
                                self.ai_prompt = q_trim.to_string();
                                self.search_open = false;
                                self.ai_open = true;
                                self.send_ai_prompt(&ctx);
                            } else {
                                let _ = self.search_state.open_item(item);
                            }
                            open = false;
                        }
                    }

                    if enter_pressed {
                        let q_trim = self.search_state.query.trim();
                        if ai::note_command(q_trim).is_some() {
                            self.ai_prompt = q_trim.to_string();
                            self.search_open = false;
                            self.ai_open = true;
                            self.send_ai_prompt(&ctx);
                            open = false;
                        } else if let Some(item) = results.get(self.search_selection) {
                            let _ = self.search_state.open_item(item);
                            open = false;
                        }
                    }
                } else if enter_pressed
                    && ai::note_command(self.search_state.query.trim()).is_some()
                {
                    // Cho phép Enter kích hoạt @note ngay cả khi không có danh sách
                    self.ai_prompt = self.search_state.query.trim().to_string();
                    self.search_open = false;
                    self.ai_open = true;
                    self.send_ai_prompt(&ctx);
                    open = false;
                }
            });

        open
    }

    pub(super) fn ai_reply_bubble_visible(&self) -> bool {
        !self.ai_history_expanded
            && !self.ai_bubble_dismissed
            && (!self.ai_history.is_empty() || self.ai_job.is_some() || self.ai_status.is_some())
    }

    pub(super) fn ai_reply_bubble_ui(&mut self, viewport_ui: &mut egui::Ui) {
        let ctx = viewport_ui.ctx().clone();
        if !egui::Popup::is_any_open(&ctx)
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            if self.ai_job.is_some() {
                self.cancel_ai();
            } else {
                self.ai_bubble_dismissed = true;
            }
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(viewport_ui, |ui| {
                self.ai_reply_bubble_contents(ui);
            });
    }

    pub(super) fn ai_reply_bubble_contents(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let preview = self.ai_bubble_preview();
        let busy =
            self.ai_job.is_some() || self.music_mcp_job.is_some() || self.clip_mcp_job.is_some();
        let max_w = ui.available_width().max(180.0);

        let mut bubble_menu_open = false;
        let mut expand_requested = false;
        let mut dismiss_requested = false;
        let mut reset_requested = false;
        let mut save_reply = None;
        let mut stop_requested = false;

        let card = egui::Frame::new()
            .fill(Color32::from_rgba_unmultiplied(24, 24, 28, 242))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(255, 255, 255, 36),
            ))
            .corner_radius(22)
            .inner_margin(egui::Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.set_width((max_w - 28.0).max(120.0));
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 4.0);

                ui.horizontal(|ui| {
                    let (badge_rect, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                    match preview.tone {
                        BubbleTone::Busy => {
                            ui.scope_builder(egui::UiBuilder::new().max_rect(badge_rect), |ui| {
                                ui.add(
                                    egui::Spinner::new()
                                        .size(15.0)
                                        .color(Color32::from_rgb(96, 165, 250)),
                                );
                            });
                        }
                        BubbleTone::Error => {
                            let c = badge_rect.center();
                            ui.painter()
                                .circle_filled(c, 8.0, Color32::from_rgb(239, 68, 68));
                            ui.painter().text(
                                c,
                                egui::Align2::CENTER_CENTER,
                                "!",
                                egui::FontId::proportional(11.5),
                                Color32::WHITE,
                            );
                        }
                        BubbleTone::Success => {
                            let c = badge_rect.center();
                            ui.painter()
                                .circle_filled(c, 8.0, Color32::from_rgb(34, 197, 94));
                            let s = Stroke::new(1.8, Color32::WHITE);
                            ui.painter().line_segment(
                                [c + Vec2::new(-3.6, 0.2), c + Vec2::new(-1.0, 2.8)],
                                s,
                            );
                            ui.painter().line_segment(
                                [c + Vec2::new(-1.0, 2.8), c + Vec2::new(3.8, -2.8)],
                                s,
                            );
                        }
                    }

                    let speaking = self.tts_player.is_speaking();
                    let actions_reserve = if busy || speaking || !self.ai_history.is_empty() {
                        48.0
                    } else {
                        24.0
                    };
                    let title_w = (ui.available_width() - actions_reserve).max(40.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(title_w, 20.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.set_max_width(title_w);
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&preview.title)
                                        .size(13.5)
                                        .strong()
                                        .color(theme::TEXT),
                                )
                                .truncate(),
                            );
                        },
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if bubble_micro_icon(ui, BubbleIcon::Close, "Ẩn bong bóng · Esc").clicked()
                        {
                            dismiss_requested = true;
                        }
                        if busy {
                            if bubble_micro_icon(ui, BubbleIcon::Stop, "Dừng tác vụ").clicked()
                            {
                                stop_requested = true;
                            }
                        } else if speaking {
                            if bubble_micro_icon(ui, BubbleIcon::Stop, "Dừng giọng đọc").clicked()
                            {
                                self.tts_player.stop();
                            }
                        } else if !self.ai_history.is_empty()
                            && bubble_micro_icon(ui, BubbleIcon::Expand, "Mở rộng hội thoại")
                                .clicked()
                        {
                            expand_requested = true;
                        }
                    });
                });

                if !preview.body.is_empty() {
                    ui.horizontal(|ui| {
                        ui.add_space(26.0);
                        ui.vertical(|ui| {
                            ui.set_max_width((max_w - 54.0).max(100.0));
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&preview.body)
                                        .size(13.0)
                                        .color(Color32::from_rgb(205, 207, 214)),
                                )
                                .wrap(),
                            );
                            if preview.truncated
                                && ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Xem đầy đủ →")
                                                .size(11.5)
                                                .color(Color32::from_rgb(125, 178, 255)),
                                        )
                                        .frame(false),
                                    )
                                    .clicked()
                            {
                                expand_requested = true;
                            }
                            if preview.tone == BubbleTone::Error
                                && (preview.body.contains("Key")
                                    || preview.body.contains("Cài đặt"))
                                && ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Mở Cài đặt (Alt + Q) →")
                                                .size(11.5)
                                                .color(Color32::from_rgb(125, 178, 255)),
                                        )
                                        .frame(false),
                                    )
                                    .clicked()
                            {
                                self.settings_open = true;
                            }
                        });
                    });
                }
            });

        let measured = (card.response.rect.height() + 2.0).clamp(60.0, 180.0);
        if (measured - self.ai_bubble_height).abs() > 1.0 {
            self.ai_bubble_height = measured;
            ctx.request_repaint();
        }

        let reply_to_copy = preview
            .full_reply
            .clone()
            .unwrap_or_else(|| preview.body.clone());
        let can_save = preview.full_reply.is_some();
        show_context_menu(
            &card.response,
            egui::Id::new("ai-bubble-context-menu"),
            |ui| {
                bubble_menu_open = true;
                compact_menu(ui);
                if !self.ai_history.is_empty() && ui.button("Mở rộng hội thoại").clicked() {
                    expand_requested = true;
                    ui.close();
                }
                if !reply_to_copy.is_empty() && ui.button("Sao chép câu trả lời").clicked() {
                    ctx.copy_text(reply_to_copy.clone());
                    ui.close();
                }
                if can_save
                    && ui.button("Thêm vào ghi chú đang chọn").clicked()
                    && let Some(full) = preview.full_reply.clone()
                {
                    save_reply = Some(full);
                    ui.close();
                }
                if ui
                    .add_enabled(!busy, egui::Button::new("Hội thoại mới"))
                    .clicked()
                {
                    reset_requested = true;
                    ui.close();
                }
                if self.tts_player.is_speaking() && ui.button("Dừng giọng đọc").clicked() {
                    self.tts_player.stop();
                    ui.close();
                }
                if ui.button("Cài đặt AI").clicked() {
                    self.settings_open = true;
                    ui.close();
                }
                if ui.button("Ẩn bong bóng").clicked() {
                    dismiss_requested = true;
                    ui.close();
                }
            },
        );
        if bubble_menu_open != self.ai_bubble_menu_open {
            ctx.request_repaint();
        }
        self.ai_bubble_menu_open = bubble_menu_open;

        if stop_requested {
            self.cancel_ai();
        }
        if dismiss_requested {
            self.ai_bubble_dismissed = true;
        }
        if expand_requested {
            self.ai_history_expanded = true;
            self.ai_open = true;
        }
        if reset_requested {
            self.reset_chat();
        }
        if let Some(reply) = save_reply {
            self.ai_status = Some(match self.notes_mgr.append_ai_reply(&reply) {
                Ok(message) => {
                    self.notes_open = true;
                    self.workspace.preview = true;
                    message
                }
                Err(error) => error,
            });
        }
    }

    fn ai_bubble_preview(&self) -> BubblePreview {
        let latest_user = self
            .ai_history
            .iter()
            .rev()
            .find(|m| m.role == ai::Role::User)
            .map(|m| clean_prompt_title(&m.text));
        let latest_assistant = self
            .ai_history
            .iter()
            .rev()
            .find(|m| m.role == ai::Role::Assistant);

        if self.ai_job.is_some() || self.music_mcp_job.is_some() || self.clip_mcp_job.is_some() {
            return BubblePreview {
                tone: BubbleTone::Busy,
                title: latest_user.unwrap_or_else(|| "Đang xử lý…".into()),
                body: self
                    .ai_status
                    .clone()
                    .unwrap_or_else(|| "AI đang suy nghĩ…".into()),
                truncated: false,
                full_reply: None,
            };
        }

        if let Some(status) = &self.ai_status {
            let is_err = status.starts_with("Lỗi")
                || status.starts_with("Chưa ")
                || status.contains("Chưa cấu hình")
                || status.contains("Bật Penta Music")
                || status.contains("Không ")
                || status.starts_with("Nhập ");
            if is_err || latest_assistant.is_none() {
                let (body, truncated) = compact_bubble_text(status, 3, 210);
                return BubblePreview {
                    tone: if is_err {
                        BubbleTone::Error
                    } else {
                        BubbleTone::Success
                    },
                    title: if is_err {
                        "Thông báo AI".into()
                    } else {
                        latest_user.unwrap_or_else(|| "PentaKuru".into())
                    },
                    body,
                    truncated,
                    full_reply: None,
                };
            }
        }

        if let Some(reply) = latest_assistant {
            let is_err =
                reply.text.starts_with("Lỗi:") || reply.text.starts_with("Không thực hiện được");
            let (mut body, mut truncated) = compact_bubble_text(&reply.text, 4, 220);
            if !reply.images.is_empty() {
                truncated = true;
            }
            if let Some(status) = &self.ai_status {
                body = format!("{body}\n{status}");
            }
            return BubblePreview {
                tone: if is_err {
                    BubbleTone::Error
                } else {
                    BubbleTone::Success
                },
                title: latest_user.unwrap_or_else(|| "PentaKuru".into()),
                body,
                truncated,
                full_reply: Some(reply.text.clone()),
            };
        }

        BubblePreview {
            tone: BubbleTone::Success,
            title: latest_user.unwrap_or_else(|| "PentaKuru".into()),
            body: String::new(),
            truncated: false,
            full_reply: None,
        }
    }

    pub(super) fn ai_ui(&mut self, viewport_ui: &mut egui::Ui) -> bool {
        let mut open = true;
        let ctx = viewport_ui.ctx().clone();
        let busy = self.ai_job.is_some();
        if ctx.input(|i| i.viewport().close_requested()) {
            open = false;
        }
        // Escape dismisses a menu first, then stops a task, then collapses/closes the dock.
        if !egui::Popup::is_any_open(&ctx)
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            if busy {
                self.cancel_ai();
            } else if self.ai_history_expanded {
                self.ai_history_expanded = false;
            } else {
                open = false;
            }
        }
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            let path = file.path().to_path_buf();
            if path.is_file() && !self.ai_attached_files.contains(&path) {
                self.ai_attached_files.push(path);
            }
        }

        let expanded = self.ai_history_expanded;
        let mut reset = false;
        let mut attach = false;
        let mut save_reply = None;

        let panel = egui::CentralPanel::default()
            .frame(if expanded {
                egui::Frame::new()
                    .fill(theme::PANEL)
                    .stroke(Stroke::new(1.0, theme::BORDER))
                    .corner_radius(16)
                    .inner_margin(10)
            } else {
                egui::Frame::NONE.inner_margin(10)
            })
            .show(viewport_ui, |ui| self.ai_contents(ui));

        let (inner_open, inner_reset, inner_attach, inner_save) = panel.inner;
        if !inner_open {
            open = false;
        }
        reset |= inner_reset;
        attach |= inner_attach;
        save_reply = save_reply.or(inner_save);

        let background = panel.response;
        if matches!(
            self.ui_preview.as_deref(),
            Some("ai-menu" | "ai-menu-compact")
        ) {
            egui::Popup::context_menu(&background)
                .at_position(Pos2::new(240.0, 52.0))
                .open(true)
                .show(|ui| self.chat_menu(ui, &mut open, &mut reset));
        } else {
            show_context_menu(&background, egui::Id::new("chat-background-menu"), |ui| {
                self.chat_menu(ui, &mut open, &mut reset)
            });
        }
        // Native popups share their parent's surface. Give the compact dock a
        // transparent extension while a menu is open, then restore its height.
        let menu_open = egui::Popup::is_any_open(&ctx);
        if menu_open != self.ai_menu_open {
            ctx.request_repaint();
        }
        self.ai_menu_open = menu_open;
        if reset {
            self.reset_chat();
        }
        if let Some(reply) = save_reply {
            self.ai_status = Some(match self.notes_mgr.append_ai_reply(&reply) {
                Ok(message) => {
                    self.notes_open = true;
                    self.workspace.preview = true;
                    message
                }
                Err(error) => error,
            });
        }
        if attach {
            for path in system::pick_files() {
                if !self.ai_attached_files.contains(&path) {
                    self.ai_attached_files.push(path);
                }
            }
        }
        open
    }

    pub(super) fn ai_contents(&mut self, ui: &mut egui::Ui) -> (bool, bool, bool, Option<String>) {
        let ctx = ui.ctx().clone();
        let busy = self.ai_job.is_some();
        let expanded = self.ai_history_expanded;
        let plain_enter = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        let note_mode = ai::note_command(&self.ai_prompt).is_some();
        let mut open = true;
        let mut reset = false;
        let mut attach = false;
        let mut save_reply = None;
        let mut music_picker_requested = false;

        if !expanded {
            let shared = self
                .notes_mgr
                .notes
                .iter()
                .filter(|note| note.ai_shared)
                .count();
            let has_strip = shared > 0 || !self.ai_attached_files.is_empty() || note_mode;
            let height = composer_height(&self.ai_prompt, (ui.available_width() - 84.0).max(1.0))
                + 20.0
                + if has_strip { 20.0 } else { 0.0 };
            let rect = Rect::from_min_size(
                ui.max_rect().min - Vec2::splat(10.0),
                Vec2::new(ui.available_width() + 20.0, height),
            );
            ui.painter().rect(
                rect,
                26,
                Color32::from_rgba_unmultiplied(24, 24, 28, 242),
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 36)),
                egui::StrokeKind::Inside,
            );
            if has_strip {
                let mut tags = Vec::new();
                if note_mode {
                    tags.push("@note Tạo ghi chú".to_owned());
                }
                if shared > 0 {
                    tags.push(format!("{shared} note"));
                }
                if !self.ai_attached_files.is_empty() {
                    tags.push(format!("{} tệp", self.ai_attached_files.len()));
                }
                ui.label(
                    egui::RichText::new(format!("AI · {}", tags.join(" · ")))
                        .small()
                        .color(Color32::from_rgb(100, 220, 175)),
                );
            }
        }

        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);

        // Composer row: `[+] [ TextEdit ] [↑ / Stop]`
        ui.horizontal_top(|ui| {
            let plus_hint = if !self.ai_attached_files.is_empty() {
                format!(
                    "Tùy chọn & tệp ({} tệp đính kèm)",
                    self.ai_attached_files.len()
                )
            } else if note_mode {
                "Tùy chọn · đang ở chế độ @note".to_owned()
            } else {
                "Tùy chọn chat & đính kèm".to_owned()
            };
            let plus_resp = plus_pill_button(
                ui,
                &plus_hint,
                !busy,
                !self.ai_attached_files.is_empty() || note_mode,
            );
            egui::Popup::menu(&plus_resp).show(|ui| {
                compact_menu(ui);
                if ui
                    .add_enabled(
                        !busy && (!self.ai_history.is_empty() || self.ai_status.is_some()),
                        egui::Button::new("Hội thoại mới"),
                    )
                    .clicked()
                {
                    reset = true;
                    ui.close();
                }
                if ui
                    .add_enabled(!busy, egui::Button::new("Đính kèm tệp…"))
                    .clicked()
                {
                    attach = true;
                    ui.close();
                }
                if !self.ai_attached_files.is_empty()
                    && ui
                        .add_enabled(!busy, egui::Button::new("Gỡ tất cả tệp"))
                        .clicked()
                {
                    self.ai_attached_files.clear();
                    ui.close();
                }
                if ui.button("Chọn nhạc nhỏ (♪)").clicked() {
                    music_picker_requested = true;
                    ui.close();
                }
                ui.separator();
                if ui
                    .add_enabled(!busy, egui::Button::new("Chuyển sang @note"))
                    .clicked()
                {
                    let body = ai::note_command(&self.ai_prompt)
                        .unwrap_or(&self.ai_prompt)
                        .to_owned();
                    self.ai_prompt = if body.is_empty() {
                        "@note ".into()
                    } else {
                        format!("@note {body}")
                    };
                    self.ai_focus = 1;
                    ui.close();
                }
                if note_mode
                    && ui
                        .add_enabled(!busy, egui::Button::new("Về Chat thường"))
                        .clicked()
                {
                    self.ai_prompt = ai::note_command(&self.ai_prompt)
                        .unwrap_or(&self.ai_prompt)
                        .to_owned();
                    self.ai_focus = 1;
                    ui.close();
                }
                ui.separator();
                if !self.ai_history.is_empty()
                    && ui
                        .button(if expanded {
                            "Thu gọn bong bóng"
                        } else {
                            "Mở rộng lịch sử chat"
                        })
                        .clicked()
                {
                    self.ai_history_expanded = !expanded;
                    ui.close();
                }
                if self.tts_player.is_speaking() && ui.button("Dừng giọng đọc").clicked() {
                    self.tts_player.stop();
                    ui.close();
                }
                if ui.button("Cài đặt AI").clicked() {
                    self.settings_open = true;
                    ui.close();
                }
                if ui.button("Đóng thanh chat").clicked() {
                    open = false;
                    ui.close();
                }
            });

            let right_reserve = if expanded { 76.0 } else { 40.0 };
            let width = (ui.available_width() - right_reserve).max(1.0);
            let editor_height = composer_height(&self.ai_prompt, width)
                .min((ui.available_height() - if expanded { 100.0 } else { 0.0 }).max(32.0));
            let editor = ui
                .allocate_ui_with_layout(
                    Vec2::new(width, editor_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(width);
                        egui::ScrollArea::vertical()
                            .id_salt("ai-composer-scroll")
                            .max_height(editor_height)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.ai_prompt)
                                        .id_salt("pill-chat-input")
                                        .frame(egui::Frame::NONE)
                                        .margin(egui::vec2(0.0, 6.0))
                                        .font(egui::FontId::proportional(14.0))
                                        .desired_width(ui.available_width())
                                        .desired_rows(1)
                                        .min_size(Vec2::new(0.0, 32.0))
                                        .lock_focus(true)
                                        .return_key(egui::KeyboardShortcut::new(
                                            egui::Modifiers::SHIFT,
                                            egui::Key::Enter,
                                        ))
                                        .hint_text(if note_mode {
                                            "Mô tả ghi chú hoặc công việc cần AI tạo…"
                                        } else {
                                            "Hỏi AI hoặc bắt đầu chat mới…"
                                        }),
                                )
                            })
                            .inner
                    },
                )
                .inner;
            editor.context_menu(|ui| {
                compact_menu(ui);
                if ui
                    .add_enabled(
                        !self.ai_prompt.is_empty(),
                        egui::Button::new("Sao chép nội dung"),
                    )
                    .clicked()
                {
                    ctx.copy_text(self.ai_prompt.clone());
                    ui.close();
                }
                if ui.button("Dán").clicked() {
                    if let Ok(mut clipboard) = arboard::Clipboard::new()
                        && let Ok(text) = clipboard.get_text()
                    {
                        self.ai_prompt.push_str(&text);
                    }
                    self.ai_focus = 1;
                    ui.close();
                }
                if ui
                    .add_enabled(
                        !self.ai_prompt.is_empty(),
                        egui::Button::new("Xóa nội dung"),
                    )
                    .clicked()
                {
                    self.ai_prompt.clear();
                    ui.close();
                }
            });
            if self.ai_focus > 0 {
                ui.ctx().send_viewport_cmd(ViewportCommand::Focus);
                editor.request_focus();
                self.ai_focus = if editor.has_focus() {
                    0
                } else {
                    self.ai_focus.saturating_sub(1)
                };
            }

            if expanded {
                let attachment_label = if self.ai_attached_files.is_empty() {
                    "Đính kèm tệp".to_owned()
                } else {
                    format!(
                        "{} tệp đính kèm · chuột phải để gỡ",
                        self.ai_attached_files.len()
                    )
                };
                let files = theme::icon(ui, Glyph::Attach, &attachment_label, !busy);
                attach |= files.clicked() && !busy;
                files.context_menu(|ui| {
                    compact_menu(ui);
                    ui.label(format!("{} tệp đính kèm", self.ai_attached_files.len()));
                    if ui
                        .add_enabled(
                            !busy && !self.ai_attached_files.is_empty(),
                            egui::Button::new("Gỡ tất cả tệp"),
                        )
                        .clicked()
                    {
                        self.ai_attached_files.clear();
                        ui.close();
                    }
                });
            }

            let can_send =
                !busy && (!self.ai_prompt.trim().is_empty() || !self.ai_attached_files.is_empty());
            if busy {
                let stop = theme::icon(ui, Glyph::Stop, "Dừng tác vụ · Esc", true);
                if stop.clicked() {
                    self.cancel_ai();
                }
            } else {
                let send = send_pill_button(ui, can_send);
                if can_send && (send.clicked() || (plain_enter && editor.has_focus())) {
                    self.send_ai_prompt(&ctx);
                }
            }
        });

        if music_picker_requested {
            self.toggle_ai_music_picker(&ctx);
        }

        if expanded {
            ui.separator();
            let header = ui
                .horizontal(|ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new("PentaKuru").small().color(theme::MUTED),
                        )
                        .truncate(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if busy && theme::icon(ui, Glyph::Close, "Đóng khung chat", true).clicked()
                        {
                            open = false;
                        }
                        if !busy && theme::icon(ui, Glyph::Plus, "Hội thoại mới", true).clicked()
                        {
                            reset = true;
                        }
                        if ui
                            .small_button("Thu gọn")
                            .on_hover_text("Thu gọn về bong bóng nổi")
                            .clicked()
                        {
                            self.ai_history_expanded = false;
                            self.ai_bubble_dismissed = false;
                        }
                    });
                })
                .response;
            show_context_menu(&header, egui::Id::new("chat-header-menu"), |ui| {
                self.chat_menu(ui, &mut open, &mut reset)
            });
            egui::ScrollArea::vertical()
                .id_salt("chat-history")
                .auto_shrink([false, false])
                .max_height(ui.available_height().max(1.0))
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    for (index, message) in self.ai_history.iter().enumerate() {
                        ui.push_id(index, |ui| {
                            let message_frame = egui::Frame::new()
                                .fill(if message.role == ai::Role::User {
                                    theme::SURFACE
                                } else {
                                    theme::PANEL
                                })
                                .corner_radius(8)
                                .inner_margin(8)
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    theme::caption(
                                        ui,
                                        if message.role == ai::Role::User {
                                            "Bạn"
                                        } else {
                                            "PentaKuru"
                                        },
                                    );
                                    theme::render_markdown(ui, &message.text);
                                    if message.role == ai::Role::Assistant
                                        && ui.small_button("Thêm vào ghi chú đang chọn").clicked()
                                    {
                                        save_reply = Some(message.text.clone());
                                    }
                                    for img in &message.images {
                                        if let Some(tex) = get_or_load_thumbnail(
                                            ui.ctx(),
                                            &mut self.thumbnail_cache,
                                            img,
                                        ) {
                                            ui.add(
                                                egui::Image::new(&tex)
                                                    .max_width(ui.available_width().min(320.0))
                                                    .max_height(130.0)
                                                    .corner_radius(6),
                                            );
                                        }
                                        if let Some(path) = &img.file_path
                                            && ui.small_button("Mở ảnh gốc").clicked()
                                        {
                                            let _ = system::open(path);
                                        }
                                    }
                                });
                            show_context_menu(
                                &message_frame.response,
                                egui::Id::new(("chat-message-menu", index)),
                                |ui| {
                                    compact_menu(ui);
                                    if ui.button("Sao chép tin nhắn").clicked() {
                                        ctx.copy_text(message.text.clone());
                                        ui.close();
                                    }
                                    if ui
                                        .add_enabled(!busy, egui::Button::new("Hội thoại mới"))
                                        .clicked()
                                    {
                                        reset = true;
                                        ui.close();
                                    }
                                },
                            );
                        });
                    }
                    // Progress belongs to the scrollable content: a long goal or
                    // model status never takes space from the editor or its controls.
                    if let Some(status) = &self.ai_status {
                        if self.ai_job.is_some() {
                            ui.spinner();
                        }
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(status).small().color(theme::MUTED),
                            )
                            .wrap(),
                        );
                    }
                });
        }

        (open, reset, attach, save_reply)
    }

    fn chat_menu(&mut self, ui: &mut egui::Ui, open: &mut bool, reset: &mut bool) {
        compact_menu(ui);
        let busy = self.ai_job.is_some();
        if busy && ui.button("Dừng tác vụ").clicked() {
            self.cancel_ai();
            ui.close();
        }
        if !self.ai_history.is_empty()
            && ui
                .button(if self.ai_history_expanded {
                    "Thu gọn bong bóng"
                } else {
                    "Mở rộng hội thoại"
                })
                .clicked()
        {
            self.ai_history_expanded = !self.ai_history_expanded;
            if !self.ai_history_expanded {
                self.ai_bubble_dismissed = false;
            }
            ui.close();
        }
        if ui
            .add_enabled(
                !self.ai_history.is_empty(),
                egui::Button::new("Sao chép hội thoại"),
            )
            .clicked()
        {
            ui.ctx().copy_text(
                self.ai_history
                    .iter()
                    .map(|m| {
                        format!(
                            "{}: {}",
                            if m.role == ai::Role::User {
                                "Bạn"
                            } else {
                                "PentaKuru"
                            },
                            m.text
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n"),
            );
            ui.close();
        }
        if ui
            .add_enabled(!busy, egui::Button::new("Hội thoại mới"))
            .clicked()
        {
            *reset = true;
            ui.close();
        }
        if ui.button("Cài đặt AI").clicked() {
            self.settings_open = true;
            ui.close();
        }
        if ui.button("Đóng khung chat").clicked() {
            *open = false;
            ui.close();
        }
    }

    fn reset_chat(&mut self) {
        self.ai_history.clear();
        self.ai_status = None;
        self.ai_prompt.clear();
        self.ai_attached_files.clear();
        self.thumbnail_cache.clear();
        self.ai_history_expanded = false;
        self.ai_bubble_dismissed = false;
        self.tts_player.stop();
        self.ai_focus = 1;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BubbleTone {
    Success,
    Busy,
    Error,
}

#[derive(Clone, Copy)]
enum BubbleIcon {
    Close,
    Expand,
    Stop,
}

struct BubblePreview {
    tone: BubbleTone,
    title: String,
    body: String,
    truncated: bool,
    full_reply: Option<String>,
}

fn bubble_micro_icon(ui: &mut egui::Ui, kind: BubbleIcon, label: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::click());
    if response.hovered() {
        ui.painter().circle_filled(
            rect.center(),
            10.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, 20),
        );
    }
    let c = rect.center();
    let color = if response.hovered() {
        theme::TEXT
    } else {
        theme::MUTED
    };
    let s = Stroke::new(1.4, color);
    match kind {
        BubbleIcon::Close => {
            ui.painter()
                .line_segment([c + Vec2::new(-3.5, -3.5), c + Vec2::new(3.5, 3.5)], s);
            ui.painter()
                .line_segment([c + Vec2::new(3.5, -3.5), c + Vec2::new(-3.5, 3.5)], s);
        }
        BubbleIcon::Expand => {
            ui.painter()
                .line_segment([c + Vec2::new(-4.0, 4.0), c + Vec2::new(4.0, -4.0)], s);
            ui.painter()
                .line_segment([c + Vec2::new(0.5, -4.0), c + Vec2::new(4.0, -4.0)], s);
            ui.painter()
                .line_segment([c + Vec2::new(4.0, -4.0), c + Vec2::new(4.0, -0.5)], s);
        }
        BubbleIcon::Stop => {
            ui.painter()
                .rect_filled(Rect::from_center_size(c, Vec2::splat(7.0)), 1.5, color);
        }
    }
    response.on_hover_text(label)
}

fn plus_pill_button(
    ui: &mut egui::Ui,
    label: &str,
    enabled: bool,
    accented: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::click());
    let fill = if accented {
        Color32::from_rgb(34, 65, 54)
    } else if response.hovered() && enabled {
        Color32::from_rgba_unmultiplied(255, 255, 255, 28)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, 12)
    };
    ui.painter().circle_filled(rect.center(), 15.0, fill);
    let color = if !enabled {
        theme::MUTED
    } else if accented {
        Color32::from_rgb(100, 220, 175)
    } else {
        theme::TEXT
    };
    theme::draw_icon(ui.painter(), rect.center(), Glyph::Plus, color);
    response.on_hover_text(label)
}

fn send_pill_button(ui: &mut egui::Ui, ready: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::click());
    let fill = if ready {
        if response.hovered() {
            Color32::from_rgb(72, 130, 238)
        } else {
            Color32::from_rgb(56, 114, 224)
        }
    } else {
        Color32::from_rgb(36, 44, 62)
    };
    ui.painter().circle_filled(rect.center(), 15.0, fill);
    let color = if ready { Color32::WHITE } else { theme::MUTED };
    theme::draw_icon(ui.painter(), rect.center(), Glyph::Arrow, color);
    response.on_hover_text(if ready {
        "Gửi · Enter"
    } else {
        "Nhập nội dung để gửi · Esc để đóng"
    })
}

fn clean_prompt_title(raw: &str) -> String {
    let stripped = raw
        .trim()
        .trim_start_matches("🤖 **Nhiệm vụ GUI:**")
        .trim_start_matches("Đồng hồ · Windows:")
        .trim();
    let single_line: String = stripped.split_whitespace().collect::<Vec<_>>().join(" ");
    if single_line.chars().count() > 54 {
        let head: String = single_line.chars().take(53).collect();
        format!("{head}…")
    } else if single_line.is_empty() {
        "PentaKuru".into()
    } else {
        single_line
    }
}

fn compact_bubble_text(text: &str, max_lines: usize, max_chars: usize) -> (String, bool) {
    let mut lines = Vec::new();
    let mut in_code = false;
    let mut truncated = false;
    for raw in text.lines() {
        let trimmed = raw.trim();
        if trimmed.starts_with("```") {
            in_code = !in_code;
            if in_code {
                truncated = true;
            }
            continue;
        }
        if in_code || trimmed.is_empty() || trimmed == "---" {
            if in_code {
                truncated = true;
            }
            continue;
        }
        let clean = trimmed
            .trim_start_matches('#')
            .trim()
            .replace("**", "")
            .replace('`', "");
        if clean.is_empty() {
            continue;
        }
        if lines.len() >= max_lines {
            truncated = true;
            break;
        }
        lines.push(clean);
    }
    let mut joined = lines.join("\n");
    if joined.chars().count() > max_chars {
        joined = format!("{}…", joined.chars().take(max_chars).collect::<String>());
        truncated = true;
    } else if truncated && !joined.is_empty() && !joined.ends_with('…') {
        joined.push('…');
    }
    (joined, truncated)
}

#[derive(Clone, Copy)]
enum MiniBtn {
    Shuffle,
    Prev,
    Next,
    Repeat,
    Download,
    Volume(u8),
    Queue,
    Close,
}

fn mini_media_btn(ui: &mut egui::Ui, kind: MiniBtn, label: &str, active: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
    let emerald = Color32::from_rgb(16, 185, 129);
    if active {
        ui.painter().rect(
            rect,
            8,
            Color32::from_rgba_unmultiplied(16, 185, 129, 32),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(16, 185, 129, 130)),
            egui::StrokeKind::Inside,
        );
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, 8, Color32::from_rgba_unmultiplied(255, 255, 255, 16));
    }
    let c = rect.center();
    let color = if active {
        Color32::from_rgb(52, 211, 153)
    } else if response.hovered() {
        theme::TEXT
    } else {
        Color32::from_rgb(190, 195, 204)
    };
    let s = Stroke::new(1.5, color);
    let p = ui.painter();
    match kind {
        MiniBtn::Shuffle => {
            p.line_segment([c + Vec2::new(-6.0, -3.5), c + Vec2::new(-1.0, -3.5)], s);
            p.line_segment([c + Vec2::new(-1.0, -3.5), c + Vec2::new(2.0, 3.5)], s);
            p.line_segment([c + Vec2::new(2.0, 3.5), c + Vec2::new(6.0, 3.5)], s);
            p.line_segment([c + Vec2::new(-6.0, 3.5), c + Vec2::new(-1.0, 3.5)], s);
            p.line_segment([c + Vec2::new(-1.0, 3.5), c + Vec2::new(2.0, -3.5)], s);
            p.line_segment([c + Vec2::new(2.0, -3.5), c + Vec2::new(6.0, -3.5)], s);
            p.line_segment([c + Vec2::new(4.0, -5.5), c + Vec2::new(6.0, -3.5)], s);
            p.line_segment([c + Vec2::new(4.0, 5.5), c + Vec2::new(6.0, 3.5)], s);
        }
        MiniBtn::Prev => {
            p.line_segment([c + Vec2::new(-5.0, -4.5), c + Vec2::new(-5.0, 4.5)], s);
            p.add(egui::Shape::convex_polygon(
                vec![
                    c + Vec2::new(-3.0, 0.0),
                    c + Vec2::new(4.5, -4.5),
                    c + Vec2::new(4.5, 4.5),
                ],
                color,
                Stroke::NONE,
            ));
        }
        MiniBtn::Next => {
            p.line_segment([c + Vec2::new(5.0, -4.5), c + Vec2::new(5.0, 4.5)], s);
            p.add(egui::Shape::convex_polygon(
                vec![
                    c + Vec2::new(3.0, 0.0),
                    c + Vec2::new(-4.5, -4.5),
                    c + Vec2::new(-4.5, 4.5),
                ],
                color,
                Stroke::NONE,
            ));
        }
        MiniBtn::Repeat => {
            p.line_segment([c + Vec2::new(-5.0, -3.5), c + Vec2::new(5.0, -3.5)], s);
            p.line_segment([c + Vec2::new(5.0, -3.5), c + Vec2::new(5.0, 0.0)], s);
            p.line_segment([c + Vec2::new(5.0, 3.5), c + Vec2::new(-5.0, 3.5)], s);
            p.line_segment([c + Vec2::new(-5.0, 3.5), c + Vec2::new(-5.0, 0.0)], s);
            p.line_segment([c + Vec2::new(3.0, -5.5), c + Vec2::new(5.0, -3.5)], s);
            p.line_segment([c + Vec2::new(-3.0, 5.5), c + Vec2::new(-5.0, 3.5)], s);
        }
        MiniBtn::Download => {
            p.line_segment([c + Vec2::new(0.0, -6.0), c + Vec2::new(0.0, 3.0)], s);
            p.line_segment([c + Vec2::new(-3.5, -0.5), c + Vec2::new(0.0, 3.0)], s);
            p.line_segment([c + Vec2::new(3.5, -0.5), c + Vec2::new(0.0, 3.0)], s);
            p.line_segment([c + Vec2::new(-5.5, 5.5), c + Vec2::new(5.5, 5.5)], s);
        }
        MiniBtn::Volume(pct) => {
            p.add(egui::Shape::convex_polygon(
                vec![
                    c + Vec2::new(-5.5, -2.5),
                    c + Vec2::new(-2.2, -2.5),
                    c + Vec2::new(1.0, -5.2),
                    c + Vec2::new(1.0, 5.2),
                    c + Vec2::new(-2.2, 2.5),
                    c + Vec2::new(-5.5, 2.5),
                ],
                color,
                Stroke::NONE,
            ));
            if pct == 0 {
                let err_s = Stroke::new(1.5, Color32::from_rgb(248, 113, 113));
                p.line_segment([c + Vec2::new(3.0, -2.8), c + Vec2::new(6.8, 2.8)], err_s);
                p.line_segment([c + Vec2::new(6.8, -2.8), c + Vec2::new(3.0, 2.8)], err_s);
            } else {
                let arc = |r: f32| -> Vec<Pos2> {
                    (-2..=2)
                        .map(|k| {
                            let a = k as f32 * 0.34;
                            c + Vec2::new(1.2 + a.cos() * r, a.sin() * r)
                        })
                        .collect()
                };
                p.add(egui::Shape::line(arc(3.0), Stroke::new(1.4, color)));
                if pct > 55 {
                    p.add(egui::Shape::line(arc(5.5), Stroke::new(1.4, color)));
                }
            }
        }
        MiniBtn::Queue => {
            for y in [-4.0, 0.0, 4.0] {
                p.line_segment([c + Vec2::new(-5.5, y), c + Vec2::new(5.5, y)], s);
            }
        }
        MiniBtn::Close => {
            p.line_segment([c + Vec2::new(-4.0, -4.0), c + Vec2::new(4.0, 4.0)], s);
            p.line_segment([c + Vec2::new(4.0, -4.0), c + Vec2::new(-4.0, 4.0)], s);
        }
    }
    let _ = emerald;
    response.on_hover_text(label)
}

// Containers must stay hover-only: upgrading them to click sense after their
// children steals left-click focus, text selection, and button presses.
fn show_context_menu(
    response: &egui::Response,
    id: egui::Id,
    contents: impl FnOnce(&mut egui::Ui),
) {
    let ctx = &response.ctx;
    let (right_click, position) =
        ctx.input(|i| (i.pointer.secondary_clicked(), i.pointer.interact_pos()));
    let right_click_inside = right_click
        && position.is_some_and(|pos| {
            response.interact_rect.contains(pos) && ctx.layer_id_at(pos) == Some(response.layer_id)
        });
    let open = right_click_inside && !egui::Popup::is_any_open(ctx);
    egui::Popup::context_menu(response)
        .id(id)
        // This records the pointer anchor as well as the open state.
        .open_memory(open.then_some(egui::SetOpenCommand::Bool(true)))
        .show(contents);
}

fn compact_menu(ui: &mut egui::Ui) {
    ui.set_max_width(220.0);
    ui.spacing_mut().button_padding = Vec2::new(8.0, 4.0);
    ui.spacing_mut().item_spacing = Vec2::new(4.0, 2.0);
    ui.style_mut().override_font_id = Some(egui::FontId::proportional(12.5));
}

fn prompt_rows(prompt: &str, width: f32) -> usize {
    let chars_per_line = (width / 7.0).floor().max(1.0) as usize;
    prompt
        .lines()
        .map(|line| line.chars().count().max(1).div_ceil(chars_per_line))
        .sum::<usize>()
        .clamp(1, 4)
}

fn composer_height(prompt: &str, width: f32) -> f32 {
    32.0 + (prompt_rows(prompt, width) - 1) as f32 * 20.0
}

fn get_or_load_thumbnail(
    ctx: &egui::Context,
    cache: &mut std::collections::HashMap<std::path::PathBuf, egui::TextureHandle>,
    img: &ai::ImageAttachment,
) -> Option<egui::TextureHandle> {
    if let Some(fp_str) = &img.file_path {
        let path = std::path::PathBuf::from(fp_str);
        if let Some(tex) = cache.get(&path) {
            return Some(tex.clone());
        }
        if let Ok(bytes) = std::fs::read(&path)
            && let Ok(color_img) = system::load_png_color_image(&bytes)
        {
            let tex = ctx.load_texture(
                format!("thumb_{}", path.display()),
                color_img,
                egui::TextureOptions::LINEAR,
            );
            cache.insert(path, tex.clone());
            return Some(tex);
        }
    }
    if !img.base64_data.is_empty() {
        let key = std::path::PathBuf::from(format!("b64_{}", img.base64_data.len()));
        if let Some(tex) = cache.get(&key) {
            return Some(tex.clone());
        }
        if let Ok(bytes) = system::base64_decode(&img.base64_data)
            && let Ok(color_img) = system::load_png_color_image(&bytes)
        {
            let tex = ctx.load_texture(
                format!("thumb_b64_{}", bytes.len()),
                color_img,
                egui::TextureOptions::LINEAR,
            );
            cache.insert(key, tex.clone());
            return Some(tex);
        }
    }
    None
}

fn search_height(query: &str, count: usize) -> f32 {
    let q = query.trim();
    if q.is_empty() || count == 0 {
        58.0
    } else {
        58.0 + 12.0 + (count.clamp(1, 5) as f32 * 54.0)
    }
}

pub(crate) fn ai_height(has_content: bool, prompt_lines: usize) -> f32 {
    if has_content {
        480.0
    } else {
        58.0 + (prompt_lines.saturating_sub(1).min(3) as f32 * 20.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_search_is_only_a_bar_and_results_are_bounded() {
        assert_eq!(search_height("", 0), 58.0);
        assert_eq!(search_height("rust", 0), 58.0);
        assert!(search_height("rust", 2) > 58.0);
        assert_eq!(search_height("rust", 100), search_height("rust", 5));
    }

    #[test]
    fn ai_dock_pill_height_and_expansion() {
        assert_eq!(ai_height(false, 1), 58.0);
        assert_eq!(ai_height(false, 2), 78.0);
        assert_eq!(ai_height(true, 1), 480.0);
        assert_eq!(ai_height(true, 5), 480.0);
    }

    #[test]
    fn compact_bubble_text_truncates_utf8_and_code_safely() {
        let (short, truncated_short) = compact_bubble_text("**Chào anh!**", 4, 220);
        assert_eq!(short, "Chào anh!");
        assert!(!truncated_short);

        let long = format!(
            "# Tiêu đề\n{}\n```rust\nlet x = 1;\n```\nDòng cuối",
            "Tiếng Việt có dấu rất dài. ".repeat(12)
        );
        let (trimmed, truncated_long) = compact_bubble_text(&long, 4, 80);
        assert!(truncated_long);
        assert!(trimmed.ends_with('…'));
        assert!(trimmed.chars().count() <= 81);
    }
}
