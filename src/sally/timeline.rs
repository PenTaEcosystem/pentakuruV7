use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub id: usize,
    pub timestamp: String,
    pub speaker: String,
    pub original: String,
    pub translated: String,
}

impl TimelineEntry {
    pub fn new(
        id: usize,
        timestamp: String,
        speaker: String,
        original: String,
        translated: String,
    ) -> Self {
        Self {
            id,
            timestamp,
            speaker,
            original,
            translated,
        }
    }
}

pub fn current_timestamp_str() -> String {
    #[cfg(target_os = "windows")]
    unsafe {
        let mut st = std::mem::zeroed();
        windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut st);
        format!("{:02}:{:02}:{:02}", st.wHour, st.wMinute, st.wSecond)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let d = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let s = d % 60;
        let m = (d / 60) % 60;
        let h = (d / 3600) % 24;
        format!("{:02}:{:02}:{:02}", h, m, s)
    }
}
