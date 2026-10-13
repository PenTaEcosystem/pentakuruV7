//! Windows per-application audio capture (WASAPI process loopback).
//!
//! Capturing a single meeting app instead of the whole output device means
//! Sally's own translated-voice readout is never part of the captured
//! stream. Whole-system capture can instead exclude PentaKuru's process tree.
//! Availability depends on Windows updates; device loopback remains a fallback
//! with translated-voice playback disabled when isolation is unavailable.

#![cfg(windows)]

use super::{AudioSource, RawFrame};
use crate::error::{Result, SallyError};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

const CAPTURE_RATE: u32 = 48_000;
const CAPTURE_CHANNELS: u16 = 2;

#[derive(Debug, Clone, Serialize)]
pub struct AudioApp {
    pub pid: u32,
    pub name: String,
}

fn process_name(pid: u32) -> Option<String> {
    use windows::Win32::System::ProcessStatus::K32GetProcessImageFileNameW;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 512];
        let len = K32GetProcessImageFileNameW(handle, &mut buf);
        let _ = windows::Win32::Foundation::CloseHandle(handle);
        if len == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        let name = path.rsplit(['\\', '/']).next()?.to_string();
        if name.is_empty() {
            None
        } else {
            Some(name)
        }
    }
}

/// Applications that currently have an audio session on the default output
/// device (wasapi-rs exposes no session enumeration, so this goes through
/// COM directly). Call from a thread where blocking is acceptable.
pub fn list_audio_apps() -> Vec<AudioApp> {
    let _ = wasapi::initialize_mta();
    let mut apps: Vec<AudioApp> = Vec::new();
    let pids = unsafe { session_pids() }.unwrap_or_default();
    for pid in pids {
        if pid == 0 || pid == std::process::id() {
            continue; // system sounds
        }
        let Some(name) = process_name(pid) else {
            continue;
        };
        if apps.iter().any(|a| a.name.eq_ignore_ascii_case(&name)) {
            continue;
        }
        apps.push(AudioApp { pid, name });
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

unsafe fn session_pids() -> windows::core::Result<Vec<u32>> {
    use windows::core::Interface;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, AudioSessionStateExpired, IAudioSessionControl2, IAudioSessionManager2,
        IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

    let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
    let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
    let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
    let sessions = manager.GetSessionEnumerator()?;
    let count = sessions.GetCount()?;
    let mut pids = Vec::new();
    for i in 0..count {
        let Ok(control) = sessions.GetSession(i) else {
            continue;
        };
        if control
            .GetState()
            .map(|s| s == AudioSessionStateExpired)
            .unwrap_or(true)
        {
            continue;
        }
        let Ok(control2) = control.cast::<IAudioSessionControl2>() else {
            continue;
        };
        if let Ok(pid) = control2.GetProcessId() {
            pids.push(pid);
        }
    }
    Ok(pids)
}

/// Resolve an executable name (chosen in Settings) to a live audio-session
/// PID. Names are stable across runs; PIDs are not.
pub fn resolve_app_pid(exe_name: &str) -> Option<u32> {
    list_audio_apps()
        .into_iter()
        .find(|a| a.name.eq_ignore_ascii_case(exe_name))
        .map(|a| a.pid)
}

/// An included process tree must not contain our host (e.g. Explorer).
/// If enumeration fails, keep readout disabled rather than assume isolation.
pub fn capture_tree_is_external(pid: u32) -> bool {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return false;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut parents = std::collections::HashMap::new();
        let mut result = Process32FirstW(snapshot, &mut entry);
        while result.is_ok() {
            parents.insert(entry.th32ProcessID, entry.th32ParentProcessID);
            result = Process32NextW(snapshot, &mut entry);
        }
        let _ = windows::Win32::Foundation::CloseHandle(snapshot);
        let mut current = std::process::id();
        for _ in 0..parents.len() {
            if current == pid {
                return false;
            }
            let Some(&parent) = parents.get(&current) else {
                return current == 0;
            };
            if parent == 0 {
                return pid != 0;
            }
            current = parent;
        }
        false
    }
}

/// Capture one process tree's audio into `tx` as `AudioSource::System`
/// frames. Runs until `stop` is set.
pub fn spawn_app_capture(
    pid: u32,
    session_start: Instant,
    tx: mpsc::Sender<RawFrame>,
    stop: Arc<AtomicBool>,
) -> Result<std::thread::JoinHandle<()>> {
    spawn_process_capture(pid, true, session_start, tx, stop)
}

/// Capture other applications, excluding all audio rendered by this host
/// and its child WebViews. This also excludes PentaKuru's non-Sally sounds.
pub fn spawn_without_self(
    session_start: Instant,
    tx: mpsc::Sender<RawFrame>,
    stop: Arc<AtomicBool>,
) -> Result<std::thread::JoinHandle<()>> {
    spawn_process_capture(std::process::id(), false, session_start, tx, stop)
}

fn spawn_process_capture(
    pid: u32,
    include_tree: bool,
    session_start: Instant,
    tx: mpsc::Sender<RawFrame>,
    stop: Arc<AtomicBool>,
) -> Result<std::thread::JoinHandle<()>> {
    super::spawn_with_ready_signal("per-app capture", move |ready_tx| {
        let _ = wasapi::initialize_mta();
        let result = (|| -> std::result::Result<
            (wasapi::AudioClient, wasapi::Handle, wasapi::AudioCaptureClient),
            wasapi::WasapiError,
        > {
            let mut client = wasapi::AudioClient::new_application_loopback_client(pid, include_tree)?;
            let format = wasapi::WaveFormat::new(
                32,
                32,
                &wasapi::SampleType::Float,
                CAPTURE_RATE as usize,
                CAPTURE_CHANNELS as usize,
                None,
            );
            let mode = wasapi::StreamMode::EventsShared {
                autoconvert: true,
                buffer_duration_hns: 200_000, // 20 ms
            };
            client.initialize_client(&format, &wasapi::Direction::Capture, &mode)?;
            let event = client.set_get_eventhandle()?;
            let capture = client.get_audiocaptureclient()?;
            client.start_stream()?;
            Ok((client, event, capture))
        })();

        let (client, event, capture) = match result {
            Ok(v) => {
                let _ = ready_tx.send(Ok(()));
                v
            }
            Err(e) => {
                let _ = ready_tx.send(Err(SallyError::Audio(format!(
                    "per-app capture failed to start: {e}"
                ))));
                return;
            }
        };

        let block_align = (CAPTURE_CHANNELS as usize) * 4; // f32 frames
        let mut byte_queue: VecDeque<u8> = VecDeque::new();
        while !stop.load(Ordering::SeqCst) {
            if event.wait_for_event(500).is_err() {
                continue; // timeout: check stop flag and keep waiting
            }
            if capture.read_from_device_to_deque(&mut byte_queue).is_err() {
                log::error!("per-app capture read failed");
                break;
            }
            let whole_frames = byte_queue.len() / block_align;
            if whole_frames == 0 {
                continue;
            }
            let byte_count = whole_frames * block_align;
            let bytes: Vec<u8> = byte_queue.drain(..byte_count).collect();
            let samples: Vec<f32> = bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_le_bytes(*b))
                .collect();
            let _ = tx.try_send(RawFrame {
                source: AudioSource::System,
                t_ms: session_start.elapsed().as_millis() as u64,
                sample_rate: CAPTURE_RATE,
                channels: CAPTURE_CHANNELS,
                samples,
            });
        }
        let _ = client.stop_stream();
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_process_is_never_an_external_capture_tree() {
        assert!(!capture_tree_is_external(std::process::id()));
    }

    #[test]
    #[ignore = "plays a quiet one-second test tone; no microphone, disk recording or API"]
    fn self_exclusion_removes_readout_with_an_inclusive_positive_control() {
        use crate::audio::playback::Player;
        use std::time::Duration;
        let stop = Arc::new(AtomicBool::new(false));
        let (included_tx, mut included_rx) = mpsc::channel(256);
        let (excluded_tx, mut excluded_rx) = mpsc::channel(256);
        let start = Instant::now();
        let included =
            spawn_app_capture(std::process::id(), start, included_tx, stop.clone()).unwrap();
        let excluded = spawn_without_self(start, excluded_tx, stop.clone()).unwrap();
        let mut player = Player::new(0.2).unwrap();
        let samples: Vec<i16> = (0..24_000)
            .map(|i| ((std::f32::consts::TAU * 997.0 * i as f32 / 24_000.0).sin() * 3276.0) as i16)
            .collect();
        player.push(&samples);
        std::thread::sleep(Duration::from_millis(1300));
        stop.store(true, Ordering::SeqCst);
        included.join().unwrap();
        excluded.join().unwrap();
        fn tone_peak(rx: &mut mpsc::Receiver<RawFrame>) -> f64 {
            // Integrate the observation: short-frame peaks can mistake
            // unrelated video speech for the narrow-band test tone.
            let mut samples = Vec::new();
            while let Ok(frame) = rx.try_recv() {
                assert_eq!(frame.sample_rate, CAPTURE_RATE);
                samples.extend(crate::audio::downmix(&frame.samples, frame.channels));
            }
            let mut sin = 0.0;
            let mut cos = 0.0;
            for (i, &sample) in samples.iter().enumerate() {
                let phase = std::f64::consts::TAU * 997.0 * i as f64 / CAPTURE_RATE as f64;
                sin += sample as f64 * phase.sin();
                cos += sample as f64 * phase.cos();
            }
            2.0 * sin.hypot(cos) / samples.len().max(1) as f64
        }
        let included_peak = tone_peak(&mut included_rx);
        let excluded_peak = tone_peak(&mut excluded_rx);
        assert!(
            included_peak > 0.005,
            "positive control must hear readout: {included_peak}"
        );
        assert!(excluded_peak < included_peak * 0.2, "readout leaked into self-excluded audio: included={included_peak}, excluded={excluded_peak}");
    }
}
