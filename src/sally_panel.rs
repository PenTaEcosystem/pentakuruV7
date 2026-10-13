use crate::{PentaKuru, sally};
use eframe::egui::{
    self, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, Vec2, ViewportBuilder, ViewportCommand,
    ViewportId,
};

// Sally's exact design palette from Sally/src/styles.css
pub const SALLY_BG: Color32 = Color32::from_rgb(20, 22, 26); // #14161a (App container)
pub const SALLY_BG_PANEL: Color32 = Color32::from_rgb(27, 30, 36); // #1b1e24 (Titlebar, session-bar, headers)
pub const SALLY_BG_RAISED: Color32 = Color32::from_rgb(35, 39, 47); // #23272f (Elevated buttons, inputs)
pub const SALLY_BORDER: Color32 = Color32::from_rgb(44, 49, 58); // #2c313a (Subtle clean border)
pub const SALLY_TEXT: Color32 = Color32::from_rgb(230, 233, 238); // #e6e9ee (Primary readable text)
pub const SALLY_TEXT_DIM: Color32 = Color32::from_rgb(154, 163, 176); // #9aa3b0 (Muted metadata & labels)
pub const SALLY_ACCENT: Color32 = Color32::from_rgb(90, 169, 230); // #5aa9e6 (Electric sky blue accent)
pub const SALLY_ACCENT_SOFT: Color32 = Color32::from_rgba_premultiplied(90, 169, 230, 38); // rgba(90, 169, 230, 0.15)
pub const SALLY_OK: Color32 = Color32::from_rgb(111, 207, 151); // #6fcf97 (Emerald live indicator)
pub const SALLY_WARN: Color32 = Color32::from_rgb(230, 196, 106); // #e6c46a (Amber connecting / paused)
pub const SALLY_DANGER: Color32 = Color32::from_rgb(230, 106, 106); // #e66a6a (Soft crimson for stop)

impl PentaKuru {
    pub(super) fn sally_window(&mut self, ctx: &egui::Context) {
        if !self.sally_open {
            return;
        }

        let mut open = true;
        let always_on_top = self.prefs.sally.always_on_top;
        let mut vb = ViewportBuilder::default()
            .with_title("Sally")
            .with_inner_size([720.0, 500.0])
            .with_min_inner_size([460.0, 320.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_active(!self.smoke_test);
        if always_on_top {
            vb = vb.with_always_on_top();
        }
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("sally_translator"),
            vb,
            |viewport_ui, _| {
                crate::system::apply_window_dark_and_transparency("Sally");
                open = self.sally_ui(viewport_ui);
            },
        );
        self.sally_open = open;
    }

    fn sally_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut keep_open = true;

        if ui.input(|i| i.viewport().close_requested())
            || ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            return false;
        }

        // Sally outer frame: 8px radius, 1px border, deep dark background
        let frame = egui::Frame::new()
            .fill(SALLY_BG)
            .stroke(Stroke::new(1.0, SALLY_BORDER))
            .corner_radius(8)
            .inner_margin(0.0);

        frame.show(ui, |ui| {
            let win_rect = ui.max_rect();
            let avail_w = ui.available_width();
            let titlebar_h = 32.0;
            let sessionbar_h = 40.0;

            // ==========================================
            // 1. SALLY TITLEBAR (Height 32px, bg #1b1e24)
            // ==========================================
            let titlebar_rect =
                Rect::from_min_size(ui.cursor().min, Vec2::new(avail_w, titlebar_h));
            let titlebar_response = ui.interact(
                titlebar_rect,
                ui.id().with("sally_titlebar_drag"),
                Sense::click_and_drag(),
            );
            if titlebar_response.dragged() {
                ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
            }

            // Draw titlebar background & bottom border
            let p = ui.painter();
            p.rect_filled(
                titlebar_rect,
                CornerRadius {
                    nw: 8,
                    ne: 8,
                    sw: 0,
                    se: 0,
                },
                SALLY_BG_PANEL,
            );
            p.line_segment(
                [
                    Pos2::new(titlebar_rect.left(), titlebar_rect.bottom()),
                    Pos2::new(titlebar_rect.right(), titlebar_rect.bottom()),
                ],
                Stroke::new(1.0, SALLY_BORDER),
            );

            // Titlebar contents
            let mut title_child_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(titlebar_rect.shrink2(Vec2::new(8.0, 4.0)))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            title_child_ui.horizontal(|ui| {
                // App Name "Sally"
                ui.label(
                    egui::RichText::new("Sally")
                        .color(SALLY_TEXT)
                        .size(12.5)
                        .strong(),
                );

                ui.add_space(4.0);

                // Status Dot (8x8 circle)
                let dot_color = if self.sally_mgr.is_active {
                    if self.sally_mgr.is_paused {
                        SALLY_WARN
                    } else if self.sally_mgr.is_connected {
                        SALLY_OK
                    } else {
                        SALLY_WARN
                    }
                } else {
                    SALLY_TEXT_DIM
                };

                let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                ui.painter()
                    .circle_filled(dot_rect.center(), 3.5, dot_color);

                // Status text (11px, dim)
                let status_msg = if self.sally_mgr.is_active {
                    if self.sally_mgr.is_paused {
                        "Đã tạm dừng"
                    } else if self.sally_mgr.is_connected {
                        "Trực tiếp"
                    } else {
                        "Đang kết nối..."
                    }
                } else {
                    "Sẵn sàng"
                };
                ui.label(
                    egui::RichText::new(status_msg)
                        .color(SALLY_TEXT_DIM)
                        .size(11.0),
                );

                // Right-aligned controls
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Close button (✕)
                    if sally_icon_button(ui, "✕", SALLY_TEXT_DIM, SALLY_DANGER).clicked() {
                        keep_open = false;
                    }

                    // Audio Settings (⚙)
                    let gear_color = if self.sally_device_menu_open {
                        SALLY_ACCENT
                    } else {
                        SALLY_TEXT_DIM
                    };
                    if sally_icon_button(ui, "⚙", gear_color, SALLY_TEXT).clicked() {
                        self.sally_device_menu_open = !self.sally_device_menu_open;
                    }

                    // Always On Top pin (📌)
                    let pin_color = if self.prefs.sally.always_on_top {
                        SALLY_ACCENT
                    } else {
                        SALLY_TEXT_DIM
                    };
                    if sally_icon_button(ui, "📌", pin_color, SALLY_TEXT).clicked() {
                        self.prefs.sally.always_on_top = !self.prefs.sally.always_on_top;
                        ui.ctx().send_viewport_cmd(ViewportCommand::WindowLevel(
                            if self.prefs.sally.always_on_top {
                                egui::WindowLevel::AlwaysOnTop
                            } else {
                                egui::WindowLevel::Normal
                            },
                        ));
                    }

                    // Target Language Selector
                    egui::ComboBox::from_id_salt("sally_title_target_lang")
                        .selected_text(
                            egui::RichText::new(&self.prefs.sally.target_lang)
                                .size(11.5)
                                .color(SALLY_TEXT),
                        )
                        .show_ui(ui, |ui| {
                            for lang in [
                                "Tiếng Việt",
                                "English",
                                "日本語",
                                "中文",
                                "한국어",
                                "Français",
                                "Deutsch",
                            ] {
                                ui.selectable_value(
                                    &mut self.prefs.sally.target_lang,
                                    lang.to_string(),
                                    lang,
                                );
                            }
                        });
                });
            });

            ui.advance_cursor_after_rect(titlebar_rect);

            // ==========================================
            // 2. PANELS (STACKED: TRANSCRIPT & LIVE TRANSLATION)
            // ==========================================
            let middle_h = (ui.available_height() - sessionbar_h).max(100.0);
            let header_h = 24.0;
            let divider_h = 5.0;
            let usable_scroll_h = (middle_h - (header_h * 2.0) - divider_h).max(60.0);

            let ratio = self.prefs.sally.split_ratio.clamp(0.2, 0.8);
            let top_scroll_h = usable_scroll_h * ratio;
            let bot_scroll_h = usable_scroll_h * (1.0 - ratio);

            // ---- UPPER PANEL: TRANSCRIPT ----
            let top_header_rect =
                Rect::from_min_size(ui.cursor().min, Vec2::new(avail_w, header_h));
            let p = ui.painter();
            p.rect_filled(top_header_rect, 0.0, SALLY_BG_PANEL);
            p.line_segment(
                [
                    Pos2::new(top_header_rect.left(), top_header_rect.bottom()),
                    Pos2::new(top_header_rect.right(), top_header_rect.bottom()),
                ],
                Stroke::new(1.0, SALLY_BORDER),
            );

            let mut top_hdr_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(top_header_rect.shrink2(Vec2::new(10.0, 3.0)))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            top_hdr_ui.label(
                egui::RichText::new("TRANSCRIPT")
                    .color(SALLY_TEXT_DIM)
                    .size(10.0)
                    .strong(),
            );
            ui.advance_cursor_after_rect(top_header_rect);

            // Transcript Scroll Area
            let transcript_rect =
                Rect::from_min_size(ui.cursor().min, Vec2::new(avail_w, top_scroll_h));
            let mut transcript_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(transcript_rect.shrink2(Vec2::new(10.0, 6.0)))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );

            egui::ScrollArea::vertical()
                .id_salt("sally_transcript_scroll")
                .max_height(top_scroll_h - 12.0)
                .stick_to_bottom(true)
                .show(&mut transcript_ui, |ui| {
                    if self.sally_mgr.entries.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "Chưa có lời thoại. Nhấn 'Bắt đầu cuộc họp' để thu âm và gỡ băng.",
                            )
                            .color(SALLY_TEXT_DIM)
                            .italics()
                            .size(12.0),
                        );
                    } else {
                        for entry in &self.sally_mgr.entries {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("[{}]", entry.timestamp))
                                        .color(SALLY_TEXT_DIM)
                                        .size(11.0)
                                        .monospace(),
                                );
                                ui.label(
                                    egui::RichText::new(&entry.speaker)
                                        .color(SALLY_ACCENT)
                                        .size(11.5)
                                        .strong(),
                                );
                                let text = if entry.original.is_empty() {
                                    "(Âm thanh cuộc họp)"
                                } else {
                                    &entry.original
                                };
                                ui.label(egui::RichText::new(text).color(SALLY_TEXT).size(13.0));
                            });
                            ui.add_space(3.0);
                        }
                    }
                });
            ui.advance_cursor_after_rect(transcript_rect);

            // ---- DRAGGABLE DIVIDER (Height 5px) ----
            let divider_rect = Rect::from_min_size(ui.cursor().min, Vec2::new(avail_w, divider_h));
            let div_resp = ui.interact(
                divider_rect,
                ui.id().with("sally_panel_divider"),
                Sense::click_and_drag(),
            );
            if div_resp.hovered() || div_resp.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
            }
            if div_resp.dragged() {
                let delta_y = div_resp.drag_delta().y;
                let new_ratio = self.prefs.sally.split_ratio + (delta_y / usable_scroll_h);
                self.prefs.sally.split_ratio = new_ratio.clamp(0.2, 0.8);
            }

            let p = ui.painter();
            p.rect_filled(
                divider_rect,
                0.0,
                if div_resp.hovered() || div_resp.dragged() {
                    SALLY_ACCENT_SOFT
                } else {
                    SALLY_BG_PANEL
                },
            );
            p.line_segment(
                [
                    Pos2::new(divider_rect.left(), divider_rect.top()),
                    Pos2::new(divider_rect.right(), divider_rect.top()),
                ],
                Stroke::new(1.0, SALLY_BORDER),
            );
            p.line_segment(
                [
                    Pos2::new(divider_rect.left(), divider_rect.bottom()),
                    Pos2::new(divider_rect.right(), divider_rect.bottom()),
                ],
                Stroke::new(1.0, SALLY_BORDER),
            );
            // Grip indicator in center
            let grip_center = divider_rect.center();
            p.line_segment(
                [
                    Pos2::new(grip_center.x - 16.0, grip_center.y),
                    Pos2::new(grip_center.x + 16.0, grip_center.y),
                ],
                Stroke::new(1.0, SALLY_BORDER),
            );

            ui.advance_cursor_after_rect(divider_rect);

            // ---- LOWER PANEL: LIVE TRANSLATION ----
            let bot_header_rect =
                Rect::from_min_size(ui.cursor().min, Vec2::new(avail_w, header_h));
            let p = ui.painter();
            p.rect_filled(bot_header_rect, 0.0, SALLY_BG_PANEL);
            p.line_segment(
                [
                    Pos2::new(bot_header_rect.left(), bot_header_rect.bottom()),
                    Pos2::new(bot_header_rect.right(), bot_header_rect.bottom()),
                ],
                Stroke::new(1.0, SALLY_BORDER),
            );

            let mut bot_hdr_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(bot_header_rect.shrink2(Vec2::new(10.0, 3.0)))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            bot_hdr_ui.label(
                egui::RichText::new(
                    format!("BẢN DỊCH TRỰC TIẾP ({})", self.prefs.sally.target_lang).to_uppercase(),
                )
                .color(SALLY_ACCENT)
                .size(10.0)
                .strong(),
            );
            ui.advance_cursor_after_rect(bot_header_rect);

            // Translation Scroll Area
            let trans_rect = Rect::from_min_size(ui.cursor().min, Vec2::new(avail_w, bot_scroll_h));
            let mut trans_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(trans_rect.shrink2(Vec2::new(10.0, 6.0)))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );

            egui::ScrollArea::vertical()
                .id_salt("sally_translation_scroll")
                .max_height(bot_scroll_h - 12.0)
                .stick_to_bottom(true)
                .show(&mut trans_ui, |ui| {
                    for entry in &self.sally_mgr.entries {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new(format!("[{}]", entry.timestamp))
                                    .color(SALLY_TEXT_DIM)
                                    .size(11.0)
                                    .monospace(),
                            );
                            ui.label(
                                egui::RichText::new(&entry.speaker)
                                    .color(SALLY_ACCENT)
                                    .size(11.5)
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(&entry.translated)
                                    .color(Color32::WHITE)
                                    .size(14.0)
                                    .strong(),
                            );
                        });
                        ui.add_space(4.0);
                    }

                    // Câu đang dịch thời gian thực (Live streaming chunk)
                    if !self.sally_mgr.current_turn_translated.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new("[Live]")
                                    .color(SALLY_WARN)
                                    .size(11.0)
                                    .monospace()
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(&self.sally_mgr.current_turn_translated)
                                    .color(SALLY_WARN)
                                    .size(14.0)
                                    .italics(),
                            );
                        });
                    }
                });
            ui.advance_cursor_after_rect(trans_rect);

            // ==========================================
            // 3. SALLY SESSION BAR (Height 40px at bottom)
            // ==========================================
            let session_rect =
                Rect::from_min_size(ui.cursor().min, Vec2::new(avail_w, sessionbar_h));
            let p = ui.painter();
            p.rect_filled(
                session_rect,
                CornerRadius {
                    nw: 0,
                    ne: 0,
                    sw: 8,
                    se: 8,
                },
                SALLY_BG_PANEL,
            );
            p.line_segment(
                [
                    Pos2::new(session_rect.left(), session_rect.top()),
                    Pos2::new(session_rect.right(), session_rect.top()),
                ],
                Stroke::new(1.0, SALLY_BORDER),
            );

            let mut session_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(session_rect.shrink2(Vec2::new(8.0, 5.0)))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );

            session_ui.horizontal(|ui| {
                // Meeting Action Buttons (Start / Pause / End)
                if !self.sally_mgr.is_active {
                    // Button: ▶ Bắt đầu cuộc họp (.btn.primary)
                    if sally_styled_button(
                        ui,
                        "▶ Bắt đầu cuộc họp",
                        SALLY_ACCENT_SOFT,
                        SALLY_ACCENT,
                        SALLY_ACCENT,
                    )
                    .clicked()
                    {
                        let api_key = self.prefs.ai.api_key.clone();
                        let settings = self.prefs.sally.clone();
                        let _ = self.sally_mgr.start_session(api_key, &settings);
                    }
                } else {
                    // Active controls: Pause and End
                    let pause_label = if self.sally_mgr.is_paused {
                        "▶ Tiếp tục"
                    } else {
                        "⏸ Tạm dừng"
                    };
                    if sally_styled_button(
                        ui,
                        pause_label,
                        Color32::from_rgba_premultiplied(230, 196, 106, 35),
                        SALLY_WARN,
                        SALLY_WARN,
                    )
                    .clicked()
                    {
                        self.sally_mgr.toggle_pause();
                    }

                    if sally_styled_button(
                        ui,
                        "⏹ Kết thúc",
                        Color32::from_rgba_premultiplied(230, 106, 106, 35),
                        SALLY_DANGER,
                        SALLY_DANGER,
                    )
                    .clicked()
                    {
                        self.sally_mgr.stop_session();
                    }
                }

                // Monospace elapsed timer (.elapsed)
                ui.label(
                    egui::RichText::new(self.sally_mgr.duration_str())
                        .color(SALLY_TEXT_DIM)
                        .size(12.0)
                        .monospace(),
                );

                ui.add_space(4.0);

                // VU Meters for Loa and Mic
                draw_sally_vu_meter(ui, "Loa", self.sally_mgr.speaker_level, SALLY_OK);
                draw_sally_vu_meter(ui, "Mic", self.sally_mgr.mic_level, SALLY_ACCENT);

                // Right-aligned actions: Copy, Save Markdown, Entry Count
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Entry count (.meta)
                    ui.label(
                        egui::RichText::new(format!("{} đoạn", self.sally_mgr.entries.len()))
                            .color(SALLY_TEXT_DIM)
                            .size(11.0),
                    );

                    // Save to Notes (.md)
                    if sally_styled_button(
                        ui,
                        "💾 Lưu (.md)",
                        SALLY_BG_RAISED,
                        SALLY_BORDER,
                        SALLY_TEXT,
                    )
                    .clicked()
                    {
                        let notes_dir = self.notes_mgr.root_dir.clone();
                        match self
                            .sally_mgr
                            .save_meeting(&notes_dir, &self.prefs.sally.target_lang)
                        {
                            Ok(p) => {
                                self.status = Some(format!(
                                    "Đã lưu biên bản họp: {}",
                                    p.file_name().unwrap_or_default().to_string_lossy()
                                ));
                            }
                            Err(e) => {
                                self.status = Some(format!("Lỗi lưu biên bản: {e}"));
                            }
                        }
                    }

                    // Copy translation
                    if sally_styled_button(
                        ui,
                        "📋 Sao chép",
                        SALLY_BG_RAISED,
                        SALLY_BORDER,
                        SALLY_TEXT,
                    )
                    .clicked()
                    {
                        let mut full_text = String::new();
                        for e in &self.sally_mgr.entries {
                            full_text.push_str(&format!(
                                "[{}] {}: {}\n",
                                e.timestamp, e.speaker, e.translated
                            ));
                        }
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(full_text);
                            self.status = Some("Đã sao chép toàn bộ bản dịch vào clipboard".into());
                        }
                    }
                });
            });

            ui.advance_cursor_after_rect(session_rect);

            // ==========================================
            // 4. AUDIO DEVICES & SETTINGS MODAL SHEET
            // (.overlay .sheet matching Sally design §6)
            // ==========================================
            if self.sally_device_menu_open {
                let overlay_rect = win_rect;
                // Semi-transparent backdrop
                ui.painter().rect_filled(
                    overlay_rect,
                    8.0,
                    Color32::from_rgba_premultiplied(0, 0, 0, 160),
                );

                let sheet_w = (420.0_f32).min(avail_w - 32.0);
                let sheet_rect =
                    Rect::from_center_size(overlay_rect.center(), Vec2::new(sheet_w, 280.0));

                let p = ui.painter();
                p.rect_filled(sheet_rect, 8.0, SALLY_BG_PANEL);
                p.rect_stroke(
                    sheet_rect,
                    8.0,
                    Stroke::new(1.0, SALLY_BORDER),
                    egui::StrokeKind::Outside,
                );

                let mut sheet_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(sheet_rect.shrink2(Vec2::new(16.0, 14.0)))
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );

                sheet_ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("⚙ Cài đặt thiết bị & Sally")
                            .color(SALLY_TEXT)
                            .size(14.0)
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if sally_icon_button(ui, "✕", SALLY_TEXT_DIM, SALLY_DANGER).clicked() {
                            self.sally_device_menu_open = false;
                        }
                    });
                });

                sheet_ui.label(
                    egui::RichText::new(
                        "Chọn nguồn âm thanh cho cuộc họp (WASAPI Loopback) và Microphone.",
                    )
                    .color(SALLY_TEXT_DIM)
                    .size(11.0),
                );
                sheet_ui.add_space(8.0);

                let speakers = sally::audio::list_output_devices();
                let mics = sally::audio::list_input_devices();

                // Loa cuộc họp
                sheet_ui.label(
                    egui::RichText::new("🔊 Loa cuộc họp (WASAPI Loopback):")
                        .color(SALLY_TEXT)
                        .size(12.0),
                );
                let spk_sel = if self.prefs.sally.selected_speaker.is_empty() {
                    "Mặc định hệ thống".to_string()
                } else {
                    self.prefs.sally.selected_speaker.clone()
                };
                egui::ComboBox::from_id_salt("sally_sheet_speaker")
                    .selected_text(spk_sel)
                    .width(sheet_w - 40.0)
                    .show_ui(&mut sheet_ui, |ui| {
                        ui.selectable_value(
                            &mut self.prefs.sally.selected_speaker,
                            String::new(),
                            "Mặc định hệ thống",
                        );
                        for spk in speakers {
                            ui.selectable_value(
                                &mut self.prefs.sally.selected_speaker,
                                spk.clone(),
                                spk,
                            );
                        }
                    });

                sheet_ui.add_space(6.0);

                // Micro
                sheet_ui.label(
                    egui::RichText::new("🎙️ Microphone của bạn:")
                        .color(SALLY_TEXT)
                        .size(12.0),
                );
                let mic_sel = if self.prefs.sally.selected_mic.is_empty() {
                    "Mặc định hệ thống".to_string()
                } else {
                    self.prefs.sally.selected_mic.clone()
                };
                egui::ComboBox::from_id_salt("sally_sheet_mic")
                    .selected_text(mic_sel)
                    .width(sheet_w - 40.0)
                    .show_ui(&mut sheet_ui, |ui| {
                        ui.selectable_value(
                            &mut self.prefs.sally.selected_mic,
                            String::new(),
                            "Mặc định hệ thống",
                        );
                        for mic in mics {
                            ui.selectable_value(
                                &mut self.prefs.sally.selected_mic,
                                mic.clone(),
                                mic,
                            );
                        }
                    });

                sheet_ui.add_space(8.0);
                sheet_ui.label(
                    egui::RichText::new(
                        "✓ Tự động dùng chung Google Gemini API Key của PentaKuru.",
                    )
                    .color(SALLY_TEXT_DIM)
                    .size(10.5),
                );

                sheet_ui.add_space(10.0);
                sheet_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if sally_styled_button(
                        ui,
                        "Xong",
                        SALLY_ACCENT_SOFT,
                        SALLY_ACCENT,
                        SALLY_ACCENT,
                    )
                    .clicked()
                    {
                        self.sally_device_menu_open = false;
                    }
                });
            }
        });

        keep_open
    }
}

// ---------------------------------------------------------------------------
// Helper UI components for Sally's sleek design
// ---------------------------------------------------------------------------

fn sally_styled_button(
    ui: &mut egui::Ui,
    text: &str,
    bg: Color32,
    border: Color32,
    text_color: Color32,
) -> egui::Response {
    let rich = egui::RichText::new(text)
        .color(text_color)
        .size(12.0)
        .strong();
    let btn = egui::Button::new(rich)
        .fill(bg)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(5);
    ui.add(btn)
}

fn sally_icon_button(
    ui: &mut egui::Ui,
    icon: &str,
    normal_color: Color32,
    hover_color: Color32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
    let color = if resp.hovered() {
        hover_color
    } else {
        normal_color
    };
    if resp.hovered() {
        ui.painter().rect_filled(rect, 4.0, SALLY_BG_RAISED);
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(12.0),
        color,
    );
    resp
}

fn draw_sally_vu_meter(ui: &mut egui::Ui, label: &str, level: f32, active_color: Color32) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(SALLY_TEXT_DIM).size(10.5));
        let (rect, _) = ui.allocate_exact_size(Vec2::new(34.0, 5.0), Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 2.0, SALLY_BG_RAISED);
        let active_w = (rect.width() * level.clamp(0.0, 1.0)).max(0.0);
        if active_w > 0.0 {
            let filled = Rect::from_min_size(rect.min, Vec2::new(active_w, rect.height()));
            p.rect_filled(filled, 2.0, active_color);
        }
    });
}
