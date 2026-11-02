use crate::{PentaKuru, ui_theme as theme};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const COLUMNS: [(&str, &str); 3] = [
    ("todo", "Cần làm"),
    ("doing", "Đang làm"),
    ("done", "Hoàn tất"),
];

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Card {
    pub id: String,
    pub title: String,
    pub note: String,
    pub status: String,
    #[serde(default)]
    pub archived: bool,
}

fn card_content(card: &Card) -> String {
    let body = match card.note.split_once('\n') {
        Some((first, rest)) if first.starts_with('#') => rest.trim_start_matches('\n'),
        _ if card.note.starts_with('#') => "",
        _ => card.note.as_str(),
    };
    format!("# {}\n\n{body}", card.title)
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Board {
    pub cards: Vec<Card>,
}

#[derive(Default)]
pub struct State {
    pub memory: crate::note_memory::State,
    pub library_open: bool,
    pub preview: bool,
    pub board_open: bool,
    pub profile_open: bool,
    pub board: Board,
    pub status: String,
    loaded_root: Option<PathBuf>,
    pub(super) storage_ready: bool,
    pub(super) new_card_open: bool,
    new_title: String,
    new_content: String,
    show_archived: bool,
    narrow_column: usize,
    editing: Option<Card>,
    pub ai_note_input: String,
    pub ai_note_bar_open: bool,
    pub pending_note_command: bool,
    pub pending_note_target_id: Option<String>,
}
impl State {
    pub fn with_preview() -> Self {
        Self {
            preview: true,
            ..Default::default()
        }
    }
}

enum Request {
    Create {
        title: String,
        content: String,
    },
    FromNote {
        note_id: String,
        card_id: Option<String>,
        title: String,
        content: String,
    },
    Edit(Card),
    Move {
        card_id: String,
        status: String,
    },
    Archive {
        card_id: String,
        archived: bool,
    },
}

impl Board {
    fn load(root: &Path) -> Result<Self, String> {
        match std::fs::read(root.join(".kanban.json")) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| error.to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.to_string()),
        }
    }
    fn save(&self, root: &Path) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp = root.join(format!(".kanban-{}-{stamp}.tmp", std::process::id()));
        std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
        if let Err(error) = std::fs::rename(&temp, root.join(".kanban.json")) {
            let _ = std::fs::remove_file(temp);
            return Err(error.to_string());
        }
        Ok(())
    }
    fn create(&mut self, title: String, note: String) -> Result<String, String> {
        let title = title.trim().to_owned();
        if title.is_empty() {
            return Err("Nhập tiêu đề công việc.".into());
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let id = format!("local-{}-{stamp}-{}", std::process::id(), self.cards.len());
        self.cards.push(Card {
            id: id.clone(),
            title,
            note,
            status: "todo".into(),
            archived: false,
        });
        Ok(id)
    }
    fn apply(&mut self, operation: Request) -> Result<Option<(String, String)>, String> {
        match operation {
            Request::Create { title, content } => {
                self.create(title, content)?;
            }
            Request::FromNote {
                note_id,
                card_id,
                title,
                content,
            } => {
                let id = if let Some(card) = self
                    .cards
                    .iter_mut()
                    .find(|card| Some(&card.id) == card_id.as_ref())
                {
                    card.title = title;
                    card.note = content;
                    card.archived = false;
                    card.id.clone()
                } else {
                    self.create(title, content)?
                };
                return Ok(Some((note_id, id)));
            }
            Request::Edit(updated) => {
                if updated.title.trim().is_empty() {
                    return Err("Nhập tiêu đề công việc.".into());
                }
                let card = self
                    .cards
                    .iter_mut()
                    .find(|card| card.id == updated.id)
                    .ok_or("Không tìm thấy thẻ.")?;
                card.title = updated.title.trim().into();
                card.note = updated.note;
            }
            Request::Move { card_id, status } => {
                if !COLUMNS.iter().any(|(value, _)| *value == status) {
                    return Err("Trạng thái không hợp lệ.".into());
                }
                let card = self
                    .cards
                    .iter_mut()
                    .find(|card| card.id == card_id)
                    .ok_or("Không tìm thấy thẻ.")?;
                card.status = status;
            }
            Request::Archive { card_id, archived } => {
                self.cards
                    .iter_mut()
                    .find(|card| card.id == card_id)
                    .ok_or("Không tìm thấy thẻ.")?
                    .archived = archived;
            }
        }
        Ok(None)
    }

    pub fn stats(&self) -> (usize, usize, usize, usize) {
        let mut todo = 0;
        let mut doing = 0;
        let mut done = 0;
        for card in self.cards.iter().filter(|c| !c.archived) {
            match card.status.as_str() {
                "doing" => doing += 1,
                "done" => done += 1,
                _ => todo += 1,
            }
        }
        (todo + doing + done, todo, doing, done)
    }

    pub fn ai_context_summary(&self) -> String {
        let (total, todo, doing, done) = self.stats();
        if total == 0 {
            return "Chưa có công việc nào trong Kanban.".into();
        }
        let mut out = format!(
            "Thống kê công việc Kanban: Tổng {total} việc (Cần làm: {todo}, Đang làm: {doing}, Hoàn tất: {done}).\n"
        );
        for (status_key, status_label) in COLUMNS {
            let items: Vec<_> = self
                .cards
                .iter()
                .filter(|c| !c.archived && c.status == status_key)
                .collect();
            if !items.is_empty() {
                out.push_str(&format!("- {status_label}:\n"));
                for card in items.into_iter().take(25) {
                    let desc: String = card.note.lines().next().unwrap_or("").chars().take(80).collect();
                    if desc.trim().is_empty() {
                        out.push_str(&format!("  • {}\n", card.title));
                    } else {
                        out.push_str(&format!("  • {}: {}\n", card.title, desc.trim()));
                    }
                }
            }
        }
        out
    }

    pub fn upsert_tasks(&mut self, tasks: &[(String, String, String)]) -> (usize, Option<String>) {
        let mut count = 0;
        let mut first_id = None;
        for (title, note, status) in tasks {
            let clean_title = title.trim();
            if clean_title.is_empty() {
                continue;
            }
            let valid_status = if COLUMNS.iter().any(|(k, _)| *k == status.as_str()) {
                status.as_str()
            } else {
                "todo"
            };
            if let Some(existing) = self
                .cards
                .iter_mut()
                .find(|c| c.title.eq_ignore_ascii_case(clean_title))
            {
                if !note.trim().is_empty() {
                    existing.note = note.clone();
                }
                existing.status = valid_status.to_owned();
                existing.archived = false;
                if first_id.is_none() {
                    first_id = Some(existing.id.clone());
                }
                count += 1;
            } else if let Ok(id) = self.create(clean_title.to_owned(), note.clone()) {
                if let Some(card) = self.cards.iter_mut().find(|c| c.id == id) {
                    card.status = valid_status.to_owned();
                }
                if first_id.is_none() {
                    first_id = Some(id);
                }
                count += 1;
            }
        }
        (count, first_id)
    }
}

impl PentaKuru {
    pub(super) fn poll_workspace(&mut self) {
        let root = &self.notes_mgr.root_dir;
        if self.workspace.loaded_root.as_ref() == Some(root) {
            return;
        }
        self.workspace.loaded_root = Some(root.clone());
        match Board::load(root) {
            Ok(board) => {
                self.workspace.board = board;
                self.workspace.storage_ready = true;
            }
            Err(error) => {
                self.workspace.storage_ready = false;
                self.workspace.status =
                    format!("Chưa đọc được Kanban: {error}. Dữ liệu hiện có được giữ nguyên.");
            }
        }
    }
    fn board_request(&mut self, operation: Request) -> bool {
        if !self.workspace.storage_ready {
            return false;
        }
        let mut board = self.workspace.board.clone();
        match board
            .apply(operation)
            .and_then(|link| board.save(&self.notes_mgr.root_dir).map(|()| link))
        {
            Ok(link) => {
                self.workspace.board = board;
                if let Some((note_id, card_id)) = link {
                    let selected = self.notes_mgr.selected_note_id.clone();
                    if let Some(note) = self
                        .notes_mgr
                        .notes
                        .iter_mut()
                        .find(|note| note.id == note_id)
                    {
                        note.task_card_id = Some(card_id);
                        self.notes_mgr.selected_note_id = Some(note_id);
                        self.notes_mgr.save_selected_note();
                        self.notes_mgr.selected_note_id = selected;
                    }
                }
                self.workspace.status = "Đã lưu Kanban trên máy.".into();
                true
            }
            Err(error) => {
                self.workspace.status = format!("Chưa lưu được: {error}");
                false
            }
        }
    }

    /// Đồng bộ ghi chú hiện tại sang Kanban (tự tách các dòng công việc `- [ ]` hoặc tạo thẻ từ note).
    pub(super) fn sync_note_to_kanban(&mut self, index: usize) {
        self.poll_workspace();
        if !self.workspace.storage_ready || index >= self.notes_mgr.notes.len() {
            return;
        }
        let note_id = self.notes_mgr.notes[index].id.clone();
        let note_title = self.notes_mgr.notes[index].title.clone();
        let note_content = self.notes_mgr.notes[index].content.clone();
        let note_card_id = self.notes_mgr.notes[index].task_card_id.clone();

        let plan = crate::ai::parse_ai_note_response(&note_content, &note_title);
        if !plan.tasks.is_empty() {
            let mut board = self.workspace.board.clone();
            let (count, first_id) = board.upsert_tasks(&plan.tasks);
            if count > 0 && board.save(&self.notes_mgr.root_dir).is_ok() {
                self.workspace.board = board;
                if let Some(card_id) = first_id {
                    self.notes_mgr.notes[index].task_card_id = Some(card_id);
                    self.notes_mgr.save_selected_note();
                }
                self.workspace.status = format!("Đã đồng bộ {count} công việc vào Kanban.");
                return;
            }
        }
        self.board_request(Request::FromNote {
            note_id,
            card_id: note_card_id,
            title: note_title,
            content: note_content,
        });
    }

    /// Áp dụng kết quả từ lệnh `@note` của AI: tạo hoặc cập nhật ghi chú và tự động tạo công việc trong section Kanban.
    pub(super) fn apply_ai_note_plan(&mut self, reply: &str) -> String {
        self.poll_workspace();
        let target_id = self.workspace.pending_note_target_id.take();
        let plan = crate::ai::parse_ai_note_response(reply, "Ghi chú & Công việc AI");

        let note_id = if let Some(existing_id) = target_id
            && let Some(note) = self
                .notes_mgr
                .notes
                .iter_mut()
                .find(|n| n.id == existing_id)
        {
            note.title = plan.title.clone();
            note.content = plan.content.clone();
            note.ai_shared = true;
            self.notes_mgr.selected_note_id = Some(existing_id.clone());
            self.notes_mgr.save_selected_note();
            existing_id
        } else {
            let new_id = self.notes_mgr.create_note(&plan.title, &plan.content);
            if let Some(note) = self.notes_mgr.notes.iter_mut().find(|n| n.id == new_id) {
                note.ai_shared = true;
                note.icon = if plan.tasks.is_empty() {
                    "spark".into()
                } else {
                    "check".into()
                };
            }
            self.notes_mgr.save_selected_note();
            new_id
        };

        let mut synced_tasks = 0;
        if self.workspace.storage_ready && !plan.tasks.is_empty() {
            let mut board = self.workspace.board.clone();
            let (count, first_card_id) = board.upsert_tasks(&plan.tasks);
            if count > 0 && board.save(&self.notes_mgr.root_dir).is_ok() {
                self.workspace.board = board;
                synced_tasks = count;
                if let Some(card_id) = first_card_id
                    && let Some(note) = self.notes_mgr.notes.iter_mut().find(|n| n.id == note_id)
                {
                    note.task_card_id = Some(card_id);
                    self.notes_mgr.save_selected_note();
                }
            }
        }

        self.notes_open = true;
        self.workspace.preview = true;
        let summary_msg = if synced_tasks > 0 {
            format!(
                "Đã tạo ghi chú «{}» và thêm {} công việc vào Kanban.",
                plan.title, synced_tasks
            )
        } else {
            format!("Đã lưu ghi chú «{}» từ lệnh @note.", plan.title)
        };
        self.workspace.status = summary_msg.clone();
        summary_msg
    }

    /// Kích hoạt AI thống kê các công việc người dùng làm và tổng hợp vào ghi chú.
    pub(super) fn trigger_ai_work_stats(&mut self, ctx: &egui::Context) {
        self.poll_workspace();
        let stats_context = self.workspace.board.ai_context_summary();
        self.workspace.pending_note_target_id = None;
        self.ai_prompt = format!(
            "@note Thống kê các công việc tôi đang làm, đã hoàn tất và cần làm tiếp theo dựa trên bảng công việc hiện tại. Phân tích ngắn gọn tiến độ và tạo danh sách việc ưu tiên tiếp theo.\n{stats_context}"
        );
        self.ai_open = true;
        self.send_ai_prompt(ctx);
    }

    pub(super) fn note_inline_ai_bar(&mut self, ui: &mut egui::Ui, target_note_id: Option<String>) {
        if !self.workspace.ai_note_bar_open {
            return;
        }
        let busy = self.ai_job.is_some() || self.ui_agent.is_active;
        egui::Frame::new()
            .fill(theme::SIDEBAR)
            .stroke(egui::Stroke::new(1.0, theme::BORDER))
            .corner_radius(8)
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 2.0);
                    ui.label(
                        egui::RichText::new("@note")
                            .size(11.0)
                            .strong()
                            .color(egui::Color32::from_rgb(100, 220, 175)),
                    );
                    let width = (ui.available_width() - 56.0).max(80.0);
                    let resp = ui.add_sized(
                        [width, 22.0],
                        egui::TextEdit::singleline(&mut self.workspace.ai_note_input)
                            .frame(egui::Frame::NONE)
                            .font(egui::FontId::proportional(12.0))
                            .hint_text("Nhập yêu cầu để AI viết ghi chú & tạo công việc…"),
                    );
                    let enter = resp.has_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.is_none());
                    let can_run = !busy && !self.workspace.ai_note_input.trim().is_empty();
                    if theme::mini_icon(
                        ui,
                        theme::Glyph::Arrow,
                        "Gửi lệnh @note tạo ghi chú & công việc (Enter)",
                        can_run,
                        false,
                    )
                    .clicked()
                        || (enter && can_run)
                    {
                        let raw = self.workspace.ai_note_input.trim().to_string();
                        self.workspace.ai_note_input.clear();
                        self.workspace.pending_note_target_id = target_note_id;
                        self.ai_prompt = if crate::ai::note_command(&raw).is_some() {
                            raw
                        } else {
                            format!("@note {raw}")
                        };
                        self.send_ai_prompt(ui.ctx());
                    }
                    if theme::mini_icon(
                        ui,
                        theme::Glyph::Chart,
                        "AI thống kê tiến độ các việc đang làm",
                        !busy,
                        false,
                    )
                    .clicked()
                    {
                        self.trigger_ai_work_stats(ui.ctx());
                    }
                });
            });
    }

    pub(super) fn kanban_ui(
        &mut self,
        ui: &mut egui::Ui,
    ) -> egui::scroll_area::ScrollAreaOutput<()> {
        ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
        egui::ScrollArea::vertical()
            .id_salt("kanban-page")
            .auto_shrink([false, false])
            .min_scrolled_height(0.0)
            .max_height(ui.available_height())
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                self.kanban_contents_ui(ui);
            })
    }

    fn kanban_contents_ui(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = egui::vec2(5.0, 5.0);
        ui.spacing_mut().button_padding = egui::vec2(6.0, 3.0);
        self.poll_workspace();

        let (total, todo_c, doing_c, done_c) = self.workspace.board.stats();
        let pct = if total > 0 { (done_c * 100) / total } else { 0 };
        let busy = self.ai_job.is_some() || self.ui_agent.is_active;

        // Hàng điều khiển Kanban thu gọn cỡ chữ nhỏ + Icon mini
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
            ui.label(egui::RichText::new("Công việc").size(12.5).strong());
            ui.label(
                egui::RichText::new(format!(
                    "· {total} việc ({pct}% hoàn tất)"
                ))
                .size(10.5)
                .color(theme::MUTED),
            );
            let form_label = if self.workspace.new_card_open {
                "− Thu gọn"
            } else {
                "+ Công việc mới"
            };
            if ui
                .selectable_label(
                    self.workspace.new_card_open,
                    egui::RichText::new(form_label).size(11.0),
                )
                .clicked()
            {
                self.workspace.new_card_open = !self.workspace.new_card_open;
            }
            if theme::mini_icon(
                ui,
                theme::Glyph::Spark,
                "Lệnh @note: AI viết ghi chú & tạo danh sách công việc",
                true,
                self.workspace.ai_note_bar_open,
            )
            .clicked()
            {
                self.workspace.ai_note_bar_open = !self.workspace.ai_note_bar_open;
            }
            if theme::mini_icon(
                ui,
                theme::Glyph::Chart,
                "AI thống kê các việc người dùng làm",
                !busy,
                false,
            )
            .clicked()
            {
                self.trigger_ai_work_stats(ui.ctx());
            }
            if theme::mini_icon(
                ui,
                theme::Glyph::Archive,
                "Chỉ xem thẻ đã lưu trữ",
                true,
                self.workspace.show_archived,
            )
            .clicked()
            {
                self.workspace.show_archived = !self.workspace.show_archived;
            }
        });

        self.note_inline_ai_bar(ui, None);

        if !self.workspace.storage_ready {
            theme::caption(ui, &self.workspace.status);
            if ui.small_button("Thử đọc lại").clicked() {
                self.workspace.loaded_root = None;
            }
            return;
        }
        if self.workspace.new_card_open {
            egui::Frame::new()
                .fill(theme::SIDEBAR)
                .corner_radius(8)
                .inner_margin(6)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.workspace.new_title)
                            .font(egui::FontId::proportional(11.5))
                            .hint_text("Tiêu đề công việc")
                            .desired_width(f32::INFINITY),
                    );
                    ui.add(
                        egui::TextEdit::multiline(&mut self.workspace.new_content)
                            .font(egui::FontId::proportional(11.0))
                            .hint_text("Mô tả ngắn")
                            .desired_rows(2)
                            .desired_width(f32::INFINITY),
                    );
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add_enabled(
                                !self.workspace.new_title.trim().is_empty(),
                                egui::Button::new("Thêm công việc"),
                            )
                            .clicked()
                        {
                            if self.board_request(Request::Create {
                                title: self.workspace.new_title.clone(),
                                content: self.workspace.new_content.clone(),
                            }) {
                                self.workspace.new_title.clear();
                                self.workspace.new_content.clear();
                                self.workspace.new_card_open = false;
                            }
                        }
                        if ui.small_button("Hủy").clicked() {
                            self.workspace.new_card_open = false;
                        }
                    });
                });
        }
        if let Some(mut card) = self.workspace.editing.take() {
            egui::Frame::new()
                .fill(theme::SIDEBAR)
                .stroke(egui::Stroke::new(1.0, theme::BORDER))
                .corner_radius(8)
                .inner_margin(6)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Sửa công việc").size(11.5).strong());
                    ui.add(
                        egui::TextEdit::singleline(&mut card.title)
                            .font(egui::FontId::proportional(11.5))
                            .desired_width(f32::INFINITY),
                    );
                    ui.add(
                        egui::TextEdit::multiline(&mut card.note)
                            .font(egui::FontId::proportional(11.0))
                            .desired_rows(2)
                            .desired_width(f32::INFINITY),
                    );
                    let mut close = false;
                    ui.horizontal(|ui| {
                        if ui.small_button("Lưu").clicked() {
                            close = self.board_request(Request::Edit(card.clone()));
                        }
                        if ui.small_button("Hủy").clicked() {
                            close = true;
                        }
                    });
                    if !close {
                        self.workspace.editing = Some(card);
                    }
                });
        }
        if !self.workspace.status.is_empty() {
            ui.label(
                egui::RichText::new(&self.workspace.status)
                    .size(10.0)
                    .color(theme::MUTED),
            );
        }

        let cards = self.workspace.board.cards.clone();
        let mut operation = None;

        // Gom tất cả các công việc nằm gọn trong 1 Section tối ưu duy nhất
        egui::Frame::new()
            .fill(theme::SIDEBAR)
            .stroke(egui::Stroke::new(1.0, theme::BORDER))
            .corner_radius(10)
            .inner_margin(8)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                // Thanh lọc trạng thái chữ nhỏ ngay đầu section
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                    let all_count = cards
                        .iter()
                        .filter(|c| c.archived == self.workspace.show_archived)
                        .count();
                    ui.selectable_value(
                        &mut self.workspace.narrow_column,
                        3,
                        egui::RichText::new(format!("Tất cả ({all_count})")).size(11.0),
                    );
                    for (index, (status, label)) in COLUMNS.iter().enumerate() {
                        let count = cards
                            .iter()
                            .filter(|card| {
                                card.status == *status
                                    && card.archived == self.workspace.show_archived
                            })
                            .count();
                        ui.selectable_value(
                            &mut self.workspace.narrow_column,
                            index,
                            egui::RichText::new(format!("{label} ({count})")).size(11.0),
                        );
                    }
                    ui.label(
                        egui::RichText::new(format!(
                            "Cần làm {todo_c} · Đang làm {doing_c} · Xong {done_c}"
                        ))
                        .size(10.0)
                        .color(theme::MUTED),
                    );
                });
                ui.separator();

                let groups: Vec<(&str, &str)> = match self.workspace.narrow_column {
                    0 => vec![COLUMNS[0]],
                    1 => vec![COLUMNS[1]],
                    2 => vec![COLUMNS[2]],
                    _ => COLUMNS.to_vec(),
                };

                for (status, label) in groups {
                    let group_cards: Vec<_> = cards
                        .iter()
                        .filter(|c| {
                            c.status == status && c.archived == self.workspace.show_archived
                        })
                        .collect();
                    let (_, dropped) = ui.dnd_drop_zone::<String, _>(
                        egui::Frame::new()
                            .fill(theme::PANEL)
                            .corner_radius(6)
                            .inner_margin(6),
                        |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(
                                egui::RichText::new(format!("{} · {}", label, group_cards.len()))
                                    .size(11.5)
                                    .strong()
                                    .color(theme::MUTED),
                            );
                            if group_cards.is_empty() {
                                ui.label(
                                    egui::RichText::new("Chưa có mục nào · kéo thả thẻ vào đây")
                                        .size(10.0)
                                        .color(theme::MUTED),
                                );
                            }
                            for card in group_cards {
                                ui.push_id(&card.id, |ui| {
                                    egui::Frame::new()
                                        .fill(theme::SIDEBAR)
                                        .stroke(egui::Stroke::new(1.0, theme::BORDER))
                                        .corner_radius(6)
                                        .inner_margin(6)
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.dnd_drag_source(
                                                egui::Id::new(("kanban-card", &card.id)),
                                                card.id.clone(),
                                                |ui| {
                                                    ui.set_max_width(
                                                        (ui.available_width() - 8.0).max(120.0),
                                                    );
                                                    ui.add(
                                                        egui::Label::new(
                                                            egui::RichText::new(&card.title)
                                                                .size(11.5)
                                                                .strong(),
                                                        )
                                                        .wrap(),
                                                    );
                                                    let snippet: String = card
                                                        .note
                                                        .chars()
                                                        .take(100)
                                                        .collect();
                                                    if !snippet.trim().is_empty() {
                                                        ui.add(
                                                            egui::Label::new(
                                                                egui::RichText::new(snippet)
                                                                    .size(10.5)
                                                                    .color(theme::MUTED),
                                                            )
                                                            .wrap(),
                                                        );
                                                    }
                                                },
                                            );
                                            ui.horizontal_wrapped(|ui| {
                                                ui.spacing_mut().item_spacing =
                                                    egui::vec2(4.0, 2.0);
                                                egui::ComboBox::from_id_salt(("status", &card.id))
                                                    .width(86.0)
                                                    .selected_text(
                                                        egui::RichText::new(label).size(10.5),
                                                    )
                                                    .show_ui(ui, |ui| {
                                                        for (next, title) in COLUMNS {
                                                            if ui
                                                                .selectable_label(
                                                                    card.status == next,
                                                                    egui::RichText::new(title)
                                                                        .size(11.0),
                                                                )
                                                                .clicked()
                                                            {
                                                                operation = Some(Request::Move {
                                                                    card_id: card.id.clone(),
                                                                    status: next.into(),
                                                                });
                                                            }
                                                        }
                                                    });
                                                if theme::mini_icon(
                                                    ui,
                                                    theme::Glyph::Edit,
                                                    "Sửa công việc",
                                                    true,
                                                    false,
                                                )
                                                .clicked()
                                                {
                                                    self.workspace.editing = Some(card.clone());
                                                }
                                                if theme::mini_icon(
                                                    ui,
                                                    theme::Glyph::Archive,
                                                    if card.archived {
                                                        "Khôi phục thẻ"
                                                    } else {
                                                        "Lưu trữ thẻ"
                                                    },
                                                    true,
                                                    card.archived,
                                                )
                                                .clicked()
                                                {
                                                    operation = Some(Request::Archive {
                                                        card_id: card.id.clone(),
                                                        archived: !card.archived,
                                                    });
                                                }
                                                if theme::mini_icon(
                                                    ui,
                                                    theme::Glyph::File,
                                                    "Mở ghi chú liên kết",
                                                    true,
                                                    false,
                                                )
                                                .clicked()
                                                {
                                                    if let Some(note) =
                                                        self.notes_mgr.notes.iter().find(|note| {
                                                            note.task_card_id.as_ref()
                                                                == Some(&card.id)
                                                        })
                                                    {
                                                        self.notes_mgr.selected_note_id =
                                                            Some(note.id.clone());
                                                    } else {
                                                        let id = self.notes_mgr.create_note(
                                                            &card.title,
                                                            &card_content(card),
                                                        );
                                                        if let Some(note) = self
                                                            .notes_mgr
                                                            .notes
                                                            .iter_mut()
                                                            .find(|note| note.id == id)
                                                        {
                                                            note.task_card_id =
                                                                Some(card.id.clone());
                                                        }
                                                        self.notes_mgr.save_selected_note();
                                                    }
                                                    self.workspace.board_open = false;
                                                }
                                            });
                                        });
                                });
                            }
                        },
                    );
                    if let Some(id) = dropped {
                        operation = Some(Request::Move {
                            card_id: (*id).clone(),
                            status: status.into(),
                        });
                    }
                }
            });

        if let Some(operation) = operation {
            self.board_request(operation);
        }
    }
}

pub fn format_selection(content: &mut String, start: usize, end: usize, marker: &str) {
    let offset = |index| {
        content
            .char_indices()
            .nth(index)
            .map_or(content.len(), |(offset, _)| offset)
    };
    let mut first = offset(start.min(end));
    let last = offset(start.max(end));
    if marker.starts_with('#') {
        first = content[..first].rfind('\n').map_or(0, |index| index + 1);
        let existing = content[first..]
            .chars()
            .take_while(|character| *character == '#')
            .count();
        if existing > 0 && content[first + existing..].starts_with(' ') {
            content.replace_range(first..first + existing + 1, marker);
        } else {
            content.insert_str(first, marker);
        }
    } else {
        let selected = if first == last {
            "văn bản".to_owned()
        } else {
            content[first..last].to_owned()
        };
        content.replace_range(first..last, &format!("{marker}{selected}{marker}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expanded_kanban_forms_stay_inside_resized_viewport_and_collapse_after_create() {
        let ctx = egui::Context::default();
        let mut app = PentaKuru::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            None,
            true,
        );
        let root = std::env::temp_dir().join(format!("penta-kanban-layout-{}", std::process::id()));
        app.notes_mgr = crate::notes::NotesManager::at(root.clone());
        app.poll_workspace();
        app.workspace.new_card_open = true;
        app.workspace.new_title = "Công việc mới".into();
        app.workspace.new_content = "Mô tả công việc".into();
        app.workspace.board.cards = (0..12)
            .map(|i| Card {
                id: format!("card-{i}"),
                title: "Tiêu đề dài khi thu nhỏ cửa sổ ghi chú".repeat(4),
                note: "Nội dung công việc cần được xuống dòng.".repeat(4),
                status: "todo".into(),
                archived: false,
            })
            .collect();
        let render = |app: &mut PentaKuru, size: egui::Vec2, events| {
            let mut bounds = None;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    events,
                    ..Default::default()
                },
                |ui| {
                    let page = app.kanban_ui(ui);
                    bounds = Some((page.inner_rect, page.content_size));
                },
            );
            output.textures_delta.clear();
            (output, bounds.unwrap())
        };
        for size in [
            egui::vec2(380.0, 460.0),
            egui::vec2(480.0, 560.0),
            egui::vec2(900.0, 620.0),
        ] {
            for editing in [false, true] {
                app.workspace.editing = editing.then(|| app.workspace.board.cards[0].clone());
                let (_, (viewport, content)) = render(&mut app, size, vec![]);
                assert!(viewport.right() <= size.x + 1.0 && viewport.bottom() <= size.y + 1.0);
                assert!(
                    content.x <= viewport.width() + 1.0,
                    "Horizontal overflow at {size:?}: {content:?}, {viewport:?}"
                );
                assert!(content.y > viewport.height(), "Long board must scroll");
            }
        }
        app.workspace.editing = None;
        app.workspace.board.cards.clear();
        let (output, _) = render(&mut app, egui::vec2(480.0, 560.0), vec![]);
        let position = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape {
                    (text.galley.job.text == "Thêm công việc")
                        .then(|| text.pos + text.galley.size() * 0.5)
                } else {
                    None
                }
            })
            .expect("Create action must remain visible inside the form");
        for pressed in [true, false] {
            render(
                &mut app,
                egui::vec2(480.0, 560.0),
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert!(
            !app.workspace.new_card_open,
            "Successful creation must collapse the form"
        );
        assert_eq!(Board::load(&root).unwrap().cards[0].title, "Công việc mới");
        // Remove only this test's own files, without a recursive directory delete.
        for entry in std::fs::read_dir(&root).unwrap().flatten() {
            if entry.file_type().unwrap().is_dir() {
                for file in std::fs::read_dir(entry.path()).unwrap().flatten() {
                    std::fs::remove_file(file.path()).unwrap();
                }
                std::fs::remove_dir(entry.path()).unwrap();
            } else {
                std::fs::remove_file(entry.path()).unwrap();
            }
        }
        std::fs::remove_dir(root).unwrap();
    }

    #[test]
    fn formatting_preserves_vietnamese_and_replaces_heading_level() {
        let mut text = "Học tiếng Việt".to_owned();
        format_selection(&mut text, 9, 4, "**");
        assert_eq!(text, "Học **tiếng** Việt");
        format_selection(&mut text, 0, 0, "# ");
        format_selection(&mut text, 3, 3, "## ");
        assert_eq!(text, "## Học **tiếng** Việt");
        let mut empty = String::new();
        format_selection(&mut empty, 0, 0, "*");
        assert_eq!(empty, "*văn bản*");
    }

    #[test]
    fn local_board_persists_note_links_moves_edits_and_archive() {
        let root = std::env::temp_dir().join(format!("penta-kanban-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let mut board = Board::load(&root).unwrap();
        board.cards.clear();
        let link = board
            .apply(Request::FromNote {
                note_id: "note-1".into(),
                card_id: Some("old-remote-id".into()),
                title: "Công việc".into(),
                content: "Nội dung".repeat(2000),
            })
            .unwrap()
            .unwrap();
        assert_eq!(link.0, "note-1");
        let id = link.1;
        board
            .apply(Request::Move {
                card_id: id.clone(),
                status: "doing".into(),
            })
            .unwrap();
        board
            .apply(Request::FromNote {
                note_id: "note-1".into(),
                card_id: Some(id.clone()),
                title: "Đã cập nhật".into(),
                content: "Nội dung mới".into(),
            })
            .unwrap();
        assert_eq!(board.cards.len(), 1);
        assert_eq!(board.cards[0].status, "doing");
        board.save(&root).unwrap();
        let mut reloaded = Board::load(&root).unwrap();
        assert_eq!(reloaded.cards, board.cards);
        reloaded
            .apply(Request::Archive {
                card_id: id,
                archived: true,
            })
            .unwrap();
        reloaded.save(&root).unwrap();
        assert!(Board::load(&root).unwrap().cards[0].archived);
        std::fs::remove_file(root.join(".kanban.json")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn corrupt_board_is_reported_and_invalid_moves_are_rejected() {
        let root =
            std::env::temp_dir().join(format!("penta-kanban-invalid-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(".kanban.json"), "broken").unwrap();
        assert!(Board::load(&root).is_err());
        assert_eq!(
            std::fs::read_to_string(root.join(".kanban.json")).unwrap(),
            "broken"
        );
        let mut board = Board::default();
        assert!(board.create("  ".into(), "".into()).is_err());
        let id = board.create("Task".into(), "".into()).unwrap();
        assert!(
            board
                .apply(Request::Move {
                    card_id: id,
                    status: "invalid".into()
                })
                .is_err()
        );
        assert_eq!(board.cards[0].status, "todo");
        std::fs::remove_file(root.join(".kanban.json")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
