use crate::{PentaKuru, office, system, ui_theme as theme};
use eframe::egui::{self, ViewportId};
use serde_json::json;
use std::sync::atomic::Ordering;

impl PentaKuru {
    pub(super) fn poll_office(&mut self, ctx: &egui::Context) {
        if !self.prefs.utilities.enable_office {
            self.office.cancel.store(true, Ordering::Relaxed);
            self.office_open = false;
            self.office.installation_attempted = false;
        }
        if let Some(result) = system::take(&mut self.office.job) {
            let chat_reply = matches!(&result, Ok(office::ResultItem::Updated(_)) | Err(_));
            match result {
                Ok(office::ResultItem::Indexed(files)) => {
                    self.office.files = files;
                    self.office.indexed = true;
                    self.office.status =
                        format!("{} tài liệu · nhãn theo thư mục", self.office.files.len());
                }
                Ok(office::ResultItem::Labeled(files)) => {
                    self.office.files = files;
                    self.office.status = "Đã dán nhãn AI theo thư mục.".into();
                }
                Ok(office::ResultItem::Opened(file, dirty)) => {
                    if !self.office.files.iter().any(|item| item.path == file.path) {
                        self.office.files.push(file.clone());
                    }
                    self.office.focused = Some(file.path.clone());
                    if let Some(session) = self
                        .office
                        .active
                        .iter_mut()
                        .find(|s| s.file.path == file.path)
                    {
                        session.dirty = dirty;
                    } else {
                        self.office.active.push(office::Session {
                            file,
                            dirty,
                            targeted: false,
                        });
                    }
                    self.office.library_open = false;
                    self.office.status =
                        "Đã mở trong LibreOffice · chỉnh trực tiếp trên tài liệu.".into();
                }
                Ok(office::ResultItem::Updated(results)) => {
                    let mut messages = Vec::new();
                    for (path, dirty, status) in results {
                        if status.starts_with("OK")
                            && let Some(session) =
                                self.office.active.iter_mut().find(|s| s.file.path == path)
                        {
                            session.dirty = dirty;
                        }
                        messages.push(format!(
                            "{}: {status}",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        ));
                    }
                    self.office.status = messages.join("\n");
                }
                #[cfg(windows)]
                Ok(office::ResultItem::Installed(message)) => self.office.status = message,
                Err(error) => self.office.status = error,
            }
            if chat_reply {
                if self.office.chat.len() >= 60 {
                    self.office.chat.remove(0);
                }
                self.office.chat.push((false, self.office.status.clone()));
            }
        }
        if self.prefs.utilities.enable_office
            && !self.smoke_test
            && !self.office.installation_attempted
            && self.office.job.is_none()
            && office::program(&self.prefs.office).is_none()
        {
            self.office.installation_attempted = true;
            self.office.status = "Đang tải/cài LibreOffice lần đầu…".into();
            self.office.job = Some(system::Job::spawn(ctx, office::install));
        }
    }
    fn start_office_index(&mut self, ctx: &egui::Context) {
        if self.office.job.is_some() {
            return;
        }
        let settings = self.prefs.office.clone();
        let cancel = self.office.cancel.clone();
        cancel.store(false, Ordering::Relaxed);
        self.office.status = "Đang quét tài liệu…".into();
        self.office.job = Some(system::Job::spawn(ctx, move || {
            office::index(&settings, &cancel).map(office::ResultItem::Indexed)
        }));
    }
    pub(super) fn office_settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Office · Writer / Impress / Calc", |ui| {
            if ui
                .checkbox(
                    &mut self.prefs.utilities.enable_office,
                    "Bật tiện ích Office",
                )
                .changed()
                && !self.prefs.utilities.enable_office
            {
                self.office.cancel.store(true, Ordering::Relaxed);
                self.office_open = false;
            }
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut self.prefs.office.writer, "Writer · Word");
                ui.checkbox(&mut self.prefs.office.impress, "Impress · PowerPoint");
                ui.checkbox(&mut self.prefs.office.calc, "Calc · Excel");
            });
            theme::caption(
                ui,
                "Mở tài liệu trong LibreOffice thật. Không thêm phím tắt.",
            );
            ui.label("Thư mục tài liệu (mỗi dòng một đường dẫn)");
            ui.add(
                egui::TextEdit::multiline(&mut self.prefs.office.roots)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            );
            ui.label("Thư mục program của LibreOffice (để trống: tự tìm)");
            ui.text_edit_singleline(&mut self.prefs.office.program);
            let found = office::program(&self.prefs.office).is_some();
            theme::caption(
                ui,
                if found {
                    "LibreOffice đã sẵn sàng trên máy."
                } else {
                    "Chưa tìm thấy LibreOffice; cài khi bật tiện ích."
                },
            );
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.prefs.utilities.enable_office,
                        egui::Button::new("Mở Office"),
                    )
                    .clicked()
                {
                    self.office_open = true;
                }
                if ui
                    .add_enabled(
                        self.prefs.utilities.enable_office && !found && self.office.job.is_none(),
                        egui::Button::new("Cài LibreOffice"),
                    )
                    .clicked()
                {
                    self.office.status = "Đang tải/cài LibreOffice…".into();
                    self.office.job = Some(system::Job::spawn(ui.ctx(), office::install));
                }
            });
            if !self.office.status.is_empty() {
                theme::caption(ui, &self.office.status);
            }
        });
    }
    pub(super) fn office_window(&mut self, ctx: &egui::Context) {
        if !self.office_open || !self.prefs.utilities.enable_office {
            return;
        }
        if !self.office.indexed && self.office.job.is_none() && !self.smoke_test {
            self.start_office_index(ctx);
        }
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("office"),
            egui::ViewportBuilder::default()
                .with_title("PentaKuru · Office")
                .with_inner_size([
                    380.0,
                    if self.office.library_open {
                        380.0
                    } else if self.office.operation == 3 || !self.office.chat.is_empty() {
                        270.0
                    } else {
                        210.0
                    },
                ])
                .with_min_inner_size([320.0, 180.0])
                .with_resizable(true)
                .with_always_on_top(),
            |ui, _| {
                if ui.input(|i| i.viewport().close_requested()) {
                    self.office_open = false;
                }
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(theme::PANEL).inner_margin(10))
                    .show(ui, |ui| {
                        self.office_ui(ui);
                    });
            },
        );
    }
    pub(super) fn office_ui(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
        ui.spacing_mut().button_padding = egui::vec2(4.0, 3.0);
        let busy = self.office.job.is_some();
        let mut open_file = None;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Office").size(13.0).strong());
            if office_icon(
                ui,
                theme::Glyph::Folder,
                "Hiện / ẩn thư viện tài liệu",
                true,
                self.office.library_open,
            )
            .on_hover_text("Hiện / ẩn thư viện tài liệu")
            .clicked()
            {
                self.office.library_open = !self.office.library_open;
            }
            if office_icon(ui, theme::Glyph::Settings, "Cài đặt Office", true, false).clicked()
            {
                self.settings_open = true;
            }
            if office_icon(ui, theme::Glyph::Refresh, "Quét lại tài liệu", !busy, false).clicked()
            {
                self.start_office_index(ui.ctx());
            }
            if office_icon(
                ui,
                theme::Glyph::Tag,
                "AI dán nhãn theo thư mục",
                !busy && !self.office.files.is_empty(),
                false,
            )
            .clicked()
            {
                let files = self.office.files.clone();
                let ai = self.prefs.ai.clone();
                let cancel = self.office.cancel.clone();
                cancel.store(false, Ordering::Relaxed);
                self.office.status = "AI đang phân loại thư mục…".into();
                self.office.job = Some(system::Job::spawn(ui.ctx(), move || {
                    office::classify(files, &ai, &cancel)
                }));
            }
            if busy {
                ui.spinner();
                if office_icon(ui, theme::Glyph::Stop, "Dừng thao tác", true, false).clicked() {
                    self.office.cancel.store(true, Ordering::Relaxed);
                    self.office.status = "Đang dừng trước thao tác tiếp theo…".into();
                }
            }
        });
        if self.office.library_open {
            ui.add(
                egui::TextEdit::singleline(&mut self.office.query)
                    .font(egui::FontId::proportional(12.0))
                    .hint_text("Tìm tên, thư mục hoặc nhãn…")
                    .desired_width(f32::INFINITY),
            );
            ui.horizontal(|ui| {
                let response = office_icon(
                    ui,
                    theme::Glyph::File,
                    "Lọc loại tài liệu",
                    true,
                    self.office.filter.is_some(),
                );
                egui::Popup::menu(&response).show(|ui| {
                    if ui
                        .selectable_value(&mut self.office.filter, None, "Tất cả tài liệu")
                        .clicked()
                    {
                        ui.close();
                    }
                    for kind in [
                        office::Kind::Word,
                        office::Kind::PowerPoint,
                        office::Kind::Excel,
                    ] {
                        if ui
                            .selectable_value(&mut self.office.filter, Some(kind), kind.label())
                            .clicked()
                        {
                            ui.close();
                        }
                    }
                });
                let response = office_icon(
                    ui,
                    theme::Glyph::Tag,
                    "Lọc theo nhãn",
                    true,
                    !self.office.label.is_empty(),
                );
                egui::Popup::menu(&response).show(|ui| {
                    if ui
                        .selectable_value(&mut self.office.label, String::new(), "Tất cả nhãn")
                        .clicked()
                    {
                        ui.close();
                    }
                    let mut tags: Vec<_> = self
                        .office
                        .files
                        .iter()
                        .flat_map(|f| f.labels.clone())
                        .collect();
                    tags.sort();
                    tags.dedup();
                    for tag in tags {
                        if ui
                            .selectable_value(&mut self.office.label, tag.clone(), tag)
                            .clicked()
                        {
                            ui.close();
                        }
                    }
                });
                theme::caption(
                    ui,
                    format!(
                        "{} · {}",
                        self.office.filter.map_or("Tất cả", office::Kind::label),
                        if self.office.label.is_empty() {
                            "Mọi nhãn"
                        } else {
                            &self.office.label
                        }
                    ),
                );
            });
            let query = self.office.query.trim().to_lowercase();
            let files: Vec<_> = self
                .office
                .files
                .iter()
                .filter(|f| {
                    self.office.filter.is_none_or(|k| k == f.kind)
                        && (self.office.label.is_empty() || f.labels.contains(&self.office.label))
                        && format!("{} {} {}", f.name, f.path.display(), f.labels.join(" "))
                            .to_lowercase()
                            .contains(&query)
                })
                .cloned()
                .collect();
            theme::caption(ui, format!("{} tài liệu", files.len()));
            let height = (ui.available_height() - 235.0).clamp(40.0, 150.0);
            egui::ScrollArea::vertical()
                .id_salt("office-library")
                .min_scrolled_height(20.0)
                .max_height(height)
                .show_rows(ui, 24.0, files.len(), |ui, range| {
                    for file in &files[range] {
                        let active = self.office.active.iter().any(|s| s.file.path == file.path);
                        let label = egui::RichText::new(format!(
                            "{}{}",
                            if active { "• " } else { "" },
                            file.name
                        ))
                        .size(12.0)
                        .color(if active {
                            theme::TEXT
                        } else {
                            theme::MUTED
                        });
                        let response = ui
                            .add_enabled(!busy, egui::Button::new(label).frame(false).truncate())
                            .on_hover_text(format!(
                                "{}\n{}",
                                file.path.display(),
                                file.labels.join(" · ")
                            ));
                        if response.clicked() {
                            open_file = Some(file.clone());
                        }
                        response.context_menu(|ui| {
                            if ui.button("Đổi nhãn").clicked() {
                                self.office.label_edit = Some(file.path.clone());
                                self.office.label_text = file.labels.join(", ");
                                ui.close();
                            }
                        });
                    }
                });
        }
        if let Some(path) = self.office.label_edit.clone() {
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.office.label_text)
                        .desired_width((ui.available_width() - 76.0).max(100.0)),
                );
                if office_icon(ui, theme::Glyph::Check, "Lưu nhãn", true, false).clicked() {
                    if let Some(file) = self.office.files.iter_mut().find(|f| f.path == path) {
                        file.labels = self
                            .office
                            .label_text
                            .split(',')
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(str::to_owned)
                            .collect();
                        if file.labels.is_empty() {
                            file.labels.push("Unknown".into());
                        }
                        file.manual = true;
                    }
                    if let Err(error) = office::save_library(&self.office.files) {
                        self.office.status = error;
                    }
                    self.office.label_edit = None;
                }
                if office_icon(ui, theme::Glyph::Close, "Hủy đổi nhãn", true, false).clicked()
                {
                    self.office.label_edit = None;
                }
            });
        }
        let mut detach = None;
        if !self.office.active.is_empty() {
            theme::caption(ui, "Đang làm việc");
            egui::ScrollArea::vertical()
                .id_salt("office-active")
                .min_scrolled_height(20.0)
                .max_height(66.0)
                .show(ui, |ui| {
                    for session in &mut self.office.active {
                        ui.horizontal(|ui| {
                            if self.office.scope == 2 {
                                let target = ui
                                    .checkbox(&mut session.targeted, "")
                                    .on_hover_text("Giao file này cho AI");
                                target.widget_info(|| {
                                    egui::WidgetInfo::selected(
                                        egui::WidgetType::Checkbox,
                                        ui.is_enabled(),
                                        session.targeted,
                                        format!("Giao {} cho AI", session.file.name),
                                    )
                                });
                            }
                            let focused = self.office.focused.as_ref() == Some(&session.file.path);
                            let text = egui::RichText::new(format!(
                                "{}{}{}",
                                if focused { "• " } else { "" },
                                session.file.name,
                                if session.dirty { " *" } else { "" }
                            ))
                            .size(12.0)
                            .color(if focused {
                                theme::TEXT
                            } else {
                                theme::MUTED
                            });
                            let response = ui
                                .allocate_ui_with_layout(
                                    egui::vec2((ui.available_width() - 24.0).max(80.0), 22.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.add_enabled(
                                            !busy,
                                            egui::Button::new(text).frame(false).truncate(),
                                        )
                                    },
                                )
                                .inner
                                .on_hover_text(session.file.path.display().to_string());
                            response.widget_info(|| {
                                egui::WidgetInfo::selected(
                                    egui::WidgetType::Button,
                                    !busy,
                                    focused,
                                    &session.file.name,
                                )
                            });
                            if response.clicked() {
                                open_file = Some(session.file.clone());
                            }
                            let close = ui
                                .add(
                                    egui::Button::new(egui::RichText::new("×").size(13.0))
                                        .frame(false),
                                )
                                .on_hover_text("Bỏ active; tài liệu vẫn mở trong LibreOffice");
                            close.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    true,
                                    format!("Bỏ active: {}", session.file.name),
                                )
                            });
                            if close.clicked() {
                                detach = Some(session.file.path.clone());
                            }
                        });
                    }
                });
        }
        if let Some(path) = detach {
            self.office.active.retain(|s| s.file.path != path);
            if self.office.focused.as_ref() == Some(&path) {
                self.office.focused = self.office.active.last().map(|s| s.file.path.clone());
            }
        }
        if let Some(file) = open_file {
            if !file.kind.enabled(&self.prefs.office) {
                self.office.status =
                    format!("Bật {} trong Settings để mở tài liệu.", file.kind.label());
            } else {
                let settings = self.prefs.office.clone();
                self.office.cancel.store(false, Ordering::Relaxed);
                self.office.job = Some(system::Job::spawn(ui.ctx(), move || {
                    office::call(&settings, &file.path, "open", json!({})).map(|v| {
                        office::ResultItem::Opened(file, v["dirty"].as_bool().unwrap_or(false))
                    })
                }));
            }
        }
        let mut action = None;
        let mut create_kind = None;
        egui::Frame::new()
            .fill(theme::SIDEBAR)
            .stroke(egui::Stroke::new(1.0, theme::BORDER))
            .corner_radius(12)
            .inner_margin(6)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let history_height = (ui.available_height()
                    - 100.0
                    - if self.office.operation == 3 {
                        30.0
                    } else {
                        0.0
                    })
                .clamp(20.0, 68.0);
                if !self.office.chat.is_empty() {
                    egui::ScrollArea::vertical()
                        .id_salt("office-chat")
                        .min_scrolled_height(20.0)
                        .max_height(history_height)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            if self.office.chat.is_empty() {
                                theme::caption(ui, "Mở tài liệu rồi nhắn yêu cầu chỉnh sửa.");
                            }
                            for (user, text) in &self.office.chat {
                                ui.label(
                                    egui::RichText::new(if *user { "Bạn" } else { "AI" })
                                        .size(10.0)
                                        .color(theme::MUTED),
                                );
                                ui.label(egui::RichText::new(text).size(12.0).color(theme::TEXT));
                            }
                        });
                }
                if !self.office.status.is_empty()
                    && !self.office.status.contains("tài liệu · nhãn theo thư mục")
                    && !self.office.status.starts_with("Đã mở trong LibreOffice")
                    && (busy
                        || self
                            .office
                            .chat
                            .last()
                            .is_none_or(|(_, text)| text != &self.office.status))
                {
                    theme::caption(ui, &self.office.status);
                }
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.office.prompt)
                            .font(egui::FontId::proportional(12.0))
                            .hint_text("Nhắn yêu cầu chỉnh tài liệu…")
                            .desired_rows(1)
                            .desired_width((ui.available_width() - 38.0).max(100.0))
                            .frame(egui::Frame::NONE),
                    );
                    if office_icon(
                        ui,
                        theme::Glyph::Arrow,
                        "Gửi yêu cầu / chạy thao tác đang chọn",
                        !busy && !self.office.active.is_empty(),
                        false,
                    )
                    .clicked()
                    {
                        action = Some(self.office.operation);
                    }
                });
                ui.horizontal(|ui| {
                    for (kind, glyph) in [
                        (office::Kind::Word, theme::Glyph::Word),
                        (office::Kind::Excel, theme::Glyph::Excel),
                        (office::Kind::PowerPoint, theme::Glyph::Presentation),
                    ] {
                        if office_icon(
                            ui,
                            glyph,
                            &format!("Tạo {} mới", kind.label()),
                            !busy,
                            false,
                        )
                        .clicked()
                        {
                            create_kind = Some(kind);
                        }
                    }
                    let response = office_icon(
                        ui,
                        theme::Glyph::Settings,
                        "Thao tác chỉnh sửa",
                        !busy,
                        false,
                    );
                    egui::Popup::menu(&response).show(|ui| {
                        for (i, name) in OFFICE_OPERATIONS.iter().take(4).enumerate() {
                            if ui
                                .selectable_value(&mut self.office.operation, i, *name)
                                .clicked()
                            {
                                ui.close();
                            }
                        }
                        ui.separator();
                        for (i, name) in OFFICE_SCOPES.iter().enumerate() {
                            ui.selectable_value(&mut self.office.scope, i, *name);
                        }
                        ui.separator();
                        for (i, name) in [(4, "Hoàn tác"), (5, "Lưu tài liệu")] {
                            if ui
                                .add_enabled(
                                    !self.office.active.is_empty(),
                                    egui::Button::new(name),
                                )
                                .clicked()
                            {
                                action = Some(i);
                                ui.close();
                            }
                        }
                    });
                });
                theme::caption(
                    ui,
                    format!(
                        "{} · {}",
                        OFFICE_OPERATIONS[self.office.operation.min(3)],
                        OFFICE_SCOPES[self.office.scope.min(2)]
                    ),
                );
                if self.office.operation == 3 {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.office.font)
                                .desired_width((ui.available_width() - 85.0).max(100.0)),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.office.font_size)
                                .range(6.0..=96.0)
                                .suffix(" pt"),
                        );
                    });
                }
            });
        if let Some(kind) = create_kind {
            match kind {
                office::Kind::Word => self.prefs.office.writer = true,
                office::Kind::Excel => self.prefs.office.calc = true,
                office::Kind::PowerPoint => self.prefs.office.impress = true,
            }
            let settings = self.prefs.office.clone();
            self.office.status = format!("Đang tạo {} mới…", kind.label());
            self.office.job = Some(system::Job::spawn(ui.ctx(), move || {
                office::create(&settings, kind)
            }));
        }
        if let Some(action) = action {
            self.run_office_action(ui, action);
        }
    }
    fn run_office_action(&mut self, ui: &mut egui::Ui, action: usize) {
        let paths = if self.office.scope == 2 {
            self.office
                .active
                .iter()
                .filter(|s| s.targeted && s.file.kind.enabled(&self.prefs.office))
                .map(|s| s.file.path.clone())
                .collect()
        } else {
            self.office
                .focused
                .iter()
                .filter(|path| {
                    self.office.active.iter().any(|session| {
                        &session.file.path == *path && session.file.kind.enabled(&self.prefs.office)
                    })
                })
                .cloned()
                .collect::<Vec<_>>()
        };
        if paths.is_empty() {
            self.office.status = "Chọn ít nhất một file giao AI.".into();
        } else {
            let settings = self.prefs.office.clone();
            let ai = self.prefs.ai.clone();
            let prompt = self.office.prompt.clone();
            let font = self.office.font.clone();
            let size = self.office.font_size;
            let selection = self.office.scope == 0;
            let cancel = self.office.cancel.clone();
            cancel.store(false, Ordering::Relaxed);
            self.office.status = "Đang chỉnh tài liệu trong LibreOffice…".into();
            let command = if action < 2 && !prompt.trim().is_empty() {
                prompt.clone()
            } else {
                OFFICE_OPERATIONS[action.min(5)].to_owned()
            };
            let request = office::EditRequest {
                paths,
                operation: action,
                selection,
                prompt,
                font,
                size,
            };
            self.office.chat.push((true, command));
            if self.office.chat.len() > 60 {
                self.office.chat.drain(..self.office.chat.len() - 60);
            }
            if action < 2 {
                self.office.prompt.clear();
            }
            self.office.job = Some(system::Job::spawn(ui.ctx(), move || {
                office::edit(&settings, &ai, &request, &cancel)
            }));
        }
    }
}

const OFFICE_OPERATIONS: [&str; 6] = [
    "Sửa chính tả",
    "Nâng cấp bài viết",
    "Căn giữa ảnh",
    "Đổi kiểu chữ",
    "Hoàn tác",
    "Lưu tài liệu",
];
const OFFICE_SCOPES: [&str; 3] = ["Vùng đang chọn", "File đang xem", "Các file giao AI"];
fn office_icon(
    ui: &mut egui::Ui,
    glyph: theme::Glyph,
    label: &str,
    enabled: bool,
    selected: bool,
) -> egui::Response {
    let response = ui
        .add_enabled_ui(enabled, |ui| theme::icon(ui, glyph, label, enabled))
        .inner;
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, selected, label)
    });
    if selected {
        let rect = response.rect;
        ui.painter().line_segment(
            [
                rect.left_bottom() + egui::vec2(8.0, -1.0),
                rect.right_bottom() + egui::vec2(-8.0, -1.0),
            ],
            egui::Stroke::new(1.5, theme::TEXT),
        );
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.shrink(1.0),
            6,
            egui::Stroke::new(1.0, theme::TEXT),
            egui::StrokeKind::Inside,
        );
    }
    response
}
