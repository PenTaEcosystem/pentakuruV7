//! Mini HTTP Server tích hợp trong PentaKuru phục vụ đồng hồ thông minh (Amazfit Bip / Zepp OS)
//! và kết nối qua mạng an toàn Tailscale.
//!
//! Tự động lắng nghe trên 0.0.0.0:3899 khi PentaKuru khởi động.
//! Có thể truy cập qua:
//! - Localhost: http://127.0.0.1:3899
//! - IP Tailscale: http://100.115.158.11:3899 (từ điện thoại bất kỳ đâu qua Tailscale)

use crate::ai::{self, AiSettings, Message, Role};
use crate::notes::NotesManager;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

pub const DEFAULT_PORT: u16 = 3899;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ServerNotification {
    pub id: String,
    pub title: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Clone)]
pub struct ServerState {
    pub ai_settings: AiSettings,
    pub notes_dir: PathBuf,
    pub tailscale_ip: String,
    pub port: u16,
    pub notifications: Vec<ServerNotification>,
    pub control: crate::watch_control::Control,
}

pub struct ServerHandle {
    pub state: Arc<RwLock<ServerState>>,
    pub port: u16,
    pub tailscale_ip: String,
    pub listening: Arc<RwLock<Result<(), String>>>,
}

/// Phát hiện IP Tailscale của máy tính
pub fn detect_tailscale_ip() -> String {
    // 1. Thử gọi lệnh tailscale CLI
    let candidate_paths = ["tailscale", r"C:\Program Files\Tailscale\tailscale.exe"];

    for bin in candidate_paths {
        if let Ok(output) = std::process::Command::new(bin).args(["ip", "-4"]).output()
            && output.status.success()
        {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if let Some(first_ip) = s.lines().next() {
                let trimmed = first_ip.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }

    // 2. Dự phòng: IP Tailscale hiện tại của máy người dùng
    "100.115.158.11".to_string()
}

/// Khởi động Mini Server trên background thread
pub fn start_server(ai_settings: AiSettings, notes_dir: PathBuf) -> ServerHandle {
    let port = DEFAULT_PORT;
    let tailscale_ip = detect_tailscale_ip();

    let state = Arc::new(RwLock::new(ServerState {
        ai_settings,
        notes_dir,
        tailscale_ip: tailscale_ip.clone(),
        port,
        notifications: vec![ServerNotification {
            id: "welcome".into(),
            title: "PentaKuru Sẵn Sàng".into(),
            content: "Đã kết nối thành công với PentaKuru trên máy tính!".into(),
            timestamp: crate::system::format_date(std::time::SystemTime::now()),
        }],
        control: crate::watch_control::Control::default(),
    }));

    let state_clone = Arc::clone(&state);
    let ip_for_log = tailscale_ip.clone();
    let listening = Arc::new(RwLock::new(Err("Đang khởi động máy chủ".to_string())));
    let listening_thread = Arc::clone(&listening);

    thread::spawn(move || {
        let bind_addr = format!("0.0.0.0:{port}");
        let listener = match TcpListener::bind(&bind_addr) {
            Ok(l) => {
                *listening_thread.write().unwrap() = Ok(());
                println!(
                    "[PentaKuru Server] Đang lắng nghe trên {bind_addr} (Tailscale: http://{ip_for_log}:{port})"
                );
                l
            }
            Err(e) => {
                *listening_thread.write().unwrap() = Err(format!(
                    "Không mở được cổng {port}: {e}. Đóng bản Penta cũ đang giữ cổng rồi mở lại."
                ));
                eprintln!("[PentaKuru Server] Không thể bind {bind_addr}: {e}");
                return;
            }
        };

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let st = Arc::clone(&state_clone);
                    thread::spawn(move || {
                        handle_client(stream, st);
                    });
                }
                Err(e) => {
                    eprintln!("[PentaKuru Server] Lỗi kết nối incoming: {e}");
                }
            }
        }
    });

    ServerHandle {
        state,
        port,
        tailscale_ip,
        listening,
    }
}

fn handle_client(mut stream: TcpStream, state: Arc<RwLock<ServerState>>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(15)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(15)));

    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() || request_line.is_empty() {
        return;
    }

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let method = parts[0];
    let path = parts[1];

    // Đọc headers
    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
            break;
        }
        let lower = line.to_lowercase();
        if lower.starts_with("content-length:")
            && let Some(val) = lower.split(':').nth(1)
        {
            content_length = val.trim().parse().unwrap_or(0);
        }
    }

    // Bound the watch upload before allocating memory.
    if content_length > 256 * 1024 {
        send_response(
            &mut stream,
            413,
            "Payload Too Large",
            "application/json",
            br#"{"status":"error","error":"Audio upload too large"}"#,
        );
        return;
    }
    // Đọc body nếu có
    let mut body = vec![0u8; content_length];
    if content_length > 0 && reader.read_exact(&mut body).is_err() {
        send_response(
            &mut stream,
            400,
            "Bad Request",
            "application/json",
            br#"{"status":"error","error":"Incomplete request body"}"#,
        );
        return;
    }
    let body_str = String::from_utf8_lossy(&body).to_string();

    // CORS pre-flight
    if method == "OPTIONS" {
        send_response(&mut stream, 200, "OK", "application/json", b"{}");
        return;
    }

    // Định tuyến API
    let (status_code, body_res) = route_request(method, path, &body_str, &state);
    send_response(
        &mut stream,
        status_code,
        if status_code == 200 { "OK" } else { "Error" },
        "application/json",
        body_res.as_bytes(),
    );
}

fn route_request(
    method: &str,
    path: &str,
    body: &str,
    state: &Arc<RwLock<ServerState>>,
) -> (u16, String) {
    let base_path = path.split('?').next().unwrap_or(path);

    match (method, base_path) {
        ("POST", "/api/windows/execute" | "/api/windows/status" | "/api/windows/cancel") => {
            let value: serde_json::Value = match serde_json::from_str(body) {
                Ok(value) => value,
                Err(_) => {
                    return (
                        400,
                        json!({"status":"error","error":"Dữ liệu lệnh Windows không hợp lệ"})
                            .to_string(),
                    );
                }
            };
            let mut st = state.write().unwrap();
            if !st
                .control
                .authorize(value["control_key"].as_str().unwrap_or(""))
            {
                return (403, json!({"status":"error","error":"Nhập đúng mã ghép Windows từ cài đặt Penta vào Zepp"}).to_string());
            }
            let task = match base_path {
                "/api/windows/execute" => {
                    match st.control.enqueue(value["command"].as_str().unwrap_or("")) {
                        Ok(task) => Some(task),
                        Err(error) => {
                            return (409, json!({"status":"error","error":error}).to_string());
                        }
                    }
                }
                "/api/windows/cancel" => st.control.cancel(value["id"].as_str().unwrap_or("")),
                _ => st.control.get(value["id"].as_str().unwrap_or("")),
            };
            match task {
                Some(task) => (200, json!({"status":"ok","task":task}).to_string()),
                None => (
                    404,
                    json!({"status":"error","error":"Không tìm thấy tác vụ Windows"}).to_string(),
                ),
            }
        }
        // 1. Kiểm tra trạng thái server
        ("GET", "/" | "/api/status") => {
            let st = state.read().unwrap();
            let notes_count = {
                let mgr = NotesManager::at(st.notes_dir.clone());
                mgr.notes.len()
            };
            let resp = json!({
                "status": "ok",
                "app": "PentaKuru",
                "version": "0.1.0",
                "watch_protocol": 2,
                "windows_control": true,
                "windows_ready": st.control.ready,
                "voice_supported": st.ai_settings.resolved() == Some(ai::Provider::Gemini),
                "voice_configured": st.ai_settings.resolved() == Some(ai::Provider::Gemini) && !st.ai_settings.api_key.trim().is_empty(),
                "tailscale_ip": st.tailscale_ip,
                "port": st.port,
                "ai_provider": st.ai_settings.summary(),
                "notes_count": notes_count,
                "time": crate::system::format_date(std::time::SystemTime::now())
            });
            (200, resp.to_string())
        }

        // 2. Lấy danh sách ghi chú cho đồng hồ
        ("GET", "/api/notes") => {
            let st = state.read().unwrap();
            let mgr = NotesManager::at(st.notes_dir.clone());
            let notes_data: Vec<_> = mgr
                .notes
                .iter()
                .take(15) // Lấy tối đa 15 ghi chú mới nhất cho đồng hồ
                .map(|n| {
                    json!({
                        "id": n.id,
                        "title": n.title,
                        "folder": n.folder,
                        "content": n.content,
                        "updated_at": n.updated_at
                    })
                })
                .collect();

            let resp = json!({
                "status": "ok",
                "notes": notes_data
            });
            (200, resp.to_string())
        }

        ("POST", "/api/search") => {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
                return (
                    400,
                    json!({"status":"error", "error":"JSON không hợp lệ"}).to_string(),
                );
            };
            let query = value["query"].as_str().unwrap_or("").trim();
            if query.is_empty() {
                return (
                    400,
                    json!({"status":"error", "error":"Nội dung tìm kiếm trống"}).to_string(),
                );
            }
            let st = state.read().unwrap();
            (
                200,
                json!({"status":"ok", "notes": search_notes(&st.notes_dir, query)}).to_string(),
            )
        }

        // 3. Tạo ghi chú mới từ đồng hồ
        ("POST", "/api/notes") => {
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(body);
            match parsed {
                Ok(val) => {
                    let title = val["title"].as_str().unwrap_or("Ghi chú từ Bip");
                    let content = val["content"].as_str().unwrap_or("");
                    let mut st = state.write().unwrap();
                    let mut mgr = NotesManager::at(st.notes_dir.clone());
                    let note_id = mgr.create_note(title, content);

                    // Thêm thông báo
                    st.notifications.push(ServerNotification {
                        id: note_id.clone(),
                        title: format!("Đã tạo ghi chú: {title}"),
                        content: content.to_string(),
                        timestamp: crate::system::format_date(std::time::SystemTime::now()),
                    });

                    let resp = json!({
                        "status": "ok",
                        "id": note_id,
                        "message": "Đã lưu ghi chú thành công vào PentaKuru"
                    });
                    (200, resp.to_string())
                }
                Err(e) => (
                    400,
                    json!({ "error": format!("JSON lỗi: {e}") }).to_string(),
                ),
            }
        }

        // 4. Hỏi AI trực tiếp từ đồng hồ
        ("POST", "/api/ai/ask") => {
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(body);
            match parsed {
                Ok(val) => {
                    let prompt = val["prompt"].as_str().unwrap_or("").trim();
                    if prompt.is_empty() {
                        return (400, json!({ "error": "Prompt trống" }).to_string());
                    }

                    let ai_settings = {
                        let st = state.read().unwrap();
                        st.ai_settings.clone()
                    };

                    let system_prompt = "Bạn là trợ lý AI trên đồng hồ thông minh Amazfit của PentaKuru. Hãy trả lời ngắn gọn, cô đọng, súc tích (dưới 100 từ), thân thiện, định dạng rõ ràng để dễ đọc trên màn hình nhỏ đồng hồ.";
                    let messages = vec![Message {
                        role: Role::User,
                        text: prompt.to_string(),
                        images: vec![],
                    }];

                    match ai::chat(&ai_settings, system_prompt, &messages) {
                        Ok(reply) => {
                            let resp = json!({
                                "status": "ok",
                                "prompt": prompt,
                                "reply": reply.trim(),
                                "model": ai_settings.model
                            });
                            (200, resp.to_string())
                        }
                        Err(e) => {
                            let resp = json!({
                                "status": "error",
                                "error": format!("Lỗi AI: {e}")
                            });
                            (500, resp.to_string())
                        }
                    }
                }
                Err(e) => (
                    400,
                    json!({ "error": format!("JSON lỗi: {e}") }).to_string(),
                ),
            }
        }

        // 5. STT (Speech-to-Text) giọng nói từ đồng hồ
        ("POST", "/api/voice/stt") => {
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(body);
            match parsed {
                Ok(val) => {
                    let audio_base64 = val["audio_base64"].as_str().unwrap_or("").trim();
                    let mime_type = val["mime_type"].as_str().unwrap_or("audio/opus");
                    let action = val["action"].as_str().unwrap_or("ask"); // "transcribe", "ask", "note"
                    if !matches!(
                        action,
                        "transcribe" | "ask" | "note" | "search" | "chat" | "execute"
                    ) {
                        return (
                            400,
                            json!({"status":"error", "error":"Tác vụ giọng nói không được hỗ trợ"})
                                .to_string(),
                        );
                    }

                    if action == "execute" {
                        let st = state.read().unwrap();
                        if !st
                            .control
                            .authorize(val["control_key"].as_str().unwrap_or(""))
                        {
                            return (403, json!({"status":"error","error":"Nhập mã ghép Windows trong cài đặt PentaKuru AI trên Zepp"}).to_string());
                        }
                        if !st.control.ready || st.control.busy {
                            return (409, json!({"status":"error","error":"Penta chưa sẵn sàng hoặc đang xử lý tác vụ khác"}).to_string());
                        }
                    }
                    println!(
                        "[PentaKuru Server] 🎙️ Nhận âm thanh từ đồng hồ: action='{}', mime='{}', base64_len={}",
                        action,
                        mime_type,
                        audio_base64.len()
                    );

                    if audio_base64.is_empty() {
                        eprintln!("[PentaKuru Server] ⚠️ Dữ liệu âm thanh từ đồng hồ bị rỗng!");
                        return (
                            400,
                            json!({ "error": "Dữ liệu âm thanh trống" }).to_string(),
                        );
                    }

                    let (ai_settings, notes_dir) = {
                        let st = state.read().unwrap();
                        (st.ai_settings.clone(), st.notes_dir.clone())
                    };

                    match ai::stt(&ai_settings, audio_base64, mime_type, action) {
                        Ok((text, reply)) => {
                            if action == "execute" {
                                return match state.write().unwrap().control.enqueue(&text) {
                                    Ok(task) => (
                                        200,
                                        json!({"status":"ok", "text":text, "task":task})
                                            .to_string(),
                                    ),
                                    Err(error) => {
                                        (409, json!({"status":"error", "error":error}).to_string())
                                    }
                                };
                            }
                            // Do not print the user's spoken content into logs.
                            if action == "search" {
                                return (200, json!({"status":"ok", "text":text, "notes":search_notes(&notes_dir, &text)}).to_string());
                            }

                            let now_str = crate::system::format_date(std::time::SystemTime::now());
                            let note_title = if text.chars().count() > 30 {
                                format!("{}...", text.chars().take(30).collect::<String>())
                            } else if text.is_empty() {
                                "Ghi chú từ đồng hồ".to_string()
                            } else {
                                text.clone()
                            };

                            let mut st = state.write().unwrap();
                            if action == "note" && !text.is_empty() {
                                let mut mgr = NotesManager::at(notes_dir);
                                let note_id = mgr.create_note(&note_title, &text);
                                st.notifications.push(ServerNotification {
                                    id: note_id,
                                    title: format!("Đã lưu ghi chú: {note_title}"),
                                    content: text.clone(),
                                    timestamp: now_str.clone(),
                                });
                            } else {
                                st.notifications.push(ServerNotification {
                                    id: format!(
                                        "voice_{}",
                                        std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .map(|d| d.as_secs())
                                            .unwrap_or(0)
                                    ),
                                    title: format!("Đồng hồ: {note_title}"),
                                    content: if reply.is_empty() {
                                        text.clone()
                                    } else {
                                        format!("Bạn nói: {}\nTrả lời: {}", text, reply)
                                    },
                                    timestamp: now_str,
                                });
                            }

                            let resp = json!({
                                "status": "ok",
                                "text": text,
                                "reply": reply
                            });
                            (200, resp.to_string())
                        }
                        Err(e) => {
                            eprintln!("[PentaKuru Server] ❌ Lỗi xử lý STT: {e}");
                            let resp = json!({
                                "status": "error",
                                "error": format!("Lỗi STT: {e}")
                            });
                            (500, resp.to_string())
                        }
                    }
                }
                Err(e) => (
                    400,
                    json!({ "error": format!("JSON lỗi: {e}") }).to_string(),
                ),
            }
        }

        // 6. Lấy danh sách thông báo
        ("GET", "/api/notifications") => {
            let st = state.read().unwrap();
            let resp = json!({
                "status": "ok",
                "notifications": st.notifications
            });
            (200, resp.to_string())
        }

        // 6. Gửi thông báo mới lên PC
        ("POST", "/api/notify") => {
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(body);
            match parsed {
                Ok(val) => {
                    let title = val["title"].as_str().unwrap_or("Thông báo đồng hồ");
                    let content = val["content"].as_str().unwrap_or("");
                    let mut st = state.write().unwrap();
                    let notif = ServerNotification {
                        id: format!(
                            "notif_{}",
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs())
                                .unwrap_or(0)
                        ),
                        title: title.to_string(),
                        content: content.to_string(),
                        timestamp: crate::system::format_date(std::time::SystemTime::now()),
                    };
                    st.notifications.push(notif);
                    (
                        200,
                        json!({ "status": "ok", "message": "Đã nhận thông báo" }).to_string(),
                    )
                }
                Err(e) => (
                    400,
                    json!({ "error": format!("JSON lỗi: {e}") }).to_string(),
                ),
            }
        }

        _ => (
            404,
            json!({ "error": "Endpoint không tồn tại" }).to_string(),
        ),
    }
}

fn search_notes(notes_dir: &std::path::Path, query: &str) -> Vec<serde_json::Value> {
    let query = query.to_lowercase();
    NotesManager::at(notes_dir.to_path_buf()).notes.iter()
        .filter(|note| note.title.to_lowercase().contains(&query) || note.content.to_lowercase().contains(&query))
        .take(15).map(|note| json!({"id":note.id, "title":note.title, "content":note.content, "folder":note.folder})).collect()
}

fn send_response(
    stream: &mut TcpStream,
    status_code: u16,
    status_text: &str,
    content_type: &str,
    body: &[u8],
) {
    let header = format!(
        "HTTP/1.1 {status_code} {status_text}\r\n\
         Content-Type: {content_type}; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         Access-Control-Allow-Headers: *\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );

    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        state: Arc<RwLock<ServerState>>,
        root: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "pentakuru-watch-test-{}-{unique}",
                std::process::id()
            ));
            let state = Arc::new(RwLock::new(ServerState {
                ai_settings: AiSettings::default(),
                notes_dir: root.clone(),
                tailscale_ip: "127.0.0.1".into(),
                port: 0,
                notifications: vec![],
                control: crate::watch_control::Control::default(),
            }));
            Self { state, root }
        }
        fn request(&self, method: &str, path: &str, body: &str) -> (u16, serde_json::Value) {
            let (status, text) = route_request(method, path, body, &self.state);
            (status, serde_json::from_str(&text).unwrap())
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn windows_commands_require_pairing_and_report_progress_and_cancellation() {
        let fixture = Fixture::new();
        {
            let mut state = fixture.state.write().unwrap();
            state.control.key = "private-watch-key".into();
            state.control.ready = true;
        }
        for path in [
            "/api/windows/execute",
            "/api/windows/status",
            "/api/windows/cancel",
        ] {
            assert_eq!(
                fixture
                    .request("POST", path, r#"{"command":"Mở Notepad"}"#)
                    .0,
                403
            );
        }
        assert_eq!(
            fixture
                .request(
                    "POST",
                    "/api/voice/stt",
                    r#"{"action":"execute","audio_base64":"AAAA"}"#
                )
                .0,
            403
        );
        let (_, status) = fixture.request("GET", "/api/status", "");
        assert_eq!(status["windows_ready"], true);
        assert!(!status.to_string().contains("private-watch-key"));
        let (code, response) = fixture.request(
            "POST",
            "/api/windows/execute",
            r#"{"command":"Mở Notepad","control_key":"private-watch-key"}"#,
        );
        assert_eq!(code, 200);
        assert_eq!(response["task"]["phase"], "pending");
        let id = response["task"]["id"].as_str().unwrap();
        let request = json!({"id": id, "control_key":"private-watch-key"}).to_string();
        assert_eq!(
            fixture
                .request(
                    "POST",
                    "/api/windows/execute",
                    r#"{"command":"Lệnh thứ hai","control_key":"private-watch-key"}"#
                )
                .0,
            409
        );
        {
            let mut state = fixture.state.write().unwrap();
            assert_eq!(state.control.take_pending().unwrap().id, id);
            state
                .control
                .update(id, "running", "Bước 2: đang mở ứng dụng");
        }
        let (_, progress) = fixture.request("POST", "/api/windows/status", &request);
        assert_eq!(progress["task"]["message"], "Bước 2: đang mở ứng dụng");
        fixture.request("POST", "/api/windows/cancel", &request);
        assert!(
            fixture
                .state
                .read()
                .unwrap()
                .control
                .get(id)
                .unwrap()
                .cancel_requested
        );
        fixture
            .state
            .write()
            .unwrap()
            .control
            .update(id, "cancelled", "Đã dừng");
        let (_, stopped) = fixture.request("POST", "/api/windows/status", &request);
        assert_eq!(stopped["task"]["phase"], "cancelled");
        assert!(!stopped.to_string().contains("private-watch-key"));
    }

    #[test]
    fn windows_commands_reject_unconfigured_pc_and_unknown_task() {
        let fixture = Fixture::new();
        fixture.state.write().unwrap().control.key = "test-key".into();
        assert_eq!(
            fixture
                .request(
                    "POST",
                    "/api/windows/execute",
                    r#"{"command":"Mở Notepad","control_key":"test-key"}"#
                )
                .0,
            409
        );
        assert_eq!(
            fixture
                .request(
                    "POST",
                    "/api/windows/status",
                    r#"{"id":"missing","control_key":"test-key"}"#
                )
                .0,
            404
        );
        assert_eq!(
            fixture
                .request(
                    "POST",
                    "/api/voice/stt",
                    r#"{"action":"execute","audio_base64":"AAAA","control_key":"test-key"}"#
                )
                .0,
            409
        );
    }

    #[test]
    fn watch_status_reports_voice_configuration_without_exposing_key() {
        let fixture = Fixture::new();
        let (status, data) = fixture.request("GET", "/api/status", "");
        assert_eq!(status, 200);
        assert_eq!(data["watch_protocol"], 2);
        assert_eq!(data["voice_configured"], false);
        {
            let mut state = fixture.state.write().unwrap();
            state.ai_settings.provider = ai::Provider::Gemini;
            state.ai_settings.api_key = "test-secret-only-no-network".into();
        }
        let (_, data) = fixture.request("GET", "/api/status", "");
        assert_eq!(data["voice_configured"], true);
        assert!(!data.to_string().contains("test-secret-only-no-network"));
    }

    #[test]
    fn watch_search_matches_title_and_content_across_folders() {
        let fixture = Fixture::new();
        let mut notes = NotesManager::at(fixture.root.clone());
        notes.create_note("Lịch họp", "Dự án đồng hồ");
        notes.selected_folder = "Dự án & Lập trình".into();
        notes.create_note("Thiết bị", "Kiểm tra đồng hồ BLE");
        notes.create_note("Không liên quan", "Công thức nấu ăn");
        let (status, data) = fixture.request("POST", "/api/search", r#"{"query":"ĐỒNG HỒ"}"#);
        assert_eq!(status, 200);
        assert_eq!(data["notes"].as_array().unwrap().len(), 2);
        assert_eq!(
            fixture.request("POST", "/api/search", r#"{"query":" "}"#).0,
            400
        );
    }

    #[test]
    fn invalid_voice_requests_fail_before_network_or_note_creation() {
        let fixture = Fixture::new();
        assert_eq!(
            fixture
                .request(
                    "POST",
                    "/api/voice/stt",
                    r#"{"action":"delete","audio_base64":"AAAA"}"#
                )
                .0,
            400
        );
        assert_eq!(
            fixture
                .request("POST", "/api/voice/stt", r#"{"action":"ask"}"#)
                .0,
            400
        );
        let (_, data) = fixture.request(
            "POST",
            "/api/voice/stt",
            r#"{"action":"note","audio_base64":"AAAA"}"#,
        );
        assert_eq!(data["status"], "error");
        assert!(data["error"].as_str().unwrap().contains("API Key"));
        let settings = AiSettings {
            provider: ai::Provider::OpenAi,
            api_key: "test-only".into(),
            ..Default::default()
        };
        assert!(
            ai::stt(&settings, "AAAA", "audio/opus", "ask")
                .unwrap_err()
                .contains("Gemini")
        );
    }

    #[test]
    fn malformed_http_audio_body_is_rejected() {
        let fixture = Fixture::new();
        for (length, expected) in [(262145, "413"), (100, "400")] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let state = Arc::clone(&fixture.state);
            let worker = thread::spawn(move || handle_client(listener.accept().unwrap().0, state));
            let mut socket = TcpStream::connect(address).unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            write!(
                socket,
                "POST /api/voice/stt HTTP/1.1\r\nContent-Length: {length}\r\n\r\n{{}}"
            )
            .unwrap();
            socket.shutdown(std::net::Shutdown::Write).unwrap();
            let mut response = String::new();
            socket.read_to_string(&mut response).unwrap();
            assert!(
                response.starts_with(&format!("HTTP/1.1 {expected}")),
                "{response}"
            );
            worker.join().unwrap();
        }
    }
}
