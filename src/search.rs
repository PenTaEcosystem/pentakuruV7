//! Thanh tìm kiếm truy xuất link, file, thư mục và ứng dụng trong máy tính (Alt + S).
//! Tích hợp RAG Thư mục (Folder RAG) & Quét phát hiện file trùng lặp (Duplicate Detection).

use crate::system::{data_dir, format_date, format_size, home_dir, open, reveal};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::SystemTime;
use walkdir::WalkDir;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SearchKind {
    Folder,
    Document,
    Code,
    App,
    Media,
    WebLink,
    Other,
}

impl SearchKind {
    pub fn from_path(path: &Path) -> Self {
        if path.is_dir() {
            return SearchKind::Folder;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "exe" | "lnk" | "bat" | "cmd" | "ps1" | "msi" => SearchKind::App,
            "rs" | "py" | "js" | "ts" | "html" | "css" | "json" | "c" | "cpp" | "java" | "sql"
            | "php" | "go" | "sh" => SearchKind::Code,
            "doc" | "docx" | "pdf" | "txt" | "md" | "ppt" | "pptx" | "xls" | "xlsx" | "csv" => {
                SearchKind::Document
            }
            "png" | "jpg" | "jpeg" | "gif" | "mp4" | "mp3" | "wav" | "svg" | "webp" => {
                SearchKind::Media
            }
            _ => SearchKind::Other,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub name: String,
    pub path: String,
    pub kind: SearchKind,
    pub size: String,
    #[allow(dead_code)]
    pub size_bytes: u64,
    #[allow(dead_code)]
    pub modified: String,
    pub snippet: Option<String>,
    pub dup_copies: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct DuplicateGroup {
    pub file_name: String,
    pub size_bytes: u64,
    pub formatted_size: String,
    pub paths: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SearchFilter {
    #[default]
    All,
    Duplicates,
    RagContent,
}

impl SearchFilter {
    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            SearchFilter::All => "Tất cả",
            SearchFilter::Duplicates => "File trùng lặp",
            SearchFilter::RagContent => "RAG Nội dung",
        }
    }
}

#[derive(Clone)]
pub struct SearchState {
    pub query: String,
    pub filter: SearchFilter,
    pub results: Vec<SearchResult>,
    pub indexed_files: Arc<Mutex<Vec<SearchResult>>>,
    pub duplicate_groups: Arc<Mutex<Vec<DuplicateGroup>>>,
    pub total_wasted_bytes: Arc<Mutex<u64>>,
    pub is_indexing: Arc<Mutex<bool>>,
    #[allow(dead_code)]
    pub status: Option<String>,
}

impl Default for SearchState {
    fn default() -> Self {
        let state = Self::empty();
        state.start_background_indexing();
        state
    }
}

impl SearchState {
    pub fn empty() -> Self {
        Self {
            query: String::new(),
            filter: SearchFilter::All,
            results: Vec::new(),
            indexed_files: Arc::new(Mutex::new(Vec::new())),
            duplicate_groups: Arc::new(Mutex::new(Vec::new())),
            total_wasted_bytes: Arc::new(Mutex::new(0)),
            is_indexing: Arc::new(Mutex::new(false)),
            status: None,
        }
    }

    /// Tính băm nhanh (quick hash) dựa trên kích thước + 4KB đầu + 4KB cuối của file
    fn quick_hash(path: &Path, size: u64) -> Option<u64> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;
        use std::io::{Read, Seek};

        if size == 0 {
            return Some(0);
        }
        let mut file = std::fs::File::open(path).ok()?;
        let mut hasher = DefaultHasher::new();
        hasher.write_u64(size);

        let mut buf = [0u8; 4096];
        let n = file.read(&mut buf).ok()?;
        hasher.write(&buf[..n]);

        if size > 8192 {
            let _ = file.seek(std::io::SeekFrom::End(-4096));
            let n2 = file.read(&mut buf).ok().unwrap_or(0);
            hasher.write(&buf[..n2]);
        }
        Some(hasher.finish())
    }

    /// Quét background các thư mục trọng yếu (Desktop, Documents, Downloads, Notes)
    /// Đồng thời phát hiện file trùng lặp (Duplicate Detection)
    pub fn start_background_indexing(&self) {
        let files_arc = self.indexed_files.clone();
        let dups_arc = self.duplicate_groups.clone();
        let wasted_arc = self.total_wasted_bytes.clone();
        let indexing_arc = self.is_indexing.clone();

        thread::spawn(move || {
            {
                let mut lock = indexing_arc.lock().unwrap();
                if *lock {
                    return;
                }
                *lock = true;
            }

            let mut items: Vec<SearchResult> = Vec::new();
            let mut hash_map: HashMap<(u64, u64), Vec<usize>> = HashMap::new();

            let home = home_dir();
            let notes = data_dir();

            let target_dirs = vec![
                home.join("Desktop"),
                home.join("Documents"),
                home.join("Downloads"),
                notes,
            ];

            for root in target_dirs {
                if !root.is_dir() {
                    continue;
                }
                for entry in WalkDir::new(&root)
                    .max_depth(3)
                    .into_iter()
                    .filter_entry(|e| {
                        let name = e.file_name().to_string_lossy();
                        !name.starts_with('.')
                            && name != "node_modules"
                            && name != "target"
                            && name != ".git"
                    })
                    .flatten()
                {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();
                    let kind = SearchKind::from_path(path);
                    let metadata = entry.metadata().ok();
                    let size_bytes = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                    let size = metadata
                        .as_ref()
                        .map(|m| format_size(m.len()))
                        .unwrap_or_else(|| "".into());
                    let modified = metadata
                        .and_then(|m| m.modified().ok())
                        .map(format_date)
                        .unwrap_or_else(|| "".into());

                    let idx = items.len();
                    items.push(SearchResult {
                        name: name.clone(),
                        path: path.to_string_lossy().to_string(),
                        kind,
                        size,
                        size_bytes,
                        modified,
                        snippet: None,
                        dup_copies: Vec::new(),
                    });

                    // Tính hash cho file để phát hiện trùng lặp
                    if entry.file_type().is_file()
                        && size_bytes > 0
                        && size_bytes <= 1_000_000_000
                        && let Some(h) = Self::quick_hash(path, size_bytes)
                    {
                        hash_map.entry((size_bytes, h)).or_default().push(idx);
                    }

                    if items.len() >= 4000 {
                        break;
                    }
                }
            }

            // Xử lý các nhóm file trùng lặp
            let mut groups = Vec::new();
            let mut total_wasted: u64 = 0;

            for ((size_bytes, _hash), indices) in hash_map {
                if indices.len() > 1 {
                    let group_paths: Vec<String> =
                        indices.iter().map(|&i| items[i].path.clone()).collect();
                    let first_name = items[indices[0]].name.clone();

                    total_wasted += size_bytes * (indices.len() as u64 - 1);

                    for &idx in &indices {
                        let other_paths: Vec<String> = group_paths
                            .iter()
                            .filter(|p| **p != items[idx].path)
                            .cloned()
                            .collect();
                        items[idx].dup_copies = other_paths;
                    }

                    groups.push(DuplicateGroup {
                        file_name: first_name,
                        size_bytes,
                        formatted_size: format_size(size_bytes),
                        paths: group_paths,
                    });
                }
            }

            // Sắp xếp nhóm trùng lặp theo dung lượng lãng phí nhiều nhất lên đầu
            groups.sort_by(|a, b| {
                let waste_b = b.size_bytes * b.paths.len() as u64;
                let waste_a = a.size_bytes * a.paths.len() as u64;
                waste_b.cmp(&waste_a)
            });

            if let Ok(mut lock) = files_arc.lock() {
                *lock = items;
            }
            if let Ok(mut lock) = dups_arc.lock() {
                *lock = groups;
            }
            if let Ok(mut lock) = wasted_arc.lock() {
                *lock = total_wasted;
            }
            if let Ok(mut lock) = indexing_arc.lock() {
                *lock = false;
            }
        });
    }

    /// Trích xuất ngữ cảnh RAG từ các tệp văn bản/mã nguồn máy tính
    pub fn get_rag_context(&self, query: &str, max_files: usize) -> String {
        let q_lower = query.to_lowercase();
        let terms: Vec<&str> = q_lower.split_whitespace().collect();
        if terms.is_empty() {
            return String::new();
        }

        let mut context = String::new();
        let mut count = 0;

        if let Ok(files) = self.indexed_files.lock() {
            for item in files.iter() {
                if count >= max_files {
                    break;
                }
                if matches!(item.kind, SearchKind::Document | SearchKind::Code) {
                    let p = Path::new(&item.path);
                    if let Ok(content) = std::fs::read_to_string(p) {
                        let content_lower = content.to_lowercase();
                        // Kiểm tra nếu có bất kỳ từ khóa nào xuất hiện
                        if terms.iter().any(|t| content_lower.contains(t)) {
                            // Trích xuất đoạn chứa từ khóa
                            let mut snippet = String::new();
                            for line in content.lines() {
                                let line_l = line.to_lowercase();
                                if terms.iter().any(|t| line_l.contains(t)) {
                                    snippet.push_str(line.trim());
                                    snippet.push('\n');
                                    if snippet.len() > 600 {
                                        break;
                                    }
                                }
                            }
                            if !snippet.is_empty() {
                                context.push_str(&format!(
                                    "--- [Tệp: {} | Đường dẫn: {}] ---\n{}\n\n",
                                    item.name,
                                    item.path,
                                    snippet.trim()
                                ));
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
        context
    }

    /// Tìm kiếm RAG full-text trong nội dung tệp
    pub fn search_rag_content(&self, q: &str) -> Vec<SearchResult> {
        let q_lower = q.to_lowercase();
        let terms: Vec<&str> = q_lower.split_whitespace().collect();
        if terms.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();
        if let Ok(files) = self.indexed_files.lock() {
            for item in files.iter() {
                if results.len() >= 12 {
                    break;
                }
                if matches!(item.kind, SearchKind::Document | SearchKind::Code) {
                    let p = Path::new(&item.path);
                    // Đọc tối đa 64KB để không làm chậm UI
                    if let Ok(file) = std::fs::File::open(p) {
                        use std::io::Read;
                        let mut handle = file.take(65536);
                        let mut text = String::new();
                        if handle.read_to_string(&mut text).is_ok() {
                            let text_l = text.to_lowercase();
                            if terms.iter().all(|t| text_l.contains(t)) {
                                // Trích xuất dòng khớp đầu tiên
                                let mut matched_line = String::new();
                                for line in text.lines() {
                                    if terms.iter().any(|t| line.to_lowercase().contains(t)) {
                                        matched_line = line.trim().to_string();
                                        break;
                                    }
                                }
                                let mut res = item.clone();
                                if !matched_line.is_empty() {
                                    res.snippet = Some(matched_line);
                                }
                                results.push(res);
                            }
                        }
                    }
                }
            }
        }
        results
    }

    /// Cập nhật kết quả tìm kiếm theo query và bộ lọc
    pub fn update_search(&mut self) {
        let raw_q = self.query.trim();
        self.results.clear();

        // Xử lý bộ lọc thông qua tiền tố query
        let (filter, q) = if raw_q.starts_with("#dup") || raw_q.starts_with("dup:") {
            (
                SearchFilter::Duplicates,
                raw_q
                    .trim_start_matches("#dup")
                    .trim_start_matches("dup:")
                    .trim(),
            )
        } else if raw_q.starts_with("#rag") || raw_q.starts_with("rag:") {
            (
                SearchFilter::RagContent,
                raw_q
                    .trim_start_matches("#rag")
                    .trim_start_matches("rag:")
                    .trim(),
            )
        } else {
            (self.filter, raw_q)
        };

        // 1. Chế độ hiển thị file trùng lặp
        if filter == SearchFilter::Duplicates {
            if let Ok(groups) = self.duplicate_groups.lock() {
                let q_lower = q.to_lowercase();
                for g in groups.iter() {
                    if self.results.len() >= 15 {
                        break;
                    }
                    if q.is_empty()
                        || g.file_name.to_lowercase().contains(&q_lower)
                        || g.paths.iter().any(|p| p.to_lowercase().contains(&q_lower))
                    {
                        let waste = g.size_bytes * (g.paths.len() as u64 - 1);
                        let waste_str = format_size(waste);
                        let info_snippet = format!(
                            "Trùng {} bản (lãng phí ~{}) | {}",
                            g.paths.len(),
                            waste_str,
                            g.paths.join(" · ")
                        );
                        self.results.push(SearchResult {
                            name: format!("{} [Trùng x{}]", g.file_name, g.paths.len()),
                            path: g.paths.first().cloned().unwrap_or_default(),
                            kind: SearchKind::from_path(Path::new(&g.file_name)),
                            size: g.formatted_size.clone(),
                            size_bytes: g.size_bytes,
                            modified: format!("Tiết kiệm: {}", waste_str),
                            snippet: Some(info_snippet),
                            dup_copies: g.paths.clone(),
                        });
                    }
                }
            }
            return;
        }

        // 2. Chế độ tìm kiếm nội dung tài liệu (Folder RAG)
        if filter == SearchFilter::RagContent {
            if !q.is_empty() {
                self.results = self.search_rag_content(q);
            }
            return;
        }

        // 3. Chế độ tìm kiếm thông thường (Tất cả)
        if q.is_empty() {
            // Khi ô tìm kiếm trống, giữ sạch danh sách kết quả để thanh tìm kiếm tối giản tuyệt đối dạng pill
            return;
        }

        let q_trimmed = q.trim();
        let q_lower = q_trimmed.to_lowercase();

        // Hỗ trợ chế độ UI Agent ngay trong thanh tìm kiếm
        if q_lower.starts_with("@ui") || q_lower.starts_with("@tars") {
            let action_text = q_trimmed
                .trim_start_matches("@ui")
                .trim_start_matches("@UI")
                .trim_start_matches("@tars")
                .trim_start_matches("@TARS")
                .trim();
            self.results.push(SearchResult {
                name: if action_text.is_empty() {
                    "Chế độ UI Agent: Gõ thao tác (ví dụ: @ui nhấp vào nút Đăng nhập)".into()
                } else {
                    format!("Thực thi UI: {action_text}")
                },
                path: "Nhấn Enter để chụp màn hình và phân tích thị giác".into(),
                kind: SearchKind::WebLink,
                size: "[UI]".into(),
                size_bytes: 0,
                modified: "Hành động AI".into(),
                snippet: Some("Tự động nhận diện giao diện và thực thi click chuột".into()),
                dup_copies: Vec::new(),
            });
            return;
        }

        // Nếu người dùng nhập URL hoặc web link
        if q.starts_with("http://") || q.starts_with("https://") || q.starts_with("www.") {
            let url = if q.starts_with("www.") {
                format!("https://{q}")
            } else {
                q.to_string()
            };
            self.results.push(SearchResult {
                name: format!("Mở liên kết web: {url}"),
                path: url,
                kind: SearchKind::WebLink,
                size: "Web Link".into(),
                size_bytes: 0,
                modified: format_date(SystemTime::now()),
                snippet: None,
                dup_copies: Vec::new(),
            });
        }

        let q_lower = q.to_lowercase();
        if let Ok(files) = self.indexed_files.lock() {
            let is_path_query = q.contains('\\') || q.contains('/');
            let mut matches: Vec<SearchResult> = files
                .iter()
                .filter(|item| {
                    if is_path_query {
                        item.path.to_lowercase().contains(&q_lower)
                    } else if q_lower.len() == 1 {
                        item.name.to_lowercase().starts_with(&q_lower)
                    } else {
                        item.name.to_lowercase().contains(&q_lower)
                    }
                })
                .take(15)
                .cloned()
                .collect();

            // Sắp xếp ưu tiên khớp tên chính xác hơn
            matches.sort_by(|a, b| {
                let a_exact = a.name.to_lowercase().starts_with(&q_lower);
                let b_exact = b.name.to_lowercase().starts_with(&q_lower);
                b_exact.cmp(&a_exact)
            });

            self.results.extend(matches);
        }
    }

    pub fn open_item(&self, item: &SearchResult) -> Result<(), String> {
        open(&item.path)
    }

    pub fn reveal_item(&self, item: &SearchResult) -> Result<(), String> {
        let p = Path::new(&item.path);
        reveal(p)
    }
}
