//! Optional Penta media and background process helpers.
use serde::{Deserialize, Serialize};
use std::{
    process::{Child, Command, Stdio},
    sync::{Arc, atomic::AtomicBool},
};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub music: bool,
    pub clip: bool,
}
#[derive(Default)]
pub struct State {
    pub status: String,
    pub seen_music: bool,
    pub seen_clip: bool,
    pub music_cancel: Arc<AtomicBool>,
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
pub fn background_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut command = Command::new(program);
    command.env_remove("PYTHONHOME").env_remove("PYTHONPATH");
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
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
