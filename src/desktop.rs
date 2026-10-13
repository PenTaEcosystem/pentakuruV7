//! Ensure the actual Windows window participates in alpha composition.
//! Transparent pixels in a renderer screenshot alone do not prove this.
#[cfg(target_os = "windows")]
pub fn enable_transparency(cc: &eframe::CreationContext<'_>) -> Result<(), String> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Graphics::{
        Dwm::{DWM_BB_BLURREGION, DWM_BB_ENABLE, DWM_BLURBEHIND, DwmEnableBlurBehindWindow},
        Gdi::{CreateRectRgn, DeleteObject},
    };
    let window = cc.winit_window().ok_or("Missing native window")?;
    let handle = window.window_handle().map_err(|e| e.to_string())?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("Missing Windows handle".into());
    };
    // An empty blur region enables alpha composition without blurring wallpaper.
    // SAFETY: the window and region handles stay valid throughout both calls;
    // the owned region is released after DWM consumes its configuration.
    unsafe {
        let region = CreateRectRgn(0, 0, -1, -1);
        if region.is_null() {
            return Err("Cannot create composition region".into());
        }
        let configuration = DWM_BLURBEHIND {
            dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
            fEnable: 1,
            hRgnBlur: region,
            fTransitionOnMaximized: 0,
        };
        let result = DwmEnableBlurBehindWindow(handle.hwnd.get() as _, &configuration);
        DeleteObject(region);
        if result < 0 {
            return Err(format!("DWM composition failed: {result:#x}"));
        }
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn enable_transparency(_: &eframe::CreationContext<'_>) -> Result<(), String> {
    // eframe configures native transparency on macOS and Linux.
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn cursor_position(frame: &eframe::Frame, pixels_per_point: f32) -> Option<eframe::egui::Pos2> {
    use windows_sys::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};
    let window = frame.winit_window()?;
    let origin = window.inner_position().ok()?;
    let mut point = POINT::default();
    // SAFETY: GetCursorPos writes into our valid POINT; it only reads cursor state.
    if unsafe { GetCursorPos(&mut point) } == 0 {
        return None;
    }
    Some(eframe::egui::Pos2::new(
        (point.x - origin.x) as f32 / pixels_per_point,
        (point.y - origin.y) as f32 / pixels_per_point,
    ))
}

#[cfg(not(target_os = "windows"))]
pub fn cursor_position(_: &eframe::Frame, _: f32) -> Option<eframe::egui::Pos2> {
    None
}
