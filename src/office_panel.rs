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
                    self.office.status.clear();
                }
                Ok(office::ResultItem::Labeled(files)) => self.office.files = files,
                Ok(office::ResultItem::Opened(mut file)) => {
                    file.last_opened = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |duration| duration.as_nanos() as u64);
                    if let Some(saved) = self
                        .office
                        .files
                        .iter_mut()
                        .find(|item| item.path == file.path)
                    {
                        saved.last_opened = file.last_opened;
                    } else {
                        self.office.files.push(file);
                    }
                    let _ = office::save_library(&self.office.files);
                    self.office.status = "Đã mở trong LibreOffice.".into();
                }
                Ok(office::ResultItem::Updated(results)) => {
                    self.office.status = results
                        .into_iter()
                        .map(|(path, _, status)| {
                            format!(
                                "{}: {status}",
                                path.file_name().unwrap_or_default().to_string_lossy()
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                }
                #[cfg(windows)]
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
                .with_transparent(false)
                .with_has_shadow(false)
                .with_inner_size([400.0, 440.0])
                .with_min_inner_size([340.0, 260.0])
                .with_resizable(true)
                .with_always_on_top(),
            |ui, _| {
                crate::system::apply_flat_window("PentaKuru · Office");
                if ui.input(|i| i.viewport().close_requested()) {
                    self.office_open = false;
                    self.office.cancel.store(true, Ordering::Relaxed);
                }
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(theme::PANEL).inner_margin(10))
                    .show(ui, |ui| {
                        self.office_ui(ui);
                    });
            },
        );
    }
    /// Launcher only: create a Writer/Calc/Impress file, or find and open one quickly.
    /// AI editing (`office::edit` / `office::classify`) gets wired back in later.
    pub(super) fn office_ui(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
        let busy = self.office.job.is_some();
        let mut open_file = None;
        let mut create_kind = None;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Office").size(13.5).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if office_icon(ui, theme::Glyph::Settings, "Cài đặt Office", true, false).clicked()
                {
                    self.settings_open = true;
                }
                if office_icon(ui, theme::Glyph::Refresh, "Quét lại tài liệu", !busy, false)
                    .clicked()
                {
                    self.start_office_index(ui.ctx());
                }
                if busy {
                    if office_icon(ui, theme::Glyph::Stop, "Dừng", true, false).clicked() {
                        self.office.cancel.store(true, Ordering::Relaxed);
                    }
                    ui.spinner();
                }
            });
        });
        let width = (ui.available_width() - 12.0) / 3.0;
        ui.horizontal(|ui| {
            for (kind, glyph, name) in [
                (office::Kind::Word, theme::Glyph::Word, "Writer"),
                (office::Kind::Excel, theme::Glyph::Excel, "Calc"),
                (
                    office::Kind::PowerPoint,
                    theme::Glyph::Presentation,
                    "Impress",
                ),
            ] {
                if create_button(ui, glyph, name, width, !busy).clicked() {
                    create_kind = Some(kind);
                }
            }
        });
        let search = ui.add_sized(
            [ui.available_width(), 30.0],
            egui::TextEdit::singleline(&mut self.office.query)
                .font(egui::FontId::proportional(13.0))
                .hint_text("Tìm nhanh tài liệu… (Enter mở file đầu)")
                .margin(egui::vec2(10.0, 6.0)),
        );
        let enter = search.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if !self.office.status.is_empty() {
            theme::caption(ui, &self.office.status);
        }
        let query = system::fold(self.office.query.trim());
        let mut files: Vec<_> = self
            .office
            .files
            .iter()
            .filter(|f| {
                query.is_empty()
                    || system::matches_terms(
                        &format!("{} {} {}", f.name, f.path.display(), f.labels.join(" ")),
                        &query,
                    )
            })
            .cloned()
            .collect();
        files.sort_by_cached_key(|file| std::cmp::Reverse(file.last_opened));
        if enter && !busy {
            open_file = files.first().cloned();
        }
        theme::caption(
            ui,
            if self.office.indexed || !files.is_empty() {
                format!("{} tài liệu · vừa mở ở đầu", files.len())
            } else {
                "Đang quét tài liệu…".to_owned()
            },
        );
        egui::ScrollArea::vertical()
            .id_salt("office-library")
            .min_scrolled_height(20.0)
            .auto_shrink([false, false])
            .show_rows(ui, 28.0, files.len(), |ui, range| {
                for file in &files[range] {
                    let response = ui
                        .add_enabled(
                            !busy,
                            egui::Button::new(egui::RichText::new(&file.name).size(14.0))
                                .frame(false)
                                .truncate(),
                        )
                        .on_hover_text(file.path.display().to_string());
                    if response.clicked() {
                        open_file = Some(file.clone());
                    }
                }
            });
        if let Some(file) = open_file {
            self.open_office_file(ui.ctx(), file);
        }
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
    }
    fn open_office_file(&mut self, ctx: &egui::Context, file: office::File) {
        if !file.kind.enabled(&self.prefs.office) {
            self.office.status =
                format!("Bật {} trong Settings để mở tài liệu.", file.kind.label());
        } else {
            let settings = self.prefs.office.clone();
            self.office.cancel.store(false, Ordering::Relaxed);
            self.office.status = format!("Đang mở {}…", file.name);
            self.office.job = Some(system::Job::spawn(ctx, move || {
                office::call(&settings, &file.path, "open", json!({}))
                    .map(|_| office::ResultItem::Opened(file))
            }));
        }
    }
}

fn create_button(
    ui: &mut egui::Ui,
    glyph: theme::Glyph,
    name: &str,
    width: f32,
    enabled: bool,
) -> egui::Response {
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width.max(80.0), 36.0), sense);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, format!("Tạo {name} mới"))
    });
    let color = if enabled { theme::TEXT } else { theme::MUTED };
    let fill = if enabled && response.hovered() {
        theme::SURFACE
    } else {
        theme::SIDEBAR
    };
    let painter = ui.painter();
    painter.rect(
        rect,
        10,
        fill,
        egui::Stroke::new(1.0, theme::BORDER),
        egui::StrokeKind::Inside,
    );
    theme::draw_icon(
        painter,
        egui::pos2(rect.left() + 20.0, rect.center().y),
        glyph,
        color,
    );
    painter.text(
        egui::pos2(rect.left() + 38.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(13.0),
        color,
    );
    if response.has_focus() {
        painter.rect_stroke(
            rect.shrink(1.0),
            10,
            egui::Stroke::new(1.0, theme::TEXT),
            egui::StrokeKind::Inside,
        );
    }
    response.on_hover_text(format!("Tạo {name} mới trong LibreOffice"))
}
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
