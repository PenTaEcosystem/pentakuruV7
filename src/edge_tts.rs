//! Microsoft Edge Read Aloud TTS client (chunked WebSocket streaming),
//! curated voice catalog UI, and lightweight local work-rhythm analyzer.

#[cfg(windows)]
use crate::mcp_media::{AudioEngine, PlaybackState};
use crate::system;
use crate::ui_theme as theme;
use eframe::egui::{self, Color32, CornerRadius, Stroke, Vec2};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
#[cfg(any(windows, test))]
use std::collections::HashMap;
use std::collections::VecDeque;
#[cfg(any(windows, test))]
use std::io::{BufRead, BufReader, Write};
#[cfg(any(windows, test))]
use std::net::{TcpListener, TcpStream};
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;
use tokio::sync::mpsc as tokio_mpsc;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message as WsMessage, client::IntoClientRequest, http::HeaderValue},
};

pub const DEFAULT_VOICE_ID: &str = "fr-FR-VivienneMultilingualNeural";
const TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
const SEC_MS_GEC_VERSION: &str = "2-143.0.3650.75";
const EDGE_ORIGIN: &str = "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold";
const EDGE_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36 Edg/143.0.0.0";
const WIN_EPOCH_SECS: f64 = 11_644_473_600.0;

#[derive(Clone, Copy, Debug)]
pub struct VoiceOption {
    pub id: &'static str,
    pub name: &'static str,
    pub tag: &'static str,
    pub desc: &'static str,
    pub preview_text: &'static str,
}

/// Curated Microsoft Edge neural voices (Vietnamese native + Multilingual).
pub const EDGE_VOICES: &[VoiceOption] = &[
    VoiceOption {
        id: "fr-FR-VivienneMultilingualNeural",
        name: "Vivienne Multilingual",
        tag: "Đa ngôn ngữ · Thanh lịch",
        desc: "Giọng nữ nhẹ nhàng, tinh tế, phù hợp nội dung kể chuyện, trò chuyện và làm việc tập trung.",
        preview_text: "Xin chào anh, em là Vivienne. Chúc anh một phiên làm việc thật nhẹ nhàng và hiệu quả nhé.",
    },
    VoiceOption {
        id: "vi-VN-HoaiMyNeural",
        name: "Hoài Mỹ (HoaiMy)",
        tag: "Tiếng Việt · Nữ ấm áp",
        desc: "Giọng nữ tiếng Việt tự nhiên, phát âm tròn vành rõ chữ, đọc tiếng Việt hàng ngày rất mượt.",
        preview_text: "Chào anh, em là Hoài Mỹ. Đã đến lúc nghỉ mắt một chút rồi, anh nhớ uống nước nhé.",
    },
    VoiceOption {
        id: "vi-VN-NamMinhNeural",
        name: "Nam Minh (NamMinh)",
        tag: "Tiếng Việt · Nam trầm ấm",
        desc: "Giọng nam tiếng Việt rõ ràng, chững chạc, phù hợp nhắc việc và tóm tắt công việc nhanh.",
        preview_text: "Chào anh, mình là Nam Minh. Hiện có việc chưa xong trong lịch hôm nay, mình nhắc nhẹ để anh theo dõi.",
    },
    VoiceOption {
        id: "en-US-EmmaMultilingualNeural",
        name: "Emma Multilingual",
        tag: "Đa ngôn ngữ · Thân thiện",
        desc: "Giọng nữ mềm mại, gần gũi, chuyển đổi mượt giữa tiếng Việt và thuật ngữ tiếng Anh.",
        preview_text: "Chào anh, em là Emma. Khi anh làm việc lâu không mở nhạc, em sẽ nhắc nhẹ đúng nhịp nhé.",
    },
    VoiceOption {
        id: "en-US-AvaMultilingualNeural",
        name: "Ava Multilingual",
        tag: "Đa ngôn ngữ · Sáng rõ",
        desc: "Giọng nữ tươi tắn, phát âm sắc nét, giúp tỉnh táo khi xử lý công việc dài.",
        preview_text: "Xin chào, mình là Ava. Sẵn sàng đồng hành và đọc phản hồi từng chunk thật nhanh cho anh.",
    },
    VoiceOption {
        id: "en-US-AndrewMultilingualNeural",
        name: "Andrew Multilingual",
        tag: "Đa ngôn ngữ · Điềm tĩnh",
        desc: "Giọng nam trầm ổn, mạch lạc, tạo cảm giác tin cậy khi tổng hợp thông tin kỹ thuật.",
        preview_text: "Chào anh, mình là Andrew. Nhịp làm việc đang rất đều, nhưng hãy thả lỏng vai một chút nhé.",
    },
    VoiceOption {
        id: "en-US-BrianMultilingualNeural",
        name: "Brian Multilingual",
        tag: "Đa ngôn ngữ · Sâu lắng",
        desc: "Giọng nam ấm, nhịp vừa phải, dễ chịu khi làm việc muộn trong không gian yên tĩnh.",
        preview_text: "Chào anh, mình là Brian. Làm việc muộn rất dễ mỏi mắt, nghỉ ngơi hai phút rồi tiếp tục nhé.",
    },
    VoiceOption {
        id: "fr-FR-RemyMultilingualNeural",
        name: "Remy Multilingual",
        tag: "Đa ngôn ngữ · Thư giãn",
        desc: "Giọng nam tự nhiên, êm dịu, phù hợp đọc lời khuyên nghỉ ngơi và thư giãn cuối ngày.",
        preview_text: "Chào anh, mình là Remy. Không có nhạc nền cũng không sao, mình sẽ giữ nhịp nhẹ nhàng cùng anh.",
    },
    VoiceOption {
        id: "ko-KR-HyunsuMultilingualNeural",
        name: "Hyunsu Multilingual",
        tag: "Đa ngôn ngữ · Hiện đại",
        desc: "Giọng nam trẻ trung, gọn gàng, nói đa ngôn ngữ tự nhiên với tốc độ ổn định.",
        preview_text: "Xin chào anh, mình là Hyunsu. Hệ thống giọng đọc Edge đã sẵn sàng hỗ trợ anh.",
    },
];

pub fn find_voice(id: &str) -> &'static VoiceOption {
    EDGE_VOICES
        .iter()
        .find(|v| v.id == id)
        .unwrap_or(&EDGE_VOICES[0])
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct EdgeTtsSettings {
    pub enabled: bool,
    pub voice_id: String,
    pub chunked_streaming: bool,
    pub read_ai_replies: bool,
    pub proactive_work_rhythm: bool,
    pub rate_percent: i8,
    pub volume: f32,
    pub focus_alert_mins: u32,
}

impl Default for EdgeTtsSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            voice_id: DEFAULT_VOICE_ID.into(),
            chunked_streaming: true,
            read_ai_replies: true,
            proactive_work_rhythm: true,
            rate_percent: 0,
            volume: 0.9,
            focus_alert_mins: 45,
        }
    }
}

/// Persisted-friendly summary of the user's current work session rhythm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkRhythmSnapshot {
    pub active_streak_mins: u32,
    pub switches_per_10m: u32,
    pub recent_apps: Vec<String>,
    pub hour: u32,
    pub late_night: bool,
    pub intensity_label: &'static str,
}

/// Lightweight work-rhythm analyzer driven by OS idle & foreground window samples.
pub struct WorkRhythmTracker {
    active_streak_secs: u64,
    last_sample: Instant,
    last_window: String,
    switch_instants: VecDeque<Instant>,
    recent_apps: VecDeque<String>,
    last_spoken_alert: Option<Instant>,
}

impl Default for WorkRhythmTracker {
    fn default() -> Self {
        Self {
            active_streak_secs: 0,
            last_sample: Instant::now(),
            last_window: String::new(),
            switch_instants: VecDeque::new(),
            recent_apps: VecDeque::new(),
            last_spoken_alert: None,
        }
    }
}

impl WorkRhythmTracker {
    /// Updates work streak & window switch cadence with a new sample (`idle_secs`, optional app title).
    pub fn record_sample(&mut self, idle_secs: u32, fg_title: Option<&str>, elapsed_secs: u64) {
        let step = elapsed_secs.clamp(1, 120);
        if idle_secs >= 180 {
            self.active_streak_secs = 0;
            self.switch_instants.clear();
        } else if idle_secs < 90 {
            self.active_streak_secs = self.active_streak_secs.saturating_add(step);
        }

        let now = Instant::now();
        while self
            .switch_instants
            .front()
            .is_some_and(|t| now.duration_since(*t) > Duration::from_secs(600))
        {
            self.switch_instants.pop_front();
        }

        if let Some(raw) = fg_title.map(str::trim).filter(|s| !s.is_empty()) {
            let clean = compact_app_label(raw);
            if !clean.is_empty() && clean != self.last_window {
                if !self.last_window.is_empty() {
                    self.switch_instants.push_back(now);
                }
                self.last_window = clean.clone();
                if let Some(pos) = self.recent_apps.iter().position(|x| x == &clean) {
                    self.recent_apps.remove(pos);
                }
                self.recent_apps.push_back(clean);
                while self.recent_apps.len() > 5 {
                    self.recent_apps.pop_front();
                }
            }
        }
    }

    pub fn poll_os(&mut self) -> bool {
        let elapsed = self.last_sample.elapsed();
        if elapsed < Duration::from_secs(15) {
            return false;
        }
        self.last_sample = Instant::now();
        let idle = system::last_input_idle_secs();
        let fg = system::foreground_window_title();
        self.record_sample(idle, fg.as_deref(), elapsed.as_secs());
        true
    }

    pub fn snapshot(&self, hour: u32) -> WorkRhythmSnapshot {
        let active_mins = (self.active_streak_secs / 60) as u32;
        let switches = self.switch_instants.len() as u32;
        let late_night = !(5..22).contains(&hour);
        let intensity_label = if late_night && active_mins >= 25 {
            "Làm việc khuya kéo dài"
        } else if switches >= 12 {
            "Chuyển tác vụ nhanh (dễ căng thẳng)"
        } else {
            "Tập trung làm việc liên tục"
        };
        WorkRhythmSnapshot {
            active_streak_mins: active_mins,
            switches_per_10m: switches,
            recent_apps: self.recent_apps.iter().cloned().collect(),
            hour,
            late_night,
            intensity_label,
        }
    }

    pub fn should_check_in(&self, threshold_mins: u32, music_playing: bool) -> bool {
        if music_playing || threshold_mins == 0 {
            return false;
        }
        let active_mins = (self.active_streak_secs / 60) as u32;
        if active_mins < threshold_mins {
            return false;
        }
        let cooldown = Duration::from_secs(u64::from(threshold_mins.max(15)) * 60);
        self.last_spoken_alert
            .is_none_or(|t| t.elapsed() >= cooldown)
    }

    pub fn mark_alert_triggered(&mut self) {
        self.last_spoken_alert = Some(Instant::now());
    }

    #[cfg(test)]
    pub fn active_mins(&self) -> u32 {
        (self.active_streak_secs / 60) as u32
    }
}

fn compact_app_label(raw: &str) -> String {
    let tail = raw
        .rsplit(" - ")
        .next()
        .unwrap_or(raw)
        .rsplit(" — ")
        .next()
        .unwrap_or(raw)
        .trim();
    let chosen = if tail.len() >= 3 { tail } else { raw.trim() };
    chosen.chars().take(48).collect()
}

#[cfg(any(windows, test))]
type StreamSlot = Arc<Mutex<Option<mpsc::Receiver<Vec<u8>>>>>;

/// Manages chunked Edge TTS synthesis & low-latency playback across Windows, macOS, and Linux.
pub struct SpeechPlayer {
    #[cfg(windows)]
    pub engine: AudioEngine,
    pub status: Option<String>,
    pub previewing_voice: Option<String>,
    speech_gen: Arc<AtomicU64>,
    cancel_flag: Arc<AtomicBool>,
    #[cfg(windows)]
    server_port: Option<u16>,
    #[cfg(windows)]
    streams: Arc<Mutex<HashMap<u64, StreamSlot>>>,
    #[cfg(not(windows))]
    unix_speaking: Arc<AtomicBool>,
    #[cfg(not(windows))]
    unix_child: Arc<Mutex<Option<std::process::Child>>>,
    error_rx: Option<mpsc::Receiver<String>>,
}

impl Default for SpeechPlayer {
    fn default() -> Self {
        Self {
            #[cfg(windows)]
            engine: AudioEngine::default(),
            status: None,
            previewing_voice: None,
            speech_gen: Arc::new(AtomicU64::new(0)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            #[cfg(windows)]
            server_port: None,
            #[cfg(windows)]
            streams: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(not(windows))]
            unix_speaking: Arc::new(AtomicBool::new(false)),
            #[cfg(not(windows))]
            unix_child: Arc::new(Mutex::new(None)),
            error_rx: None,
        }
    }
}

impl SpeechPlayer {
    pub fn is_speaking(&self) -> bool {
        #[cfg(windows)]
        {
            matches!(
                self.engine.snapshot().state,
                PlaybackState::Playing | PlaybackState::Opening
            )
        }
        #[cfg(not(windows))]
        {
            self.unix_speaking.load(Ordering::SeqCst)
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        #[cfg(windows)]
        self.engine.set_volume(volume);
        #[cfg(not(windows))]
        let _ = volume;
    }

    pub fn poll(&mut self) {
        if let Some(rx) = &self.error_rx {
            while let Ok(err) = rx.try_recv() {
                if !err.trim().is_empty() {
                    self.status = Some(err);
                }
            }
        }
        if !self.is_speaking() {
            self.previewing_voice = None;
        }
    }

    pub fn stop(&mut self) {
        self.speech_gen.fetch_add(1, Ordering::SeqCst);
        self.cancel_flag.store(true, Ordering::SeqCst);
        self.previewing_voice = None;
        #[cfg(windows)]
        self.engine.stop_playback();
        #[cfg(not(windows))]
        {
            self.unix_speaking.store(false, Ordering::SeqCst);
            if let Ok(mut guard) = self.unix_child.lock()
                && let Some(mut child) = guard.take()
            {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    pub fn speak_with_voice(
        &mut self,
        ctx: &egui::Context,
        text: &str,
        settings: &EdgeTtsSettings,
        voice_id_override: Option<&str>,
        is_preview: bool,
    ) {
        let chunks = prepare_tts_chunks(text, settings.chunked_streaming);
        if chunks.is_empty() {
            return;
        }

        let speech_id = self.speech_gen.fetch_add(1, Ordering::SeqCst) + 1;
        self.cancel_flag.store(true, Ordering::SeqCst);
        let my_cancel = Arc::new(AtomicBool::new(false));
        self.cancel_flag = my_cancel.clone();

        let voice = voice_id_override
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(&settings.voice_id)
            .to_owned();
        if is_preview {
            self.previewing_voice = Some(voice.clone());
        } else {
            self.previewing_voice = None;
        }

        let (err_tx, err_rx) = mpsc::channel::<String>();
        self.error_rx = Some(err_rx);
        self.status = Some(format!(
            "Đang phát giọng {} ({} chunk)…",
            find_voice(&voice).name,
            chunks.len()
        ));

        let rate = settings.rate_percent;
        let chunked = settings.chunked_streaming;
        let ctx_bg = ctx.clone();

        #[cfg(windows)]
        {
            let port = match self.ensure_server() {
                Ok(p) => p,
                Err(err) => {
                    self.status = Some(err);
                    return;
                }
            };
            let (chunk_tx, chunk_rx) = mpsc::channel::<Vec<u8>>();
            if let Ok(mut map) = self.streams.lock() {
                map.retain(|k, _| *k + 4 >= speech_id);
                map.insert(speech_id, Arc::new(Mutex::new(Some(chunk_rx))));
            }
            let url = format!("http://127.0.0.1:{port}/tts/{speech_id}.mp3");
            if let Err(err) = self.engine.play(&url, settings.volume, ctx) {
                self.status = Some(err);
                return;
            }
            std::thread::spawn(move || {
                let rt = shared_runtime();
                let res = rt.block_on(synthesize_chunks_pipelined(
                    chunks, voice, rate, chunked, my_cancel, chunk_tx,
                ));
                if let Err(err) = res {
                    let _ = err_tx.send(err);
                }
                ctx_bg.request_repaint();
            });
        }

        #[cfg(not(windows))]
        {
            if let Ok(mut guard) = self.unix_child.lock()
                && let Some(mut child) = guard.take()
            {
                let _ = child.kill();
                let _ = child.wait();
            }
            let speaking = self.unix_speaking.clone();
            let child_slot = self.unix_child.clone();
            let volume = settings.volume;
            speaking.store(true, Ordering::SeqCst);
            std::thread::spawn(move || {
                let rt = shared_runtime();
                let res = rt.block_on(play_chunks_unix(
                    chunks,
                    voice,
                    rate,
                    chunked,
                    volume,
                    speech_id,
                    my_cancel.clone(),
                    child_slot.clone(),
                ));
                if let Err(err) = res {
                    let _ = err_tx.send(err);
                }
                if !my_cancel.load(Ordering::SeqCst) {
                    speaking.store(false, Ordering::SeqCst);
                }
                ctx_bg.request_repaint();
            });
        }
    }

    #[cfg(windows)]
    fn ensure_server(&mut self) -> Result<u16, String> {
        if let Some(port) = self.server_port {
            return Ok(port);
        }
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("Không mở được cổng stream âm thanh cục bộ: {e}"))?;
        let port = listener
            .local_addr()
            .map_err(|e| format!("Không đọc được cổng loopback: {e}"))?
            .port();
        let streams = self.streams.clone();
        std::thread::spawn(move || {
            for incoming in listener.incoming() {
                let Ok(stream) = incoming else {
                    break;
                };
                let streams = streams.clone();
                std::thread::spawn(move || serve_audio_stream(stream, streams));
            }
        });
        self.server_port = Some(port);
        Ok(port)
    }
}

#[cfg(not(windows))]
#[allow(clippy::too_many_arguments)]
async fn play_chunks_unix(
    chunks: Vec<String>,
    voice: String,
    rate: i8,
    chunked: bool,
    volume: f32,
    speech_id: u64,
    cancel: Arc<AtomicBool>,
    child_slot: Arc<Mutex<Option<std::process::Child>>>,
) -> Result<(), String> {
    let items = if chunked {
        chunks
    } else {
        vec![chunks.join(" ")]
    };
    let mut next_task = None;
    for idx in 0..items.len() {
        if cancel.load(Ordering::SeqCst) {
            break;
        }
        let (mut cur_rx, handle) = if let Some(pair) = next_task.take() {
            pair
        } else {
            spawn_chunk_task(items[idx].clone(), voice.clone(), rate, cancel.clone())
        };
        if idx + 1 < items.len() {
            next_task = Some(spawn_chunk_task(
                items[idx + 1].clone(),
                voice.clone(),
                rate,
                cancel.clone(),
            ));
        }
        let mut mp3 = Vec::new();
        while let Some(part) = cur_rx.recv().await {
            if cancel.load(Ordering::SeqCst) {
                return Ok(());
            }
            mp3.extend_from_slice(&part);
        }
        if let Ok(Err(err)) = handle.await {
            return Err(err);
        }
        if mp3.is_empty() || cancel.load(Ordering::SeqCst) {
            continue;
        }
        let path = std::env::temp_dir().join(format!(
            "pentakuru-tts-{}-{speech_id}-{idx}.mp3",
            std::process::id()
        ));
        std::fs::write(&path, &mp3).map_err(|e| format!("Không ghi được chunk âm thanh: {e}"))?;
        let play_res = tokio::task::spawn_blocking({
            let path = path.clone();
            let cancel = cancel.clone();
            let child_slot = child_slot.clone();
            move || play_mp3_file_unix(&path, volume, &cancel, &child_slot)
        })
        .await
        .map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(&path);
        play_res?;
    }
    Ok(())
}

#[cfg(not(windows))]
fn play_mp3_file_unix(
    path: &std::path::Path,
    volume: f32,
    cancel: &Arc<AtomicBool>,
    child_slot: &Arc<Mutex<Option<std::process::Child>>>,
) -> Result<(), String> {
    if cancel.load(Ordering::SeqCst) {
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    let spawned = std::process::Command::new("afplay")
        .args(["-v", &format!("{:.2}", volume.clamp(0.0, 1.0))])
        .arg(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Không chạy được afplay trên macOS: {e}"));

    #[cfg(not(target_os = "macos"))]
    let spawned = {
        let vol_pct = format!("{}", (volume.clamp(0.0, 1.0) * 100.0).round() as u32);
        std::process::Command::new("ffplay")
            .args([
                "-nodisp",
                "-autoexit",
                "-loglevel",
                "quiet",
                "-volume",
                &vol_pct,
            ])
            .arg(path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .or_else(|_| {
                std::process::Command::new("mpv")
                    .args([
                        "--no-video",
                        "--really-quiet",
                        &format!("--volume={vol_pct}"),
                    ])
                    .arg(path)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
            })
            .or_else(|_| {
                std::process::Command::new("mpg123")
                    .arg("-q")
                    .arg(path)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
            })
            .map_err(|_| "Không tìm thấy trình phát MP3 (ffplay/mpv/mpg123) trên Linux.".to_owned())
    };

    let child = spawned?;
    if let Ok(mut guard) = child_slot.lock() {
        *guard = Some(child);
    }
    loop {
        if cancel.load(Ordering::SeqCst) {
            if let Ok(mut guard) = child_slot.lock()
                && let Some(mut c) = guard.take()
            {
                let _ = c.kill();
                let _ = c.wait();
            }
            return Ok(());
        }
        let done = if let Ok(mut guard) = child_slot.lock() {
            match guard.as_mut() {
                Some(c) => c.try_wait().ok().flatten().is_some(),
                None => true,
            }
        } else {
            true
        };
        if done {
            if let Ok(mut guard) = child_slot.lock() {
                let _ = guard.take();
            }
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

#[cfg(any(windows, test))]
fn serve_audio_stream(mut stream: TcpStream, streams: Arc<Mutex<HashMap<u64, StreamSlot>>>) {
    let Ok(reader_stream) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(reader_stream);
    let mut req_line = String::new();
    if reader.read_line(&mut req_line).is_err() {
        return;
    }
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
            break;
        }
    }

    let id = req_line
        .split_whitespace()
        .nth(1)
        .and_then(|p| p.strip_prefix("/tts/"))
        .and_then(|s| s.strip_suffix(".mp3"))
        .and_then(|s| s.parse::<u64>().ok());

    let rx = id.and_then(|req_id| {
        let map = streams.lock().ok()?;
        let slot = map.get(&req_id)?;
        slot.lock().ok()?.take()
    });

    let Some(rx) = rx else {
        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
        return;
    };

    let header = b"HTTP/1.1 200 OK\r\nContent-Type: audio/mpeg\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n";
    if stream.write_all(header).is_err() || stream.flush().is_err() {
        return;
    }
    while let Ok(chunk) = rx.recv() {
        if chunk.is_empty() {
            continue;
        }
        if stream.write_all(&chunk).is_err() || stream.flush().is_err() {
            break;
        }
    }
}

fn shared_runtime() -> &'static Runtime {
    static RT: OnceLock<Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("edge_tts tokio runtime")
    })
}

#[cfg(any(windows, test))]
async fn synthesize_chunks_pipelined(
    chunks: Vec<String>,
    voice: String,
    rate: i8,
    chunked: bool,
    cancel: Arc<AtomicBool>,
    out_tx: mpsc::Sender<Vec<u8>>,
) -> Result<(), String> {
    if !chunked {
        let joined = chunks.join(" ");
        let (frame_tx, mut frame_rx) = tokio_mpsc::unbounded_channel();
        fetch_chunk_audio(joined, voice, rate, cancel.clone(), frame_tx).await?;
        let mut combined = Vec::new();
        while let Some(part) = frame_rx.recv().await {
            combined.extend_from_slice(&part);
        }
        if !combined.is_empty() && !cancel.load(Ordering::SeqCst) {
            let _ = out_tx.send(combined);
        }
        return Ok(());
    }

    // Pipeline next chunk synthesis while the current chunk is already streaming to playback.
    let mut next_handle = None;
    for idx in 0..chunks.len() {
        if cancel.load(Ordering::SeqCst) {
            break;
        }
        let (cur_rx, task_handle) = if let Some(pair) = next_handle.take() {
            pair
        } else {
            spawn_chunk_task(chunks[idx].clone(), voice.clone(), rate, cancel.clone())
        };

        if idx + 1 < chunks.len() {
            next_handle = Some(spawn_chunk_task(
                chunks[idx + 1].clone(),
                voice.clone(),
                rate,
                cancel.clone(),
            ));
        }

        let mut rx = cur_rx;
        while let Some(bytes) = rx.recv().await {
            if cancel.load(Ordering::SeqCst) || out_tx.send(bytes).is_err() {
                return Ok(());
            }
        }
        if let Ok(Err(err)) = task_handle.await {
            return Err(err);
        }
    }
    Ok(())
}

fn spawn_chunk_task(
    text: String,
    voice: String,
    rate: i8,
    cancel: Arc<AtomicBool>,
) -> (
    tokio_mpsc::UnboundedReceiver<Vec<u8>>,
    tokio::task::JoinHandle<Result<(), String>>,
) {
    let (tx, rx) = tokio_mpsc::unbounded_channel();
    let handle =
        tokio::spawn(async move { fetch_chunk_audio(text, voice, rate, cancel, tx).await });
    (rx, handle)
}

async fn fetch_chunk_audio(
    text: String,
    voice: String,
    rate: i8,
    cancel: Arc<AtomicBool>,
    frame_tx: tokio_mpsc::UnboundedSender<Vec<u8>>,
) -> Result<(), String> {
    if cancel.load(Ordering::SeqCst) {
        return Ok(());
    }
    let cid = random_hex(16).to_lowercase();
    let muid = random_hex(16).to_uppercase();
    let gec = generate_sec_ms_gec_v2(None);
    let url = format!(
        "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1?TrustedClientToken={TRUSTED_CLIENT_TOKEN}&ConnectionId={cid}&Sec-MS-GEC={gec}&Sec-MS-GEC-Version={SEC_MS_GEC_VERSION}"
    );

    let mut req = url
        .into_client_request()
        .map_err(|e| format!("URL Edge TTS lỗi: {e}"))?;
    let hdrs = req.headers_mut();
    hdrs.insert("Pragma", HeaderValue::from_static("no-cache"));
    hdrs.insert("Cache-Control", HeaderValue::from_static("no-cache"));
    hdrs.insert("Origin", HeaderValue::from_static(EDGE_ORIGIN));
    hdrs.insert("User-Agent", HeaderValue::from_static(EDGE_UA));
    hdrs.insert(
        "Accept-Language",
        HeaderValue::from_static("vi-VN,vi;q=0.9,en-US;q=0.8,en;q=0.7"),
    );
    if let Ok(cookie) = HeaderValue::from_str(&format!("muid={muid};")) {
        hdrs.insert("Cookie", cookie);
    }

    let (mut ws, _) = connect_async(req)
        .await
        .map_err(|e| format!("Không kết nối được Edge TTS: {e}"))?;

    let ts = js_utc_timestamp();
    let cfg_msg = format!(
        "X-Timestamp:{ts}\r\nContent-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n{{\"context\":{{\"synthesis\":{{\"audio\":{{\"metadataoptions\":{{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"false\"}},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"}}}}}}}}\r\n"
    );
    ws.send(WsMessage::Text(cfg_msg))
        .await
        .map_err(|e| format!("Lỗi gửi cấu hình Edge TTS: {e}"))?;

    let rate_str = if rate >= 0 {
        format!("+{rate}%")
    } else {
        format!("{rate}%")
    };
    let escaped = xml_escape(&text);
    let ssml = format!(
        "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='en-US'><voice name='{voice}'><prosody pitch='+0Hz' rate='{rate_str}' volume='+0%'>{escaped}</prosody></voice></speak>"
    );
    let ssml_msg = format!(
        "X-RequestId:{cid}\r\nContent-Type:application/ssml+xml\r\nX-Timestamp:{ts}Z\r\nPath:ssml\r\n\r\n{ssml}"
    );
    ws.send(WsMessage::Text(ssml_msg))
        .await
        .map_err(|e| format!("Lỗi gửi văn bản Edge TTS: {e}"))?;

    let mut received_any = false;
    while let Some(item) = ws.next().await {
        if cancel.load(Ordering::SeqCst) {
            break;
        }
        match item.map_err(|e| format!("Lỗi luồng Edge TTS: {e}"))? {
            WsMessage::Binary(bin) => {
                if bin.len() < 2 {
                    continue;
                }
                let hdr_len = u16::from_be_bytes([bin[0], bin[1]]) as usize;
                if 2 + hdr_len > bin.len() {
                    continue;
                }
                let header = &bin[2..2 + hdr_len];
                let payload = &bin[2 + hdr_len..];
                let hdr_txt = String::from_utf8_lossy(header);
                if hdr_txt.contains("Path:audio")
                    && hdr_txt.contains("Content-Type:audio/mpeg")
                    && !payload.is_empty()
                {
                    received_any = true;
                    if frame_tx.send(payload.to_vec()).is_err() {
                        break;
                    }
                }
            }
            WsMessage::Text(txt) => {
                if txt.contains("Path:turn.end") {
                    break;
                }
            }
            WsMessage::Close(_) => break,
            _ => {}
        }
    }

    if !received_any && !cancel.load(Ordering::SeqCst) {
        return Err("Edge TTS không trả về dữ liệu âm thanh.".into());
    }
    Ok(())
}

/// Cleans Markdown/control tags and splits text into natural speech chunks for low latency.
pub fn prepare_tts_chunks(raw: &str, chunked: bool) -> Vec<String> {
    let cleaned = clean_speech_text(raw);
    if cleaned.is_empty() {
        return Vec::new();
    }
    if !chunked {
        let single: String = cleaned.chars().take(1200).collect();
        return vec![single];
    }

    let mut sentences = Vec::new();
    let mut cur = String::new();
    for ch in cleaned.chars() {
        cur.push(ch);
        let at_sentence = matches!(ch, '.' | '!' | '?' | ';' | '\n');
        let at_clause = ch == ',' && cur.chars().count() >= 48;
        let too_long = ch == ' ' && cur.chars().count() >= 140;
        if at_sentence || at_clause || too_long {
            let trimmed = cur.trim();
            if !trimmed.is_empty() {
                sentences.push(trimmed.to_owned());
            }
            cur.clear();
        }
    }
    if !cur.trim().is_empty() {
        sentences.push(cur.trim().to_owned());
    }

    // Keep the first chunk compact for fastest time-to-first-audio; merge tiny trailing crumbs.
    let mut out: Vec<String> = Vec::new();
    for seg in sentences {
        let target_limit = if out.is_empty() { 90 } else { 160 };
        let has_prev_chunk = out.len() > 1;
        if let Some(last) = out.last_mut()
            && (last.chars().count() < 22
                || (has_prev_chunk && last.chars().count() + seg.chars().count() < target_limit))
        {
            last.push(' ');
            last.push_str(&seg);
            continue;
        }
        out.push(seg);
    }
    out
}

fn clean_speech_text(raw: &str) -> String {
    let mut stripped = raw.to_owned();
    while let Some(start) = stripped.find("[[") {
        if let Some(rel_end) = stripped[start..].find("]]") {
            stripped.replace_range(start..start + rel_end + 2, " ");
        } else {
            stripped.truncate(start);
            break;
        }
    }
    for pat in ["**", "__", "`", "##", "#", "* "] {
        stripped = stripped.replace(pat, " ");
    }
    let words: Vec<&str> = stripped
        .split_whitespace()
        .filter(|w| !w.starts_with("http://") && !w.starts_with("https://"))
        .collect();
    words.join(" ").trim().chars().take(1800).collect()
}

fn xml_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

fn random_hex(bytes_len: usize) -> String {
    let mut buf = vec![0u8; bytes_len];
    let _ = getrandom::fill(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

fn js_utc_timestamp() -> String {
    let unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let secs_of_day = unix.rem_euclid(86_400);
    let days = unix.div_euclid(86_400);
    let weekday = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"][days.rem_euclid(7) as usize];
    let (y, m, d, _, _) = system::civil_from_unix(unix);
    let month = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][(m.saturating_sub(1)).min(11) as usize];
    let hh = secs_of_day / 3600;
    let mm = (secs_of_day % 3600) / 60;
    let ss = secs_of_day % 60;
    format!(
        "{weekday} {month} {d:02} {y:04} {hh:02}:{mm:02}:{ss:02} GMT+0000 (Coordinated Universal Time)"
    )
}

// ---------------------------------------------------------------------------
// Edge Sec-MS-GEC v2 token generator (matches edge-tts drm.py v2 bit-for-bit)
// ---------------------------------------------------------------------------

const SHA256_INITIAL: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];
const SHA256_STATE: [u32; 8] = [
    0xFD9FBEA2, 0x91243CE7, 0xBE499094, 0xCD2E1E0A, 0x6C006F49, 0x04BD35AB, 0xFEB0A7E4, 0x03EFFBD2,
];
const SHA256_SCHEDULE: [u32; 14] = [
    0xEB24BF9E, 0x1558691F, 0xA0C09EE2, 0x0CFA32BA, 0xA622EE8C, 0xD1DD9491, 0xC0DD25A0, 0xF9E70AB6,
    0xF0C31573, 0x7058CE4E, 0x7188E606, 0x00E5AB33, 0xD258B060, 0x72766564,
];
const SHA256_ROUND_CONSTANTS: [u32; 64] = [
    0x428A2F98, 0x71374491, 0xB5C0FBCF, 0xE9B5DBA5, 0x3956C25B, 0x59F111F1, 0x923F82A4, 0xAB1C5ED5,
    0xD807AA98, 0x12835B01, 0x243185BE, 0x550C7DC3, 0x72BE5D74, 0x80DEB1FE, 0x9BDC06A7, 0xC19BF174,
    0xE49B69C1, 0xEFBE4786, 0x0FC19DC6, 0x240CA1CC, 0x2DE92C6F, 0x4A7484AA, 0x5CB0A9DC, 0x76F988DA,
    0x983E5152, 0xA831C66D, 0xB00327C8, 0xBF597FC7, 0xC6E00BF3, 0xD5A79147, 0x06CA6351, 0x14292967,
    0x27B70A85, 0x2E1B2138, 0x4D2C6DFC, 0x53380D13, 0x650A7354, 0x766A0ABB, 0x81C2C92E, 0x92722C85,
    0xA2BFE8A1, 0xA81A664B, 0xC24B8B70, 0xC76C51A3, 0xD192E819, 0xD6990624, 0xF40E3585, 0x106AA070,
    0x19A4C116, 0x1E376C08, 0x2748774C, 0x34B0BCB5, 0x391C0CB3, 0x4ED8AA4A, 0x5B9CCA4F, 0x682E6FF3,
    0x748F82EE, 0x78A5636F, 0x84C87814, 0x8CC70208, 0x90BEFFFA, 0xA4506CEB, 0xBEF9A3F7, 0xC67178F2,
];
const SHA256_ADJUSTMENT: u32 = 0xC9DC06A7;
const SHA256_SIGMA0_ADJUSTMENT: u32 = 0x059C0B80;
const SHA256_SIGMA1_ADJUSTMENT: u32 = 0x000B92C0;

const CS64_COEFFICIENTS: [(u32, u32); 14] = [
    (0xE82E91B1, 0xB32700D0),
    (0x44DE24C6, 0x88DFF51D),
    (0x3BEB65BC, 0x29514038),
    (0xC89C5933, 0xB328CBDF),
    (0x8D1EC211, 0xD405B4D5),
    (0x314EF5D5, 0x26E55F3C),
    (0xE92491B1, 0x607801C6),
    (0xF0A5B9D5, 0x82B2C1FB),
    (0x06064CF3, 0xFA51A3DA),
    (0xDA79FA60, 0x207CC464),
    (0xBED20560, 0x3C3C74F3),
    (0x89F9C9F8, 0xD2D14B61),
    (0x35FA38C6, 0xBB04A2C1),
    (0xCB49E3C6, 0xC5626167),
];
const CS64_FACTOR: u32 = 0x82893933;
const CS64_ADJUSTMENT0: u32 = 0xF2F90B1A;
const CS64_ADJUSTMENT1: u32 = 0x0B1A0000;

fn sigma0(v: u32) -> u32 {
    v.rotate_right(7) ^ v.rotate_right(18) ^ (v >> 3)
}

fn sigma1(v: u32) -> u32 {
    v.rotate_right(17) ^ v.rotate_right(19) ^ (v >> 10)
}

fn expand_sha256_schedule(words: &mut Vec<u32>, first_word: usize) {
    for i in first_word..64 {
        let s1 = sigma1(words[i - 2]);
        let s0 = sigma0(words[i - 15]);
        words.push(
            s1.wrapping_add(words[i - 7])
                .wrapping_add(s0)
                .wrapping_add(words[i - 16]),
        );
    }
}

fn sha256_rounds(mut s: [u32; 8], words: &[u32], first_round: usize) -> [u32; 8] {
    for (idx, &w) in words.iter().enumerate().take(64).skip(first_round) {
        let k = if first_round == 14 && idx == 14 {
            SHA256_ADJUSTMENT
        } else {
            SHA256_ROUND_CONSTANTS[idx]
        };
        let big_s1 = s[4].rotate_right(6) ^ s[4].rotate_right(11) ^ s[4].rotate_right(25);
        let ch = s[6] ^ (s[4] & (s[5] ^ s[6]));
        let t1 = s[7]
            .wrapping_add(big_s1)
            .wrapping_add(ch)
            .wrapping_add(k)
            .wrapping_add(w);
        let big_s0 = s[0].rotate_right(2) ^ s[0].rotate_right(13) ^ s[0].rotate_right(22);
        let maj = (s[0] & s[1]) | (s[2] & (s[0] | s[1]));
        let t2 = big_s0.wrapping_add(maj);
        s = [
            t1.wrapping_add(t2),
            s[0],
            s[1],
            s[2],
            s[3].wrapping_add(t1),
            s[4],
            s[5],
            s[6],
        ];
    }
    s
}

fn token_sha256_digest(data: &[u8]) -> [u8; 32] {
    assert!(data.len() >= 7);
    let mut init_words = vec![0u32; 14];
    init_words.push(u32::from_be_bytes([0, data[0], data[1], data[2]]));
    init_words.push(u32::from_be_bytes([data[3], data[4], data[5], data[6]]));
    for idx in 16..30 {
        let mut val = SHA256_SCHEDULE[idx - 16];
        let s1 = sigma1(init_words[idx - 2]);
        val = val.wrapping_add(if idx == 16 {
            s1 ^ SHA256_SIGMA1_ADJUSTMENT
        } else {
            s1
        });
        val = val.wrapping_add(init_words[idx - 7]);
        let s0 = sigma0(init_words[idx - 15]);
        val = val.wrapping_add(if idx == 29 {
            s0 ^ SHA256_SIGMA0_ADJUSTMENT
        } else {
            s0
        });
        init_words.push(val);
    }
    let w30 = sigma1(init_words[28])
        .wrapping_add(init_words[23])
        .wrapping_add(sigma0(init_words[15]))
        .wrapping_add(init_words[14])
        .wrapping_add(SHA256_ADJUSTMENT)
        .wrapping_sub(SHA256_ROUND_CONSTANTS[14]);
    init_words.push(w30);
    expand_sha256_schedule(&mut init_words, 31);
    let working = sha256_rounds(SHA256_STATE, &init_words, 14);
    let mut state = [0u32; 8];
    for i in 0..8 {
        state[i] = SHA256_INITIAL[i].wrapping_add(working[i]);
    }

    let length = 57 + data.len();
    let mut padded = Vec::with_capacity(data.len() + 72);
    padded.extend_from_slice(&data[7..]);
    padded.push(0x80);
    let zeros = (55usize.wrapping_sub(length)) % 64;
    padded.resize(padded.len() + zeros, 0);
    padded.extend_from_slice(&((length as u64) * 8).to_be_bytes());

    for block in padded.chunks_exact(64) {
        let mut words = Vec::with_capacity(64);
        for chunk in block.chunks_exact(4) {
            words.push(u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }
        expand_sha256_schedule(&mut words, 16);
        let w = sha256_rounds(state, &words, 0);
        for i in 0..8 {
            state[i] = state[i].wrapping_add(w[i]);
        }
    }

    let mut out = [0u8; 32];
    for (i, word) in state.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn cs64_digest(data: &[u8], digest: &[u8; 32]) -> (u32, u32) {
    let key_a = u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]]) | 1;
    let key_b = u32::from_le_bytes([digest[4], digest[5], digest[6], digest[7]]) | 1;
    let factors = [
        key_a.wrapping_mul(CS64_FACTOR),
        key_b.wrapping_mul(CS64_FACTOR),
    ];

    let mut terms: Vec<u32> = CS64_COEFFICIENTS
        .iter()
        .enumerate()
        .map(|(i, &(coeff, _))| coeff.wrapping_mul(factors[i % 2]))
        .collect();
    let mut offsets: Vec<u32> = CS64_COEFFICIENTS.iter().map(|&(_, off)| off).collect();

    let mut prefixed = Vec::with_capacity(data.len() + 8);
    prefixed.push(0);
    prefixed.extend_from_slice(data);
    while prefixed.len() % 4 != 0 {
        prefixed.push(0);
    }
    let mut words: Vec<u32> = prefixed
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    if words.len() % 2 == 1 {
        words.push(0);
    }

    for (idx, word) in words.into_iter().enumerate() {
        let mut term = word.wrapping_mul(if idx % 2 == 0 { key_a } else { key_b });
        let mut offset =
            word.rotate_left(16)
                .wrapping_mul(if idx % 2 == 0 { 0xEF0569FB } else { 0xC31713DB });
        if idx == 0 {
            term = term.wrapping_add(CS64_ADJUSTMENT0.wrapping_mul(factors[0]));
            offset = offset.wrapping_add(CS64_ADJUSTMENT1);
        }
        terms.push(term);
        offsets.push(offset);
    }

    let (mut swap_acc, mut swap_last, mut state, mut acc) = (0u32, 0u32, 0u32, 0u32);
    for idx in (0..terms.len()).step_by(2) {
        let (x, y) = (terms[idx], terms[idx + 1]);
        let t = offsets[idx].wrapping_add(x).rotate_left(16);
        let q = t.wrapping_mul(0x76F50F34);
        swap_acc =
            swap_acc.wrapping_add(q.rotate_left(16).wrapping_mul(0xC3EFEA97).wrapping_add(q));

        let t2 = offsets[idx + 1].wrapping_add(y).rotate_left(16);
        let q2 = t2.wrapping_mul(0x1881CE3C);
        swap_last = q2.rotate_left(16).wrapping_mul(0x35BD1EC9).wrapping_add(q2);
        swap_acc = swap_acc.wrapping_add(swap_last);

        state = state.wrapping_mul(key_a).wrapping_add(x);
        state = state.rotate_left(16).wrapping_mul(0xCF98B111);
        state = state.rotate_left(16).wrapping_mul(0x87085B9F);
        state = state.rotate_left(16).wrapping_mul(0x12CEB96D);
        state = state.rotate_left(16).wrapping_mul(0x257E1D83);
        acc = acc.wrapping_add(state);

        let mut u = state.wrapping_mul(key_b).wrapping_add(y);
        u = u.rotate_left(16).wrapping_mul(0xA27416F5);
        u = u.rotate_left(16).wrapping_mul(0xD38396FF);
        state = u
            .rotate_left(16)
            .wrapping_mul(0x7C932B89)
            .rotate_left(16)
            .wrapping_mul(0xBFA49F69);
        acc = acc.wrapping_add(state);
    }
    (swap_last ^ state, swap_acc ^ acc)
}

/// Computes Edge TTS `Sec-MS-GEC` v2 (`<16 hex CS64>-<64 hex SHA256>`).
pub fn generate_sec_ms_gec_v2(unix_timestamp: Option<f64>) -> String {
    let mut ticks = unix_timestamp.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    });
    ticks += WIN_EPOCH_SECS;
    ticks -= ticks % 300.0;
    ticks *= 1e7;
    let payload = format!("{ticks:.0}{TRUSTED_CLIENT_TOKEN}");
    let bytes = payload.as_bytes();
    let digest = token_sha256_digest(bytes);
    let (c0, c1) = cs64_digest(bytes, &digest);
    let mut cs_bytes = [0u8; 8];
    cs_bytes[0..4].copy_from_slice(&c0.to_le_bytes());
    cs_bytes[4..8].copy_from_slice(&c1.to_le_bytes());
    let cs_hex: String = cs_bytes.iter().map(|b| format!("{b:02X}")).collect();
    let sha_hex: String = digest.iter().map(|b| format!("{b:02X}")).collect();
    format!("{cs_hex}-{sha_hex}")
}

// ---------------------------------------------------------------------------
// Settings UI (Voice picker matching the user's card mockup)
// ---------------------------------------------------------------------------

pub enum SettingsAction {
    None,
    PreviewVoice(&'static VoiceOption),
    StopSpeech,
}

/// Renders the Edge TTS settings & voice cards inside Preferences (`Alt + Q`).
pub fn edge_tts_settings_panel_ui(
    ui: &mut egui::Ui,
    settings: &mut EdgeTtsSettings,
    previewing_voice: Option<&str>,
    is_speaking: bool,
    status: Option<&str>,
    default_open: bool,
) -> (bool, SettingsAction) {
    let mut changed = false;
    let mut action = SettingsAction::None;

    egui::CollapsingHeader::new("Giọng đọc AI (Edge TTS)")
        .open(default_open.then_some(true))
        .show(ui, |ui| {
            changed |= ui
                .checkbox(
                    &mut settings.enabled,
                    "Bật API giọng đọc Microsoft Edge TTS",
                )
                .changed();
            ui.add_enabled_ui(settings.enabled, |ui| {
                changed |= ui
                    .checkbox(
                        &mut settings.chunked_streaming,
                        "Phát âm thanh theo từng chunk (giảm độ trễ khởi phát)",
                    )
                    .changed();
                changed |= ui
                    .checkbox(
                        &mut settings.read_ai_replies,
                        "Thông báo bằng giọng nói sau khi AI hoàn thành tác vụ",
                    )
                    .changed();
                theme::caption(
                    ui,
                    "Lịch nhắc nhở được thiết lập trực tiếp qua chat AI và tự động dừng khi anh nhắn tin với AI.",
                );

                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label("Tốc độ đọc:");
                    for (r, label) in [
                        (-15i8, "Chậm"),
                        (0, "Mặc định"),
                        (15, "Nhanh"),
                        (30, "Rất nhanh"),
                    ] {
                        if ui
                            .selectable_value(&mut settings.rate_percent, r, label)
                            .changed()
                        {
                            changed = true;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Âm lượng:");
                    if ui
                        .add(egui::Slider::new(&mut settings.volume, 0.1..=1.0).show_value(false))
                        .changed()
                    {
                        changed = true;
                    }
                    ui.label(format!("{}%", (settings.volume * 100.0).round() as u32));
                });

                ui.horizontal_wrapped(|ui| {
                    if is_speaking && ui.small_button("Dừng giọng đọc").clicked() {
                        action = SettingsAction::StopSpeech;
                    }
                    if let Some(msg) = status {
                        theme::caption(ui, msg);
                    }
                });

                ui.add_space(6.0);
                ui.label(egui::RichText::new("Chọn giọng đọc (mặc định: Vivienne Multilingual):").strong());
                ui.add_space(4.0);

            for voice in EDGE_VOICES {
                let selected = settings.voice_id == voice.id;
                let this_playing = is_speaking && previewing_voice == Some(voice.id);
                let fill = if selected {
                    Color32::from_rgb(24, 27, 34)
                } else {
                    Color32::from_rgb(17, 19, 24)
                };
                let stroke_color = if selected {
                    Color32::from_rgb(208, 214, 226)
                } else {
                    Color32::from_rgb(56, 60, 70)
                };

                let frame = egui::Frame::NONE
                    .fill(fill)
                    .stroke(Stroke::new(if selected { 1.3 } else { 1.0 }, stroke_color))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(egui::Margin::symmetric(12, 10));

                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Radio circle matching mockup
                        let (radio_rect, radio_resp) =
                            ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::click());
                        let center = radio_rect.center();
                        ui.painter().circle_stroke(
                            center,
                            7.5,
                            Stroke::new(
                                1.5,
                                if selected {
                                    Color32::from_rgb(236, 239, 246)
                                } else {
                                    Color32::from_rgb(120, 126, 138)
                                },
                            ),
                        );
                        if selected {
                            ui.painter().circle_filled(
                                center,
                                3.8,
                                Color32::from_rgb(240, 242, 248),
                            );
                        }
                        if radio_resp.clicked() {
                            settings.voice_id = voice.id.to_owned();
                            changed = true;
                        }

                        ui.add_space(4.0);

                        let preview_btn_w = 78.0;
                        let text_w = (ui.available_width() - preview_btn_w - 8.0).max(120.0);
                        ui.allocate_ui_with_layout(
                            Vec2::new(text_w, 0.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_max_width(text_w);
                                ui.horizontal_wrapped(|ui| {
                                    let title_resp = ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(voice.name)
                                                .size(13.5)
                                                .strong()
                                                .color(theme::TEXT),
                                        )
                                        .sense(egui::Sense::click()),
                                    );
                                    if title_resp.clicked() {
                                        settings.voice_id = voice.id.to_owned();
                                        changed = true;
                                    }

                                    egui::Frame::NONE
                                        .fill(Color32::from_rgb(28, 31, 38))
                                        .stroke(Stroke::new(1.0, Color32::from_rgb(68, 72, 82)))
                                        .corner_radius(CornerRadius::same(5))
                                        .inner_margin(egui::Margin::symmetric(6, 2))
                                        .show(ui, |ui| {
                                            ui.label(
                                                egui::RichText::new(voice.tag)
                                                    .size(11.0)
                                                    .color(Color32::from_rgb(186, 192, 204)),
                                            );
                                        });
                                });
                                ui.add_space(2.0);
                                let desc_resp = ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(voice.desc)
                                            .size(12.0)
                                            .color(Color32::from_rgb(148, 154, 166)),
                                    )
                                    .wrap()
                                    .sense(egui::Sense::click()),
                                );
                                if desc_resp.clicked() {
                                    settings.voice_id = voice.id.to_owned();
                                    changed = true;
                                }
                            },
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let label = if this_playing { "Dừng" } else { "Nghe thử" };
                            let btn = egui::Button::new(
                                egui::RichText::new(label)
                                    .size(12.0)
                                    .color(theme::TEXT),
                            )
                            .fill(Color32::from_rgb(26, 29, 36))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(76, 80, 92)))
                            .corner_radius(CornerRadius::same(7))
                            .min_size(Vec2::new(72.0, 28.0));
                            if ui.add(btn).clicked() {
                                settings.voice_id = voice.id.to_owned();
                                changed = true;
                                action = if this_playing {
                                    SettingsAction::StopSpeech
                                } else {
                                    SettingsAction::PreviewVoice(voice)
                                };
                            }
                        });
                    });
                });
                ui.add_space(4.0);
            }
        });
    });

    (changed, action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_exact_sec_ms_gec_v2_token() {
        let token = generate_sec_ms_gec_v2(Some(1_763_400_000.0));
        assert_eq!(
            token,
            "4F62A0CA5F1858EE-7766CAE33C20690B001FD5BE7B59F83A425570AF4F38ECFAE455D825FF2EFB11"
        );
    }

    #[test]
    fn splits_long_speech_into_low_latency_chunks_and_cleans_markdown() {
        let text = "**Chào anh!** Hôm nay anh đã làm việc hơn 50 phút rồi. Hãy đứng dậy uống nước, hít thở sâu nhé! Xem thêm tại https://example.com [[TASK_ACTION:{}]]";
        let chunks = prepare_tts_chunks(text, true);
        assert!(chunks.len() >= 2, "expected multiple chunks: {chunks:?}");
        assert!(!chunks.join(" ").contains("https://"));
        assert!(!chunks.join(" ").contains("TASK_ACTION"));

        let single = prepare_tts_chunks(text, false);
        assert_eq!(single.len(), 1);
    }

    #[test]
    fn tracks_work_rhythm_breaks_and_switches_without_music() {
        let mut tracker = WorkRhythmTracker::default();
        for i in 0..45 {
            let app = if i % 2 == 0 {
                "main.rs - Visual Studio Code"
            } else {
                "Docs - Firefox"
            };
            tracker.record_sample(5, Some(app), 60);
        }
        assert_eq!(tracker.active_mins(), 45);
        assert!(!tracker.should_check_in(45, true));
        assert!(tracker.should_check_in(45, false));

        // Break >= 3 minutes resets active streak cleanly.
        tracker.record_sample(200, Some("Docs - Firefox"), 30);
        assert_eq!(tracker.active_mins(), 0);
        assert!(!tracker.should_check_in(45, false));
    }

    #[test]
    fn default_voice_matches_vivienne_multilingual() {
        let def = EdgeTtsSettings::default();
        assert_eq!(def.voice_id, "fr-FR-VivienneMultilingualNeural");
        assert_eq!(find_voice(&def.voice_id).name, "Vivienne Multilingual");
    }

    #[test]
    fn loopback_http_streams_chunks_in_order() {
        use std::io::Read;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let port = listener.local_addr().unwrap().port();
        let streams: Arc<Mutex<HashMap<u64, StreamSlot>>> = Arc::new(Mutex::new(HashMap::new()));
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        streams
            .lock()
            .unwrap()
            .insert(7, Arc::new(Mutex::new(Some(rx))));

        let srv_streams = streams.clone();
        let server = std::thread::spawn(move || {
            let (sock, _) = listener.accept().unwrap();
            serve_audio_stream(sock, srv_streams);
        });

        let sender = std::thread::spawn(move || {
            tx.send(b"CHUNK1-".to_vec()).unwrap();
            std::thread::sleep(Duration::from_millis(20));
            tx.send(b"CHUNK2".to_vec()).unwrap();
        });

        let mut client = TcpStream::connect(("127.0.0.1", port)).unwrap();
        client
            .write_all(b"GET /tts/7.mp3 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
            .unwrap();
        let mut resp = Vec::new();
        client.read_to_end(&mut resp).unwrap();
        let text = String::from_utf8_lossy(&resp);
        assert!(text.starts_with("HTTP/1.1 200 OK\r\nContent-Type: audio/mpeg"));
        assert!(text.ends_with("CHUNK1-CHUNK2"));
        sender.join().unwrap();
        server.join().unwrap();
    }

    #[test]
    fn settings_ui_renders_voice_cards_and_triggers_preview() {
        let ctx = egui::Context::default();
        let mut settings = EdgeTtsSettings::default();
        let mut action = SettingsAction::None;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.set_max_width(460.0);
            let (_, act) = edge_tts_settings_panel_ui(ui, &mut settings, None, false, None, true);
            action = act;
        });
        out.textures_delta.clear();
        assert!(matches!(action, SettingsAction::None));
        assert!(EDGE_VOICES.len() >= 8);
    }

    #[test]
    #[ignore = "Calls live Microsoft Edge Read Aloud WebSocket TTS"]
    fn live_edge_tts_smoke_test() {
        let rt = shared_runtime();
        let chunks = prepare_tts_chunks(
            "Chào anh, em là Vivienne. Âm thanh đang được truyền theo từng chunk độ trễ thấp.",
            true,
        );
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        rt.block_on(synthesize_chunks_pipelined(
            chunks,
            DEFAULT_VOICE_ID.into(),
            0,
            true,
            cancel,
            tx,
        ))
        .expect("Edge WebSocket TTS live synthesis");
        let received: Vec<Vec<u8>> = rx.try_iter().collect();
        let total: usize = received.iter().map(Vec::len).sum();
        assert!(received.len() >= 2, "expected multiple binary audio frames");
        assert!(total > 2000, "expected real MP3 audio bytes, got {total}");
        println!("EDGE_TTS_LIVE_OK frames={} bytes={total}", received.len());
    }
}
