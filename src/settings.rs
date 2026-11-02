//! Compact controls scoped to the settings window.
use crate::{PentaKuru, ai, theme};
use eframe::egui::{self, FontId, Vec2};

impl PentaKuru {
    pub(crate) fn compact_settings_style(ui: &mut egui::Ui) {
        let style = ui.style_mut();
        style.spacing.item_spacing = Vec2::new(8.0, 6.0);
        style.spacing.button_padding = Vec2::new(9.0, 4.0);
        style.spacing.interact_size.y = 28.0;
        style.spacing.combo_width = 220.0;
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(13.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(13.0));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, FontId::proportional(18.0));
    }

    pub(crate) fn ai_connection_settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let before = (
            self.prefs.ai.provider,
            self.prefs.ai.api_key.clone(),
            self.prefs.ai.base_url.clone(),
            self.prefs.ai.model.clone(),
        );
        egui::CollapsingHeader::new("Kết nối AI")
            .default_open(true)
            .show(ui, |ui| {
                theme::caption(ui, "Dùng chung cho Chat, sửa chính tả và trợ lý AI.");
                ui.label("Nhà cung cấp");
                egui::ComboBox::from_id_salt("provider_select")
                    .width(ui.available_width())
                    .selected_text(self.prefs.ai.provider.label())
                    .show_ui(ui, |ui| {
                        for provider in ai::Provider::ALL {
                            ui.selectable_value(&mut self.prefs.ai.provider, provider, provider.label());
                        }
                    });
                if before.0 != self.prefs.ai.provider {
                    self.prefs.ai.base_url.clear();
                    self.prefs.ai.models.clear();
                    self.prefs.ai.model.clear();
                    self.ai_models_job = None;
                    self.ai_status = None;
                }

                if matches!(self.prefs.ai.provider, ai::Provider::Custom | ai::Provider::Ollama) {
                    ui.label("Base URL");
                    let hint = self.prefs.ai.provider.default_base();
                    if ui.add(
                        egui::TextEdit::singleline(&mut self.prefs.ai.base_url)
                            .desired_width(f32::INFINITY)
                            .hint_text(hint),
                    ).changed() {
                        self.prefs.ai.models.clear();
                        self.ai_models_job = None;
                        self.ai_status = None;
                    }
                    theme::caption(ui, "Dán URL gốc của proxy, gồm /v1 nếu có.");
                } else if !self.prefs.ai.base_url.is_empty() {
                    // Keep existing endpoint overrides visible and editable.
                    ui.label("Base URL (tùy chỉnh)");
                    ui.add(egui::TextEdit::singleline(&mut self.prefs.ai.base_url)
                        .desired_width(f32::INFINITY));
                }

                ui.label("API key");
                ui.horizontal(|ui| {
                    let width = (ui.available_width() - 54.0).max(80.0);
                    ui.add_sized([width, 28.0],
                        egui::TextEdit::singleline(&mut self.prefs.ai.api_key)
                            .password(!self.show_api_key)
                            .hint_text("Dán khóa của nhà cung cấp hoặc proxy"));
                    if ui.button(if self.show_api_key { "Ẩn" } else { "Hiện" }).clicked() {
                        self.show_api_key = !self.show_api_key;
                    }
                });
                if self.prefs.ai.api_key.trim() != self.last_scanned_key.trim() {
                    self.last_scanned_key = self.prefs.ai.api_key.trim().to_owned();
                    self.prefs.ai.models.clear();
                    self.ai_models_job = None;
                    self.ai_status = None;
                    // Explicit providers must keep their endpoint even for sk-* proxy keys.
                    if self.prefs.ai.provider == ai::Provider::Auto {
                        self.trigger_fetch_models(ui.ctx());
                    }
                }

                ui.label("Mã mô hình (Model ID)");
                ui.add(egui::TextEdit::singleline(&mut self.prefs.ai.model)
                    .desired_width(f32::INFINITY)
                    .hint_text("Nhập mã mô hình do nhà cung cấp cung cấp"));
                ui.horizontal_wrapped(|ui| {
                    if ui.add_enabled(self.ai_models_job.is_none() && self.prefs.ai.can_list(),
                        egui::Button::new("Tải danh sách mô hình")).clicked() {
                        self.trigger_fetch_models(ui.ctx());
                    }
                    if self.ai_models_job.is_some() {
                        ui.spinner();
                        ui.weak("Đang tải…");
                    }
                });
                if !self.prefs.ai.models.is_empty() {
                    egui::ComboBox::from_id_salt("model_select")
                        .width(ui.available_width())
                        .selected_text("Chọn từ danh sách đã tải")
                        .show_ui(ui, |ui| {
                            for model in &self.prefs.ai.models {
                                ui.selectable_value(&mut self.prefs.ai.model, model.clone(), model);
                            }
                        });
                }
                theme::caption(ui, "Có thể nhập Model ID trực tiếp nếu proxy không hỗ trợ danh sách. Tên hiển thị có thể khác Model ID.");
                if let Some(status) = &self.ai_status {
                    theme::caption(ui, status);
                }
            });
        before
            != (
                self.prefs.ai.provider,
                self.prefs.ai.api_key.clone(),
                self.prefs.ai.base_url.clone(),
                self.prefs.ai.model.clone(),
            )
    }
}
