use crate::{PentaKuru, system, ui_theme as theme};
use eframe::egui;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Clone, Deserialize)]
pub struct Card {
    pub id: String,
    pub title: String,
    pub note: String,
    pub status: String,
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

#[derive(Clone, Default, Deserialize)]
pub struct Board {
    pub cards: Vec<Card>,
}

pub struct Reply {
    board: Board,
    link: Option<(String, String)>,
}

#[derive(Default)]
pub struct State {
    pub memory: crate::note_memory::State,
    pub library_open: bool,
    pub preview: bool,
    pub board_open: bool,
    pub profile_open: bool,
    pub board: Board,
    pub job: Option<system::Job<Result<Reply, String>>>,
    pub status: String,
    last_refresh: Option<std::time::Instant>,
}

enum Request {
    Read,
    Create {
        note_id: String,
        title: String,
        content: String,
    },
    Update {
        card_id: String,
        payload: Value,
    },
}

fn read_board(response: ureq::http::Response<ureq::Body>) -> Result<Board, String> {
    let text = response
        .into_body()
        .read_to_string()
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

fn request(port: u16, operation: Request) -> Result<Reply, String> {
    let agent = ureq::Agent::config_builder()
        .proxy(None)
        .timeout_global(Some(std::time::Duration::from_secs(8)))
        .build()
        .new_agent();
    let url = format!("http://127.0.0.1:{port}/api/task-board");
    let mut link = None;
    let board = match operation {
        Request::Read => read_board(agent.get(&url).call().map_err(|error| error.to_string())?)?,
        Request::Create {
            note_id,
            title,
            content,
        } => {
            let before = read_board(agent.get(&url).call().map_err(|error| error.to_string())?)?;
            let payload = json!({"title": title, "note": content});
            let response = agent
                .post(format!("{url}/cards"))
                .header("Content-Type", "application/json")
                .send(payload.to_string())
                .map_err(|error| error.to_string())?;
            let board = read_board(response)?;
            let matches: Vec<_> = board
                .cards
                .iter()
                .filter(|card| {
                    card.title == title
                        && card.note == content
                        && !before.cards.iter().any(|old| old.id == card.id)
                })
                .collect();
            if matches.len() == 1 {
                link = Some((note_id, matches[0].id.clone()));
            } else {
                return Err("Đã gửi yêu cầu tạo thẻ nhưng chưa xác định được liên kết. Kiểm tra Kanban trước khi thử lại.".into());
            }
            board
        }
        Request::Update { card_id, payload } => {
            if card_id.is_empty()
                || !card_id
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
            {
                return Err("Mã thẻ không hợp lệ.".into());
            }
            let response = agent
                .patch(format!("{url}/cards/{card_id}"))
                .header("Content-Type", "application/json")
                .send(payload.to_string())
                .map_err(|error| error.to_string())?;
            read_board(response)?
        }
    };
    Ok(Reply { board, link })
}

impl PentaKuru {
    fn board_request(&mut self, ctx: &egui::Context, operation: Request) {
        if self.workspace.job.is_some() {
            return;
        }
        let Some(child) = self.integrations.child.as_ref() else {
            self.workspace.status = "Mở DeepTutor để kết nối Kanban trên máy này.".into();
            return;
        };
        let port = child.3;
        self.workspace.last_refresh = Some(std::time::Instant::now());
        self.workspace.job = Some(system::Job::spawn(ctx, move || request(port, operation)));
    }

    pub(super) fn poll_workspace(&mut self) {
        if let Some(result) = system::take(&mut self.workspace.job) {
            match result {
                Ok(reply) => {
                    self.workspace.board = reply.board;
                    if let Some((note_id, card_id)) = reply.link {
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
                    self.workspace.status = "Đã cập nhật Kanban từ DeepTutor.".into();
                }
                Err(error) => {
                    self.workspace.status =
                        format!("Chưa đồng bộ được: {error}. Ghi chú vẫn được giữ trên máy.")
                }
            }
        }
    }

    pub(super) fn note_connection_ui(&mut self, ui: &mut egui::Ui, index: usize) {
        let shared = self.notes_mgr.notes[index].ai_shared;
        ui.horizontal_wrapped(|ui| {
            if theme::toolbar_icon(ui, theme::Glyph::AiMemory, if shared { "Tắt ghi nhớ AI cho note" } else { "Bật ghi nhớ AI: tự tóm tắt sau khi ngừng gõ 5 giây, gửi note tới nhà cung cấp AI đã chọn và lưu tóm tắt trên máy" }, true, shared).clicked() {
                self.notes_mgr.notes[index].ai_shared = !shared;
                self.notes_mgr.save_selected_note();
            }
            if theme::toolbar_icon(ui, theme::Glyph::Chat, "Hỏi AI về note", true, false).clicked() {
                if shared {
                    self.ai_prompt = format!("Hãy giúp tôi với ghi chú «{}»: ", self.notes_mgr.notes[index].title);
                    self.ai_open = true;
                    self.ai_mode = crate::AiMode::Chat;
                } else { self.workspace.status = "Bật AI trên note này trước khi dùng làm ngữ cảnh.".into(); }
            }
            let linked = self.notes_mgr.notes[index].task_card_id.clone();
            if theme::toolbar_icon(ui, theme::Glyph::Kanban, if linked.is_some() { "Gửi nội dung note sang thẻ liên kết" } else { "Tạo thẻ Kanban từ note" }, self.workspace.job.is_none(), linked.is_some()).clicked() {
                let note = &self.notes_mgr.notes[index];
                if note.content.chars().count() > 2000 || note.title.chars().count() > 160 {
                    self.workspace.status = "Thẻ DeepTutor giới hạn 160 ký tự tiêu đề và 2.000 ký tự nội dung. Hãy rút gọn hoặc tách note trước khi gửi.".into();
                } else {
                    let operation = if let Some(card_id) = linked {
                        Request::Update { card_id, payload: json!({"title":note.title,"note":note.content}) }
                    } else { Request::Create { note_id: note.id.clone(), title: note.title.clone(), content: note.content.clone() } };
                    self.board_request(ui.ctx(), operation);
                }
            }
        });
        if shared {
            let note = &self.notes_mgr.notes[index];
            if self.workspace.memory.active_id.as_ref() == Some(&note.id) {
                ui.horizontal(|ui| {
                    ui.spinner();
                    theme::caption(ui, "AI đang tóm tắt…");
                });
            } else if let Some(error) = self.workspace.memory.errors.get(&note.id) {
                theme::caption(ui, format!("Chưa ghi nhớ được: {error}"));
            } else if note.summary_is_current() {
                ui.collapsing("Đã ghi nhớ ý chính", |ui| {
                    theme::render_markdown(ui, &note.ai_summary);
                });
            } else {
                theme::caption(
                    ui,
                    if self.prefs.ai.ready() {
                        "Sẽ tự ghi nhớ sau khi ngừng gõ 5 giây…"
                    } else {
                        "Cấu hình API/model AI để tự tóm tắt và ghi nhớ note."
                    },
                );
            }
        }
        if !self.workspace.status.is_empty() {
            theme::caption(ui, &self.workspace.status);
        }
    }

    pub(super) fn kanban_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Kanban · DeepTutor");
        theme::caption(
            ui,
            "Cùng bảng công việc với DeepTutor. Chuyển trạng thái được lưu trực tiếp vào thẻ gốc.",
        );
        if self.integrations.child.is_none() {
            if ui.button("Kết nối / mở DeepTutor").clicked() {
                self.open_tutor(ui.ctx());
            }
            return;
        }
        if self
            .workspace
            .last_refresh
            .is_none_or(|last| last.elapsed().as_secs() >= 5)
            && self.workspace.job.is_none()
        {
            self.board_request(ui.ctx(), Request::Read);
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs(1));
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.workspace.job.is_none(), egui::Button::new("Làm mới"))
                .clicked()
            {
                self.board_request(ui.ctx(), Request::Read);
            }
            if self.workspace.job.is_some() {
                ui.spinner();
            }
        });
        theme::caption(ui, &self.workspace.status);
        let cards = self.workspace.board.cards.clone();
        egui::ScrollArea::both().id_salt("shared-kanban").show(ui, |ui| {
            ui.set_min_width(500.0);
            ui.columns(3, |columns| {
                for (column, (status, label)) in columns.iter_mut().zip([("todo", "Cần làm"), ("doing", "Đang làm"), ("done", "Hoàn tất")]) {
                    column.heading(label);
                    for card in cards.iter().filter(|card| !card.archived && card.status == status) {
                        column.push_id(&card.id, |ui| {
                            egui::Frame::group(ui.style()).show(ui, |ui| {
                                ui.label(egui::RichText::new(&card.title).strong());
                                theme::caption(ui, card.note.chars().take(140).collect::<String>());
                                egui::ComboBox::from_id_salt("status").selected_text(label).show_ui(ui, |ui| {
                                    for (next, title) in [("todo", "Cần làm"), ("doing", "Đang làm"), ("done", "Hoàn tất")] {
                                        if ui.add_enabled(self.workspace.job.is_none(), egui::Button::new(title)).clicked() {
                                            self.board_request(ui.ctx(), Request::Update { card_id: card.id.clone(), payload: json!({"status":next}) });
                                        }
                                    }
                                });
                                if ui.button("Mở note").clicked() {
                                    if let Some(note) = self.notes_mgr.notes.iter().find(|note| note.task_card_id.as_ref() == Some(&card.id)) {
                                        self.notes_mgr.selected_note_id = Some(note.id.clone());
                                    } else {
                                        let id = self.notes_mgr.create_note(&card.title, &card_content(card));
                                        if let Some(note) = self.notes_mgr.notes.iter_mut().find(|note| note.id == id) { note.task_card_id = Some(card.id.clone()); }
                                        self.notes_mgr.save_selected_note();
                                    }
                                    self.workspace.board_open = false;
                                }
                                if let Some(position) = self.notes_mgr.notes.iter().position(|note| note.task_card_id.as_ref() == Some(&card.id)) {
                                    ui.collapsing("Lấy nội dung thẻ", |ui| {
                                        theme::caption(ui, "Thay nội dung note liên kết bằng nội dung thẻ hiện tại.");
                                        if ui.button("Xác nhận thay note").clicked() {
                                            self.notes_mgr.selected_note_id = Some(self.notes_mgr.notes[position].id.clone());
                                            self.notes_mgr.notes[position].content = card_content(card);
                                            self.notes_mgr.save_selected_note();
                                            self.workspace.board_open = false;
                                        }
                                    });
                                }
                            });
                        });
                    }
                }
            });
        });
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
    use std::io::{Read, Write};

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
    fn importing_card_preserves_title_and_is_stable_on_round_trip() {
        let mut card = Card {
            id: "task-1".into(),
            title: "Tiêu đề mới".into(),
            note: "# Tiêu đề cũ\n\nNội dung".into(),
            status: "todo".into(),
            archived: false,
        };
        assert_eq!(card_content(&card), "# Tiêu đề mới\n\nNội dung");
        card.note = card_content(&card);
        assert_eq!(card_content(&card), card.note);
        card.note.clear();
        assert_eq!(card_content(&card), "# Tiêu đề mới\n\n");
    }

    #[test]
    fn board_request_preserves_remote_identity_and_moves_original_card() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            for expected in [
                "GET /api/task-board ",
                "PATCH /api/task-board/cards/card-1 ",
            ] {
                let (mut connection, _) = listener.accept().unwrap();
                connection
                    .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                    .unwrap();
                let mut bytes = Vec::new();
                loop {
                    let mut buffer = [0; 4096];
                    let count = connection.read(&mut buffer).unwrap();
                    assert!(count > 0);
                    bytes.extend_from_slice(&buffer[..count]);
                    let received = String::from_utf8_lossy(&bytes);
                    if let Some(header_end) = received.find("\r\n\r\n") {
                        let length = received[..header_end]
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().unwrap())
                            })
                            .unwrap_or(0);
                        if bytes.len() >= header_end + 4 + length {
                            break;
                        }
                    }
                }
                let received = String::from_utf8_lossy(&bytes);
                assert!(received.starts_with(expected), "{received}");
                if expected.starts_with("PATCH") {
                    let payload: Value =
                        serde_json::from_str(received.split_once("\r\n\r\n").unwrap().1).unwrap();
                    assert_eq!(payload["status"], "doing");
                }
                let body = r#"{"cards":[{"id":"card-1","title":"Task","note":"Content","status":"doing","archived":false}]}"#;
                write!(connection, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            }
        });
        let reply = request(port, Request::Read).unwrap();
        assert_eq!(reply.board.cards[0].id, "card-1");
        let reply = request(
            port,
            Request::Update {
                card_id: "card-1".into(),
                payload: json!({"status":"doing"}),
            },
        )
        .unwrap();
        assert_eq!(reply.board.cards[0].status, "doing");
        server.join().unwrap();
    }
}
