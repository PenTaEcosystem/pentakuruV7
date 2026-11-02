//! Explicit local media commands; never inferred from note or attachment text.
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "action", content = "query", rename_all = "snake_case")]
pub enum Command {
    Play,
    Pause,
    Search(String),
    PlaySearch(String),
}

pub fn parse(prompt: &str) -> Option<Command> {
    let normalized = prompt
        .trim()
        .trim_end_matches(['.', '!', '?'])
        .to_lowercase();
    let text = normalized
        .strip_prefix("anh ")
        .or_else(|| normalized.strip_prefix("tôi "))
        .or_else(|| normalized.strip_prefix("em "))
        .unwrap_or(&normalized);
    match text {
        "muốn nghe nhạc"
        | "nghe nhạc"
        | "mở nhạc"
        | "bật nhạc"
        | "phát nhạc"
        | "mở music"
        | "@music"
        | "/music"
        | "tiếp tục nhạc" => return Some(Command::Play),
        "dừng nhạc" | "tạm dừng nhạc" => return Some(Command::Pause),
        _ => {}
    }
    for prefix in [
        "tìm bài hát ",
        "tìm bài ",
        "tìm nhạc ",
        "/music ",
        "@music ",
    ] {
        if let Some(query) = text.strip_prefix(prefix).filter(|q| !q.trim().is_empty()) {
            return Some(Command::Search(query.trim().to_owned()));
        }
    }
    for prefix in ["muốn nghe bài ", "nghe bài ", "phát bài ", "mở bài "] {
        if let Some(query) = text.strip_prefix(prefix).filter(|q| !q.trim().is_empty()) {
            return Some(Command::PlaySearch(query.trim().to_owned()));
        }
    }
    None
}

pub fn execute(
    command: Command,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<String, String> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(35);
    let request_id = format!("{:?}", std::time::SystemTime::now());
    let mut payload = serde_json::to_value(&command).map_err(|e| e.to_string())?;
    payload["request_id"] = serde_json::json!(request_id);
    let request_json = serde_json::to_string(&request_id).map_err(|e| e.to_string())?;
    let mut started = false;
    while std::time::Instant::now() < deadline {
        if cancel.load(std::sync::atomic::Ordering::SeqCst) {
            let _ = crate::sally::launcher::probe_web(
                "penta-music",
                format!(
                    "(()=>{{const s=window.pentakuruMusicStatus;if(s?.request_id==={request_json})s.cancelled=true;}})()"
                ),
            );
            return Err("Đã dừng lệnh Music.".into());
        }
        let script = if started {
            "window.pentakuruMusicStatus || null".to_owned()
        } else {
            format!(
                "typeof window.pentakuruMusicCommand === 'function' ? window.pentakuruMusicCommand({payload}) : null"
            )
        };
        if let Ok(result) = crate::sally::launcher::probe_web("penta-music", script) {
            if result["started"] == true {
                started = true;
            }
            if result["done"] == true {
                let message = result["message"]
                    .as_str()
                    .unwrap_or("Penta Music chưa trả về trạng thái.")
                    .to_owned();
                return if result["ok"] == true {
                    Ok(message)
                } else {
                    Err(message)
                };
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    let _ = crate::sally::launcher::probe_web(
        "penta-music",
        format!(
            "(()=>{{const s=window.pentakuruMusicStatus;if(s?.request_id==={request_json})s.cancelled=true;}})()"
        ),
    );
    Err("Penta Music chưa phản hồi. Kiểm tra kết nối và khung nhạc trước khi thử lại.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_vietnamese_commands_only() {
        assert_eq!(parse("Anh muốn nghe nhạc!"), Some(Command::Play));
        assert_eq!(
            parse("tìm bài hát Nàng Thơ"),
            Some(Command::Search("nàng thơ".into()))
        );
        assert_eq!(
            parse("anh muốn nghe bài Nàng Thơ"),
            Some(Command::PlaySearch("nàng thơ".into()))
        );
        assert_eq!(parse("tạm dừng nhạc"), Some(Command::Pause));
        for text in [
            "viết ghi chú về mở nhạc",
            "tìm bài ",
            "vì sao anh muốn nghe nhạc",
        ] {
            assert_eq!(parse(text), None);
        }
    }
}
