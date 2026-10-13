use super::timeline::TimelineEntry;
use std::fs;
use std::path::{Path, PathBuf};

pub fn save_meeting_to_notes(
    notes_dir: &Path,
    entries: &[TimelineEntry],
    target_lang: &str,
) -> Result<PathBuf, String> {
    if entries.is_empty() {
        return Err("Chưa có nội dung cuộc họp để lưu".into());
    }

    let meetings_dir = notes_dir.join("Cuoc hop Sally");
    if !meetings_dir.exists() {
        fs::create_dir_all(&meetings_dir)
            .map_err(|e| format!("Không thể tạo thư mục cuộc họp: {e}"))?;
    }

    let timestamp_file = {
        #[cfg(target_os = "windows")]
        unsafe {
            let mut st = std::mem::zeroed();
            windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut st);
            format!(
                "{:04}-{:02}-{:02}_{:02}-{:02}-{:02}",
                st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond
            )
        }
        #[cfg(not(target_os = "windows"))]
        {
            let d = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            format!("meeting_{}", d)
        }
    };

    let filename = format!("Cuoc_hop_{}.md", timestamp_file);
    let file_path = meetings_dir.join(&filename);

    let mut content = String::new();
    content.push_str(&format!(
        "# Biên bản Cuộc họp Sally - {}\n\n",
        timestamp_file
    ));
    content.push_str(&format!("- **Ngôn ngữ đích:** {}\n", target_lang));
    content.push_str(&format!(
        "- **Tổng số đoạn ghi nhận:** {}\n\n",
        entries.len()
    ));
    content.push_str("---\n\n## Nội dung chi tiết (Song ngữ)\n\n");

    for entry in entries {
        content.push_str(&format!("### [{}] {}\n", entry.timestamp, entry.speaker));
        if !entry.original.is_empty() {
            content.push_str(&format!("- **Gốc:** {}\n", entry.original));
        }
        if !entry.translated.is_empty() {
            content.push_str(&format!("- **Dịch:** {}\n", entry.translated));
        }
        content.push('\n');
    }

    fs::write(&file_path, content).map_err(|e| format!("Không thể lưu file markdown: {e}"))?;

    Ok(file_path)
}
