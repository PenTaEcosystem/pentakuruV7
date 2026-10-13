//! Bộ xử lý sửa lỗi chính tả tiếng Việt & thuật ngữ công nghệ.
//! Hỗ trợ:
//! 1. Rule-based offline tức thì (0ms, không cần mạng, không cần API key):
//!    - Chuẩn hóa dấu câu (khoảng trắng trước/sau dấu . , : ; ! ?).
//!    - Tự động viết hoa đầu câu và sau dấu chấm.
//!    - Chuẩn hóa dấu thanh tiếng Việt (hòa, thủy,...).
//!    - Sửa các từ tiếng Việt gõ thiếu dấu/sai chính tả phổ biến.
//!    - Chuẩn hóa thuật ngữ công nghệ (API, FPS, GPU, AI, Rust, GitHub, Windows,...).
//! 2. Sửa ngữ cảnh nâng cao bằng AI nếu người dùng cấu hình API key.
//! 3. Tự động lấy text được bôi đen qua Clipboard/SendInput trên Windows và dán lại.

use crate::ai::{AiSettings, Message, Role, chat};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SpellSettings {
    pub auto_paste: bool,
    pub use_ai_if_ready: bool,
    pub show_review_window: bool,
}

impl Default for SpellSettings {
    fn default() -> Self {
        Self {
            auto_paste: true,
            use_ai_if_ready: true,
            show_review_window: false,
        }
    }
}

/// Kết quả sau khi sửa lỗi chính tả.
#[derive(Clone, Debug)]
pub struct SpellResult {
    pub original: String,
    pub fixed: String,
    pub changed: bool,
    pub method: String,
}

/// Từ điển thuật ngữ công nghệ viết hoa chuẩn
const TECH_TERMS: &[(&str, &str)] = &[
    ("api", "API"),
    ("fps", "FPS"),
    ("gpu", "GPU"),
    ("cpu", "CPU"),
    ("ram", "RAM"),
    ("rom", "ROM"),
    ("ai", "AI"),
    ("ui", "UI"),
    ("ux", "UX"),
    ("url", "URL"),
    ("http", "HTTP"),
    ("https", "HTTPS"),
    ("html", "HTML"),
    ("css", "CSS"),
    ("json", "JSON"),
    ("xml", "XML"),
    ("sql", "SQL"),
    ("exe", "EXE"),
    ("zip", "ZIP"),
    ("pdf", "PDF"),
    ("doc", "DOC"),
    ("docx", "DOCX"),
    ("ppt", "PPT"),
    ("pptx", "PPTX"),
    ("xls", "XLS"),
    ("xlsx", "XLSX"),
    ("png", "PNG"),
    ("jpg", "JPG"),
    ("jpeg", "JPEG"),
    ("svg", "SVG"),
    ("rust", "Rust"),
    ("python", "Python"),
    ("javascript", "JavaScript"),
    ("typescript", "TypeScript"),
    ("github", "GitHub"),
    ("gitlab", "GitLab"),
    ("windows", "Windows"),
    ("macos", "macOS"),
    ("linux", "Linux"),
    ("android", "Android"),
    ("ios", "iOS"),
    ("chatgpt", "ChatGPT"),
    ("gemini", "Gemini"),
    ("claude", "Claude"),
    ("ollama", "Ollama"),
    ("openai", "OpenAI"),
    ("google", "Google"),
    ("microsoft", "Microsoft"),
    ("apple", "Apple"),
    ("pentakuru", "PentaKuru"),
];

/// Bảng sửa từ tiếng Việt phổ biến (sai chính tả, thiếu dấu hoặc gõ telex sai)
const VI_WORDS: &[(&str, &str)] = &[
    ("chinh ta", "chính tả"),
    ("chính ta", "chính tả"),
    ("chinh tả", "chính tả"),
    ("to den", "tô đen"),
    ("danh may", "đánh máy"),
    ("he thong", "hệ thống"),
    ("tu dong", "tự động"),
    ("cai dat", "cài đặt"),
    ("cài đăt", "cài đặt"),
    ("cài đặi", "cài đặt"),
    ("phim tat", "phím tắt"),
    ("phím tăt", "phím tắt"),
    ("tim kiem", "tìm kiếm"),
    ("ghi chu", "ghi chú"),
    ("thu muc", "thư mục"),
    ("du an", "dự án"),
    ("giao vien", "giáo viên"),
    ("lap trinh vien", "lập trình viên"),
    ("mo hinh", "mô hình"),
    ("xuat hien", "xuất hiện"),
    ("yeu cau", "yêu cầu"),
    ("sap xep", "sắp xếp"),
    ("săp xếp", "sắp xếp"),
    ("rai rac", "rải rác"),
    ("rải rát", "rải rác"),
    ("khong", "không"),
    ("khg", "không"),
    ("k ", "không "),
    ("duoc", "được"),
    ("đc", "được"),
    ("dc", "được"),
    ("nguoi", "người"),
    ("nhieu", "nhiều"),
    ("nhìu", "nhiều"),
    ("viet", "viết"),
    ("hoac", "hoặc"),
    ("nhung", "nhưng"),
    ("chua", "chưa"),
    ("thanh search", "thanh tìm kiếm"),
    ("hiễn", "hiện"),
    ("hiễn thị", "hiển thị"),
    ("bâm", "bấm"),
    ("chĩnh", "chỉnh"),
    ("hoạc", "hoặc"),
    ("gon", "gọn"),
    ("chế đố", "chế độ"),
    ("thư một", "thư mục"),
    ("thư mục", "thư mục"),
    ("hiển thị", "hiển thị"),
    ("nut", "nút"),
    ("giông", "giống"),
    ("xuát", "xuất"),
    ("tụ động", "tự động"),
];

/// Chuẩn hóa dấu thanh kiểu mới (hòa thay vì hoà, thủy thay vì thuỷ)
const TONE_FIXES: &[(&str, &str)] = &[
    ("hoà", "hòa"),
    ("toà", "tòa"),
    ("loà", "lòa"),
    ("xoà", "xòa"),
    ("thuỷ", "thủy"),
    ("tuỷ", "tủy"),
    ("luỷ", "lũy"),
    ("quỳ", "quỳ"),
    ("khoẻ", "khỏe"),
];

/// Sửa lỗi chính tả bằng luật (Offline Engine)
pub fn fix_text_offline(text: &str) -> String {
    if text.trim().is_empty() {
        return text.to_owned();
    }

    let mut result = text.to_owned();

    // 1. Sửa các cặp từ tiếng Việt phổ biến (case-insensitive nhưng giữ cấu trúc)
    for (wrong, right) in VI_WORDS {
        result = replace_case_flexible(&result, wrong, right);
    }

    // 2. Chuẩn hóa vị trí dấu thanh
    for (old_tone, new_tone) in TONE_FIXES {
        result = result.replace(old_tone, new_tone);
        let old_cap = capitalize(old_tone);
        let new_cap = capitalize(new_tone);
        result = result.replace(&old_cap, &new_cap);
    }

    // 3. Chuẩn hóa thuật ngữ công nghệ
    for (term_lower, term_proper) in TECH_TERMS {
        result = replace_standalone_word(&result, term_lower, term_proper);
    }

    // 4. Chuẩn hóa dấu câu và khoảng trắng
    result = clean_punctuation(&result);

    // 5. Viết hoa chữ cái đầu câu và sau dấu chấm
    result = capitalize_sentences(&result);

    result
}

/// Sửa lỗi chính tả bằng AI hoặc Fallback về Offline
pub fn fix_text(settings: &SpellSettings, ai_settings: &AiSettings, text: &str) -> SpellResult {
    let original = text.to_string();

    if settings.use_ai_if_ready && ai_settings.ready() {
        let system_prompt = "Bạn là trợ lý sửa lỗi chính tả tiếng Việt và chuẩn hóa văn bản chuyên nghiệp. \
Quy tắc bắt buộc:\
1. Sửa toàn bộ lỗi chính tả, dấu thanh, dấu câu và viết hoa đúng ngữ pháp tiếng Việt.\
2. Giữ nguyên thuật ngữ kỹ thuật, tên riêng, mã nguồn, lệnh, đường dẫn, URL và biến số.\
3. Giữ nguyên ngôi xưng và giọng văn tự nhiên của người viết (thân mật hoặc trang trọng).\
4. Tuyệt đối KHÔNG thêm bớt lời dẫn, giải thích hay chào hỏi. CHỈ trả về duy nhất đoạn văn bản sau khi đã sửa chính xác.";

        let messages = vec![Message {
            role: Role::User,
            text: format!("Hãy sửa lỗi chính tả đoạn văn bản sau:\n{text}"),
            images: Vec::new(),
        }];

        match chat(ai_settings, system_prompt, &messages) {
            Ok(ai_fixed) => {
                let fixed = ai_fixed.trim().to_string();
                let changed = fixed != original;
                return SpellResult {
                    original,
                    fixed,
                    changed,
                    method: format!("AI ({})", ai_settings.model),
                };
            }
            Err(e) => {
                eprintln!("AI spellcheck error ({e}), falling back to offline engine");
            }
        }
    }

    // Offline rule-based engine
    let fixed = fix_text_offline(&original);
    let changed = fixed != original;
    SpellResult {
        original,
        fixed,
        changed,
        method: "Offline Engine (Luật tiếng Việt & Từ điển)".to_string(),
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Thay thế từ/cụm từ không phân biệt hoa thường, giữ hoa nếu từ ban đầu viết hoa
fn replace_case_flexible(haystack: &str, needle: &str, replacement: &str) -> String {
    let lower_haystack = haystack.to_lowercase();
    let lower_needle = needle.to_lowercase();
    let mut result = String::new();
    let mut last_idx = 0;

    for (start_idx, _) in lower_haystack.match_indices(&lower_needle) {
        // Kiểm tra ranh giới từ (word boundary)
        let before_ok = start_idx == 0
            || !haystack[..start_idx]
                .chars()
                .last()
                .unwrap_or(' ')
                .is_alphanumeric();
        let end_idx = start_idx + needle.len();
        let after_ok = end_idx >= haystack.len()
            || !haystack[end_idx..]
                .chars()
                .next()
                .unwrap_or(' ')
                .is_alphanumeric();

        if before_ok && after_ok {
            result.push_str(&haystack[last_idx..start_idx]);
            // Kiểm tra chữ cái đầu của đoạn gốc có viết hoa không
            let orig_slice = &haystack[start_idx..end_idx];
            let is_capitalized = orig_slice.chars().next().is_some_and(|c| c.is_uppercase());
            if is_capitalized {
                result.push_str(&capitalize(replacement));
            } else {
                result.push_str(replacement);
            }
            last_idx = end_idx;
        }
    }
    result.push_str(&haystack[last_idx..]);
    result
}

/// Thay thế từ thuật ngữ đứng độc lập (word boundary)
fn replace_standalone_word(haystack: &str, word_lower: &str, target: &str) -> String {
    let lower = haystack.to_lowercase();
    let mut result = String::new();
    let mut last_idx = 0;

    for (start_idx, _) in lower.match_indices(word_lower) {
        let before_char = haystack[..start_idx].chars().last();
        let after_idx = start_idx + word_lower.len();
        let after_char = haystack[after_idx..].chars().next();

        let before_boundary = before_char.is_none_or(|c| !c.is_alphanumeric());
        let after_boundary = after_char.is_none_or(|c| !c.is_alphanumeric());

        if before_boundary && after_boundary {
            result.push_str(&haystack[last_idx..start_idx]);
            result.push_str(target);
            last_idx = after_idx;
        }
    }
    result.push_str(&haystack[last_idx..]);
    result
}

/// Chuẩn hóa dấu câu: bỏ cách trước dấu chấm phẩy, đảm bảo cách sau dấu
fn clean_punctuation(text: &str) -> String {
    let mut res = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == ' '
            && let Some(&next) = chars.peek()
            && matches!(next, '.' | ',' | ':' | ';' | '!' | '?')
        {
            // Bỏ dấu cách trước dấu câu
            continue;
        }
        res.push(c);
        if matches!(c, '.' | ',' | ':' | ';' | '!' | '?')
            && let Some(&next) = chars.peek()
            && next != ' '
            && next != '\n'
            && next != '"'
            && next != ')'
            && next != ']'
            && !next.is_numeric()
        {
            res.push(' ');
        }
    }
    res
}

/// Viết hoa chữ cái đầu câu
fn capitalize_sentences(text: &str) -> String {
    let mut res = String::with_capacity(text.len());
    let mut capitalize_next = true;

    for c in text.chars() {
        if capitalize_next && c.is_alphabetic() {
            res.extend(c.to_uppercase());
            capitalize_next = false;
        } else {
            res.push(c);
            if matches!(c, '.' | '!' | '?' | '\n') {
                capitalize_next = true;
            }
        }
    }
    res
}

// ==================== Tương tác Windows Clipboard & SendInput ====================

#[cfg(target_os = "windows")]
fn make_key_input(
    vk_code: u16,
    flags: u32,
) -> windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT {
    let scan = unsafe {
        windows_sys::Win32::UI::Input::KeyboardAndMouse::MapVirtualKeyW(vk_code as u32, 0) as u16
    };
    windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT {
        r#type: windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT_KEYBOARD,
        Anonymous: windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
            ki: windows_sys::Win32::UI::Input::KeyboardAndMouse::KEYBDINPUT {
                wVk: vk_code,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Đợi người dùng nhả các phím modifier (Alt, Ctrl, Shift) và phím nóng,
/// sau đó gửi sự kiện KEYUP giải phóng triệt để và gửi key mask để tránh menu bar bị kích hoạt.
#[cfg(target_os = "windows")]
pub fn wait_and_release_modifiers(extra_vk: u16) {
    use std::thread::sleep;
    use std::time::{Duration, Instant};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, KEYEVENTF_KEYUP, SendInput, VK_CONTROL, VK_LMENU, VK_LWIN, VK_MENU,
        VK_RMENU, VK_RWIN, VK_SHIFT,
    };

    // Đợi người dùng nhả ngón tay khỏi bàn phím (tối đa 1000ms)
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(1000) {
        let alt_down = unsafe {
            GetAsyncKeyState(VK_MENU as i32) < 0
                || GetAsyncKeyState(VK_LMENU as i32) < 0
                || GetAsyncKeyState(VK_RMENU as i32) < 0
        };
        let key_down = if extra_vk != 0 {
            unsafe { GetAsyncKeyState(extra_vk as i32) < 0 }
        } else {
            false
        };
        let ctrl_down = unsafe { GetAsyncKeyState(VK_CONTROL as i32) < 0 };
        let shift_down = unsafe { GetAsyncKeyState(VK_SHIFT as i32) < 0 };

        if !alt_down && !key_down && !ctrl_down && !shift_down {
            break;
        }
        sleep(Duration::from_millis(20));
    }

    // Gửi sự kiện KEYUP dứt điểm cho các phím modifier
    let mut releases = vec![
        make_key_input(VK_MENU, KEYEVENTF_KEYUP),
        make_key_input(VK_LMENU, KEYEVENTF_KEYUP),
        make_key_input(VK_RMENU, KEYEVENTF_KEYUP),
        make_key_input(VK_CONTROL, KEYEVENTF_KEYUP),
        make_key_input(VK_SHIFT, KEYEVENTF_KEYUP),
        make_key_input(VK_LWIN, KEYEVENTF_KEYUP),
        make_key_input(VK_RWIN, KEYEVENTF_KEYUP),
    ];
    if extra_vk != 0 {
        releases.push(make_key_input(extra_vk, KEYEVENTF_KEYUP));
    }

    unsafe {
        SendInput(
            releases.len() as u32,
            releases.as_ptr(),
            std::mem::size_of::<windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT>() as i32,
        );
    }
    sleep(Duration::from_millis(25));
}

#[cfg(target_os = "windows")]
pub fn capture_selected_text_windows(target_hwnd: isize, hotkey_vk: u16) -> Option<String> {
    use std::thread::sleep;
    use std::time::{Duration, Instant};
    use windows_sys::Win32::{
        System::DataExchange::GetClipboardSequenceNumber,
        UI::{
            Input::KeyboardAndMouse::{
                KEYEVENTF_KEYUP, SendInput, VK_CONTROL, VK_LMENU, VK_MENU, VK_RMENU, VK_SHIFT,
            },
            WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow},
        },
    };

    // 1. Chỉ kích hoạt nếu Slack/ứng dụng chưa phải là cửa sổ đang active (tránh Chromium mất vùng bôi đen)
    let current_fg = unsafe { GetForegroundWindow() as isize };
    if target_hwnd != 0 && target_hwnd != current_fg {
        unsafe { SetForegroundWindow(target_hwnd as _) };
        sleep(Duration::from_millis(30));
    }

    // 2. Chờ nhả phím Alt và phím nóng, giải phóng triệt để modifier
    wait_and_release_modifiers(if hotkey_vk != 0 { hotkey_vk } else { 0x5A });

    let initial_seq = unsafe { GetClipboardSequenceNumber() };

    // 3. Đảm bảo triệt để các phím Alt, Shift đã được nhả
    let releases = [
        make_key_input(VK_MENU, KEYEVENTF_KEYUP),
        make_key_input(VK_LMENU, KEYEVENTF_KEYUP),
        make_key_input(VK_RMENU, KEYEVENTF_KEYUP),
        make_key_input(VK_SHIFT, KEYEVENTF_KEYUP),
    ];
    unsafe {
        SendInput(
            releases.len() as u32,
            releases.as_ptr(),
            std::mem::size_of::<windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT>() as i32,
        );
    }
    sleep(Duration::from_millis(20));

    // 4. Gửi Ctrl + C: Nhấn Ctrl và C cùng đợt SendInput để tránh bị Slack hiểu nhầm là Alt+Ctrl+C (Code block)
    let press_ctrl_c = [
        make_key_input(VK_CONTROL, 0),
        make_key_input(0x43, 0), // 'C' down
    ];
    unsafe {
        SendInput(
            press_ctrl_c.len() as u32,
            press_ctrl_c.as_ptr(),
            std::mem::size_of::<windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT>() as i32,
        );
    }
    sleep(Duration::from_millis(35));

    let release_ctrl_c = [
        make_key_input(0x43, KEYEVENTF_KEYUP), // 'C' up
        make_key_input(VK_CONTROL, KEYEVENTF_KEYUP),
    ];
    unsafe {
        SendInput(
            release_ctrl_c.len() as u32,
            release_ctrl_c.as_ptr(),
            std::mem::size_of::<windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT>() as i32,
        );
    }

    // 5. Chờ ứng dụng đưa dữ liệu vào Clipboard (tối đa 800ms cho các ứng dụng Electron/Slack)
    let wait_start = Instant::now();
    let mut clipboard_changed = false;
    while wait_start.elapsed() < Duration::from_millis(800) {
        let current_seq = unsafe { GetClipboardSequenceNumber() };
        if current_seq != initial_seq {
            clipboard_changed = true;
            break;
        }
        sleep(Duration::from_millis(25));
    }

    if !clipboard_changed {
        // Kiểm tra dự phòng nếu clipboard đã có text
        if let Ok(mut cb) = arboard::Clipboard::new()
            && let Ok(t) = cb.get_text()
            && !t.trim().is_empty()
        {
            return Some(t);
        }
        return None;
    }
    sleep(Duration::from_millis(30));

    // 6. Đọc text từ Clipboard
    let mut clipboard = arboard::Clipboard::new().ok()?;
    let text = clipboard.get_text().ok()?;
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(target_os = "windows")]
pub fn paste_text_windows(text: &str, target_hwnd: isize) -> Result<(), String> {
    use std::thread::sleep;
    use std::time::Duration;
    use windows_sys::Win32::UI::{
        Input::KeyboardAndMouse::{
            KEYEVENTF_KEYUP, SendInput, VK_CONTROL, VK_LMENU, VK_MENU, VK_RMENU, VK_SHIFT,
        },
        WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow},
    };

    // 1. Đưa text đã sửa vào Clipboard
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())?;
    sleep(Duration::from_millis(35));

    // 2. Đảm bảo cửa sổ ứng dụng người dùng (Slack/Word/...) được focus mà không làm mất selection
    let current_fg = unsafe { GetForegroundWindow() as isize };
    if target_hwnd != 0 && target_hwnd != current_fg {
        unsafe { SetForegroundWindow(target_hwnd as _) };
        sleep(Duration::from_millis(30));
    }

    // 3. Giải phóng phím trước khi paste
    wait_and_release_modifiers(0);

    let releases = [
        make_key_input(VK_MENU, KEYEVENTF_KEYUP),
        make_key_input(VK_LMENU, KEYEVENTF_KEYUP),
        make_key_input(VK_RMENU, KEYEVENTF_KEYUP),
        make_key_input(VK_SHIFT, KEYEVENTF_KEYUP),
    ];
    unsafe {
        SendInput(
            releases.len() as u32,
            releases.as_ptr(),
            std::mem::size_of::<windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT>() as i32,
        );
    }
    sleep(Duration::from_millis(20));

    // 4. Gửi Ctrl + V theo nhóm đồng bộ
    let press_ctrl_v = [
        make_key_input(VK_CONTROL, 0),
        make_key_input(0x56, 0), // 'V' down
    ];
    unsafe {
        SendInput(
            press_ctrl_v.len() as u32,
            press_ctrl_v.as_ptr(),
            std::mem::size_of::<windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT>() as i32,
        );
    }
    sleep(Duration::from_millis(35));

    let release_ctrl_v = [
        make_key_input(0x56, KEYEVENTF_KEYUP), // 'V' up
        make_key_input(VK_CONTROL, KEYEVENTF_KEYUP),
    ];
    unsafe {
        SendInput(
            release_ctrl_v.len() as u32,
            release_ctrl_v.as_ptr(),
            std::mem::size_of::<windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT>() as i32,
        );
    }
    sleep(Duration::from_millis(40));

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn capture_selected_text_windows(_target_hwnd: isize, _hotkey_vk: u16) -> Option<String> {
    let mut clipboard = arboard::Clipboard::new().ok()?;
    clipboard.get_text().ok()
}

#[cfg(not(target_os = "windows"))]
pub fn paste_text_windows(text: &str, _target_hwnd: isize) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_text_offline() {
        let input = "anh muon mot he thong tu dong sua loi chinh ta va giu api va fps";
        let fixed = fix_text_offline(input);
        assert!(fixed.contains("hệ thống"));
        assert!(fixed.contains("tự động"));
        assert!(fixed.contains("chính tả"));
        assert!(fixed.contains("API"));
        assert!(fixed.contains("FPS"));
    }

    #[test]
    fn test_punctuation_clean() {
        let input = "chào bạn , đây là pentakuru .bạn có khỏe không ?";
        let cleaned = clean_punctuation(input);
        assert_eq!(cleaned, "chào bạn, đây là pentakuru. bạn có khỏe không?");
        let full = fix_text_offline(input);
        assert_eq!(full, "Chào bạn, đây là PentaKuru. Bạn có khỏe không?");
    }
}
