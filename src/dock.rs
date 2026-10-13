use crate::{AiMode, Icon, PentaKuru, Preferences, hotkeys, integrations, sally};
use eframe::egui::{self, Vec2};

pub fn defaults() -> Vec<Icon> {
    vec![Icon::Note, Icon::Search, Icon::Ai, Icon::Translate]
}
impl Icon {
    pub const ALL: [Self; 12] = [
        Self::Note,
        Self::Search,
        Self::Ai,
        Self::Translate,
        Self::Spell,
        Self::Motion,
        Self::Office,
        Self::Music,
        Self::Video,
        Self::Tutor,
        Self::UiTars,
        Self::Settings,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Note => "Ghi chú",
            Self::Search => "Tìm file",
            Self::Ai => "Chat AI",
            Self::Translate => "Sally",
            Self::Spell => "Sửa chính tả",
            Self::Motion => "Xoay / dừng mô hình",
            Self::Office => "Office",
            Self::Music => "Penta Music",
            Self::Video => "Penta Video",
            Self::Tutor => "DeepTutor",
            Self::UiTars => "UI-TARS",
            Self::Settings => "Cài đặt",
        }
    }
    pub fn enabled(self, p: &Preferences) -> bool {
        match self {
            Self::Note => p.utilities.enable_notes,
            Self::Search => p.utilities.enable_search,
            Self::Ai => p.utilities.enable_ai,
            Self::Translate => p.utilities.enable_sally,
            Self::Spell => p.utilities.enable_spell,
            Self::Motion => p.utilities.enable_motion,
            Self::Office => p.utilities.enable_office,
            Self::Music => p.integrations.music,
            Self::Video => p.integrations.clip,
            Self::Tutor => p.integrations.deeptutor,
            Self::UiTars => p.utilities.enable_ui_tars,
            Self::Settings => true,
        }
    }
}
pub fn normalize(items: &mut Vec<Icon>) {
    let mut seen = Vec::new();
    items.retain(|i| {
        if seen.contains(i) || seen.len() == 5 {
            false
        } else {
            seen.push(*i);
            true
        }
    });
}
pub fn set_visible(items: &mut Vec<Icon>, icon: Icon, on: bool) -> bool {
    normalize(items);
    if on {
        if !items.contains(&icon) {
            if items.len() == 5 {
                return false;
            }
            items.push(icon);
        }
    } else {
        items.retain(|i| *i != icon);
    }
    true
}
impl PentaKuru {
    pub(super) fn visible_icons(&self) -> Vec<Icon> {
        let mut v = self.prefs.dock_icons.clone();
        normalize(&mut v);
        v.retain(|i| i.enabled(&self.prefs));
        v
    }
    pub(super) fn activate_icon(&mut self, icon: Icon, ctx: &egui::Context) {
        match icon {
            Icon::Note => self.notes_open = !self.notes_open,
            Icon::Search => {
                self.search_open = !self.search_open;
                if self.search_open {
                    self.ai_open = false;
                    self.search_focus = 10;
                }
            }
            Icon::Ai => {
                let already_open = self.ai_open && self.ai_mode == AiMode::Chat;
                self.ai_mode = AiMode::Chat;
                self.ai_open = !already_open;
                if self.ai_open {
                    self.search_open = false;
                    self.ai_focus = 10;
                }
            }
            Icon::Translate => {
                if let Err(e) = sally::open_or_focus_sally(
                    &self.prefs.ai.api_key,
                    &self.prefs.sally.target_lang,
                ) {
                    self.status = Some(e);
                }
            }
            Icon::Motion => self.moving = !self.moving,
            Icon::Spell => self.handle_action(hotkeys::Action::FixText, 0, ctx),
            Icon::Office => self.office_open = true,
            Icon::Music => {
                if let Err(e) = integrations::music_open() {
                    self.integrations.status = e;
                }
            }
            Icon::Video => {
                if let Err(e) = integrations::clip_open() {
                    self.integrations.status = e;
                }
            }
            Icon::Tutor => self.open_tutor(ctx),
            Icon::UiTars => {
                let already_open = self.ai_open && self.ai_mode == AiMode::UiTars;
                self.ai_mode = AiMode::UiTars;
                self.ai_open = !already_open;
                self.ai_focus = 10;
            }
            Icon::Settings => self.settings_open = true,
        }
        self.last_activity = std::time::Instant::now();
    }
    pub(super) fn dock_context_menu(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .max_height((ui.ctx().viewport_rect().height() - 24.0).max(80.0))
            .show(ui, |ui| {
                for icon in Icon::ALL {
                    if icon.enabled(&self.prefs) && ui.button(icon.label()).clicked() {
                        let ctx = ui.ctx().clone();
                        self.activate_icon(icon, &ctx);
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button("Thoát").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
    }
    pub(super) fn dock_settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        let visible = self.visible_icons();
        self.prefs.dock_icons = visible.clone();
        ui.label(format!("Icon hiển thị: {}/5", visible.len()));
        ui.add(egui::TextEdit::singleline(&mut self.dock_search).hint_text("Tìm icon chức năng…"));
        if ui.button("Tắt tất cả icon").clicked() {
            self.prefs.dock_icons.clear();
            changed = true;
        }
        let query = self.dock_search.trim().to_lowercase();
        for icon in Icon::ALL {
            if !query.is_empty() && !icon.label().to_lowercase().contains(&query) {
                continue;
            }
            let enabled = icon.enabled(&self.prefs);
            let mut selected = self.prefs.dock_icons.contains(&icon);
            let allowed = enabled && (selected || self.prefs.dock_icons.len() < 5);
            ui.add_enabled_ui(allowed, |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::hover());
                    let clicked =
                        crate::icon_button(ui, r, icon.label(), icon, selected, 0.0, 0.8).clicked();
                    if clicked {
                        selected = !selected;
                    }
                    if ui.checkbox(&mut selected, icon.label()).changed() || clicked {
                        changed |= set_visible(&mut self.prefs.dock_icons, icon, selected);
                    }
                });
            });
        }
        crate::ui_theme::caption(
            ui,
            "Chọn từ 0 đến 5 icon. Chức năng đã bật nhưng không hiện icon vẫn có trong menu chuột phải. Bật tiện ích trước để chọn icon của nó.",
        );
        changed
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn optional_icons_are_unique_and_capped() {
        let mut v = Vec::new();
        for i in Icon::ALL {
            set_visible(&mut v, i, true);
        }
        assert_eq!(v.len(), 5);
        assert!(!set_visible(&mut v, Icon::Video, true));
        set_visible(&mut v, Icon::Ai, false);
        assert!(set_visible(&mut v, Icon::Video, true));
        set_visible(&mut v, Icon::Video, true);
        assert_eq!(v.len(), 5);
        for i in Icon::ALL {
            set_visible(&mut v, i, false);
        }
        assert!(v.is_empty());
        assert_eq!(defaults().len(), 4);
    }
}
