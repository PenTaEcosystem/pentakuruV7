use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

pub enum GeminiLiveEvent {
    Connected,
    #[allow(dead_code)]
    Original(String),
    Translated(String),
    TurnFinished,
    Error(String),
}

pub async fn run_gemini_live_session(
    api_key: String,
    model: String,
    target_language: String,
    mut audio_rx: tokio::sync::mpsc::Receiver<(super::audio::AudioSource, Vec<i16>)>,
    event_tx: std::sync::mpsc::Sender<GeminiLiveEvent>,
    stop_signal: Arc<AtomicBool>,
) {
    let key = api_key.trim();
    if key.is_empty() {
        let _ = event_tx.send(GeminiLiveEvent::Error(
            "Chưa cấu hình Google Gemini API Key trong Cài đặt (Alt + Q)".into(),
        ));
        return;
    }

    let model_name = if model.trim().is_empty() {
        "gemini-2.0-flash-exp"
    } else {
        model.trim()
    };

    let url = format!(
        "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1alpha.GenerativeService.BidiGenerateContent?key={}",
        key
    );

    let connect_res = connect_async(&url).await;
    let (mut ws_stream, _) = match connect_res {
        Ok(res) => res,
        Err(e) => {
            let _ = event_tx.send(GeminiLiveEvent::Error(format!(
                "Không thể kết nối Gemini Live WebSocket: {e}"
            )));
            return;
        }
    };

    let _ = event_tx.send(GeminiLiveEvent::Connected);

    // Gửi Setup ban đầu
    let setup_msg = json!({
        "setup": {
            "model": format!("models/{model_name}"),
            "generationConfig": {
                "responseModalities": ["TEXT"]
            },
            "systemInstruction": {
                "parts": [
                    {
                        "text": format!(
                            "You are a real-time meeting transcription and translation assistant. \
                            Listen to speech, identify the source language, and translate it smoothly into {target_language}. \
                            Format output with the translated sentence."
                        )
                    }
                ]
            }
        }
    });

    if let Err(e) = ws_stream.send(Message::Text(setup_msg.to_string())).await {
        let _ = event_tx.send(GeminiLiveEvent::Error(format!("Lỗi gửi Setup: {e}")));
        return;
    }

    let b64 = base64::engine::general_purpose::STANDARD;

    loop {
        if stop_signal.load(Ordering::SeqCst) {
            break;
        }

        tokio::select! {
            // Nhận audio từ buffer
            Some((source, samples)) = audio_rx.recv() => {
                if samples.is_empty() {
                    continue;
                }

                let mut pcm_bytes = Vec::with_capacity(samples.len() * 2);
                for sample in samples {
                    pcm_bytes.extend_from_slice(&sample.to_le_bytes());
                }

                let encoded = b64.encode(&pcm_bytes);
                let audio_msg = json!({
                    "realtimeInput": {
                        "mediaChunks": [
                            {
                                "mimeType": "audio/pcm;rate=16000",
                                "data": encoded
                            }
                        ]
                    }
                });

                if let Err(e) = ws_stream.send(Message::Text(audio_msg.to_string())).await {
                    let _ = event_tx.send(GeminiLiveEvent::Error(format!("Lỗi truyền audio: {e}")));
                    break;
                }

                let _ = source;
            }

            // Nhận kết quả dịch từ Gemini WebSocket
            Some(msg_res) = ws_stream.next() => {
                match msg_res {
                    Ok(Message::Text(text)) => {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                            if let Some(arr) = val.pointer("/serverContent/modelTurn/parts").and_then(|parts| parts.as_array()) {
                                    for part in arr {
                                        if let Some(txt) = part.get("text").and_then(|t| t.as_str()) {
                                            let _ = event_tx.send(GeminiLiveEvent::Translated(txt.to_string()));
                                        }
                                    }
                            }
                            if val.pointer("/serverContent/turnComplete").is_some() {
                                let _ = event_tx.send(GeminiLiveEvent::TurnFinished);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        break;
                    }
                    Err(e) => {
                        let _ = event_tx.send(GeminiLiveEvent::Error(format!("Lỗi nhận dữ liệu Gemini: {e}")));
                        break;
                    }
                    _ => {}
                }
            }
            else => break,
        }
    }

    let _ = ws_stream.close(None).await;
}
