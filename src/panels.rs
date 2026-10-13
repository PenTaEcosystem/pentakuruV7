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

fn section_frame(fill: Color32) -> egui::Frame {
    egui::Frame::new().fill(fill).inner_margin(18)
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
                    [900.0, 620.0]
                })
                .with_min_inner_size([660.0, 460.0])
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
            system::apply_window_dark_and_transparency("PentaKuru · Tìm kiếm");
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
                    .with_active(!self.smoke_test),
                |viewport_ui, _| {
                    system::apply_window_dark_and_transparency("PentaKuru · Tìm kiếm");
                    open = self.search_ui(viewport_ui);
                },
            );
            self.search_open = open;
            if !open {
                self.search_focus = 10;
            }
        }
        if self.ai_open {
            let has_content =
                !self.ai_history.is_empty() || self.ai_job.is_some() || self.ui_agent.is_active;

            let width = 540.0_f32.min((screen_rect.width() - 24.0).max(240.0));
            let lines = prompt_rows(&self.ai_prompt, (width - 136.0).max(1.0));
            let height = (if !has_content && self.ai_status.is_some() {
                160.0_f32
            } else {
                ai_height(has_content, lines)
            })
            .max(if self.ai_menu_open { 220.0 } else { 0.0 })
            .min((screen_rect.height() - 64.0).max(160.0));
            let position = calc_pos(width, height);
            system::apply_window_dark_and_transparency("PentaKuru · AI");
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
                    .with_active(!self.smoke_test),
                |viewport_ui, _| {
                    open = self.ai_ui(viewport_ui);
                },
            );
            self.ai_open = open;
            if self.ui_agent.is_active {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            if !open {
                self.ai_focus = 10;
            }
        }
    }

    pub(super) fn notes_ui(&mut self, viewport_ui: &mut egui::Ui) -> bool {
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
            .frame(section_frame(theme::PANEL))
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
                    ui.label(egui::RichText::new("Ghi chú").size(20.0).strong());
                    theme::caption(ui, "PENTAKURU");
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
                        if ui.button("Ghi chú mới").clicked() {
                            self.notes_mgr.create_note("Ghi chú mới", "");
                        }
                        if theme::icon(ui, Glyph::Folder, "Mở thư mục lưu ghi chú", true).clicked()
                        {
                            let _ = system::open(&self.notes_mgr.root_dir.to_string_lossy());
                        }
                    });
                });
            });

        egui::Panel::left("notes-library")
            .exact_size(250.0)
            .resizable(false)
            .frame(section_frame(theme::SIDEBAR))
            .show(viewport_ui, |ui| {
                let previous_folder = self.notes_mgr.selected_folder.clone();
                theme::caption(ui, "THƯ VIỆN");
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("notes-folder")
                        .width(164.0)
                        .selected_text(&self.notes_mgr.selected_folder)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.notes_mgr.selected_folder,
                                "Tất cả".to_owned(),
                                "Tất cả ghi chú",
                            );
                            for folder in &self.notes_mgr.folders {
                                ui.selectable_value(
                                    &mut self.notes_mgr.selected_folder,
                                    folder.clone(),
                                    folder,
                                );
                            }
                        });
                    if theme::icon(ui, Glyph::Plus, "Thêm thư mục", true).clicked() {
                        self.folder_form = !self.folder_form;
                    }
                });
                if self.folder_form {
                    ui.horizontal(|ui| {
                        let resp = ui.add_sized(
                            [150.0, 30.0],
                            egui::TextEdit::singleline(&mut self.new_folder_input)
                                .hint_text("Tên thư mục"),
                        );
                        if (ui.button("Tạo").clicked()
                            || (resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                            && !self.new_folder_input.trim().is_empty()
                        {
                            self.notes_mgr.add_folder(&self.new_folder_input);
                            self.new_folder_input.clear();
                            self.folder_form = false;
                        }
                    });
                }
                if previous_folder != self.notes_mgr.selected_folder {
                    self.notes_mgr.selected_note_id =
                        self.notes_mgr.visible_notes().first().map(|n| n.id.clone());
                    self.pending_note_delete = false;
                }
                ui.add_space(8.0);
                ui.add_sized(
                    [ui.available_width(), 36.0],
                    egui::TextEdit::singleline(&mut self.notes_mgr.search_query)
                        .hint_text("Tìm trong ghi chú")
                        .margin(10),
                );
                let items: Vec<_> = self
                    .notes_mgr
                    .visible_notes()
                    .into_iter()
                    .map(|n| {
                        (
                            n.id.clone(),
                            n.title.clone(),
                            n.updated_at.clone(),
                            n.attachments.len(),
                            n.icon.clone(),
                        )
                    })
                    .collect();
                theme::caption(ui, format!("{} ghi chú", items.len()));
                egui::ScrollArea::vertical()
                    .id_salt("note-list-scroll")
                    .show(ui, |ui| {
                        for (id, title, updated, attachments, icon) in items {
                            let selected = self.notes_mgr.selected_note_id.as_ref() == Some(&id);
                            let response = egui::Frame::new()
                                .fill(if selected {
                                    theme::SURFACE
                                } else {
                                    theme::SIDEBAR
                                })
                                .stroke(Stroke::new(
                                    1.0,
                                    if selected {
                                        theme::BORDER
                                    } else {
                                        theme::SIDEBAR
                                    },
                                ))
                                .corner_radius(10)
                                .inner_margin(12)
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        let glyph = theme::glyph_from_name(&icon);
                                        let (icon_rect, _) = ui
                                            .allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                                        theme::draw_icon(
                                            ui.painter(),
                                            icon_rect.center(),
                                            glyph,
                                            if selected { theme::TEXT } else { theme::MUTED },
                                        );
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(title).size(14.0).strong(),
                                            )
                                            .truncate(),
                                        );
                                    });
                                    theme::caption(ui, updated);
                                    if attachments > 0 {
                                        theme::caption(ui, format!("{attachments} tệp đính kèm"));
                                    }
                                })
                                .response
                                .interact(Sense::click());
                            if response.clicked() {
                                self.notes_mgr.selected_note_id = Some(id);
                                self.pending_note_delete = false;
                            }
                            if response.hovered() {
                                ui.painter().rect_stroke(
                                    response.rect,
                                    10,
                                    Stroke::new(1.0, theme::BORDER),
                                    egui::StrokeKind::Inside,
                                );
                            }
                        }
                    });
            });

        egui::CentralPanel::default()
            .frame(section_frame(theme::PANEL))
            .show(viewport_ui, |ui| {
                let index = self
                    .notes_mgr
                    .selected_note_id
                    .as_ref()
                    .and_then(|id| self.notes_mgr.notes.iter().position(|n| &n.id == id));
                let Some(index) = index else {
                    ui.add_space(60.0);
                    ui.heading("Một nơi cho ý tưởng của anh");
                    theme::caption(ui, "Tạo ghi chú mới để bắt đầu.");
                    return;
                };
                let note_folder = self.notes_mgr.notes[index].folder.clone();
                let note_icon = self.notes_mgr.notes[index].icon.clone();
                let note_title = self.notes_mgr.notes[index].title.clone();
                let note_updated = self.notes_mgr.notes[index].updated_at.clone();
                ui.horizontal(|ui| {
                    theme::caption(ui, "Thư mục:");
                    let mut chosen_folder = note_folder.clone();
                    egui::ComboBox::from_id_salt("note-move-folder")
                        .width(160.0)
                        .selected_text(&note_folder)
                        .show_ui(ui, |ui| {
                            for f in &self.notes_mgr.folders {
                                if ui.selectable_label(note_folder == *f, f).clicked() {
                                    chosen_folder = f.clone();
                                }
                            }
                        });
                    if chosen_folder != note_folder {
                        self.notes_mgr.move_selected_note_to_folder(&chosen_folder);
                    }
                });
                ui.horizontal(|ui| {
                    let current_glyph = theme::glyph_from_name(&note_icon);
                    let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
                    theme::draw_icon(ui.painter(), icon_rect.center(), current_glyph, theme::TEXT);

                    let mut chosen_icon = note_icon.clone();
                    egui::ComboBox::from_id_salt("note-icon-picker")
                        .width(130.0)
                        .selected_text(
                            theme::NOTE_ICONS
                                .iter()
                                .find(|(k, _, _)| *k == note_icon)
                                .map(|(_, _, label)| *label)
                                .unwrap_or("Văn bản"),
                        )
                        .show_ui(ui, |ui| {
                            for (key, glyph, label) in theme::NOTE_ICONS {
                                ui.horizontal(|ui| {
                                    let (g_rect, _) =
                                        ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                                    theme::draw_icon(
                                        ui.painter(),
                                        g_rect.center(),
                                        *glyph,
                                        theme::TEXT,
                                    );
                                    if ui.selectable_label(note_icon == *key, *label).clicked() {
                                        chosen_icon = key.to_string();
                                    }
                                });
                            }
                        });
                    if chosen_icon != note_icon {
                        self.notes_mgr.set_selected_note_icon(&chosen_icon);
                    }

                    ui.add_sized(
                        [(ui.available_width() - 68.0).max(120.0), 36.0],
                        egui::Label::new(egui::RichText::new(&note_title).size(24.0).strong())
                            .truncate(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Xóa").clicked() {
                            self.pending_note_delete = !self.pending_note_delete;
                        }
                    });
                });
                theme::caption(ui, format!("Đã lưu · {}", note_updated));
                if self.pending_note_delete {
                    let mut remove = false;
                    ui.horizontal(|ui| {
                        ui.label("Xóa ghi chú này?");
                        remove = ui.button("Xóa ghi chú").clicked();
                        if ui.button("Hủy").clicked() {
                            self.pending_note_delete = false;
                        }
                    });
                    if remove {
                        self.notes_mgr.delete_selected_note();
                        self.pending_note_delete = false;
                        return;
                    }
                }
                ui.add_space(10.0);
                let editor_height = (ui.available_height() - 90.0).max(120.0);
                let mut changed = false;
                egui::Frame::new()
                    .fill(theme::SIDEBAR)
                    .corner_radius(12)
                    .inner_margin(18)
                    .show(ui, |ui| {
                        ui.set_min_height(editor_height - 36.0);
                        egui::ScrollArea::vertical()
                            .id_salt("note-editor-scroll")
                            .max_height(editor_height - 36.0)
                            .show(ui, |ui| {
                                changed = ui
                                    .add(
                                        egui::TextEdit::multiline(
                                            &mut self.notes_mgr.notes[index].content,
                                        )
                                        .frame(egui::Frame::NONE)
                                        .font(egui::FontId::proportional(15.0))
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(8)
                                        .hint_text("Viết ý tưởng, kế hoạch hoặc ghi chú ở đây…"),
                                    )
                                    .changed();
                            });
                    });
                if changed {
                    self.notes_mgr.save_selected_note();
                }
                let attachments = self.notes_mgr.notes[index].attachments.clone();
                ui.add_space(2.0);
                egui::CollapsingHeader::new(format!("Tệp đính kèm · {}", attachments.len())).show(
                    ui,
                    |ui| {
                        ui.horizontal(|ui| {
                            theme::icon(ui, Glyph::Attach, "Đính kèm tài liệu", true);
                            if ui.button("Thêm tệp").clicked() {
                                let paths = system::pick_files();
                                self.notes_mgr.upload_files_to_selected(&paths);
                            }
                            theme::caption(ui, "Hoặc kéo thả tệp vào cửa sổ");
                        });
                        let mut removed = None;
                        for att in attachments {
                            ui.horizontal(|ui| {
                                ui.add(egui::Label::new(&att.name).truncate());
                                theme::caption(ui, system::format_size(att.size_bytes));
                                let path = self.notes_mgr.root_dir.join(&att.relative_path);
                                if ui.small_button("Mở").clicked() {
                                    let _ = system::open(&path.to_string_lossy());
                                }
                                if ui.small_button("Vị trí").clicked() {
                                    let _ = system::reveal(&path);
                                }
                                if ui
                                    .small_button("Gỡ")
                                    .on_hover_text("Gỡ bản sao đính kèm khỏi ghi chú")
                                    .clicked()
                                {
                                    removed = Some((att.relative_path.clone(), path));
                                }
                            });
                        }
                        if let Some((relative, path)) = removed {
                            self.notes_mgr.notes[index]
                                .attachments
                                .retain(|a| a.relative_path != relative);
                            let _ = std::fs::remove_file(path);
                        }
                    },
                );
                if let Some(status) = &self.notes_mgr.status_message {
                    theme::caption(ui, status);
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
                            if self
                                .search_state
                                .query
                                .trim()
                                .to_lowercase()
                                .starts_with("@ui")
                                || item.size == "[UI]"
                            {
                                self.ai_prompt = self.search_state.query.trim().to_string();
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
                        if self
                            .search_state
                            .query
                            .trim()
                            .to_lowercase()
                            .starts_with("@ui")
                        {
                            self.ai_prompt = self.search_state.query.trim().to_string();
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
                    && self
                        .search_state
                        .query
                        .trim()
                        .to_lowercase()
                        .starts_with("@ui")
                {
                    // Cho phép Enter kích hoạt @ui ngay cả khi không có danh sách
                    self.ai_prompt = self.search_state.query.trim().to_string();
                    self.search_open = false;
                    self.ai_open = true;
                    self.send_ai_prompt(&ctx);
                    open = false;
                }
            });

        open
    }

    pub(super) fn ai_ui(&mut self, viewport_ui: &mut egui::Ui) -> bool {
        let mut open = true;
        let ctx = viewport_ui.ctx().clone();
        let busy = self.ai_job.is_some() || self.ui_agent.is_active;
        if ctx.input(|i| i.viewport().close_requested()) {
            open = false;
        }
        // Escape dismisses a menu first, then stops a task, then closes the dock.
        if !egui::Popup::is_any_open(&ctx)
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            if busy {
                self.cancel_ai();
            } else {
                open = false;
            }
        }
        let plain_enter = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            let path = file.path().to_path_buf();
            if path.is_file() && !self.ai_attached_files.contains(&path) {
                self.ai_attached_files.push(path);
            }
        }
        let has_content = !self.ai_history.is_empty() || busy || self.ai_status.is_some();
        let ui_mode = ai::ui_command(&self.ai_prompt).is_some() || self.ui_agent.is_active;
        let mut reset = false;
        let mut attach = false;
        let panel = egui::CentralPanel::default()
            .frame(if has_content {
                egui::Frame::new()
                    .fill(theme::PANEL)
                    .stroke(Stroke::new(1.0, theme::BORDER))
                    .corner_radius(16)
                    .inner_margin(10)
            } else {
                egui::Frame::NONE.inner_margin(10)
            })
            .show(viewport_ui, |ui| {
                if !has_content {
                    let height =
                        composer_height(&self.ai_prompt, (ui.available_width() - 114.0).max(1.0))
                            + 22.0;
                    let rect = Rect::from_min_size(
                        ui.max_rect().min - Vec2::splat(10.0),
                        Vec2::new(ui.available_width() + 20.0, height),
                    );
                    ui.painter().rect(
                        rect,
                        24,
                        theme::PANEL,
                        Stroke::new(1.0, theme::BORDER),
                        egui::StrokeKind::Inside,
                    );
                }
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                // Allocate the editor's column explicitly. TextEdit can grow vertically,
                // so the scroll area bounds both pasted lines and soft-wrapped lines.
                ui.horizontal_top(|ui| {
                    let mode = theme::icon(
                        ui,
                        if ui_mode { Glyph::Camera } else { Glyph::Spark },
                        if ui_mode {
                            "Điều khiển màn hình · chuyển sang Chat"
                        } else {
                            "Chat · chuyển sang @ui"
                        },
                        !busy,
                    );
                    if mode.clicked() && !busy {
                        self.ai_prompt = if let Some(goal) = ai::ui_command(&self.ai_prompt) {
                            goal.to_owned()
                        } else {
                            format!("@ui {}", self.ai_prompt)
                        };
                        self.ai_focus = 1;
                    }
                    let width = (ui.available_width() - 76.0).max(1.0);
                    let editor_height = composer_height(&self.ai_prompt, width).min(
                        (ui.available_height() - if has_content { 100.0 } else { 0.0 }).max(32.0),
                    );
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
                                                .font(egui::FontId::proportional(14.0))
                                                .desired_width(ui.available_width())
                                                .desired_rows(1)
                                                .min_size(Vec2::new(0.0, 32.0))
                                                .lock_focus(true)
                                                .return_key(egui::KeyboardShortcut::new(
                                                    egui::Modifiers::SHIFT,
                                                    egui::Key::Enter,
                                                ))
                                                .hint_text(if ui_mode {
                                                    "Mô tả thao tác trên màn hình…"
                                                } else {
                                                    "Hỏi AI…"
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
                    if self.ai_focus > 0 && !self.ui_agent.is_active {
                        ui.ctx().send_viewport_cmd(ViewportCommand::Focus);
                        editor.request_focus();
                        self.ai_focus = if editor.has_focus() {
                            0
                        } else {
                            self.ai_focus.saturating_sub(1)
                        };
                    }
                    let attachment_label = if self.ai_attached_files.is_empty() {
                        "Đính kèm tệp".to_owned()
                    } else {
                        format!(
                            "{} tệp đính kèm · chuột phải để gỡ",
                            self.ai_attached_files.len()
                        )
                    };
                    let files = theme::icon(ui, Glyph::Attach, &attachment_label, !busy);
                    attach = files.clicked() && !busy;
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
                    let can_send = !busy
                        && (!self.ai_prompt.trim().is_empty()
                            || !self.ai_attached_files.is_empty());
                    if busy {
                        let stop = theme::icon(ui, Glyph::Stop, "Dừng tác vụ · Esc", true);
                        if stop.clicked() {
                            self.cancel_ai();
                        }
                    } else if can_send {
                        if theme::icon(ui, Glyph::Arrow, "Gửi · Enter", true).clicked()
                            || (plain_enter && editor.has_focus())
                        {
                            self.send_ai_prompt(&ctx);
                        }
                    } else if theme::icon(ui, Glyph::Close, "Đóng · Esc", true).clicked() {
                        open = false;
                    }
                });

                if has_content {
                    ui.separator();
                    let header = ui
                        .horizontal(|ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(if self.ui_agent.is_active {
                                        format!("Màn hình · Bước {}", self.ui_agent.current_step)
                                    } else if self.ai_mode == crate::AiMode::UiTars {
                                        "Điều khiển màn hình".to_owned()
                                    } else {
                                        "PentaKuru".to_owned()
                                    })
                                    .small()
                                    .color(theme::MUTED),
                                )
                                .truncate(),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if busy
                                        && theme::icon(ui, Glyph::Close, "Đóng khung chat", true)
                                            .clicked()
                                    {
                                        open = false;
                                    }
                                    if !busy
                                        && theme::icon(ui, Glyph::Plus, "Hội thoại mới", true)
                                            .clicked()
                                    {
                                        reset = true;
                                    }
                                },
                            );
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
                                            for img in &message.images {
                                                if let Some(tex) = get_or_load_thumbnail(
                                                    ui.ctx(),
                                                    &mut self.thumbnail_cache,
                                                    img,
                                                ) {
                                                    ui.add(
                                                        egui::Image::new(&tex)
                                                            .max_width(
                                                                ui.available_width().min(320.0),
                                                            )
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
                                                .add_enabled(
                                                    !busy,
                                                    egui::Button::new("Hội thoại mới"),
                                                )
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
                            if self.ui_agent.is_active {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label(format!("Bước {}", self.ui_agent.current_step));
                                });
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&self.ui_agent.last_status_message)
                                            .small()
                                            .color(theme::MUTED),
                                    )
                                    .wrap(),
                                );
                            } else if let Some(status) = &self.ai_status {
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
            });
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
        if attach {
            for path in system::pick_files() {
                if !self.ai_attached_files.contains(&path) {
                    self.ai_attached_files.push(path);
                }
            }
        }
        open
    }

    fn chat_menu(&mut self, ui: &mut egui::Ui, open: &mut bool, reset: &mut bool) {
        compact_menu(ui);
        let busy = self.ai_job.is_some() || self.ui_agent.is_active;
        if busy && ui.button("Dừng tác vụ").clicked() {
            self.cancel_ai();
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
        self.ui_tars_executed_clicks.clear();
        self.ui_tars_last_screenshot = None;
        self.ui_agent = ai::UiAgentState::default();
        self.ai_mode = crate::AiMode::Chat;
        self.ai_focus = 1;
    }
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
}
