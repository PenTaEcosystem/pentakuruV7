use crate::{PentaKuru, integrations, mcp_media, system, ui_theme as theme};
use eframe::egui;
impl PentaKuru {
    pub(super) fn integrations_settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut save = false;
        let auto_open = self.prefs.integrations.resolved_music_key().is_empty()
            || self.prefs.integrations.resolved_clip_key().is_empty();
        egui::CollapsingHeader::new("Music · Clip & MCP")
            .default_open(auto_open)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .checkbox(
                            &mut self.prefs.integrations.music,
                            "Bật Penta Music · WebView",
                        )
                        .changed()
                    {
                        save = true;
                    }
                    if ui
                        .add_enabled(self.prefs.integrations.music, egui::Button::new("Mở Music"))
                        .clicked()
                        && let Err(error) = integrations::music_open()
                    {
                        self.integrations.status = error;
                    }
                });

                ui.horizontal(|ui| {
                    if ui
                        .checkbox(&mut self.prefs.integrations.clip, "Bật Penta Clip · WebView")
                        .changed()
                    {
                        save = true;
                    }
                    if ui
                        .add_enabled(self.prefs.integrations.clip, egui::Button::new("Mở Clip"))
                        .clicked()
                        && let Err(error) = integrations::clip_open()
                    {
                        self.integrations.status = error;
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    theme::caption(ui, "Khóa MCP cá nhân (Music & Clip)");
                    if ui
                        .small_button(if self.integrations.show_mcp_keys {
                            "Ẩn key"
                        } else {
                            "Hiện key"
                        })
                        .clicked()
                    {
                        self.integrations.show_mcp_keys = !self.integrations.show_mcp_keys;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Music Key:");
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.prefs.integrations.music_mcp_key)
                                .password(!self.integrations.show_mcp_keys)
                                .hint_text("pmcp_...")
                                .desired_width(175.0),
                        )
                        .changed()
                    {
                        save = true;
                    }
                    if ui.small_button("Dán").clicked()
                        && let Ok(mut cb) = arboard::Clipboard::new()
                        && let Ok(clip) = cb.get_text()
                    {
                        let trimmed = clip.trim().to_owned();
                        if !trimmed.is_empty() {
                            self.prefs.integrations.music_mcp_key = trimmed;
                            save = true;
                        }
                    }
                    if ui
                        .add_enabled(self.mcp_test_job.is_none(), egui::Button::new("Kiểm tra"))
                        .clicked()
                    {
                        let endpoint = self.prefs.integrations.resolved_music_url().to_owned();
                        let key = self.prefs.integrations.resolved_music_key();
                        self.integrations.status = "Đang kiểm tra Music MCP…".into();
                        self.mcp_test_job = Some(system::Job::spawn(ui.ctx(), move || {
                            mcp_media::test_music_mcp(&endpoint, &key)
                        }));
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Clip Key:  ");
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.prefs.integrations.clip_mcp_key)
                                .password(!self.integrations.show_mcp_keys)
                                .hint_text("pc_mcp_...")
                                .desired_width(175.0),
                        )
                        .changed()
                    {
                        save = true;
                    }
                    if ui.small_button("Dán").clicked()
                        && let Ok(mut cb) = arboard::Clipboard::new()
                        && let Ok(clip) = cb.get_text()
                    {
                        let trimmed = clip.trim().to_owned();
                        if !trimmed.is_empty() {
                            self.prefs.integrations.clip_mcp_key = trimmed;
                            save = true;
                        }
                    }
                    if ui
                        .add_enabled(self.mcp_test_job.is_none(), egui::Button::new("Kiểm tra"))
                        .clicked()
                    {
                        let endpoint = self.prefs.integrations.resolved_clip_url().to_owned();
                        let key = self.prefs.integrations.resolved_clip_key();
                        self.integrations.status = "Đang kiểm tra Clip MCP…".into();
                        self.mcp_test_job = Some(system::Job::spawn(ui.ctx(), move || {
                            mcp_media::test_clip_mcp(&endpoint, &key)
                        }));
                    }
                });
                theme::caption(
                    ui,
                    "Chỉ cần dán Music/Clip Key để Chat AI tự phát nhạc/video. Bật WebView nếu muốn giữ nút Mở Music/Clip.",
                );
                if !self.integrations.status.is_empty() {
                    theme::caption(ui, &self.integrations.status);
                }
            });
        save
    }
    pub(super) fn poll_integrations(&mut self, _ctx: &egui::Context) {
        if self.smoke_test {
            return;
        }
        let settings = &self.prefs.integrations;
        if settings.music != self.integrations.seen_music {
            self.integrations.seen_music = settings.music;
            let result = if settings.music {
                Ok(())
            } else {
                crate::sally::launcher::close_web("penta-music")
            };
            if let Err(error) = result {
                self.integrations.status = error;
            }
        }
        if settings.clip != self.integrations.seen_clip {
            self.integrations.seen_clip = settings.clip;
            let result = if settings.clip {
                Ok(())
            } else {
                crate::sally::launcher::close_web("penta-clip")
            };
            if let Err(error) = result {
                self.integrations.status = error;
            }
        }
    }
}
