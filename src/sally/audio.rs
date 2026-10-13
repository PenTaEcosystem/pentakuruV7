use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AudioSource {
    System,
    Microphone,
}

#[derive(Clone, Debug)]
pub struct AudioChunk {
    pub source: AudioSource,
    /// 16kHz mono PCM i16 samples
    pub samples: Vec<i16>,
    #[allow(dead_code)]
    pub peak_level: f32,
}

pub fn list_output_devices() -> Vec<String> {
    let host = cpal::default_host();
    #[cfg(target_os = "windows")]
    {
        host.output_devices()
            .map(|devs| devs.filter_map(|d| d.name().ok()).collect())
            .unwrap_or_default()
    }
    #[cfg(not(target_os = "windows"))]
    {
        host.input_devices()
            .map(|devs| devs.filter_map(|d| d.name().ok()).collect())
            .unwrap_or_default()
    }
}

pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|devs| devs.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

pub struct AudioCaptureService {
    stop_signal: Arc<AtomicBool>,
    mic_level: Arc<Mutex<f32>>,
    speaker_level: Arc<Mutex<f32>>,
}

impl AudioCaptureService {
    pub fn new() -> Self {
        Self {
            stop_signal: Arc::new(AtomicBool::new(false)),
            mic_level: Arc::new(Mutex::new(0.0)),
            speaker_level: Arc::new(Mutex::new(0.0)),
        }
    }

    pub fn stop(&self) {
        self.stop_signal.store(true, Ordering::SeqCst);
    }

    pub fn get_levels(&self) -> (f32, f32) {
        let mic = *self.mic_level.lock().unwrap_or_else(|e| e.into_inner());
        let spk = *self.speaker_level.lock().unwrap_or_else(|e| e.into_inner());
        (mic, spk)
    }

    /// Khởi động luồng thu âm Microphone và Loa (WASAPI loopback)
    pub fn start(
        &mut self,
        mic_name: Option<String>,
        speaker_name: Option<String>,
        tx_chunks: std::sync::mpsc::Sender<AudioChunk>,
    ) -> Result<(), String> {
        self.stop_signal = Arc::new(AtomicBool::new(false));
        let stop_spk = self.stop_signal.clone();
        let stop_mic = self.stop_signal.clone();

        let host = cpal::default_host();

        // 1. Khởi động luồng thu Microphone
        let mic_level_arc = self.mic_level.clone();
        let tx_mic = tx_chunks.clone();
        let mic_dev = if let Some(ref name) = mic_name {
            host.input_devices()
                .ok()
                .and_then(|mut devs| devs.find(|d| d.name().map(|n| &n == name).unwrap_or(false)))
                .or_else(|| host.default_input_device())
        } else {
            host.default_input_device()
        };

        if let Some(device) = mic_dev {
            std::thread::spawn(move || {
                let _ = run_capture_stream(
                    device,
                    AudioSource::Microphone,
                    stop_mic,
                    mic_level_arc,
                    tx_mic,
                );
            });
        }

        // 2. Khởi động luồng thu Loa cuộc họp (WASAPI Loopback trên Windows)
        let speaker_level_arc = self.speaker_level.clone();
        let tx_spk = tx_chunks;

        let spk_dev = if let Some(ref name) = speaker_name {
            host.output_devices()
                .ok()
                .and_then(|mut devs| devs.find(|d| d.name().map(|n| &n == name).unwrap_or(false)))
                .or_else(|| host.default_output_device())
        } else {
            host.default_output_device()
        };

        if let Some(device) = spk_dev {
            std::thread::spawn(move || {
                let _ = run_capture_stream(
                    device,
                    AudioSource::System,
                    stop_spk,
                    speaker_level_arc,
                    tx_spk,
                );
            });
        }

        Ok(())
    }
}

fn run_capture_stream(
    device: cpal::Device,
    source: AudioSource,
    stop_signal: Arc<AtomicBool>,
    level_out: Arc<Mutex<f32>>,
    tx: std::sync::mpsc::Sender<AudioChunk>,
) -> Result<(), String> {
    let config = match source {
        AudioSource::Microphone => device
            .default_input_config()
            .map_err(|e| format!("Mic config err: {e}"))?,
        AudioSource::System => device
            .default_output_config()
            .map_err(|e| format!("Speaker config err: {e}"))?,
    };

    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;

    let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let buffer_producer = buffer.clone();

    let err_fn = |err| eprintln!("Audio stream error: {err}");

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data: &[f32], _| {
                if let Ok(mut b) = buffer_producer.lock() {
                    b.extend_from_slice(data);
                }
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data: &[i16], _| {
                if let Ok(mut b) = buffer_producer.lock() {
                    b.extend(data.iter().map(|&s| (s as f32) / 32768.0));
                }
            },
            err_fn,
            None,
        ),
        _ => return Err("Định dạng âm thanh không hỗ trợ".into()),
    }
    .map_err(|e| format!("build_input_stream err: {e}"))?;

    stream.play().map_err(|e| format!("stream.play err: {e}"))?;

    // Vòng lặp lấy mẫu và chuyển đổi thành 16kHz mono PCM mỗi 100ms
    let chunk_interval = Duration::from_millis(100);
    while !stop_signal.load(Ordering::SeqCst) {
        std::thread::sleep(chunk_interval);

        let raw_samples: Vec<f32> = {
            if let Ok(mut b) = buffer.lock() {
                b.drain(..).collect()
            } else {
                Vec::new()
            }
        };

        if raw_samples.is_empty() {
            if let Ok(mut lvl) = level_out.lock() {
                *lvl *= 0.8;
            }
            continue;
        }

        // Tính toán biên độ âm thanh (RMS level)
        let mut sum_sq = 0.0;
        let mut count = 0;
        for &s in &raw_samples {
            sum_sq += s * s;
            count += 1;
        }
        let rms = if count > 0 {
            (sum_sq / count as f32).sqrt()
        } else {
            0.0
        };
        let peak = (rms * 3.5).min(1.0);
        if let Ok(mut lvl) = level_out.lock() {
            *lvl = peak;
        }

        // Chuyển sang mono
        let mut mono = Vec::with_capacity(raw_samples.len() / channels.max(1));
        for frame in raw_samples.chunks(channels.max(1)) {
            let sum: f32 = frame.iter().sum();
            mono.push(sum / (frame.len() as f32));
        }

        // Downsample sang 16kHz
        let target_rate = 16000.0;
        let ratio = sample_rate as f32 / target_rate;
        let output_len = (mono.len() as f32 / ratio).floor() as usize;
        let mut pcm16 = Vec::with_capacity(output_len);

        for i in 0..output_len {
            let src_idx = (i as f32 * ratio) as usize;
            if src_idx < mono.len() {
                let sample = mono[src_idx].clamp(-1.0, 1.0);
                pcm16.push((sample * 32767.0) as i16);
            }
        }

        if !pcm16.is_empty() {
            let _ = tx.send(AudioChunk {
                source,
                samples: pcm16,
                peak_level: peak,
            });
        }
    }

    drop(stream);
    Ok(())
}
