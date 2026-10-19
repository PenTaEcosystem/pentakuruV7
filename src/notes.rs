//! Quản lý Ghi chú:
//! - Tự động tạo thư mục `Penteye Notes` cùng cấp với file exe (hoặc ~/Documents).
//! - Phân chia theo Thư mục (Categories / Folders: Chung, Giáo án, Dự án,...).
//! - Mỗi ghi chú lưu file Markdown, có ngày giờ tạo & sửa đổi (`dd/mm/yyyy HH:MM`).
//! - Hỗ trợ Upload file: chọn file qua native picker hoặc kéo thả Drag & Drop.
//! - File upload được lưu vào thư mục `attachments` của ghi chú, có thể mở trực tiếp.

use crate::system::{data_dir, format_date};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Clone, Serialize, Deserialize)]
pub struct AttachmentInfo {
    pub name: String,
    pub relative_path: String,
    pub size_bytes: u64,
    pub modified: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct NoteItem {
    pub id: String,
    pub folder: String,
    pub title: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
    pub attachments: Vec<AttachmentInfo>,
    #[serde(default = "default_note_icon")]
    pub icon: String,
    #[serde(default)]
    pub ai_shared: bool,
    #[serde(default)]
    pub task_card_id: Option<String>,
    #[serde(default)]
    pub ai_summary: String,
    #[serde(default)]
    pub summary_source: String,
}

fn default_note_icon() -> String {
    "file".into()
}

impl NoteItem {
    pub fn content_fingerprint(&self) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.content.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    pub fn summary_is_current(&self) -> bool {
        !self.ai_summary.trim().is_empty() && self.summary_source == self.content_fingerprint()
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct NoteMeta {
    #[serde(default = "default_note_icon")]
    pub icon: String,
    #[serde(default)]
    pub ai_shared: bool,
    #[serde(default)]
    pub task_card_id: Option<String>,
    #[serde(default)]
    pub ai_summary: String,
    #[serde(default)]
    pub summary_source: String,
}

impl From<&NoteItem> for NoteMeta {
    fn from(note: &NoteItem) -> Self {
        Self {
            icon: note.icon.clone(),
            ai_shared: note.ai_shared,
            task_card_id: note.task_card_id.clone(),
            ai_summary: note.ai_summary.clone(),
            summary_source: note.summary_source.clone(),
        }
    }
}

pub struct NotesManager {
    pub root_dir: PathBuf,
    pub folders: Vec<String>,
    pub selected_folder: String,
    pub notes: Vec<NoteItem>,
    pub selected_note_id: Option<String>,
    pub search_query: String,
    pub status_message: Option<String>,
}

impl Default for NotesManager {
    fn default() -> Self {
        Self::at(data_dir())
    }
}

impl NotesManager {
    pub fn at(root: PathBuf) -> Self {
        let mut mgr = Self {
            root_dir: root,
            folders: vec![
                "Chung".into(),
                "Giáo án & Bài giảng".into(),
                "Dự án & Lập trình".into(),
                "Ý tưởng & Công việc".into(),
            ],
            selected_folder: "Chung".into(),
            notes: Vec::new(),
            selected_note_id: None,
            search_query: String::new(),
            status_message: None,
        };
        mgr.init_filesystem();
        mgr.reload();
        mgr
    }
}

impl NotesManager {
    pub fn store_summary(&mut self, id: &str, source: &str, summary: &str) -> Result<bool, String> {
        let Some(note) = self.notes.iter_mut().find(|note| note.id == id) else {
            return Ok(false);
        };
        if !note.ai_shared || note.content_fingerprint() != source {
            return Ok(false);
        }
        let mut meta = NoteMeta::from(&*note);
        meta.ai_summary = summary.trim().chars().take(1200).collect();
        if meta.ai_summary.is_empty() {
            return Err("AI chưa trả về bản tóm tắt.".into());
        }
        meta.summary_source = source.to_owned();
        let path = self
            .root_dir
            .join(&note.folder)
            .join(format!("{id}.meta.json"));
        fs::write(
            path,
            serde_json::to_vec_pretty(&meta).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        note.ai_summary = meta.ai_summary;
        note.summary_source = meta.summary_source;
        Ok(true)
    }

    /// Khởi tạo cấu trúc thư mục trên ổ đĩa
    pub fn init_filesystem(&mut self) {
        let _ = fs::create_dir_all(&self.root_dir);
        // Tạo các thư mục mặc định nếu chưa có
        for folder in &self.folders {
            let f_path = self.root_dir.join(folder);
            let _ = fs::create_dir_all(&f_path);
        }

        // Quét thêm các thư mục con có sẵn trong root_dir
        if let Ok(entries) = fs::read_dir(&self.root_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type()
                    && file_type.is_dir()
                {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !name.starts_with('.') && !self.folders.contains(&name) {
                        self.folders.push(name);
                    }
                }
            }
        }
        self.folders.sort();
    }

    /// Tải lại danh sách ghi chú từ ổ đĩa
    pub fn reload(&mut self) {
        self.notes.clear();
        for folder in &self.folders {
            let folder_dir = self.root_dir.join(folder);
            if !folder_dir.is_dir() {
                continue;
            }
            if let Ok(entries) = fs::read_dir(&folder_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file()
                        && !path
                            .file_name()
                            .is_some_and(|name| name.to_string_lossy().ends_with(".meta.json"))
                        && path
                            .extension()
                            .is_some_and(|ext| ext == "md" || ext == "json")
                        && let Some(note) = self.load_note(&path, folder)
                    {
                        self.notes.push(note);
                    }
                }
            }
        }

        // Sắp xếp ghi chú theo ngày cập nhật mới nhất
        self.notes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        // Nếu chưa có ghi chú nào trong folder đã chọn, tạo 1 ghi chú chào mừng
        if self.notes.is_empty() {
            self.create_note("Ghi chú đầu tiên", "Chào mừng bạn đến với PentaKuru!\n\n- Ghi chú được lưu tự động trong thư mục 'Penteye Notes' cạnh file exe.\n- Bấm 'Tải file lên' hoặc kéo thả file vào đây để đính kèm tài liệu, giáo án, code.");
        }

        if self.selected_note_id.is_none() {
            self.selected_note_id = self.notes.first().map(|n| n.id.clone());
        }
    }

    fn load_note(&self, path: &Path, folder: &str) -> Option<NoteItem> {
        let content = fs::read_to_string(path).ok()?;
        let id = path.file_stem()?.to_string_lossy().to_string();

        // Đọc metadata file đính kèm nếu có thư mục attachments tương ứng
        let attach_dir = self.root_dir.join(folder).join(format!("{id}_files"));
        let mut attachments = Vec::new();
        if attach_dir.is_dir()
            && let Ok(entries) = fs::read_dir(&attach_dir)
        {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    let modified = entry
                        .metadata()
                        .and_then(|m| m.modified())
                        .map(format_date)
                        .unwrap_or_else(|_| "N/A".into());
                    let rel = format!("{folder}/{id}_files/{name}");
                    attachments.push(AttachmentInfo {
                        name,
                        relative_path: rel,
                        size_bytes,
                        modified,
                    });
                }
            }
        }

        let metadata = path.metadata().ok();
        let updated_at = metadata
            .as_ref()
            .and_then(|m| m.modified().ok())
            .map(format_date)
            .unwrap_or_else(|| format_date(SystemTime::now()));
        let created_at = metadata
            .as_ref()
            .and_then(|m| m.created().ok())
            .map(format_date)
            .unwrap_or_else(|| updated_at.clone());

        // Dòng đầu tiên làm tiêu đề nếu có
        let title = content
            .lines()
            .next()
            .map(|l| l.trim_start_matches('#').trim().to_string())
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| id.clone());

        let meta_path = self.root_dir.join(folder).join(format!("{id}.meta.json"));
        let meta = fs::read_to_string(meta_path)
            .ok()
            .and_then(|s| serde_json::from_str::<NoteMeta>(&s).ok())
            .unwrap_or_else(|| NoteMeta {
                icon: default_note_icon(),
                ..Default::default()
            });

        Some(NoteItem {
            id,
            folder: folder.to_string(),
            title,
            content,
            created_at,
            updated_at,
            attachments,
            icon: meta.icon,
            ai_shared: meta.ai_shared,
            task_card_id: meta.task_card_id,
            ai_summary: meta.ai_summary,
            summary_source: meta.summary_source,
        })
    }

    /// Tạo ghi chú mới trong thư mục hiện tại
    pub fn create_note(&mut self, title: &str, content: &str) -> String {
        let now = SystemTime::now();
        let date_str = format_date(now);
        let id = format!(
            "note_{}",
            now.duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );

        let initial_content = if content.is_empty() {
            format!("# {title}\n\nViết nội dung ghi chú ở đây...")
        } else {
            content.to_string()
        };

        let folder = if self.selected_folder == "Tất cả" {
            "Chung".to_owned()
        } else {
            self.selected_folder.clone()
        };
        let note = NoteItem {
            id: id.clone(),
            folder: folder.clone(),
            title: title.to_string(),
            content: initial_content,
            created_at: date_str.clone(),
            updated_at: date_str,
            attachments: Vec::new(),
            icon: default_note_icon(),
            ai_shared: false,
            task_card_id: None,
            ai_summary: String::new(),
            summary_source: String::new(),
        };

        let file_path = self.root_dir.join(&note.folder).join(format!("{id}.md"));
        let _ = fs::write(file_path, &note.content);
        let meta_path = self
            .root_dir
            .join(&note.folder)
            .join(format!("{id}.meta.json"));
        let _ = fs::write(
            meta_path,
            serde_json::to_string(&NoteMeta::from(&note)).unwrap_or_default(),
        );

        self.notes.insert(0, note);
        self.selected_note_id = Some(id.clone());
        id
    }

    pub fn ai_profile(&self) -> String {
        let mut profile = String::new();
        let mut remaining = 16000;
        let mut shared: Vec<_> = self.notes.iter().filter(|note| note.ai_shared).collect();
        shared.sort_by_key(|note| self.selected_note_id.as_ref() != Some(&note.id));
        for note in shared {
            if remaining == 0 {
                break;
            }
            let section = format!(
                "\nNguồn: {} / {} [{}]\n{}\n",
                note.folder,
                note.title,
                note.id,
                if note.summary_is_current() {
                    note.ai_summary.clone()
                } else {
                    "[Chưa có bản tóm tắt cập nhật; không suy đoán nội dung.]".into()
                }
            );
            let excerpt: String = section.chars().take(remaining).collect();
            remaining -= excerpt.chars().count();
            profile.push_str(&excerpt);
        }
        profile
    }

    /// Thêm một thư mục mới
    pub fn add_folder(&mut self, folder_name: &str) {
        let name = folder_name.trim();
        if !name.is_empty() && !self.folders.contains(&name.to_string()) {
            let p = self.root_dir.join(name);
            let _ = fs::create_dir_all(&p);
            self.folders.push(name.to_string());
            self.folders.sort();
            self.selected_folder = name.to_string();
        }
    }

    /// Đặt biểu tượng cho ghi chú đang chọn
    pub fn set_selected_note_icon(&mut self, icon_name: &str) {
        if let Some(id) = &self.selected_note_id
            && let Some(note) = self.notes.iter_mut().find(|n| &n.id == id)
        {
            note.icon = icon_name.to_string();
            let meta_path = self
                .root_dir
                .join(&note.folder)
                .join(format!("{id}.meta.json"));
            let _ = fs::write(
                meta_path,
                serde_json::to_string(&NoteMeta::from(&*note)).unwrap_or_default(),
            );
        }
    }

    /// Di chuyển ghi chú sang thư mục khác
    pub fn move_selected_note_to_folder(&mut self, target_folder: &str) {
        let target = target_folder.trim();
        if target.is_empty() {
            return;
        }
        if let Some(id) = &self.selected_note_id
            && let Some(pos) = self.notes.iter().position(|n| &n.id == id)
        {
            let old_folder = self.notes[pos].folder.clone();
            if old_folder == target {
                return;
            }

            // Di chuyển file .md
            let old_md = self.root_dir.join(&old_folder).join(format!("{id}.md"));
            let new_folder_dir = self.root_dir.join(target);
            let _ = fs::create_dir_all(&new_folder_dir);
            let new_md = new_folder_dir.join(format!("{id}.md"));
            let _ = fs::rename(&old_md, &new_md);

            // Di chuyển file .meta.json
            let old_meta = self
                .root_dir
                .join(&old_folder)
                .join(format!("{id}.meta.json"));
            let new_meta = new_folder_dir.join(format!("{id}.meta.json"));
            let _ = fs::rename(&old_meta, &new_meta);

            // Di chuyển thư mục attachments nếu có
            let old_att = self.root_dir.join(&old_folder).join(format!("{id}_files"));
            let new_att = new_folder_dir.join(format!("{id}_files"));
            let _ = fs::rename(&old_att, &new_att);

            self.notes[pos].folder = target.to_string();
            self.notes[pos].updated_at = format_date(SystemTime::now());

            // Cập nhật lại đường dẫn relative của attachments
            for att in &mut self.notes[pos].attachments {
                att.relative_path = format!("{target}/{id}_files/{}", att.name);
            }
        }
    }

    /// Lưu nội dung ghi chú hiện tại
    pub fn save_selected_note(&mut self) {
        if let Some(id) = &self.selected_note_id
            && let Some(note) = self.notes.iter_mut().find(|n| &n.id == id)
        {
            note.updated_at = format_date(SystemTime::now());
            // Cập nhật tiêu đề theo dòng đầu tiên nếu dòng đầu là heading
            if let Some(first_line) = note.content.lines().next() {
                let cleaned = first_line.trim_start_matches('#').trim();
                if !cleaned.is_empty() {
                    note.title = cleaned.to_string();
                }
            }
            let file_path = self.root_dir.join(&note.folder).join(format!("{id}.md"));
            let _ = fs::write(file_path, &note.content);
            let meta_path = self
                .root_dir
                .join(&note.folder)
                .join(format!("{id}.meta.json"));
            let _ = fs::write(
                meta_path,
                serde_json::to_string(&NoteMeta::from(&*note)).unwrap_or_default(),
            );
        }
    }

    /// Xóa ghi chú đang chọn
    pub fn delete_selected_note(&mut self) {
        if let Some(id) = self.selected_note_id.take() {
            if let Some(pos) = self.notes.iter().position(|n| n.id == id) {
                let note = self.notes.remove(pos);
                let file_path = self.root_dir.join(&note.folder).join(format!("{id}.md"));
                let _ = fs::remove_file(file_path);
                let meta_path = self
                    .root_dir
                    .join(&note.folder)
                    .join(format!("{id}.meta.json"));
                let _ = fs::remove_file(meta_path);
                let attach_dir = self.root_dir.join(&note.folder).join(format!("{id}_files"));
                let _ = fs::remove_dir_all(attach_dir);
            }
            self.selected_note_id = self
                .notes
                .iter()
                .find(|n| n.folder == self.selected_folder)
                .map(|n| n.id.clone())
                .or_else(|| self.notes.first().map(|n| n.id.clone()));
        }
    }

    /// Upload/đính kèm các file vào ghi chú hiện tại
    pub fn upload_files_to_selected(&mut self, file_paths: &[PathBuf]) {
        let Some(note_id) = self.selected_note_id.clone() else {
            return;
        };
        let Some(note) = self.notes.iter_mut().find(|n| n.id == note_id) else {
            return;
        };

        let attach_dir = self
            .root_dir
            .join(&note.folder)
            .join(format!("{note_id}_files"));
        let _ = fs::create_dir_all(&attach_dir);

        let mut added = 0;
        for src in file_paths {
            if src.is_file()
                && let Some(filename) = src.file_name()
            {
                let dest = attach_dir.join(filename);
                if fs::copy(src, &dest).is_ok() {
                    let size_bytes = dest.metadata().map(|m| m.len()).unwrap_or(0);
                    let modified = format_date(SystemTime::now());
                    let name = filename.to_string_lossy().to_string();
                    let rel = format!("{}/{note_id}_files/{name}", note.folder);

                    // Xóa trùng nếu có
                    note.attachments.retain(|a| a.name != name);
                    note.attachments.push(AttachmentInfo {
                        name,
                        relative_path: rel,
                        size_bytes,
                        modified,
                    });
                    added += 1;
                }
            }
        }

        if added > 0 {
            self.status_message = Some(format!("Đã tải lên {added} file thành công!"));
        }
    }

    /// Lấy danh sách ghi chú theo thư mục đang chọn và từ khóa tìm kiếm
    pub fn visible_notes(&self) -> Vec<&NoteItem> {
        self.notes
            .iter()
            .filter(|n| {
                let match_folder =
                    self.selected_folder == "Tất cả" || n.folder == self.selected_folder;
                let match_search = self.search_query.is_empty()
                    || n.title
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase())
                    || n.content
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase());
                match_folder && match_search
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> NotesManager {
        let nonce = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        NotesManager::at(PathBuf::from(format!(".build/note-context-{nonce}")))
    }

    #[test]
    fn sharing_and_task_link_survive_edits_and_reload_without_metadata_notes() {
        let mut manager = fixture();
        let id = manager.create_note("Bài học", "# Bài học\n\nMục tiêu riêng");
        let note = manager.notes.iter_mut().find(|note| note.id == id).unwrap();
        note.ai_shared = true;
        note.task_card_id = Some("card-123".into());
        let source = note.content_fingerprint();
        manager
            .store_summary(&id, &source, "Mục tiêu riêng")
            .unwrap();
        manager.save_selected_note();
        manager.set_selected_note_icon("book");
        let count = manager.notes.len();
        manager.reload();
        assert_eq!(manager.notes.len(), count);
        let note = manager.notes.iter().find(|note| note.id == id).unwrap();
        assert!(note.ai_shared);
        assert_eq!(note.task_card_id.as_deref(), Some("card-123"));
        assert_eq!(note.icon, "book");
        assert!(manager.ai_profile().contains("Mục tiêu riêng"));
        manager
            .notes
            .iter_mut()
            .find(|note| note.id == id)
            .unwrap()
            .ai_shared = false;
        manager.save_selected_note();
        manager.reload();
        assert!(manager.ai_profile().is_empty());
    }

    #[test]
    fn legacy_metadata_is_private_and_profile_excludes_other_notes() {
        let meta: NoteMeta = serde_json::from_str(r#"{"icon":"file"}"#).unwrap();
        assert!(!meta.ai_shared);
        let mut manager = fixture();
        manager.create_note("Bí mật", "Không được gửi nội dung này");
        let shared = manager.create_note("Học", "Được phép dùng làm ngữ cảnh");
        manager
            .notes
            .iter_mut()
            .find(|note| note.id == shared)
            .unwrap()
            .ai_shared = true;
        let source = manager
            .notes
            .iter()
            .find(|note| note.id == shared)
            .unwrap()
            .content_fingerprint();
        manager
            .store_summary(&shared, &source, "Được phép dùng làm ngữ cảnh")
            .unwrap();
        let profile = manager.ai_profile();
        assert!(profile.contains("Được phép"));
        assert!(!profile.contains("Không được gửi"));
        assert!(!profile.contains("Chào mừng"));
    }

    #[test]
    fn context_budget_is_bounded_and_prioritizes_selected_note() {
        let mut manager = fixture();
        for position in 0..9 {
            let id = manager.create_note(&format!("Bài {position}"), &"ế".repeat(4000));
            manager
                .notes
                .iter_mut()
                .find(|note| note.id == id)
                .unwrap()
                .ai_shared = true;
        }
        manager.selected_note_id = manager.notes.last().map(|note| note.id.clone());
        let selected = manager.selected_note_id.clone().unwrap();
        manager
            .notes
            .iter_mut()
            .find(|note| note.id == selected)
            .unwrap()
            .ai_shared = true;
        let profile = manager.ai_profile();
        assert!(profile.chars().count() <= 16000);
        assert!(profile.lines().nth(1).unwrap().contains(&selected));
    }

    #[test]
    fn memory_uses_persisted_summary_and_rejects_stale_or_unshared_results() {
        let mut manager = fixture();
        let id = manager.create_note("Dự án", "# Dự án\n\nNội dung gốc chi tiết");
        let note = manager.notes.iter_mut().find(|note| note.id == id).unwrap();
        note.ai_shared = true;
        let source = note.content_fingerprint();
        assert!(
            manager
                .store_summary(&id, &source, "- Hoàn thiện dự án thứ Sáu.")
                .unwrap()
        );
        manager.reload();
        let profile = manager.ai_profile();
        assert!(profile.contains("Hoàn thiện dự án thứ Sáu"));
        assert!(!profile.contains("Nội dung gốc chi tiết"));
        manager
            .notes
            .iter_mut()
            .find(|note| note.id == id)
            .unwrap()
            .content
            .push_str("\nĐổi kế hoạch.");
        assert!(!manager.store_summary(&id, &source, "Kết quả cũ").unwrap());
        assert!(!manager.ai_profile().contains("Hoàn thiện dự án"));
        let note = manager.notes.iter_mut().find(|note| note.id == id).unwrap();
        let current = note.content_fingerprint();
        note.ai_shared = false;
        assert!(
            !manager
                .store_summary(&id, &current, "Không được lưu")
                .unwrap()
        );
        assert!(manager.ai_profile().is_empty());
    }
}
