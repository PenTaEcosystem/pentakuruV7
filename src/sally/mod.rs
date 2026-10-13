pub mod audio;
pub mod gemini_live;
pub mod launcher;
pub mod store;
pub mod timeline;

pub use launcher::{is_sally_running, open_or_focus_sally, terminate_sally};

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::time::Instant;

use audio::{AudioCaptureService, AudioChunk, AudioSource};
use gemini_live::{GeminiLiveEvent, run_gemini_live_session};
pub use timeline::TimelineEntry;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SallySettings {
    pub selected_speaker: String,
    pub selected_mic: String,
    pub target_lang: String,
    pub model: String,
    pub always_on_top: bool,
    pub split_ratio: f32,
}

impl Default for SallySettings {
    fn default() -> Self {
        Self {
            selected_speaker: String::new(),
            selected_mic: String::new(),
            target_lang: "Tiếng Việt".to_string(),
            model: "gemini-2.0-flash-exp".to_string(),
            always_on_top: true,
            split_ratio: 0.48,
        }
    }
}

pub struct SallyManager {
    pub is_active: bool,
    pub is_connected: bool,
    pub is_paused: bool,
    pub start_time: Option<Instant>,
    pub mic_level: f32,
    pub speaker_level: f32,
    pub entries: Vec<TimelineEntry>,
    pub current_turn_translated: String,
    pub status_text: String,
    pub error_text: Option<String>,

    audio_service: AudioCaptureService,
    stop_signal: Arc<AtomicBool>,
    event_rx: Option<Receiver<GeminiLiveEvent>>,
    tokio_handle: Option<std::thread::JoinHandle<()>>,
}

impl Default for SallyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SallyManager {
    pub fn new() -> Self {
        Self {
            is_active: false,
            is_connected: false,
            is_paused: false,
            start_time: None,
            mic_level: 0.0,
            speaker_level: 0.0,
            entries: Vec::new(),
            current_turn_translated: String::new(),
            status_text: "Sẵn sàng".to_string(),
            error_text: None,
            audio_service: AudioCaptureService::new(),
            stop_signal: Arc::new(AtomicBool::new(false)),
            event_rx: None,
            tokio_handle: None,
        }
    }

    pub fn start_session(
        &mut self,
        api_key: String,
        settings: &SallySettings,
    ) -> Result<(), String> {
        if self.is_active {
            return Ok(());
        }

        if api_key.trim().is_empty() {
            let err =
                "Vui lòng nhập Google Gemini API Key trong Cài đặt (Alt + Q) để dịch trực tiếp."
                    .to_string();
            self.error_text = Some(err.clone());
            return Err(err);
        }

        self.stop_signal = Arc::new(AtomicBool::new(false));
        let stop_gemini = self.stop_signal.clone();

        // Kênh truyền audio chunks
        let (audio_tx, audio_rx) = channel::<AudioChunk>();
        // Kênh truyền sự kiện Gemini Live về UI
        let (event_tx, event_rx) = channel::<GeminiLiveEvent>();
        self.event_rx = Some(event_rx);

        // Khởi động thu âm
        let mic_name = if settings.selected_mic.is_empty() {
            None
        } else {
            Some(settings.selected_mic.clone())
        };
        let spk_name = if settings.selected_speaker.is_empty() {
            None
        } else {
            Some(settings.selected_speaker.clone())
        };

        self.audio_service
            .start(mic_name, spk_name, audio_tx)
            .map_err(|e| format!("Không thể mở thiết bị âm thanh: {e}"))?;

        // Chuyển tiếp audio chunks vào tokio channel
        let (tokio_audio_tx, tokio_audio_rx) =
            tokio::sync::mpsc::channel::<(AudioSource, Vec<i16>)>(128);
        let stop_bridge = self.stop_signal.clone();

        std::thread::spawn(move || {
            while !stop_bridge.load(Ordering::SeqCst) {
                if let Ok(chunk) = audio_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                    let _ = tokio_audio_tx.blocking_send((chunk.source, chunk.samples));
                }
            }
        });

        // Khởi động WebSocket Client trong Tokio Runtime trên luồng nền
        let model = settings.model.clone();
        let target_lang = settings.target_lang.clone();

        let handle = std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = event_tx.send(GeminiLiveEvent::Error(format!(
                        "Không thể khởi tạo Tokio Runtime: {e}"
                    )));
                    return;
                }
            };

            rt.block_on(async {
                run_gemini_live_session(
                    api_key,
                    model,
                    target_lang,
                    tokio_audio_rx,
                    event_tx,
                    stop_gemini,
                )
                .await;
            });
        });

        self.tokio_handle = Some(handle);
        self.is_active = true;
        self.is_connected = false;
        self.is_paused = false;
        self.start_time = Some(Instant::now());
        self.status_text = "Đang kết nối Gemini Live...".to_string();
        self.error_text = None;

        Ok(())
    }

    pub fn toggle_pause(&mut self) {
        if !self.is_active {
            return;
        }
        self.is_paused = !self.is_paused;
        if self.is_paused {
            self.status_text = "Đã tạm dừng".to_string();
            self.mic_level = 0.0;
            self.speaker_level = 0.0;
        } else {
            self.status_text = "Trực tiếp".to_string();
        }
    }

    pub fn stop_session(&mut self) {
        if !self.is_active {
            return;
        }

        self.stop_signal.store(true, Ordering::SeqCst);
        self.audio_service.stop();

        // Chốt câu dịch còn đang dở dang
        if !self.current_turn_translated.trim().is_empty() {
            let entry = TimelineEntry::new(
                self.entries.len() + 1,
                timeline::current_timestamp_str(),
                "Hội thoại".to_string(),
                String::new(),
                self.current_turn_translated.trim().to_string(),
            );
            self.entries.push(entry);
            self.current_turn_translated.clear();
        }

        self.is_active = false;
        self.is_connected = false;
        self.is_paused = false;
        self.status_text = "Đã dừng".to_string();
        self.mic_level = 0.0;
        self.speaker_level = 0.0;

        if let Some(h) = self.tokio_handle.take() {
            let _ = h.join();
        }
    }

    pub fn poll(&mut self) {
        if self.is_paused {
            self.mic_level = 0.0;
            self.speaker_level = 0.0;
        } else {
            let (mic, spk) = self.audio_service.get_levels();
            self.mic_level = mic;
            self.speaker_level = spk;
        }

        let mut should_stop = false;
        if let Some(ref rx) = self.event_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    GeminiLiveEvent::Connected => {
                        self.is_connected = true;
                        self.status_text = "Trực tiếp".to_string();
                    }
                    GeminiLiveEvent::Translated(chunk) => {
                        self.current_turn_translated.push_str(&chunk);
                    }
                    GeminiLiveEvent::TurnFinished => {
                        if !self.current_turn_translated.trim().is_empty() {
                            let entry = TimelineEntry::new(
                                self.entries.len() + 1,
                                timeline::current_timestamp_str(),
                                "Hội thoại".to_string(),
                                String::new(),
                                self.current_turn_translated.trim().to_string(),
                            );
                            self.entries.push(entry);
                            self.current_turn_translated.clear();
                        }
                    }
                    GeminiLiveEvent::Error(err) => {
                        self.error_text = Some(err.clone());
                        self.status_text = format!("Lỗi: {err}");
                        should_stop = true;
                    }
                    _ => {}
                }
            }
        }
        if should_stop {
            self.stop_session();
        }
    }

    pub fn save_meeting(&self, notes_dir: &Path, target_lang: &str) -> Result<PathBuf, String> {
        store::save_meeting_to_notes(notes_dir, &self.entries, target_lang)
    }

    pub fn duration_str(&self) -> String {
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed().as_secs();
            let h = elapsed / 3600;
            let m = (elapsed % 3600) / 60;
            let s = elapsed % 60;
            if h > 0 {
                format!("{:02}:{:02}:{:02}", h, m, s)
            } else {
                format!("{:02}:{:02}", m, s)
            }
        } else {
            "00:00".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sally_zero_cost_initial_state() {
        let mgr = SallyManager::new();
        assert!(!mgr.is_active);
        assert!(!mgr.is_connected);
        assert!(!mgr.is_paused);
        assert_eq!(mgr.entries.len(), 0);
        assert_eq!(mgr.duration_str(), "00:00");
    }

    #[test]
    fn test_sally_save_meeting_markdown() {
        let temp_dir = std::env::temp_dir().join(format!(
            "sally_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut mgr = SallyManager::new();
        mgr.entries.push(TimelineEntry::new(
            1,
            "10:00:00".into(),
            "Khách mời".into(),
            "Hello world".into(),
            "Xin chào thế giới".into(),
        ));

        let saved_path = mgr
            .save_meeting(&temp_dir, "Tiếng Việt")
            .expect("Should save markdown");
        assert!(saved_path.exists());
        let content = std::fs::read_to_string(&saved_path).expect("Read markdown");
        assert!(content.contains("Xin chào thế giới"));
        assert!(content.contains("Hello world"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
