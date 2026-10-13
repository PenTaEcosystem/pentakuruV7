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
                Ok(office::ResultItem::Installed(message)) => self.office.status = message,
                Err(error) => self.office.status = error,
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
                    480.0,
                    if self.office.library_open {
                        570.0
                    } else {
                        260.0
                    },
                ])
                .with_min_inner_size([360.0, 230.0])
                .with_resizable(true)
                .with_always_on_top(),
            |ui, _| {
                if ui.input(|i| i.viewport().close_requested()) {
                    self.office_open = false;
                }
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(theme::PANEL).inner_margin(12))
                    .show(ui, |ui| {
                        self.office_ui(ui);
                    });
            },
        );
    }
    pub(super) fn office_ui(&mut self, ui: &mut egui::Ui) {
        let busy = self.office.job.is_some();
        let mut open_file = None;
        ui.horizontal(|ui| {
            ui.strong("Office");
            if ui.button("Tài liệu").clicked() {
                self.office.library_open = !self.office.library_open;
            }
            if ui.button("Settings").clicked() {
                self.settings_open = true;
            }
            if busy {
                ui.spinner();
                if ui.button("Dừng").clicked() {
                    self.office.cancel.store(true, Ordering::Relaxed);
                    self.office.status = "Đang dừng trước thao tác tiếp theo…".into();
                }
            }
        });
        if self.office.library_open {
            ui.add(
                egui::TextEdit::singleline(&mut self.office.query)
                    .hint_text("Tìm tên, thư mục hoặc nhãn…")
                    .desired_width(f32::INFINITY),
            );
            ui.horizontal_wrapped(|ui| {
                egui::ComboBox::from_id_salt("office-kind")
                    .selected_text(self.office.filter.map_or("Tất cả", office::Kind::label))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.office.filter, None, "Tất cả");
                        for kind in [
                            office::Kind::Word,
                            office::Kind::PowerPoint,
                            office::Kind::Excel,
                        ] {
                            ui.selectable_value(&mut self.office.filter, Some(kind), kind.label());
                        }
                    });
                let mut tags: Vec<String> = self
                    .office
                    .files
                    .iter()
                    .flat_map(|f| f.labels.clone())
                    .collect();
                tags.sort();
                tags.dedup();
                egui::ComboBox::from_id_salt("office-label")
                    .selected_text(if self.office.label.is_empty() {
                        "Tất cả nhãn"
                    } else {
                        &self.office.label
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.office.label, String::new(), "Tất cả nhãn");
                        for tag in tags {
                            ui.selectable_value(&mut self.office.label, tag.clone(), tag);
                        }
                    });
                if ui
                    .add_enabled(!busy, egui::Button::new("Quét lại"))
                    .clicked()
                {
                    self.start_office_index(ui.ctx());
                }
                if ui
                    .add_enabled(
                        !busy && !self.office.files.is_empty(),
                        egui::Button::new("AI dán nhãn"),
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
            });
            let query = self.office.query.trim().to_lowercase();
            let files: Vec<office::File> = self
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
            egui::ScrollArea::vertical().max_height(240.0).show_rows(
                ui,
                35.0,
                files.len(),
                |ui, range| {
                    for file in &files[range] {
                        let is_active = self.office.active.iter().any(|s| s.file.path == file.path);
                        let response = ui
                            .add_enabled(
                                !busy,
                                egui::Button::new(format!(
                                    "{}{}",
                                    file.name,
                                    if is_active { " · Active" } else { "" }
                                )),
                            )
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
                },
            );
        }
        if let Some(path) = self.office.label_edit.clone() {
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.office.label_text);
                if ui.button("Lưu nhãn").clicked() {
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
                if ui.button("Hủy").clicked() {
                    self.office.label_edit = None;
                }
            });
        }
        ui.separator();
        let mut detach = None;
        ui.horizontal_wrapped(|ui| {
            for session in &self.office.active {
                let label = format!(
                    "{}{}",
                    session.file.name.chars().take(24).collect::<String>(),
                    if session.dirty { " •" } else { "" }
                );
                let response = ui
                    .add_enabled(
                        !busy,
                        egui::Button::selectable(
                            self.office.focused.as_ref() == Some(&session.file.path),
                            label,
                        ),
                    )
                    .on_hover_text(session.file.path.display().to_string());
                if response.clicked() {
                    open_file = Some(session.file.clone());
                }
                if ui
                    .small_button("×")
                    .on_hover_text("Bỏ active; tài liệu vẫn mở trong LibreOffice")
                    .clicked()
                {
                    detach = Some(session.file.path.clone());
                }
            }
        });
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
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("office-operation")
                .selected_text(
                    [
                        "Sửa chính tả",
                        "Nâng cấp bài viết",
                        "Căn giữa ảnh",
                        "Đổi kiểu chữ",
                    ][self.office.operation.min(3)],
                )
                .show_ui(ui, |ui| {
                    for (i, name) in [
                        "Sửa chính tả",
                        "Nâng cấp bài viết",
                        "Căn giữa ảnh",
                        "Đổi kiểu chữ",
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        ui.selectable_value(&mut self.office.operation, i, name);
                    }
                });
            egui::ComboBox::from_id_salt("office-scope")
                .selected_text(
                    ["Vùng đang chọn", "File đang xem", "Các file giao AI"]
                        [self.office.scope.min(2)],
                )
                .show_ui(ui, |ui| {
                    for (i, name) in ["Vùng đang chọn", "File đang xem", "Các file giao AI"]
                        .into_iter()
                        .enumerate()
                    {
                        ui.selectable_value(&mut self.office.scope, i, name);
                    }
                });
        });
        if self.office.scope == 2 {
            ui.horizontal_wrapped(|ui| {
                for session in &mut self.office.active {
                    ui.checkbox(&mut session.targeted, &session.file.name);
                }
            });
        }
        if self.office.operation == 3 {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.office.font).desired_width(170.0));
                ui.add(
                    egui::DragValue::new(&mut self.office.font_size)
                        .range(6.0..=96.0)
                        .suffix(" pt"),
                );
            });
        }
        if self.office.operation < 2 {
            ui.add(
                egui::TextEdit::singleline(&mut self.office.prompt)
                    .hint_text("Yêu cầu thêm cho AI…")
                    .desired_width(f32::INFINITY),
            );
        }
        let mut action = None;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !busy && !self.office.active.is_empty(),
                    egui::Button::new("Chạy"),
                )
                .clicked()
            {
                action = Some(self.office.operation);
            }
            if ui
                .add_enabled(
                    !busy && !self.office.active.is_empty(),
                    egui::Button::new("Hoàn tác"),
                )
                .clicked()
            {
                action = Some(4);
            }
            if ui
                .add_enabled(
                    !busy && !self.office.active.is_empty(),
                    egui::Button::new("Lưu"),
                )
                .clicked()
            {
                action = Some(5);
            }
        });
        if let Some(action) = action {
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
                            &session.file.path == *path
                                && session.file.kind.enabled(&self.prefs.office)
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
                let request = office::EditRequest {
                    paths,
                    operation: action,
                    selection,
                    prompt,
                    font,
                    size,
                };
                self.office.job = Some(system::Job::spawn(ui.ctx(), move || {
                    office::edit(&settings, &ai, &request, &cancel)
                }));
            }
        }
        if !self.office.status.is_empty() {
            egui::ScrollArea::vertical()
                .max_height(75.0)
                .show(ui, |ui| {
                    theme::caption(ui, &self.office.status);
                });
        }
    }
}
