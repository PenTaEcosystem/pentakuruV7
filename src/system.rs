//! Small operating-system helpers shared by notes, search and the AI panel.
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::SystemTime;

/// Result of work running on a background thread. The UI polls it each frame;
/// the worker requests a repaint when done so idle windows still update.
pub struct Job<T> {
    rx: Receiver<T>,
}

impl<T: Send + 'static> Job<T> {
    pub fn spawn(ctx: &eframe::egui::Context, work: impl FnOnce() -> T + Send + 'static) -> Self {
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(work());
            ctx.request_repaint();
        });
        Self { rx }
    }

    /// `Ok(Some)` exactly once when the job finishes, `Err` if the worker died.
    fn poll(&mut self) -> Result<Option<T>, ()> {
        match self.rx.try_recv() {
            Ok(value) => Ok(Some(value)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(()),
        }
    }
}

/// Poll an optional job, clearing it when finished (or when its worker died).
pub fn take<T: Send + 'static>(slot: &mut Option<Job<T>>) -> Option<T> {
    match slot.as_mut()?.poll() {
        Ok(None) => None,
        Ok(Some(value)) => {
            *slot = None;
            Some(value)
        }
        Err(()) => {
            *slot = None;
            None
        }
    }
}

/// Name of the folder created next to the executable for notes and uploads.
pub const DATA_FOLDER: &str = "Penteye Notes";

/// `<exe folder>/Penteye Notes`, or `~/Documents/Penteye Notes` when the
/// executable lives somewhere read-only (for example Program Files).
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join(DATA_FOLDER)))
        && std::fs::create_dir_all(&dir).is_ok()
        && writable(&dir)
    {
        return dir;
    }
    let fallback = home_dir().join("Documents").join(DATA_FOLDER);
    let _ = std::fs::create_dir_all(&fallback);
    fallback
}

/// Tên thư mục chứa chức năng UI-TARS cạnh file thực thi exe.
pub const UI_TARS_FOLDER: &str = "ui_tars";

/// Đường dẫn thư mục chức năng UI-TARS (`<exe folder>/ui_tars` hoặc `~/Documents/PentaKuru/ui_tars`).
pub fn ui_tars_dir() -> PathBuf {
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join(UI_TARS_FOLDER)))
        && std::fs::create_dir_all(&dir).is_ok()
        && writable(&dir)
    {
        return dir;
    }
    let fallback = home_dir()
        .join("Documents")
        .join("Penteye")
        .join(UI_TARS_FOLDER);
    let _ = std::fs::create_dir_all(&fallback);
    fallback
}

/// Tự động cài đặt & khởi tạo cấu trúc thư mục chức năng UI-TARS.
/// Đảm bảo khi người dùng mở exe, thư mục chức năng ui_tars sẽ xuất hiện sẵn
/// kèm file hướng dẫn sử dụng chi tiết và các thư mục cần thiết.
pub fn ensure_ui_tars_environment() -> Result<PathBuf, String> {
    let base = ui_tars_dir();
    let screenshots = base.join("screenshots");
    let logs = base.join("logs");
    let _ = std::fs::create_dir_all(&screenshots);
    let _ = std::fs::create_dir_all(&logs);

    let readme = base.join("Huong_dan_su_dung_UI_TARS.txt");
    if !readme.exists() {
        let content = "======================================================================\r\n\
PENTAKURU UI-TARS AGENT - HƯỚNG DẪN SỬ DỤNG VÀ THIẾT LẬP\r\n\
======================================================================\r\n\r\n\
1. TỔNG QUAN:\r\n\
   UI-TARS là trợ lý thị giác AI tự động quan sát màn hình và điều khiển\r\n\
   chuột/bàn phím để thực hiện các công việc theo yêu cầu của bạn.\r\n\r\n\
2. BẮT ĐẦU SỬ DỤNG SIÊU NHANH:\r\n\
   - Bước 1: Mở PentaKuru, nhấn [Alt + Q] để vào Cài đặt.\r\n\
   - Bước 2: Dán Google Gemini API Key vào mục 'Cấu hình Trợ lý AI'.\r\n\
     (Key này được dùng chung cho toàn bộ Chat, Sửa chính tả và UI Agent).\r\n\
   - Bước 3: Nhấn [Alt + A] để mở khung chat PentaKuru.\r\n\
   - Bước 4: Nhập câu lệnh bắt đầu bằng @ui kèm theo mục tiêu.\r\n\
     Ví dụ:\r\n\
     @ui mở Chrome tìm kiếm youtube\r\n\
     @ui click vào biểu tượng máy tính trên desktop\r\n\
     @ui gõ văn bản xin chào vào ô đang chọn\r\n\r\n\
3. PHÍM DỪNG KHẨN CẤP (EMERGENCY STOP):\r\n\
   Bất kỳ lúc nào bạn muốn dừng tác vụ của AI, chỉ cần nhấn phím [ESC]\r\n\
   hoặc click nút đỏ [🛑 DỪNG LẠI] trên thanh tiến trình.\r\n\r\n\
4. CẤU HÌNH MÁY CHỦ RIÊNG (NÂNG CAO):\r\n\
   Nếu muốn kết nối tới máy chủ ByteDance UI-TARS ngoài (Port 9099 hoặc SiliconFlow),\r\n\
   bạn có thể mở Cài đặt -> UI-TARS -> mở mục 'Máy chủ riêng'.\r\n\
======================================================================\r\n";
        let _ = std::fs::write(readme, content);
    }

    let config_path = base.join("config.json");
    if !config_path.exists() {
        let default_config = r#"{
  "version": "1.0",
  "default_engine": "gemini",
  "auto_screenshot": true,
  "delay_between_steps_ms": 1500
}"#;
        let _ = std::fs::write(config_path, default_config);
    }

    Ok(base)
}

fn writable(dir: &Path) -> bool {
    let probe = dir.join(".pentakuru-write-test");
    let ok = std::fs::write(&probe, b"ok").is_ok();
    let _ = std::fs::remove_file(probe);
    ok
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Open a file, folder, application shortcut or URL with the default handler.
pub fn open(target: &str) -> Result<(), String> {
    platform::open(target)
}

/// Show a file selected inside its parent folder.
pub fn reveal(path: &Path) -> Result<(), String> {
    platform::reveal(path)
}

/// Native multi-select file picker. Blocking: call from a worker thread.
pub fn pick_files() -> Vec<PathBuf> {
    platform::pick_files()
}

/// Local calendar time `(year, month, day, hour, minute)`.
pub fn local_parts(time: SystemTime) -> (i32, u32, u32, u32, u32) {
    platform::local_parts(time)
}

/// Bring an OS window with the given exact title to front and focus it.
pub fn focus_window_by_title(title: &str) {
    platform::focus_window_by_title(title);
}

/// Áp dụng Immersive Dark Mode và Alpha Transparency cho cửa sổ OS để tránh chớp trắng.
pub fn apply_window_dark_and_transparency(title: &str) {
    platform::apply_window_dark_and_transparency(title);
}

/// Check if PentaKuru is registered to start automatically with Windows.
pub fn is_autostart_enabled() -> bool {
    platform::is_autostart_enabled()
}

/// Enable or disable PentaKuru starting automatically with Windows via registry.
pub fn set_autostart(enable: bool) -> Result<(), String> {
    platform::set_autostart(enable)
}

/// Chụp toàn bộ màn hình desktop thành dữ liệu ảnh PNG (dành cho UI-TARS và Vision AI).
pub fn remember_external_foreground() {
    platform::remember_external_foreground();
}

pub fn check_window_state() -> Result<String, String> {
    platform::check_window_state()
}

pub fn capture_screen_png() -> Result<Vec<u8>, String> {
    platform::capture_screen_png()
}

/// Chụp màn hình và lưu ra file tạm trong thư mục ui_tars/screenshots.
pub fn capture_screen_to_temp_file() -> Result<PathBuf, String> {
    let png_bytes = capture_screen_png()?;
    let dir = ui_tars_dir().join("screenshots");
    let _ = std::fs::create_dir_all(&dir);
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let file_path = dir.join(format!("Screen_{now}.png"));
    std::fs::write(&file_path, &png_bytes)
        .map_err(|e| format!("Không thể lưu ảnh chụp màn hình: {e}"))?;
    Ok(file_path)
}

/// Thực hiện click chuột tại tọa độ (x, y) trên màn hình Windows.
pub fn execute_click(x: i32, y: i32) -> Result<(), String> {
    platform::execute_click(x, y)
}

pub fn execute_double_click(x: i32, y: i32) -> Result<(), String> {
    platform::execute_double_click(x, y)
}

pub fn execute_right_click(x: i32, y: i32) -> Result<(), String> {
    platform::execute_right_click(x, y)
}

pub fn execute_type_text(text: &str) -> Result<(), String> {
    platform::execute_type_text(text)
}

pub fn execute_key_press(key: &str) -> Result<(), String> {
    platform::execute_key_press(key)
}

pub fn execute_scroll(amount: i32) -> Result<(), String> {
    platform::execute_scroll(amount)
}

/// Called on a worker: OS input helpers include delays and clipboard access.
pub fn execute_ui_action(action: &crate::ai::UiAgentAction) -> Result<(), String> {
    use crate::ai::UiAgentAction;
    match action {
        UiAgentAction::Click { x, y } => execute_click(*x, *y),
        UiAgentAction::DoubleClick { x, y } => execute_double_click(*x, *y),
        UiAgentAction::RightClick { x, y } => execute_right_click(*x, *y),
        UiAgentAction::Type { text } => execute_type_text(text),
        UiAgentAction::KeyPress { key } => execute_key_press(key),
        UiAgentAction::Scroll { amount } => execute_scroll(*amount),
        UiAgentAction::Finished { .. } => Ok(()),
        UiAgentAction::Unknown { .. } => {
            Err("AI trả về thao tác không được hỗ trợ. Hãy thử lại.".into())
        }
    }
}

/// Terminate any existing background instance of PentaKuru to guarantee a clean launch.
pub fn take_over_single_instance() {
    platform::take_over_single_instance();
}

/// Vietnamese style `dd/mm/yyyy HH:MM`.
pub fn format_date(time: SystemTime) -> String {
    let (y, mo, d, h, mi) = local_parts(time);
    format!("{d:02}/{mo:02}/{y} {h:02}:{mi:02}")
}

pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

/// Convert civil UTC seconds to calendar parts (used by the Unix fallback and tests).
#[allow(dead_code)]
pub fn civil_from_unix(seconds: i64) -> (i32, u32, u32, u32, u32) {
    let days = seconds.div_euclid(86_400);
    let rem = seconds.rem_euclid(86_400);
    // Howard Hinnant's days-from-civil inverse.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = (yoe + era * 400 + i64::from(m <= 2)) as i32;
    (y, m, d, (rem / 3600) as u32, ((rem % 3600) / 60) as u32)
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GWL_EXSTYLE, GetCursorPos, GetForegroundWindow, GetWindowDisplayAffinity,
        GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, IsWindow,
        SetWindowDisplayAffinity, SetWindowLongPtrW, WDA_EXCLUDEFROMCAPTURE, WS_EX_LAYERED,
        WS_EX_TRANSPARENT,
    };
    use windows_sys::Win32::{
        Foundation::{FILETIME, SYSTEMTIME},
        System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime},
        UI::{
            Controls::Dialogs::{
                GetOpenFileNameW, OFN_ALLOWMULTISELECT, OFN_EXPLORER, OFN_FILEMUSTEXIST,
                OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW,
            },
            Shell::ShellExecuteW,
            WindowsAndMessaging::{
                BringWindowToTop, FindWindowW, SW_SHOWNORMAL, SetForegroundWindow,
            },
        },
    };

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    pub fn wide(text: &str) -> Vec<u16> {
        std::ffi::OsStr::new(text)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn shell(file: &str, params: Option<&str>) -> Result<(), String> {
        let operation = wide("open");
        let file = wide(file);
        let params = params.map(wide);
        // SAFETY: all strings are NUL-terminated buffers that outlive the call.
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                file.as_ptr(),
                params.as_ref().map_or(std::ptr::null(), |p| p.as_ptr()),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        } as isize;
        if result > 32 {
            Ok(())
        } else {
            Err(format!("Không mở được (mã {result})"))
        }
    }

    pub fn open(target: &str) -> Result<(), String> {
        shell(target, None)
    }

    pub fn reveal(path: &Path) -> Result<(), String> {
        shell(
            "explorer.exe",
            Some(&format!("/select,\"{}\"", path.display())),
        )
    }

    pub fn pick_files() -> Vec<PathBuf> {
        let mut buffer = vec![0u16; 64 * 1024];
        let filter = wide("Tất cả file\0*.*\0");
        let title = wide("Chọn file để tải lên ghi chú");
        // SAFETY: OPENFILENAMEW is a plain C struct; zero is a valid initial state.
        let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = buffer.as_mut_ptr();
        ofn.nMaxFile = buffer.len() as u32;
        ofn.lpstrTitle = title.as_ptr();
        ofn.Flags = OFN_ALLOWMULTISELECT
            | OFN_EXPLORER
            | OFN_FILEMUSTEXIST
            | OFN_PATHMUSTEXIST
            | OFN_NOCHANGEDIR;
        // SAFETY: every pointer in `ofn` refers to buffers alive for this call.
        if unsafe { GetOpenFileNameW(&mut ofn) } == 0 {
            return Vec::new();
        }
        let parts: Vec<String> = buffer
            .split(|&c| c == 0)
            .take_while(|part| !part.is_empty())
            .map(String::from_utf16_lossy)
            .collect();
        match parts.as_slice() {
            [] => Vec::new(),
            [single] => vec![PathBuf::from(single)],
            [dir, files @ ..] => files.iter().map(|f| Path::new(dir).join(f)).collect(),
        }
    }

    pub fn local_parts(time: SystemTime) -> (i32, u32, u32, u32, u32) {
        let unix = time
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let ticks = ((unix + 11_644_473_600) as u64) * 10_000_000;
        let file_time = FILETIME {
            dwLowDateTime: ticks as u32,
            dwHighDateTime: (ticks >> 32) as u32,
        };
        let mut utc = SYSTEMTIME::default();
        let mut local = SYSTEMTIME::default();
        // SAFETY: inputs and outputs are valid stack values owned by this function.
        let ok = unsafe {
            FileTimeToSystemTime(&file_time, &mut utc) != 0
                && SystemTimeToTzSpecificLocalTime(std::ptr::null(), &utc, &mut local) != 0
        };
        if !ok {
            return civil_from_unix(unix);
        }
        (
            i32::from(local.wYear),
            u32::from(local.wMonth),
            u32::from(local.wDay),
            u32::from(local.wHour),
            u32::from(local.wMinute),
        )
    }

    pub fn focus_window_by_title(title: &str) {
        let title_w = wide(title);
        unsafe {
            let hwnd = FindWindowW(std::ptr::null(), title_w.as_ptr());
            if !hwnd.is_null() {
                BringWindowToTop(hwnd);
                SetForegroundWindow(hwnd);
            }
        }
    }

    pub fn apply_to_hwnd(hwnd: windows_sys::Win32::Foundation::HWND) {
        use std::collections::HashSet;
        use std::sync::Mutex;
        use windows_sys::Win32::Graphics::{
            Dwm::{
                DWM_BB_BLURREGION, DWM_BB_ENABLE, DWM_BLURBEHIND, DWMWA_USE_IMMERSIVE_DARK_MODE,
                DwmEnableBlurBehindWindow, DwmSetWindowAttribute,
            },
            Gdi::{CreateRectRgn, DeleteObject},
        };

        static APPLIED_HWNDS: Mutex<Option<HashSet<isize>>> = Mutex::new(None);

        if hwnd.is_null() {
            return;
        }

        if let Ok(mut guard) = APPLIED_HWNDS.lock() {
            let set = guard.get_or_insert_with(HashSet::new);
            if set.contains(&(hwnd as isize)) {
                return;
            }
            set.insert(hwnd as isize);
        }

        unsafe {
            // 1. DWMWA_USE_IMMERSIVE_DARK_MODE: 20 (Win11/Win10 mới), 19 (Win10 cũ)
            let dark_mode: i32 = 1;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
                &dark_mode as *const _ as _,
                std::mem::size_of::<i32>() as u32,
            );
            let _ = DwmSetWindowAttribute(
                hwnd,
                19,
                &dark_mode as *const _ as _,
                std::mem::size_of::<i32>() as u32,
            );

            // 2. Ép viền cửa sổ và thanh tiêu đề thành màu đen tuyền để triệt tiêu chớp viền trắng
            let dark_color: u32 = 0x001A1A1A;
            let _ = DwmSetWindowAttribute(
                hwnd,
                34, /* DWMWA_BORDER_COLOR */
                &dark_color as *const _ as _,
                std::mem::size_of::<u32>() as u32,
            );
            let _ = DwmSetWindowAttribute(
                hwnd,
                35, /* DWMWA_CAPTION_COLOR */
                &dark_color as *const _ as _,
                std::mem::size_of::<u32>() as u32,
            );

            // 3. Áp dụng Alpha Composition để cửa sổ trong suốt ngay từ frame đầu tiên
            let region = CreateRectRgn(0, 0, -1, -1);
            if !region.is_null() {
                let configuration = DWM_BLURBEHIND {
                    dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
                    fEnable: 1,
                    hRgnBlur: region,
                    fTransitionOnMaximized: 0,
                };
                let _ = DwmEnableBlurBehindWindow(hwnd, &configuration);
                DeleteObject(region);
            }
        }
    }

    pub fn apply_window_dark_and_transparency(title: &str) {
        let title_w = wide(title);
        unsafe {
            let hwnd = FindWindowW(std::ptr::null(), title_w.as_ptr());
            if !hwnd.is_null() {
                apply_to_hwnd(hwnd);
            }
        }
    }

    pub fn is_autostart_enabled() -> bool {
        let output = std::process::Command::new("reg")
            .args([
                "query",
                "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                "/v",
                "Penteye",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        output.is_ok_and(|o| o.status.success())
    }

    pub fn set_autostart(enable: bool) -> Result<(), String> {
        let Ok(exe) = std::env::current_exe() else {
            return Err("Không xác định được đường dẫn tệp thực thi".into());
        };
        let exe_str = exe.to_string_lossy().to_string();
        if enable {
            let output = std::process::Command::new("reg")
                .args([
                    "add",
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                    "/v",
                    "Penteye",
                    "/t",
                    "REG_SZ",
                    "/d",
                    &format!("\"{}\"", exe_str),
                    "/f",
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| e.to_string())?;
            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        } else {
            let output = std::process::Command::new("reg")
                .args([
                    "delete",
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                    "/v",
                    "Penteye",
                    "/f",
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| e.to_string())?;
            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }
    }

    pub fn take_over_single_instance() {
        let current_pid = std::process::id();
        let _ = std::process::Command::new("taskkill")
            .args([
                "/F",
                "/FI",
                &format!("PID ne {current_pid}"),
                "/IM",
                "PentaKuru-Studio-Motion.exe",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        let _ = std::process::Command::new("taskkill")
            .args([
                "/F",
                "/FI",
                &format!("PID ne {current_pid}"),
                "/IM",
                "pentakuru.exe",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        std::thread::sleep(std::time::Duration::from_millis(80));
    }

    unsafe extern "system" fn enum_win_proc(
        hwnd: windows_sys::Win32::Foundation::HWND,
        lparam: windows_sys::Win32::Foundation::LPARAM,
    ) -> i32 {
        unsafe {
            let list = &mut *(lparam as *mut Vec<windows_sys::Win32::Foundation::HWND>);
            let mut pid = 0u32;
            windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, &mut pid);
            let my_pid = windows_sys::Win32::System::Threading::GetCurrentProcessId();
            if pid == my_pid
                && windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) != 0
            {
                // winit also owns a visible, layered drag/drop helper window.
                // Touch only the application's real windows, not framework helpers.
                let mut title = [0u16; 256];
                let length = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowTextW(
                    hwnd,
                    title.as_mut_ptr(),
                    title.len() as i32,
                );
                if length > 0
                    && String::from_utf16_lossy(&title[..length as usize]).starts_with("PentaKuru")
                {
                    list.push(hwnd);
                }
            }
        }
        1
    }

    fn visible_app_windows() -> Vec<windows_sys::Win32::Foundation::HWND> {
        let mut list = Vec::new();
        unsafe {
            EnumWindows(Some(enum_win_proc), &mut list as *mut _ as isize);
        }
        list
    }

    static LAST_EXTERNAL_WINDOW: std::sync::atomic::AtomicIsize =
        std::sync::atomic::AtomicIsize::new(0);

    pub fn remember_external_foreground() {
        use std::sync::atomic::Ordering;
        unsafe {
            let hwnd = GetForegroundWindow();
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if !hwnd.is_null() && pid != std::process::id() {
                LAST_EXTERNAL_WINDOW.store(hwnd as isize, Ordering::Relaxed);
            }
        }
    }

    fn focus_external_target() -> Result<(), String> {
        remember_external_foreground();
        unsafe {
            let hwnd = GetForegroundWindow();
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if !hwnd.is_null() && pid != std::process::id() {
                return Ok(());
            }
            let target = LAST_EXTERNAL_WINDOW.load(std::sync::atomic::Ordering::Relaxed)
                as windows_sys::Win32::Foundation::HWND;
            let mut target_pid = 0;
            GetWindowThreadProcessId(target, &mut target_pid);
            if IsWindow(target) == 0
                || target_pid == std::process::id()
                || SetForegroundWindow(target) == 0
            {
                return Err(
                    "Chọn cửa sổ cần điều khiển trước khi yêu cầu AI gõ hoặc nhấn phím.".into(),
                );
            }
        }
        Ok(())
    }

    struct CaptureExclusion {
        windows: Vec<(windows_sys::Win32::Foundation::HWND, u32)>,
    }

    impl CaptureExclusion {
        fn new() -> Result<Self, String> {
            let mut guard = Self {
                windows: Vec::new(),
            };
            for hwnd in visible_app_windows() {
                unsafe {
                    let mut affinity = 0;
                    // GetWindowDisplayAffinity can fail for non-layered windows.
                    // Our eframe windows start unrestricted; setting affinity is
                    // the authoritative capability check.
                    let _ = GetWindowDisplayAffinity(hwnd, &mut affinity);
                    if SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE) == 0 {
                        return Err(format!(
                            "Capture exclusion failed: Windows error {}, style {:#x}",
                            windows_sys::Win32::Foundation::GetLastError(),
                            GetWindowLongPtrW(hwnd, GWL_EXSTYLE)
                        ));
                    }
                    guard.windows.push((hwnd, affinity));
                }
            }
            // Wait for the compositor, without hiding or activating any window.
            unsafe {
                windows_sys::Win32::Graphics::Dwm::DwmFlush();
            }
            Ok(guard)
        }
    }

    impl Drop for CaptureExclusion {
        fn drop(&mut self) {
            for &(hwnd, affinity) in &self.windows {
                unsafe {
                    SetWindowDisplayAffinity(hwnd, affinity);
                }
            }
        }
    }

    struct InputPassthrough {
        windows: Vec<(windows_sys::Win32::Foundation::HWND, isize)>,
    }

    impl InputPassthrough {
        fn at(x: i32, y: i32) -> Result<Self, String> {
            let mut guard = Self {
                windows: Vec::new(),
            };
            for hwnd in visible_app_windows() {
                unsafe {
                    let mut rect = windows_sys::Win32::Foundation::RECT::default();
                    if GetWindowRect(hwnd, &mut rect) == 0
                        || x < rect.left
                        || x >= rect.right
                        || y < rect.top
                        || y >= rect.bottom
                    {
                        continue;
                    }
                    let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                    let bits = (WS_EX_TRANSPARENT | WS_EX_LAYERED) as isize;
                    windows_sys::Win32::Foundation::SetLastError(0);
                    if SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | bits) == 0
                        && windows_sys::Win32::Foundation::GetLastError() != 0
                    {
                        return Err("Không thể chuyển thao tác chuột tới cửa sổ bên dưới.".into());
                    }
                    guard.windows.push((hwnd, style & bits));
                }
            }
            Ok(guard)
        }
    }

    impl Drop for InputPassthrough {
        fn drop(&mut self) {
            let bits = (WS_EX_TRANSPARENT | WS_EX_LAYERED) as isize;
            for &(hwnd, original_bits) in &self.windows {
                unsafe {
                    let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (current & !bits) | original_bits);
                }
            }
        }
    }

    // Native smoke check: no desktop clicks/typing and no screen image is saved.
    pub fn check_window_state() -> Result<String, String> {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;
        use windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible;
        let windows = visible_app_windows();
        if windows.len() < 2 {
            return Err("Expected widget and chat windows".into());
        }
        let snapshots: Vec<_> = windows
            .iter()
            .map(|&hwnd| unsafe {
                let mut rect = windows_sys::Win32::Foundation::RECT::default();
                GetWindowRect(hwnd, &mut rect);
                (hwnd, GetWindowLongPtrW(hwnd, GWL_EXSTYLE), rect)
            })
            .collect();
        for _ in 0..6 {
            // Exercise the actual capture path, which used to hide all windows.
            capture_screen_png()?;
            for &(hwnd, style, rect) in &snapshots {
                let passthrough = InputPassthrough::at(
                    (rect.left + rect.right) / 2,
                    (rect.top + rect.bottom) / 2,
                )?;
                unsafe {
                    if IsWindowVisible(hwnd) == 0 {
                        return Err("Window disappeared during passthrough".into());
                    }
                }
                drop(passthrough);
                unsafe {
                    let mut current = windows_sys::Win32::Foundation::RECT::default();
                    GetWindowRect(hwnd, &mut current);
                    if IsWindowVisible(hwnd) == 0
                        || IsWindowEnabled(hwnd) == 0
                        || GetWindowLongPtrW(hwnd, GWL_EXSTYLE) != style
                        || (current.left, current.top, current.right, current.bottom)
                            != (rect.left, rect.top, rect.right, rect.bottom)
                    {
                        return Err(
                            "Window visibility, input style or position was not restored".into(),
                        );
                    }
                }
            }
        }
        Ok(format!(
            "Visible native windows: {}\nCapture cycles: 6\nCapture exclusion: OK\nMouse passthrough restored: OK\nWindow positions stable: OK\nWindows remain visible and enabled: OK\nNo desktop input was sent.\n",
            snapshots.len()
        ))
    }

    pub fn capture_screen_png() -> Result<Vec<u8>, String> {
        use windows_sys::Win32::Graphics::Gdi::*;
        use windows_sys::Win32::UI::WindowsAndMessaging::*;

        unsafe {
            static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
            let _capture_lock = CAPTURE_LOCK
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let exclusion = CaptureExclusion::new()?;

            let hdc_screen = GetDC(std::ptr::null_mut());
            if hdc_screen.is_null() {
                return Err("Không thể lấy context màn hình desktop".into());
            }

            let width = GetSystemMetrics(SM_CXSCREEN);
            let height = GetSystemMetrics(SM_CYSCREEN);

            if width <= 0 || height <= 0 {
                ReleaseDC(std::ptr::null_mut(), hdc_screen);
                return Err("Kích thước màn hình không hợp lệ".into());
            }

            let hdc_mem = CreateCompatibleDC(hdc_screen);
            if hdc_mem.is_null() {
                ReleaseDC(std::ptr::null_mut(), hdc_screen);
                return Err("Không thể tạo bộ đệm DC".into());
            }

            let hbitmap = CreateCompatibleBitmap(hdc_screen, width, height);
            if hbitmap.is_null() {
                DeleteDC(hdc_mem);
                ReleaseDC(std::ptr::null_mut(), hdc_screen);
                return Err("Không thể tạo Bitmap".into());
            }

            let old_bitmap = SelectObject(hdc_mem, hbitmap);
            BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, 0, 0, SRCCOPY);
            SelectObject(hdc_mem, old_bitmap);

            // All application windows stay visible throughout capture. Restore
            // their capture affinity immediately after copying the screen.
            drop(exclusion);

            let mut bi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height, // Top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [RGBQUAD {
                    rgbBlue: 0,
                    rgbGreen: 0,
                    rgbRed: 0,
                    rgbReserved: 0,
                }],
            };

            let mut raw_pixels = vec![0u8; (width * height * 4) as usize];
            let lines = GetDIBits(
                hdc_mem,
                hbitmap,
                0,
                height as u32,
                raw_pixels.as_mut_ptr() as _,
                &mut bi,
                DIB_RGB_COLORS,
            );

            DeleteObject(hbitmap);
            DeleteDC(hdc_mem);
            ReleaseDC(std::ptr::null_mut(), hdc_screen);

            if lines == 0 {
                return Err("Không thể đọc pixel màn hình".into());
            }

            // Đổi BGRA sang RGBA
            for chunk in raw_pixels.chunks_exact_mut(4) {
                let b = chunk[0];
                let r = chunk[2];
                chunk[0] = r;
                chunk[2] = b;
                chunk[3] = 255;
            }

            let mut png_bytes = Vec::new();
            let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
            use image::ImageEncoder;
            encoder
                .write_image(
                    &raw_pixels,
                    width as u32,
                    height as u32,
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|e| format!("Lỗi tạo ảnh PNG: {e}"))?;

            Ok(png_bytes)
        }
    }

    pub fn execute_click(x: i32, y: i32) -> Result<(), String> {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
        use windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos;

        let _passthrough = InputPassthrough::at(x, y)?;
        unsafe {
            SetCursorPos(x, y);
            std::thread::sleep(std::time::Duration::from_millis(40));
            mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
            std::thread::sleep(std::time::Duration::from_millis(40));
            mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        Ok(())
    }

    pub fn execute_double_click(x: i32, y: i32) -> Result<(), String> {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
        use windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos;

        let _passthrough = InputPassthrough::at(x, y)?;
        unsafe {
            SetCursorPos(x, y);
            std::thread::sleep(std::time::Duration::from_millis(40));
            mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
            std::thread::sleep(std::time::Duration::from_millis(40));
            mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
            std::thread::sleep(std::time::Duration::from_millis(70));
            mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
            std::thread::sleep(std::time::Duration::from_millis(40));
            mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        Ok(())
    }

    pub fn execute_right_click(x: i32, y: i32) -> Result<(), String> {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
        use windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos;

        let _passthrough = InputPassthrough::at(x, y)?;
        unsafe {
            SetCursorPos(x, y);
            std::thread::sleep(std::time::Duration::from_millis(40));
            mouse_event(MOUSEEVENTF_RIGHTDOWN, 0, 0, 0, 0);
            std::thread::sleep(std::time::Duration::from_millis(40));
            mouse_event(MOUSEEVENTF_RIGHTUP, 0, 0, 0, 0);
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        Ok(())
    }

    pub fn execute_type_text(text: &str) -> Result<(), String> {
        if text.is_empty() {
            return Ok(());
        }
        let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        clipboard.set_text(text).map_err(|e| e.to_string())?;

        focus_external_target()?;
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
        unsafe {
            std::thread::sleep(std::time::Duration::from_millis(30));
            keybd_event(VK_CONTROL as u8, 0, 0, 0);
            std::thread::sleep(std::time::Duration::from_millis(30));
            keybd_event(0x56 /* V */, 0, 0, 0);
            std::thread::sleep(std::time::Duration::from_millis(30));
            keybd_event(0x56 /* V */, 0, KEYEVENTF_KEYUP, 0);
            std::thread::sleep(std::time::Duration::from_millis(30));
            keybd_event(VK_CONTROL as u8, 0, KEYEVENTF_KEYUP, 0);
            std::thread::sleep(std::time::Duration::from_millis(40));
        }
        Ok(())
    }

    pub fn execute_key_press(key: &str) -> Result<(), String> {
        focus_external_target()?;
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
        let vk: u8 = match key.to_lowercase().trim() {
            "enter" | "return" => VK_RETURN as u8,
            "tab" => VK_TAB as u8,
            "escape" | "esc" => VK_ESCAPE as u8,
            "backspace" => VK_BACK as u8,
            "delete" | "del" => VK_DELETE as u8,
            "space" => VK_SPACE as u8,
            "up" => VK_UP as u8,
            "down" => VK_DOWN as u8,
            "left" => VK_LEFT as u8,
            "right" => VK_RIGHT as u8,
            "win" | "super" => VK_LWIN as u8,
            _ => return Ok(()),
        };
        unsafe {
            keybd_event(vk, 0, 0, 0);
            std::thread::sleep(std::time::Duration::from_millis(40));
            keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(())
    }

    pub fn execute_scroll(amount: i32) -> Result<(), String> {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
        let mut point = windows_sys::Win32::Foundation::POINT::default();
        unsafe {
            GetCursorPos(&mut point);
        }
        let _passthrough = InputPassthrough::at(point.x, point.y)?;
        unsafe {
            mouse_event(MOUSEEVENTF_WHEEL, 0, 0, amount * 120, 0);
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use super::*;
    use std::process::Command;

    pub fn focus_window_by_title(_title: &str) {}
    pub fn remember_external_foreground() {}
    pub fn check_window_state() -> Result<String, String> {
        Err("Native window check requires Windows".into())
    }
    pub fn is_autostart_enabled() -> bool {
        false
    }
    pub fn set_autostart(_enable: bool) -> Result<(), String> {
        Ok(())
    }
    pub fn take_over_single_instance() {}
    pub fn capture_screen_png() -> Result<Vec<u8>, String> {
        Err("Chưa hỗ trợ chụp màn hình trên OS này".into())
    }
    pub fn execute_click(_x: i32, _y: i32) -> Result<(), String> {
        Ok(())
    }
    pub fn execute_double_click(_x: i32, _y: i32) -> Result<(), String> {
        Ok(())
    }
    pub fn execute_right_click(_x: i32, _y: i32) -> Result<(), String> {
        Ok(())
    }
    pub fn execute_type_text(_text: &str) -> Result<(), String> {
        Ok(())
    }
    pub fn execute_key_press(_key: &str) -> Result<(), String> {
        Ok(())
    }
    pub fn execute_scroll(_amount: i32) -> Result<(), String> {
        Ok(())
    }

    fn opener() -> &'static str {
        if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        }
    }

    pub fn open(target: &str) -> Result<(), String> {
        Command::new(opener())
            .arg(target)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn reveal(path: &Path) -> Result<(), String> {
        if cfg!(target_os = "macos") {
            return Command::new("open")
                .arg("-R")
                .arg(path)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string());
        }
        let parent = path.parent().unwrap_or(path);
        open(&parent.to_string_lossy())
    }

    pub fn pick_files() -> Vec<PathBuf> {
        let output = if cfg!(target_os = "macos") {
            Command::new("osascript")
                .args([
                    "-e",
                    "set fs to choose file with multiple selections allowed",
                    "-e",
                    "set out to \"\"",
                    "-e",
                    "repeat with f in fs",
                    "-e",
                    "set out to out & POSIX path of f & linefeed",
                    "-e",
                    "end repeat",
                    "-e",
                    "out",
                ])
                .output()
        } else {
            Command::new("zenity")
                .args(["--file-selection", "--multiple", "--separator=\n"])
                .output()
        };
        output
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(PathBuf::from)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn local_parts(time: SystemTime) -> (i32, u32, u32, u32, u32) {
        let unix = time
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let seconds = unix as libc::time_t;
        let mut tm = std::mem::MaybeUninit::<libc::tm>::uninit();
        // SAFETY: localtime_r fills `tm`; it is read only after a non-null return.
        unsafe {
            if !libc::localtime_r(&seconds, tm.as_mut_ptr()).is_null() {
                let tm = tm.assume_init();
                return (
                    tm.tm_year + 1900,
                    (tm.tm_mon + 1) as u32,
                    tm.tm_mday as u32,
                    tm.tm_hour as u32,
                    tm.tm_min as u32,
                );
            }
        }
        civil_from_unix(unix)
    }

    pub fn apply_window_dark_and_transparency(_title: &str) {}
}

pub fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0xF) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

pub fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    fn decode_char(c: u8) -> Result<u8, String> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err("Ký tự base64 không hợp lệ".into()),
        }
    }
    let input = input.trim().as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < input.len() {
        let mut b = [0u8; 4];
        let mut count = 0;
        while i < input.len() && count < 4 {
            let c = input[i];
            i += 1;
            if c != b'\r' && c != b'\n' && c != b' ' {
                b[count] = c;
                count += 1;
            }
        }
        if count == 0 {
            break;
        }
        if count < 4 {
            return Err("Dữ liệu base64 không đầy đủ".into());
        }
        let b0 = decode_char(b[0])?;
        let b1 = decode_char(b[1])?;
        out.push((b0 << 2) | (b1 >> 4));
        if b[2] != b'=' {
            let b2 = decode_char(b[2])?;
            out.push(((b1 & 0xF) << 4) | (b2 >> 2));
            if b[3] != b'=' {
                let b3 = decode_char(b[3])?;
                out.push(((b2 & 0x3) << 6) | b3);
            }
        }
    }
    Ok(out)
}

pub fn load_png_color_image(bytes: &[u8]) -> Result<eframe::egui::ColorImage, String> {
    let dyn_img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .map_err(|e| format!("Lỗi giải mã ảnh PNG: {e}"))?;
    let rgba = dyn_img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();
    Ok(eframe::egui::ColorImage::from_rgba_unmultiplied(
        size, &pixels,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates_and_sizes() {
        assert_eq!(civil_from_unix(0), (1970, 1, 1, 0, 0));
        // 2026-10-03 16:24 UTC
        assert_eq!(civil_from_unix(1_791_044_640), (2026, 10, 3, 16, 24));
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(2048), "2.0 KB");
    }

    #[test]
    fn test_load_png_color_image() {
        let fake_png = capture_screen_png();
        if let Ok(bytes) = fake_png {
            let img = load_png_color_image(&bytes);
            assert!(img.is_ok());
        }
    }

    #[test]
    fn test_egui_image() {
        let color_image = eframe::egui::ColorImage::example();
        let _ = color_image.size;
    }
}
