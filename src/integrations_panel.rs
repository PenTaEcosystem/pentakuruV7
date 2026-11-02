use crate::{PentaKuru, integrations, ui_theme as theme};
use eframe::egui;
impl PentaKuru {
    pub(super) fn integrations_settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Music · Clip", |ui| {
            ui.checkbox(
                &mut self.prefs.integrations.music,
                "Bật Penta Music · khung nhạc nhỏ",
            );
            if ui
                .add_enabled(self.prefs.integrations.music, egui::Button::new("Mở Music"))
                .clicked()
                && let Err(error) = integrations::music_open()
            {
                self.integrations.status = error;
            }
            ui.checkbox(&mut self.prefs.integrations.clip, "Bật Penta Clip · video");
            if ui
                .add_enabled(self.prefs.integrations.clip, egui::Button::new("Mở Clip"))
                .clicked()
                && let Err(error) = integrations::clip_open()
            {
                self.integrations.status = error;
            }
            theme::caption(
                ui,
                "Đăng nhập và sử dụng trên trang Music / Clip. Cần kết nối Internet.",
            );
            if !self.integrations.status.is_empty() {
                theme::caption(ui, &self.integrations.status);
            }
        });
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
