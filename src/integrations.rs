//! Optional Penta media and background process helpers.
use serde::{Deserialize, Serialize};
use std::{
    process::Child,
    sync::{Arc, atomic::AtomicBool},
};

pub const DEFAULT_MUSIC_MCP_URL: &str = "https://music.pentasys.id.vn/api/mcp/call";
pub const DEFAULT_CLIP_MCP_URL: &str = "https://clip.pentasys.id.vn/mcp";

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub music: bool,
    pub clip: bool,
    pub music_mcp_url: String,
    pub music_mcp_key: String,
    pub clip_mcp_url: String,
    pub clip_mcp_key: String,
}

impl Settings {
    pub fn resolved_music_url(&self) -> &str {
        let u = self.music_mcp_url.trim();
        if u.is_empty() {
            DEFAULT_MUSIC_MCP_URL
        } else {
            u
        }
    }

    pub fn resolved_music_key(&self) -> String {
        let k = self.music_mcp_key.trim();
        if !k.is_empty() {
            return k.to_owned();
        }
        std::env::var("PENTAKURU_MUSIC_MCP_KEY")
            .unwrap_or_default()
            .trim()
            .to_owned()
    }

    pub fn resolved_clip_url(&self) -> &str {
        let u = self.clip_mcp_url.trim();
        if u.is_empty() {
            DEFAULT_CLIP_MCP_URL
        } else {
            u
        }
    }

    pub fn resolved_clip_key(&self) -> String {
        let k = self.clip_mcp_key.trim();
        if !k.is_empty() {
            return k.to_owned();
        }
        std::env::var("PENTAKURU_CLIP_MCP_KEY")
            .unwrap_or_default()
            .trim()
            .to_owned()
    }
}

#[derive(Default)]
pub struct State {
    pub status: String,
    pub seen_music: bool,
    pub seen_clip: bool,
    pub music_cancel: Arc<AtomicBool>,
    pub show_mcp_keys: bool,
}
pub fn music_script() -> String {
    include_str!("../assets/music-mini.js").to_owned()
}
pub fn music_open() -> Result<(), String> {
    crate::sally::launcher::open_web(
        "penta-music",
        "https://music.pentasys.id.vn/",
        "PentaKuru · Music",
        480.0,
        420.0,
        music_script(),
    )
}
pub fn clip_script() -> String {
    include_str!("../assets/clip-compact.js").to_owned()
}
pub fn clip_open() -> Result<(), String> {
    crate::sally::launcher::open_web(
        "penta-clip",
        "https://clip.pentasys.id.vn/feed/subscriptions",
        "PentaKuru · Clip",
        480.0,
        420.0,
        clip_script(),
    )
}
pub fn clip_open_pip(video_id: &str, title: &str) -> Result<(), String> {
    let clean_id: String = video_id
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if clean_id.is_empty() {
        return Err("Mã video không hợp lệ.".into());
    }
    let url = format!("https://clip.pentasys.id.vn/embed/{clean_id}?autoplay=1");
    let window_title = if title.trim().is_empty() {
        "PentaKuru · Clip PiP".to_owned()
    } else {
        format!("PentaKuru · {}", title.trim())
    };
    crate::sally::launcher::open_web(
        "penta-clip",
        &url,
        &window_title,
        384.0,
        216.0,
        clip_script(),
    )
}
#[cfg(windows)]
pub fn background_command(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    let mut command = Command::new(program);
    command.env_remove("PYTHONHOME").env_remove("PYTHONPATH");
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command.creation_flags(0x08000000);
    command
}
pub fn stop_child(child: &mut Child) {
    if child.try_wait().ok().flatten().is_some() {
        return;
    }
    #[cfg(windows)]
    {
        let _ = background_command("taskkill.exe")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = child.kill();
    }
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn music_uses_web_session_only() {
        let js = music_script();
        assert!(!js.contains("localStorage"));
        assert!(!js.contains("Bearer"));
        assert!(!js.contains("pentakuru-music-tools"));
    }
}
