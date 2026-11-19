//! Penta Music & Penta Clip MCP clients, natural command routing, and Windows audio player.
use crate::integrations;
use eframe::egui;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
#[cfg(windows)]
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Stdio},
    sync::atomic::AtomicBool,
    thread,
};
#[cfg(target_os = "macos")]
use std::{
    process::{Child, Command, Stdio},
    thread,
};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Track {
    pub id: String,
    pub video_id: String,
    pub title: String,
    pub artist: String,
    pub duration: f64,
    pub duration_formatted: String,
    pub thumbnail: String,
    pub stream_url: String,
    pub download_url: String,
    pub blend_source: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct VideoItem {
    #[serde(rename = "videoId")]
    pub video_id: String,
    pub title: String,
    pub author: String,
    #[serde(rename = "lengthSeconds")]
    pub length_seconds: u64,
    pub url: String,
    pub reason: String,
}

#[derive(Clone, Debug)]
pub struct MusicResponse {
    pub tracks: Vec<Track>,
    pub reply: String,
}

#[derive(Clone, Debug)]
pub struct ClipResponse {
    pub videos: Vec<VideoItem>,
    pub reply: String,
}

#[derive(Clone, Debug)]
pub enum PlayerAction {
    Pause,
    Resume,
    Next,
    Prev,
}

#[derive(Clone, Debug)]
pub enum MediaCommand {
    MusicTool {
        tool: String,
        arguments: Value,
        status_label: String,
    },
    MusicControl(PlayerAction),
    ClipTool {
        tool: String,
        arguments: Value,
        status_label: String,
    },
    ClipClose,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Opening,
    Playing,
    Paused,
}

#[derive(Clone, Debug, Default)]
pub struct PlaybackSnapshot {
    pub state: PlaybackState,
    pub position_secs: f64,
    pub duration_secs: f64,
    pub ended: bool,
    pub error: Option<String>,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .into()
}

/// Calls `POST /api/mcp/call` on Penta Music with `X-MCP-Key`.
pub fn call_music_mcp(
    endpoint: &str,
    key: &str,
    tool: &str,
    arguments: Value,
) -> Result<MusicResponse, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("Chưa nhập Music MCP Key (pmcp_...) trong Cài đặt.".into());
    }
    let url = if endpoint.trim().is_empty() {
        integrations::DEFAULT_MUSIC_MCP_URL
    } else {
        endpoint.trim()
    };
    let payload = json!({
        "tool": tool,
        "arguments": arguments,
    });
    let mut resp = agent()
        .post(url)
        .header("Content-Type", "application/json")
        .header("X-MCP-Key", key)
        .header(
            "User-Agent",
            concat!("PentaKuru/", env!("CARGO_PKG_VERSION")),
        )
        .send(payload.to_string())
        .map_err(|e| format!("Không kết nối được Music MCP: {e}"))?;
    let status = resp.status().as_u16();
    let text = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("Lỗi đọc phản hồi Music MCP: {e}"))?;
    let val: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Phản hồi Music MCP không hợp lệ ({status}): {e}"))?;
    if !(200..300).contains(&status) {
        let detail = val["detail"]
            .as_str()
            .unwrap_or("Music MCP trả về lỗi xác thực hoặc tham số.");
        return Err(format!("Music MCP ({status}): {detail}"));
    }
    let tracks: Vec<Track> = serde_json::from_value(val["tracks"].clone()).unwrap_or_default();
    let reply = val["ai_reply"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("Đã tải {} bài hát từ Penta Music.", tracks.len()));
    Ok(MusicResponse { tracks, reply })
}

/// Discovers brand-new tracks using `find_and_ingest_tracks` + low-history taste blend,
/// skipping tracks whose `video_id` is already in `exclude_ids`.
pub fn discover_more_tracks(
    endpoint: &str,
    key: &str,
    focus_query: &str,
    exclude_ids: &[String],
) -> Result<MusicResponse, String> {
    let seen: HashSet<&str> = exclude_ids.iter().map(String::as_str).collect();
    let mut collected = Vec::new();
    let mut picked_ids: HashSet<String> = HashSet::new();

    let query = if focus_query.trim().is_empty() {
        "nhạc chill acoustic hay mới".to_owned()
    } else {
        focus_query.trim().to_owned()
    };

    if let Ok(res) = call_music_mcp(
        endpoint,
        key,
        "find_and_ingest_tracks",
        json!({ "query": query, "limit": 8, "remote_play": false }),
    ) {
        for t in res.tracks {
            if !t.video_id.is_empty()
                && !seen.contains(t.video_id.as_str())
                && picked_ids.insert(t.video_id.clone())
            {
                collected.push(t);
            }
        }
    }

    if collected.len() < 3 {
        let blend_res = call_music_mcp(
            endpoint,
            key,
            "blend_history_and_taste",
            json!({
                "ratio_history": 0.2,
                "limit": 16,
                "focus_query": query,
                "remote_play": false
            }),
        )?;
        for t in blend_res.tracks {
            if !t.video_id.is_empty()
                && !seen.contains(t.video_id.as_str())
                && picked_ids.insert(t.video_id.clone())
            {
                collected.push(t);
            }
        }
    }

    let reply = if collected.is_empty() {
        "Chưa tìm được thêm bài mới ngoài danh sách hiện tại.".into()
    } else {
        format!("AI đã tìm thêm {} bài hát mới.", collected.len())
    };
    Ok(MusicResponse {
        tracks: collected,
        reply,
    })
}

/// Calls JSON-RPC 2.0 `tools/call` on Penta Clip (`/mcp`) with `Authorization: Bearer`.
pub fn call_clip_mcp(
    endpoint: &str,
    key: &str,
    tool: &str,
    arguments: Value,
) -> Result<ClipResponse, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("Chưa nhập Clip MCP Key (pc_mcp_...) trong Cài đặt.".into());
    }
    let url = if endpoint.trim().is_empty() {
        integrations::DEFAULT_CLIP_MCP_URL
    } else {
        endpoint.trim()
    };
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": tool,
            "arguments": arguments,
        }
    });
    let mut resp = agent()
        .post(url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Authorization", &format!("Bearer {key}"))
        .header(
            "User-Agent",
            concat!("PentaKuru/", env!("CARGO_PKG_VERSION")),
        )
        .send(payload.to_string())
        .map_err(|e| format!("Không kết nối được Clip MCP: {e}"))?;
    let status = resp.status().as_u16();
    let text = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("Lỗi đọc phản hồi Clip MCP: {e}"))?;
    let val: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Phản hồi Clip MCP không hợp lệ ({status}): {e}"))?;
    if !(200..300).contains(&status) {
        let msg = val["error"]["message"]
            .as_str()
            .or_else(|| val["detail"].as_str())
            .unwrap_or("Clip MCP từ chối yêu cầu.");
        return Err(format!("Clip MCP ({status}): {msg}"));
    }
    if let Some(err_msg) = val["error"]["message"].as_str() {
        return Err(format!("Clip MCP lỗi: {err_msg}"));
    }
    let result = &val["result"];
    let inner_text = result["content"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|item| item["text"].as_str())
        .unwrap_or("");
    if result["isError"].as_bool() == Some(true) {
        return Err(if inner_text.is_empty() {
            "Clip MCP báo lỗi thực thi.".into()
        } else {
            inner_text.to_owned()
        });
    }
    let parsed: Value = serde_json::from_str(inner_text).unwrap_or(Value::Null);
    let videos = extract_videos(&parsed);
    let reply = if let Some(first) = videos.first() {
        if first.author.is_empty() {
            format!("Đã mở video: {}", first.title)
        } else {
            format!("Đang phát video: {} — {}", first.title, first.author)
        }
    } else if !inner_text.is_empty() {
        inner_text.to_owned()
    } else {
        "Không tìm thấy video phù hợp trên Penta Clip.".into()
    };
    Ok(ClipResponse { videos, reply })
}

pub fn test_music_mcp(endpoint: &str, key: &str) -> Result<String, String> {
    let res = call_music_mcp(
        endpoint,
        key,
        "blend_history_and_taste",
        json!({ "limit": 2, "remote_play": false }),
    )?;
    let sample = res
        .tracks
        .first()
        .map(|t| format!(" · VD: {}", t.title))
        .unwrap_or_default();
    Ok(format!(
        "✓ Music MCP OK ({} bài trả về{sample})",
        res.tracks.len()
    ))
}

pub fn test_clip_mcp(endpoint: &str, key: &str) -> Result<String, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("Chưa nhập Clip MCP Key (pc_mcp_...) trong Cài đặt.".into());
    }
    let url = if endpoint.trim().is_empty() {
        integrations::DEFAULT_CLIP_MCP_URL
    } else {
        endpoint.trim()
    };
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": "get_account_info", "arguments": {} }
    });
    let mut resp = agent()
        .post(url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Authorization", &format!("Bearer {key}"))
        .header(
            "User-Agent",
            concat!("PentaKuru/", env!("CARGO_PKG_VERSION")),
        )
        .send(payload.to_string())
        .map_err(|e| format!("Không kết nối được Clip MCP: {e}"))?;
    let status = resp.status().as_u16();
    let text = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("Lỗi đọc phản hồi Clip MCP: {e}"))?;
    let val: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Phản hồi Clip MCP không hợp lệ ({status}): {e}"))?;
    if !(200..300).contains(&status) || val["error"].is_object() {
        let msg = val["error"]["message"]
            .as_str()
            .or_else(|| val["detail"].as_str())
            .unwrap_or("Không xác thực được Clip MCP.");
        return Err(format!("Clip MCP ({status}): {msg}"));
    }
    let inner_text = val["result"]["content"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|item| item["text"].as_str())
        .unwrap_or("{}");
    let info: Value = serde_json::from_str(inner_text).unwrap_or(Value::Null);
    let user = info["username"].as_str().unwrap_or("user");
    let watched = info["watchedCount"].as_u64().unwrap_or(0);
    Ok(format!("✓ Clip MCP OK (@{user}, đã xem {watched} video)"))
}

fn extract_videos(parsed: &Value) -> Vec<VideoItem> {
    for key in ["recommendations", "results", "videos", "history", "items"] {
        if let Some(arr) = parsed[key].as_array() {
            let items: Vec<VideoItem> = arr
                .iter()
                .filter_map(|item| {
                    if let Some(kind) = item["type"].as_str()
                        && kind != "video"
                    {
                        return None;
                    }
                    let mut v: VideoItem = serde_json::from_value(item.clone()).ok()?;
                    if v.video_id.is_empty() {
                        v.video_id = item["video_id"].as_str().unwrap_or("").to_owned();
                    }
                    if v.video_id.is_empty() {
                        return None;
                    }
                    Some(v)
                })
                .collect();
            if !items.is_empty() {
                return items;
            }
        }
    }
    if let Ok(single) = serde_json::from_value::<VideoItem>(parsed.clone())
        && !single.video_id.is_empty()
    {
        return vec![single];
    }
    Vec::new()
}

fn strip_leading_media_fillers(mut s: &str) -> &str {
    loop {
        let trimmed = s.trim();
        let next = [
            "ai ơi ",
            "pentakuru ơi ",
            "em ơi ",
            "bạn ơi ",
            "máy ơi ",
            "kêu ai ",
            "nhờ ai ",
            "bảo ai ",
            "anh muốn ",
            "tôi muốn ",
            "mình muốn ",
            "giúp anh ",
            "giúp tôi ",
            "giúp em ",
            "giúp mình ",
            "cho anh ",
            "cho tôi ",
            "cho em ",
            "cho mình ",
            "anh ",
            "em ",
            "tôi ",
            "mình ",
            "bạn ",
            "hãy ",
            "vui lòng ",
            "làm ơn ",
        ]
        .iter()
        .find_map(|p| trimmed.strip_prefix(p));
        match next {
            Some(rest) if !rest.trim().is_empty() => s = rest,
            _ => return trimmed,
        }
    }
}

fn strip_trailing_media_fillers(mut s: &str) -> &str {
    loop {
        let trimmed = s.trim();
        let next = [
            " cho anh nghe đi",
            " cho tôi nghe đi",
            " cho em nghe đi",
            " cho mình nghe đi",
            " cho anh xem đi",
            " cho tôi xem đi",
            " cho anh nghe",
            " cho tôi nghe",
            " cho em nghe",
            " cho mình nghe",
            " cho anh xem",
            " cho tôi xem",
            " giúp anh",
            " giúp tôi",
            " giúp em",
            " giúp mình",
            " giùm anh",
            " giùm",
            " cho anh",
            " cho tôi",
            " cho em",
            " cho mình",
            " lên",
            " đi nhé",
            " đi nha",
            " nhé",
            " nha",
            " với",
            " nào",
            " luôn",
            " thử",
            " đi",
        ]
        .iter()
        .find_map(|suf| trimmed.strip_suffix(suf));
        match next {
            Some(rest) if !rest.trim().is_empty() => s = rest,
            _ => return trimmed,
        }
    }
}

fn clean_mood_request(text: &str) -> Option<String> {
    let is_mood_or_change = [
        "bài khác",
        "nhạc khác",
        "đổi bài",
        "bài gì",
        "bài nào",
        "nhạc gì",
        "nhạc nào",
        "được không",
        "tí xíu",
        "chút xíu",
        "không thích bài",
        "chán bài",
    ]
    .iter()
    .any(|kw| text.contains(kw));
    if !is_mood_or_change {
        return None;
    }
    let mut s = format!(" {text} ");
    for phrase in [
        " có được không ",
        " được không ",
        " không em ",
        " không bạn ",
        " không ai ",
        " không thích bài này ",
        " bài gì ",
        " bài nào ",
        " nhạc gì ",
        " nhạc nào ",
        " bài khác ",
        " nhạc khác ",
        " tí xíu ",
        " chút xíu ",
        " hay hay ",
        " giúp anh ",
        " giúp tôi ",
        " giúp em ",
        " cho anh ",
        " cho tôi ",
        " cho em ",
        " đổi bài ",
        " mở bài ",
        " phát bài ",
        " nghe bài ",
        " tìm bài ",
        " mở nhạc ",
        " bật nhạc ",
        " nghe nhạc ",
        " tìm nhạc ",
    ] {
        while s.contains(phrase) {
            s = s.replace(phrase, " ");
        }
    }
    let keep: Vec<&str> = s
        .split_whitespace()
        .filter(|w| {
            !matches!(
                *w,
                "mở" | "phát"
                    | "bật"
                    | "nghe"
                    | "tìm"
                    | "đổi"
                    | "chọn"
                    | "bài"
                    | "nhạc"
                    | "hát"
                    | "khác"
                    | "gì"
                    | "nào"
                    | "đó"
                    | "tí"
                    | "chút"
                    | "một"
                    | "này"
                    | "nữa"
                    | "hơn"
                    | "lên"
                    | "không"
                    | "hông"
                    | "hay"
            )
        })
        .collect();
    Some(keep.join(" "))
}

/// Parses direct Vietnamese commands for Music MCP and Clip MCP before calling an LLM.
pub fn parse_direct_command(
    prompt: &str,
    has_music_mcp: bool,
    has_clip_mcp: bool,
) -> Option<MediaCommand> {
    let normalized = prompt
        .trim()
        .trim_end_matches(['.', '!', '?'])
        .to_lowercase();
    if normalized.contains('\n')
        || normalized.chars().count() > 140
        || normalized.starts_with("@note")
        || normalized.starts_with("@rag")
        || normalized.starts_with("#rag")
        || normalized.starts_with("vì sao ")
        || normalized.starts_with("tại sao ")
        || normalized.starts_with("viết ")
        || normalized.starts_with("giải thích ")
        || normalized.starts_with("hướng dẫn ")
        || normalized.starts_with("sửa ")
    {
        return None;
    }
    let lead = strip_leading_media_fillers(&normalized);
    let text = strip_trailing_media_fillers(lead);

    if has_clip_mcp {
        if matches!(
            text,
            "đóng video" | "tắt video" | "đóng clip" | "tắt clip" | "dừng video"
        ) {
            return Some(MediaCommand::ClipClose);
        }
        if matches!(
            text,
            "mở video"
                | "xem video"
                | "video đề xuất"
                | "mở video đề xuất"
                | "xem video đề xuất"
                | "gợi ý video"
                | "mở clip"
                | "xem clip"
                | "clip đề xuất"
                | "mở clip đề xuất"
                | "xem clip đề xuất"
                | "gợi ý clip"
                | "@clip"
                | "/clip"
        ) {
            return Some(MediaCommand::ClipTool {
                tool: "get_recommendations".into(),
                arguments: json!({ "limit": 6 }),
                status_label: "Đang lấy video đề xuất từ Penta Clip…".into(),
            });
        }
        if matches!(
            text,
            "video vừa xem" | "lịch sử video" | "xem lại video" | "lịch sử clip"
        ) {
            return Some(MediaCommand::ClipTool {
                tool: "get_watch_history".into(),
                arguments: json!({ "limit": 6 }),
                status_label: "Đang lấy lịch sử video trên Penta Clip…".into(),
            });
        }
        for prefix in [
            "tìm video ",
            "xem video ",
            "mở video ",
            "phát video ",
            "tìm clip ",
            "xem clip ",
            "mở clip ",
            "phát clip ",
            "/clip ",
            "@clip ",
        ] {
            if let Some(query) = text.strip_prefix(prefix).filter(|q| !q.trim().is_empty()) {
                return Some(MediaCommand::ClipTool {
                    tool: "search_videos".into(),
                    arguments: json!({
                        "query": query.trim(),
                        "type": "video",
                        "limit": 6
                    }),
                    status_label: format!("Đang tìm video \"{}\" trên Penta Clip…", query.trim()),
                });
            }
        }
    }

    if has_music_mcp {
        match text {
            "dừng nhạc" | "tạm dừng nhạc" | "tắt nhạc" => {
                return Some(MediaCommand::MusicControl(PlayerAction::Pause));
            }
            "tiếp tục nhạc" => return Some(MediaCommand::MusicControl(PlayerAction::Resume)),
            "qua bài" | "bài tiếp theo" | "bài tiếp" | "chuyển bài" | "next bài" => {
                return Some(MediaCommand::MusicControl(PlayerAction::Next));
            }
            "bài trước" | "lùi bài" => {
                return Some(MediaCommand::MusicControl(PlayerAction::Prev));
            }
            "muốn nghe nhạc"
            | "nghe nhạc"
            | "mở nhạc"
            | "bật nhạc"
            | "phát nhạc"
            | "mở music"
            | "mở bài nhạc"
            | "nghe bài nhạc"
            | "bật bài nhạc"
            | "phát bài nhạc"
            | "mở một bài nhạc"
            | "bật một bài nhạc"
            | "nghe một bài nhạc"
            | "mở một bài"
            | "nghe một bài"
            | "mở nhạc gì đó"
            | "bật nhạc gì đó"
            | "nhạc hợp gu"
            | "nghe nhạc hợp gu"
            | "mở nhạc hợp gu"
            | "phát nhạc hợp gu"
            | "bật nhạc hợp gu"
            | "trộn nhạc"
            | "@music"
            | "/music" => {
                return Some(MediaCommand::MusicTool {
                    tool: "blend_history_and_taste".into(),
                    arguments: json!({ "limit": 12, "remote_play": false }),
                    status_label: "Đang chọn nhạc hợp gu từ Penta Music MCP…".into(),
                });
            }
            "nhạc lịch sử" | "nghe lại nhạc" | "bài hay nghe" | "lịch sử nghe nhạc" =>
            {
                return Some(MediaCommand::MusicTool {
                    tool: "play_listening_history".into(),
                    arguments: json!({ "mode": "frequent", "limit": 12, "remote_play": false }),
                    status_label: "Đang lấy các bài nghe nhiều từ Penta Music MCP…".into(),
                });
            }
            "tìm nhạc mới"
            | "tìm bài mới"
            | "tìm bài hát mới"
            | "mở nhạc mới"
            | "nghe nhạc mới"
            | "tìm thêm nhạc"
            | "tìm thêm bài"
            | "nhạc mới cùng gu" => {
                return Some(MediaCommand::MusicTool {
                    tool: "find_and_ingest_tracks".into(),
                    arguments: json!({
                        "query": "nhạc chill acoustic mới hay",
                        "limit": 8,
                        "remote_play": false
                    }),
                    status_label: "Đang tìm các bài hát mới qua Penta Music MCP…".into(),
                });
            }
            "thư viện nhạc" | "nhạc yêu thích" | "playlist của tôi" | "mở thư viện nhạc" =>
            {
                return Some(MediaCommand::MusicTool {
                    tool: "play_my_library".into(),
                    arguments: json!({ "limit": 12, "remote_play": false }),
                    status_label: "Đang mở thư viện nhạc cá nhân…".into(),
                });
            }
            _ => {}
        }
        if let Some(mood) = clean_mood_request(text) {
            let query = if mood.is_empty() {
                "nhạc mới hay cùng gu".to_owned()
            } else if mood.starts_with("nhạc ") {
                mood
            } else {
                format!("nhạc {mood} hay")
            };
            return Some(MediaCommand::MusicTool {
                tool: "find_and_ingest_tracks".into(),
                arguments: json!({
                    "query": query,
                    "limit": 8,
                    "remote_play": false
                }),
                status_label: format!("Đang tìm \"{query}\" qua Penta Music MCP…"),
            });
        }
        for prefix in [
            "tìm nhạc mới ",
            "tìm bài mới ",
            "tìm bài hát mới ",
            "mở nhạc mới ",
            "nghe nhạc mới ",
            "tìm thêm nhạc ",
            "tìm thêm bài ",
        ] {
            if let Some(query) = text.strip_prefix(prefix).filter(|q| !q.trim().is_empty()) {
                return Some(MediaCommand::MusicTool {
                    tool: "find_and_ingest_tracks".into(),
                    arguments: json!({
                        "query": query.trim(),
                        "limit": 8,
                        "remote_play": false
                    }),
                    status_label: format!(
                        "Đang tìm nhạc mới \"{}\" qua Penta Music MCP…",
                        query.trim()
                    ),
                });
            }
        }
        for prefix in [
            "tìm bài hát ",
            "tìm bài ",
            "tìm nhạc ",
            "muốn nghe bài ",
            "nghe bài ",
            "phát bài ",
            "mở bài ",
            "nghe nhạc ",
            "mở nhạc ",
            "bật nhạc ",
            "phát nhạc ",
            "/music ",
            "@music ",
        ] {
            if let Some(query) = text.strip_prefix(prefix).filter(|q| !q.trim().is_empty()) {
                return Some(MediaCommand::MusicTool {
                    tool: "search_and_stream_music".into(),
                    arguments: json!({
                        "query": query.trim(),
                        "limit": 12,
                        "remote_play": false
                    }),
                    status_label: format!("Đang tìm \"{}\" qua Penta Music MCP…", query.trim()),
                });
            }
        }
    }

    None
}

/// Extracts optional `[[MCP_MUSIC:...]]` or `[[MCP_CLIP:...]]` action tag from an AI reply.
pub fn extract_ai_media_tags(reply: &str) -> (String, Option<MediaCommand>) {
    let mut cleaned = reply.to_owned();
    let mut found = None;
    for prefix in ["[[MCP_MUSIC:", "[[MCP_CLIP:"] {
        if let Some(start) = cleaned.find(prefix)
            && let Some(rel_end) = cleaned[start..].find("]]")
        {
            let tag_inner = cleaned[start + prefix.len()..start + rel_end].trim();
            let mut parts = tag_inner.splitn(2, '|');
            let action = parts.next().unwrap_or("").trim();
            let arg = parts.next().unwrap_or("").trim();
            if prefix == "[[MCP_MUSIC:" {
                found = Some(match action {
                    "search" if !arg.is_empty() => MediaCommand::MusicTool {
                        tool: "search_and_stream_music".into(),
                        arguments: json!({ "query": arg, "limit": 12, "remote_play": false }),
                        status_label: format!("Đang phát \"{arg}\" từ Penta Music…"),
                    },
                    "history" => MediaCommand::MusicTool {
                        tool: "play_listening_history".into(),
                        arguments: json!({ "mode": "frequent", "limit": 12, "remote_play": false }),
                        status_label: "Đang mở nhạc lịch sử…".into(),
                    },
                    "library" => MediaCommand::MusicTool {
                        tool: "play_my_library".into(),
                        arguments: json!({ "limit": 12, "remote_play": false }),
                        status_label: "Đang mở thư viện nhạc…".into(),
                    },
                    "discover" => MediaCommand::MusicTool {
                        tool: "find_and_ingest_tracks".into(),
                        arguments: json!({
                            "query": if arg.is_empty() { "nhạc chill acoustic mới" } else { arg },
                            "limit": 8,
                            "remote_play": false
                        }),
                        status_label: "Đang tìm các bài hát mới cho anh…".into(),
                    },
                    _ => {
                        let mut args = json!({ "limit": 12, "remote_play": false });
                        if !arg.is_empty() {
                            args["focus_query"] = json!(arg);
                        }
                        MediaCommand::MusicTool {
                            tool: "blend_history_and_taste".into(),
                            arguments: args,
                            status_label: "Đang chọn nhạc hợp gu…".into(),
                        }
                    }
                });
            } else {
                found = Some(match action {
                    "search" if !arg.is_empty() => MediaCommand::ClipTool {
                        tool: "search_videos".into(),
                        arguments: json!({ "query": arg, "type": "video", "limit": 6 }),
                        status_label: format!("Đang mở video \"{arg}\"…"),
                    },
                    "history" => MediaCommand::ClipTool {
                        tool: "get_watch_history".into(),
                        arguments: json!({ "limit": 6 }),
                        status_label: "Đang mở lịch sử video…".into(),
                    },
                    _ => MediaCommand::ClipTool {
                        tool: "get_recommendations".into(),
                        arguments: json!({ "limit": 6 }),
                        status_label: "Đang mở video đề xuất…".into(),
                    },
                });
            }
            let full_end = start + rel_end + 2;
            cleaned.replace_range(start..full_end, "");
            break;
        }
    }
    (cleaned.trim().to_owned(), found)
}

pub struct AudioEngine {
    snapshot: Arc<Mutex<PlaybackSnapshot>>,
    play_gen: Arc<AtomicU64>,
    #[cfg(windows)]
    worker: Option<AudioWorker>,
    #[cfg(target_os = "macos")]
    mac_child: Arc<Mutex<Option<Child>>>,
}

#[cfg(windows)]
struct AudioWorker {
    child: Child,
    stdin: ChildStdin,
    stopping: Arc<AtomicBool>,
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self {
            snapshot: Arc::new(Mutex::new(PlaybackSnapshot::default())),
            play_gen: Arc::new(AtomicU64::new(0)),
            #[cfg(windows)]
            worker: None,
            #[cfg(target_os = "macos")]
            mac_child: Arc::new(Mutex::new(None)),
        }
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        self.stop();
    }
}

impl AudioEngine {
    pub fn snapshot(&self) -> PlaybackSnapshot {
        self.snapshot.lock().unwrap().clone()
    }

    pub fn play(&mut self, url: &str, volume: f32, ctx: &egui::Context) -> Result<(), String> {
        let clean = url.trim();
        if !clean.starts_with("https://") && !clean.starts_with("http://") {
            return Err("Stream URL không hợp lệ.".into());
        }
        #[cfg(windows)]
        {
            self.ensure_worker(ctx)?;
            let play_id = self.play_gen.fetch_add(1, Ordering::SeqCst) + 1;
            {
                let mut snap = self.snapshot.lock().unwrap();
                snap.state = PlaybackState::Opening;
                snap.position_secs = 0.0;
                snap.duration_secs = 0.0;
                snap.ended = false;
                snap.error = None;
            }
            self.send_cmd(&format!("GEN {play_id}"))?;
            self.send_cmd(&format!("VOL {:.2}", volume.clamp(0.0, 1.0)))?;
            self.send_cmd(&format!("PLAY {clean}"))
        }
        #[cfg(target_os = "macos")]
        {
            self.stop_playback();
            let play_id = self.play_gen.fetch_add(1, Ordering::SeqCst) + 1;
            {
                let mut snap = self.snapshot.lock().unwrap();
                snap.state = PlaybackState::Opening;
                snap.position_secs = 0.0;
                snap.duration_secs = 0.0;
                snap.ended = false;
                snap.error = None;
            }
            let url_owned = clean.to_owned();
            let snap_ref = self.snapshot.clone();
            let gen_ref = self.play_gen.clone();
            let child_slot = self.mac_child.clone();
            let vol = volume.clamp(0.0, 1.0);
            let repaint = ctx.clone();
            thread::spawn(move || {
                let tmp = std::env::temp_dir().join(format!(
                    "pentakuru-music-{}-{play_id}.audio",
                    std::process::id()
                ));
                let dl = Command::new("curl")
                    .args(["-fsSL", "--max-time", "25", "-o"])
                    .arg(&tmp)
                    .arg(&url_owned)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                if gen_ref.load(Ordering::SeqCst) != play_id {
                    let _ = std::fs::remove_file(&tmp);
                    return;
                }
                if dl.map(|s| s.success()).unwrap_or(false) {
                    if let Ok(mut child) = Command::new("afplay")
                        .args(["-v", &format!("{vol:.2}")])
                        .arg(&tmp)
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                    {
                        if let Ok(mut snap) = snap_ref.lock() {
                            snap.state = PlaybackState::Playing;
                        }
                        repaint.request_repaint();
                        let start = Instant::now();
                        if let Ok(mut slot) = child_slot.lock() {
                            *slot = Some(child);
                        } else {
                            let _ = child.kill();
                            let _ = std::fs::remove_file(&tmp);
                            return;
                        }
                        loop {
                            if gen_ref.load(Ordering::SeqCst) != play_id {
                                if let Ok(mut slot) = child_slot.lock()
                                    && let Some(mut c) = slot.take()
                                {
                                    let _ = c.kill();
                                    let _ = c.wait();
                                }
                                break;
                            }
                            let done = child_slot
                                .lock()
                                .ok()
                                .and_then(|mut s| {
                                    s.as_mut().map(|c| c.try_wait().ok().flatten().is_some())
                                })
                                .unwrap_or(true);
                            if done {
                                if let Ok(mut slot) = child_slot.lock() {
                                    let _ = slot.take();
                                }
                                if gen_ref.load(Ordering::SeqCst) == play_id
                                    && let Ok(mut snap) = snap_ref.lock()
                                {
                                    snap.state = PlaybackState::Stopped;
                                    snap.ended = true;
                                }
                                break;
                            }
                            if let Ok(mut snap) = snap_ref.lock() {
                                snap.position_secs = start.elapsed().as_secs_f64();
                            }
                            repaint.request_repaint();
                            thread::sleep(Duration::from_millis(250));
                        }
                    } else if let Ok(mut snap) = snap_ref.lock() {
                        snap.state = PlaybackState::Stopped;
                        snap.error = Some("Không khởi chạy được afplay trên macOS.".into());
                    }
                } else if let Ok(mut snap) = snap_ref.lock() {
                    snap.state = PlaybackState::Stopped;
                    snap.error = Some("Không tải được luồng âm thanh trên macOS.".into());
                }
                let _ = std::fs::remove_file(&tmp);
                repaint.request_repaint();
            });
            Ok(())
        }
        #[cfg(all(not(windows), not(target_os = "macos")))]
        {
            let _ = (volume, ctx);
            Err("Phát stream trực tiếp trong Mini Music Player hỗ trợ Windows và macOS.".into())
        }
    }

    pub fn pause(&mut self) {
        #[cfg(windows)]
        {
            if let Ok(mut snap) = self.snapshot.lock()
                && matches!(snap.state, PlaybackState::Playing | PlaybackState::Opening)
            {
                snap.state = PlaybackState::Paused;
            }
            let _ = self.send_cmd("PAUSE");
        }
        #[cfg(target_os = "macos")]
        {
            if let Ok(slot) = self.mac_child.lock()
                && let Some(c) = slot.as_ref()
            {
                let _ = Command::new("kill")
                    .args(["-STOP", &c.id().to_string()])
                    .status();
                if let Ok(mut snap) = self.snapshot.lock()
                    && snap.state == PlaybackState::Playing
                {
                    snap.state = PlaybackState::Paused;
                }
            }
        }
    }

    pub fn resume(&mut self) {
        #[cfg(windows)]
        {
            if let Ok(mut snap) = self.snapshot.lock()
                && snap.state == PlaybackState::Paused
                && !snap.ended
            {
                snap.state = PlaybackState::Playing;
            }
            let _ = self.send_cmd("RESUME");
        }
        #[cfg(target_os = "macos")]
        {
            if let Ok(slot) = self.mac_child.lock()
                && let Some(c) = slot.as_ref()
            {
                let _ = Command::new("kill")
                    .args(["-CONT", &c.id().to_string()])
                    .status();
                if let Ok(mut snap) = self.snapshot.lock()
                    && snap.state == PlaybackState::Paused
                {
                    snap.state = PlaybackState::Playing;
                }
            }
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        #[cfg(windows)]
        let _ = self.send_cmd(&format!("VOL {:.2}", volume.clamp(0.0, 1.0)));
        #[cfg(not(windows))]
        let _ = volume;
    }

    pub fn seek(&mut self, seconds: f64) {
        #[cfg(windows)]
        let _ = self.send_cmd(&format!("SEEK {:.2}", seconds.max(0.0)));
        #[cfg(not(windows))]
        let _ = seconds;
    }

    /// Stops current audio immediately while keeping the warm background player ready.
    pub fn stop_playback(&mut self) {
        self.play_gen.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut snap) = self.snapshot.lock() {
            *snap = PlaybackSnapshot::default();
        }
        #[cfg(windows)]
        if self.worker.is_some() {
            let _ = self.send_cmd("STOP");
        }
        #[cfg(target_os = "macos")]
        if let Ok(mut slot) = self.mac_child.lock()
            && let Some(mut c) = slot.take()
        {
            let _ = c.kill();
            let _ = c.wait();
        }
    }

    pub fn stop(&mut self) {
        self.play_gen.fetch_add(1, Ordering::SeqCst);
        #[cfg(windows)]
        if let Some(mut worker) = self.worker.take() {
            worker.stopping.store(true, Ordering::SeqCst);
            let _ = writeln!(worker.stdin, "QUIT");
            let _ = worker.stdin.flush();
            integrations::stop_child(&mut worker.child);
        }
        #[cfg(target_os = "macos")]
        if let Ok(mut slot) = self.mac_child.lock()
            && let Some(mut c) = slot.take()
        {
            let _ = c.kill();
            let _ = c.wait();
        }
        let mut snap = self.snapshot.lock().unwrap();
        *snap = PlaybackSnapshot::default();
    }

    #[cfg(windows)]
    fn send_cmd(&mut self, cmd: &str) -> Result<(), String> {
        let Some(worker) = self.worker.as_mut() else {
            return Err("Audio worker chưa sẵn sàng.".into());
        };
        writeln!(worker.stdin, "{cmd}").map_err(|e| format!("Lỗi gửi lệnh audio: {e}"))?;
        worker
            .stdin
            .flush()
            .map_err(|e| format!("Lỗi flush lệnh audio: {e}"))
    }

    #[cfg(windows)]
    fn ensure_worker(&mut self, ctx: &egui::Context) -> Result<(), String> {
        if let Some(worker) = self.worker.as_mut()
            && worker.child.try_wait().ok().flatten().is_none()
        {
            return Ok(());
        }
        let script = r#"
$ErrorActionPreference = 'SilentlyContinue'
[Windows.Media.Playback.MediaPlayer, Windows.Media.Playback, ContentType=WindowsRuntime] | Out-Null
[Windows.Media.Core.MediaSource, Windows.Media.Core, ContentType=WindowsRuntime] | Out-Null
$player = [Windows.Media.Playback.MediaPlayer]::new()
$player.Volume = 0.85
Register-ObjectEvent -InputObject $player -EventName MediaEnded -SourceIdentifier 'PKMediaEnded' | Out-Null
# StreamReader over raw stdin avoids SyncTextReader.ReadLineAsync synchronous blocking in .NET 4.x.
$inReader = [System.IO.StreamReader]::new([Console]::OpenStandardInput(), [Text.Encoding]::UTF8)
$readTask = $inReader.ReadLineAsync()
$lastSig = ''
$lastPos = -1.0
$gen = 0
$wantPlay = $false
$trackStarted = $false
$endedFlag = $false
while ($true) {
    $cmdCount = 0
    $shouldQuit = $false
    while ($readTask.IsCompleted) {
        $line = $readTask.Result
        if ($null -eq $line -or $line -eq 'QUIT') { $shouldQuit = $true; break }
        $cmdCount++
        if ($line.StartsWith('GEN ')) {
            $g = 0
            if ([int]::TryParse($line.Substring(4).Trim(), [ref]$g)) { $gen = $g }
        } elseif ($line.StartsWith('PLAY ')) {
            $url = $line.Substring(5).Trim()
            $uri = $null
            if ([Uri]::TryCreate($url, [UriKind]::Absolute, [ref]$uri) -and ($uri.Scheme -eq 'https' -or $uri.Scheme -eq 'http')) {
                $endedFlag = $false
                $trackStarted = $false
                $wantPlay = $true
                Remove-Event -SourceIdentifier 'PKMediaEnded' -ErrorAction SilentlyContinue
                $player.Source = [Windows.Media.Core.MediaSource]::CreateFromUri($uri)
                $player.Play()
            }
        } elseif ($line -eq 'PAUSE') {
            $wantPlay = $false
            $player.Pause()
        } elseif ($line -eq 'RESUME') {
            $wantPlay = $true
            $player.Play()
        } elseif ($line.StartsWith('VOL ')) {
            $v = 0.85
            if ([double]::TryParse($line.Substring(4), [Globalization.NumberStyles]::Float, [Globalization.CultureInfo]::InvariantCulture, [ref]$v)) {
                $player.Volume = [Math]::Max(0.0, [Math]::Min(1.0, $v))
            }
        } elseif ($line.StartsWith('SEEK ')) {
            $sec = 0.0
            if ([double]::TryParse($line.Substring(5), [Globalization.NumberStyles]::Float, [Globalization.CultureInfo]::InvariantCulture, [ref]$sec)) {
                $endedFlag = $false
                Remove-Event -SourceIdentifier 'PKMediaEnded' -ErrorAction SilentlyContinue
                $player.PlaybackSession.Position = [TimeSpan]::FromSeconds([Math]::Max(0.0, $sec))
            }
        } elseif ($line -eq 'STOP') {
            $wantPlay = $false
            $trackStarted = $false
            $endedFlag = $false
            Remove-Event -SourceIdentifier 'PKMediaEnded' -ErrorAction SilentlyContinue
            $player.Pause()
            $player.Source = $null
        }
        $readTask = $inReader.ReadLineAsync()
    }
    if ($shouldQuit) { break }
    if (Get-Event -SourceIdentifier 'PKMediaEnded' -ErrorAction SilentlyContinue) {
        Remove-Event -SourceIdentifier 'PKMediaEnded' -ErrorAction SilentlyContinue
        $endedFlag = $true
    }
    $s = $player.PlaybackSession
    $rawSt = "$($s.PlaybackState)"
    $posNum = [Math]::Round($s.Position.TotalSeconds, 1)
    $durNum = [Math]::Round($s.NaturalDuration.TotalSeconds, 1)
    if ($rawSt -eq 'Playing') { $trackStarted = $true }
    $emitSt = if ($rawSt -eq 'Paused' -and $wantPlay -and -not $trackStarted -and -not $endedFlag) { 'Opening' } else { $rawSt }
    $nearEnd = ($durNum -gt 1.0 -and $posNum -ge ($durNum - 1.0) -and $posNum -ne $lastPos)
    $posFloor = [Math]::Floor($s.Position.TotalSeconds)
    $durStr = $durNum.ToString([Globalization.CultureInfo]::InvariantCulture)
    $sig = "$gen|$emitSt|$posFloor|$durStr|$endedFlag"
    if ($cmdCount -gt 0 -or $nearEnd -or $sig -ne $lastSig) {
        $lastSig = $sig
        $lastPos = $posNum
        $posStr = $posNum.ToString([Globalization.CultureInfo]::InvariantCulture)
        [Console]::Out.WriteLine("STATE|$emitSt|$posStr|$durStr|$endedFlag|$gen")
        [Console]::Out.Flush()
    }
    Start-Sleep -Milliseconds 100
}
Unregister-Event -SourceIdentifier 'PKMediaEnded' -ErrorAction SilentlyContinue
$player.Pause()
$player.Dispose()
"#;
        use std::os::windows::process::CommandExt;
        let mut cmd = std::process::Command::new("powershell.exe");
        cmd.args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(0x08000000);
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Không khởi động được trình phát audio: {e}"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Không mở được stdin audio.".to_owned())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Không mở được stdout audio.".to_owned())?;
        let stopping = Arc::new(AtomicBool::new(false));
        let stop_flag = stopping.clone();
        let snap_ref = self.snapshot.clone();
        let gen_ref = self.play_gen.clone();
        let repaint_ctx = ctx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }
                if let Some(rest) = line.strip_prefix("STATE|") {
                    let parts: Vec<&str> = rest.split('|').collect();
                    if parts.len() >= 4 {
                        let expected_gen = gen_ref.load(Ordering::SeqCst);
                        let msg_gen = parts
                            .get(4)
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(expected_gen);
                        if msg_gen != expected_gen {
                            continue;
                        }
                        let state = match parts[0] {
                            "Playing" => PlaybackState::Playing,
                            "Paused" => PlaybackState::Paused,
                            "Opening" | "Buffering" => PlaybackState::Opening,
                            _ => PlaybackState::Stopped,
                        };
                        let pos = parts[1].parse::<f64>().unwrap_or(0.0);
                        let dur = parts[2].parse::<f64>().unwrap_or(0.0);
                        let ended = parts[3].eq_ignore_ascii_case("true")
                            || (dur > 1.0 && pos >= dur - 0.65 && state != PlaybackState::Playing);
                        let changed = {
                            let mut snap = snap_ref.lock().unwrap();
                            let dirty = snap.state != state
                                || snap.ended != ended
                                || (snap.duration_secs - dur).abs() > 0.4
                                || (snap.position_secs - pos).abs() >= 0.8;
                            snap.state = state;
                            snap.position_secs = pos;
                            snap.duration_secs = dur;
                            snap.ended = ended;
                            dirty
                        };
                        if changed {
                            repaint_ctx.request_repaint();
                        }
                    }
                }
            }
        });
        self.worker = Some(AudioWorker {
            child,
            stdin,
            stopping,
        });
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MusicFetchMode {
    #[default]
    ChatAutoPlay,
    PickerPreviewOnly,
    PickerAutoPlay,
    AutoDiscoverAppend,
}

pub struct MiniMusicState {
    pub open: bool,
    pub queue_open: bool,
    pub collapsed_disc: bool,
    pub last_interaction: Instant,
    pub tracks: Vec<Track>,
    pub current_index: usize,
    pub active_track: Option<Track>,
    pub search_query: String,
    pub discovery_seed: String,
    pub shuffle: bool,
    pub repeat: bool,
    pub volume: f32,
    pub status: Option<String>,
    pub engine: AudioEngine,
    pub thumbs: HashMap<String, Option<egui::TextureHandle>>,
    pub thumb_jobs: HashMap<String, crate::system::Job<Option<Vec<u8>>>>,
    played_ids: HashSet<String>,
    discover_requested: bool,
    was_ended: bool,
}

impl Default for MiniMusicState {
    fn default() -> Self {
        Self {
            open: false,
            queue_open: false,
            collapsed_disc: false,
            last_interaction: Instant::now(),
            tracks: Vec::new(),
            current_index: 0,
            active_track: None,
            search_query: String::new(),
            discovery_seed: String::new(),
            shuffle: false,
            repeat: false,
            volume: 0.85,
            status: None,
            engine: AudioEngine::default(),
            thumbs: HashMap::new(),
            thumb_jobs: HashMap::new(),
            played_ids: HashSet::new(),
            discover_requested: false,
            was_ended: false,
        }
    }
}

impl MiniMusicState {
    pub fn touch(&mut self) {
        self.last_interaction = Instant::now();
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.active_track
            .as_ref()
            .or_else(|| self.tracks.get(self.current_index))
    }

    pub fn exclude_video_ids(&self) -> Vec<String> {
        let mut ids: HashSet<String> = self.played_ids.clone();
        for t in &self.tracks {
            if !t.video_id.is_empty() {
                ids.insert(t.video_id.clone());
            }
        }
        ids.into_iter().collect()
    }

    pub fn hit_playlist_end(&self) -> bool {
        if self.repeat || self.tracks.is_empty() {
            return false;
        }
        if self.shuffle {
            self.tracks
                .iter()
                .all(|t| !t.video_id.is_empty() && self.played_ids.contains(&t.video_id))
        } else {
            self.current_index + 1 >= self.tracks.len()
        }
    }

    pub fn derive_discovery_query(&self) -> String {
        if let Some(cur) = self.current_track() {
            let artist = cur.artist.trim();
            if !artist.is_empty() && !artist.eq_ignore_ascii_case("Penta Music MCP") {
                return format!("{artist} nhạc cùng gu mới");
            }
        }
        if !self.discovery_seed.trim().is_empty() {
            return self.discovery_seed.trim().to_owned();
        }
        "nhạc chill acoustic việt nam mới hay".to_owned()
    }

    pub fn take_discover_request(&mut self) -> Option<String> {
        if self.discover_requested {
            self.discover_requested = false;
            Some(self.derive_discovery_query())
        } else {
            None
        }
    }

    /// Appends newly discovered tracks (deduplicated by `video_id`) and optionally plays the first new one.
    pub fn append_discovered_tracks(
        &mut self,
        incoming: Vec<Track>,
        auto_play_first_new: bool,
        ctx: &egui::Context,
    ) -> usize {
        let existing: HashSet<String> = self
            .tracks
            .iter()
            .filter(|t| !t.video_id.is_empty())
            .map(|t| t.video_id.clone())
            .collect();
        let start_idx = self.tracks.len();
        let mut added = 0;
        for track in incoming {
            if !track.video_id.is_empty() && !existing.contains(&track.video_id) {
                self.tracks.push(track);
                added += 1;
            }
        }
        if added > 0 && auto_play_first_new {
            self.play_index(start_idx, ctx);
        }
        added
    }

    /// Loads candidate tracks into the compact picker without forcing autoplay.
    pub fn load_tracks(&mut self, tracks: Vec<Track>) {
        self.tracks = tracks;
        self.open = true;
        self.queue_open = true;
        self.collapsed_disc = false;
        self.touch();
        self.status = None;
        if let Some(active) = &self.active_track {
            self.current_index = self
                .tracks
                .iter()
                .position(|t| !active.video_id.is_empty() && t.video_id == active.video_id)
                .unwrap_or(0);
        } else {
            self.current_index = 0;
        }
    }

    pub fn load_and_play(&mut self, tracks: Vec<Track>, ctx: &egui::Context) {
        self.tracks = tracks;
        self.current_index = 0;
        self.open = !self.tracks.is_empty();
        self.collapsed_disc = false;
        self.discover_requested = false;
        self.was_ended = false;
        self.touch();
        if !self.tracks.is_empty() {
            self.play_index(0, ctx);
        }
    }

    pub fn play_index(&mut self, index: usize, ctx: &egui::Context) {
        if index >= self.tracks.len() {
            return;
        }
        self.current_index = index;
        let track = self.tracks[index].clone();
        if !track.video_id.is_empty() {
            self.played_ids.insert(track.video_id.clone());
        }
        if !track.artist.trim().is_empty()
            && !track.artist.trim().eq_ignore_ascii_case("Penta Music MCP")
        {
            self.discovery_seed = format!("{} nhạc cùng gu", track.artist.trim());
        }
        self.active_track = Some(track.clone());
        self.was_ended = false;
        let stream_url = track.stream_url;
        let id = track.video_id;
        let thumb_url = track.thumbnail;
        if !id.is_empty()
            && !thumb_url.is_empty()
            && !self.thumbs.contains_key(&id)
            && !self.thumb_jobs.contains_key(&id)
        {
            let job = crate::system::Job::spawn(ctx, move || fetch_thumbnail_png(&thumb_url));
            self.thumb_jobs.insert(id, job);
        }
        if let Err(err) = self.engine.play(&stream_url, self.volume, ctx) {
            self.status = Some(err);
        } else {
            self.status = None;
        }
    }

    pub fn toggle_play_pause(&mut self, ctx: &egui::Context) {
        self.touch();
        let snap = self.engine.snapshot();
        match snap.state {
            PlaybackState::Playing | PlaybackState::Opening => self.engine.pause(),
            PlaybackState::Paused if snap.ended => {
                if !self.tracks.is_empty() {
                    self.play_index(self.current_index, ctx);
                }
            }
            PlaybackState::Paused => self.engine.resume(),
            PlaybackState::Stopped => {
                if !self.tracks.is_empty() {
                    self.play_index(self.current_index, ctx);
                }
            }
        }
    }

    pub fn next(&mut self, ctx: &egui::Context) {
        if self.tracks.is_empty() {
            return;
        }
        let next_idx = if self.shuffle && self.tracks.len() > 1 {
            let unplayed: Vec<usize> = self
                .tracks
                .iter()
                .enumerate()
                .filter(|(i, t)| *i != self.current_index && !self.played_ids.contains(&t.video_id))
                .map(|(i, _)| i)
                .collect();
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos() as usize)
                .unwrap_or(1);
            if !unplayed.is_empty() {
                unplayed[secs % unplayed.len()]
            } else {
                let candidate = secs % self.tracks.len();
                if candidate == self.current_index {
                    (self.current_index + 1) % self.tracks.len()
                } else {
                    candidate
                }
            }
        } else {
            (self.current_index + 1) % self.tracks.len()
        };
        self.play_index(next_idx, ctx);
    }

    pub fn next_or_discover(&mut self, can_discover: bool, ctx: &egui::Context) {
        self.touch();
        if can_discover && self.hit_playlist_end() {
            self.discover_requested = true;
            self.status = Some("AI đang tìm bài hát mới cùng gu…".into());
        } else {
            self.next(ctx);
        }
    }

    pub fn prev(&mut self, ctx: &egui::Context) {
        self.touch();
        if self.tracks.is_empty() {
            return;
        }
        let prev_idx = if self.current_index == 0 {
            self.tracks.len() - 1
        } else {
            self.current_index - 1
        };
        self.play_index(prev_idx, ctx);
    }

    pub fn close(&mut self) {
        self.open = false;
        self.queue_open = false;
        self.collapsed_disc = false;
        self.discover_requested = false;
        self.active_track = None;
        self.played_ids.clear();
        self.engine.stop();
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        let keys: Vec<String> = self.thumb_jobs.keys().cloned().collect();
        for key in keys {
            let mut slot = self.thumb_jobs.remove(&key);
            if let Some(bytes_opt) = crate::system::take(&mut slot) {
                let tex = bytes_opt.and_then(|bytes| {
                    crate::system::load_png_color_image(&bytes).ok().map(|img| {
                        ctx.load_texture(
                            format!("music_thumb_{key}"),
                            img,
                            egui::TextureOptions::LINEAR,
                        )
                    })
                });
                self.thumbs.insert(key, tex);
            } else if let Some(pending) = slot {
                self.thumb_jobs.insert(key, pending);
            }
        }

        if !self.open {
            return;
        }
        let snap = self.engine.snapshot();
        if snap.ended && !self.was_ended {
            self.was_ended = true;
            if self.repeat {
                self.play_index(self.current_index, ctx);
            } else if self.hit_playlist_end() {
                self.discover_requested = true;
            } else if !self.tracks.is_empty() {
                self.next(ctx);
            }
        } else if !snap.ended {
            self.was_ended = false;
        }
        if matches!(snap.state, PlaybackState::Playing | PlaybackState::Opening) {
            ctx.request_repaint_after(Duration::from_millis(if self.collapsed_disc {
                90
            } else {
                500
            }));
        }
    }
}

/// Downloads a streamed audio track into the user's Downloads folder.
pub fn download_track_file(url: &str, title: &str) -> Result<String, String> {
    let clean = url.trim();
    if !clean.starts_with("https://") && !clean.starts_with("http://") {
        return Err("URL tải nhạc không hợp lệ.".into());
    }
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let downloads = home.join("Downloads");
    let _ = std::fs::create_dir_all(&downloads);
    let mut safe_title: String = title
        .chars()
        .filter(|c| !matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .take(72)
        .collect();
    let safe_trimmed = safe_title.trim();
    if safe_trimmed.is_empty() {
        safe_title = "pentakuru-track".into();
    } else {
        safe_title = safe_trimmed.to_owned();
    }
    let target = downloads.join(format!("{safe_title}.m4a"));
    let mut resp = agent()
        .get(clean)
        .call()
        .map_err(|e| format!("Lỗi tải bài hát: {e}"))?;
    if !(200..300).contains(&resp.status().as_u16()) {
        return Err(format!("Máy chủ trả về mã {}.", resp.status().as_u16()));
    }
    let bytes = resp
        .body_mut()
        .read_to_vec()
        .map_err(|e| format!("Lỗi đọc dữ liệu âm thanh: {e}"))?;
    std::fs::write(&target, &bytes).map_err(|e| format!("Không lưu được file tải: {e}"))?;
    Ok(format!("Đã tải về Downloads/{safe_title}.m4a"))
}

fn is_webp_bytes(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP"
}

fn thumbnail_candidate_urls(url: &str) -> Vec<String> {
    let clean = url.trim();
    if !clean.starts_with("https://") && !clean.starts_with("http://") {
        return Vec::new();
    }
    let mut out = Vec::new();
    let is_ytimg = clean.contains("i.ytimg.com/")
        || clean.contains("ytimg.com/")
        || clean.contains("img.youtube.com/");
    if is_ytimg {
        let base = clean.split('?').next().unwrap_or(clean);
        let mut normalized = base.replace("/vi_webp/", "/vi/");
        if let Some(stem) = normalized.strip_suffix(".webp") {
            normalized = format!("{stem}.jpg");
        }
        // i.ytimg.com frequently serves WebP when `?sqp=` or `.webp` is present; try clean JPEG first.
        if clean.contains("?sqp=") || clean.contains(".webp") {
            out.push(normalized.clone());
            out.push(clean.to_owned());
        } else {
            out.push(clean.to_owned());
            if normalized != clean {
                out.push(normalized.clone());
            }
        }
        if let Some(video_id) = normalized
            .split("/vi/")
            .nth(1)
            .and_then(|rest| rest.split('/').next())
            .filter(|id| !id.is_empty())
        {
            let hq = format!("https://i.ytimg.com/vi/{video_id}/hqdefault.jpg");
            if !out.contains(&hq) {
                out.push(hq);
            }
        }
    } else {
        out.push(clean.to_owned());
    }
    out.dedup();
    out
}

fn fetch_thumbnail_png(url: &str) -> Option<Vec<u8>> {
    for candidate in thumbnail_candidate_urls(url) {
        let Ok(mut resp) = agent().get(&candidate).call() else {
            continue;
        };
        if !(200..300).contains(&resp.status().as_u16()) {
            continue;
        }
        let Ok(bytes) = resp.body_mut().read_to_vec() else {
            continue;
        };
        if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
            return Some(bytes);
        }
        if is_webp_bytes(&bytes) {
            continue;
        }
        #[cfg(windows)]
        if let Some(png) = convert_image_bytes_to_square_png(&bytes) {
            return Some(png);
        }
    }
    None
}

#[cfg(windows)]
fn convert_image_bytes_to_square_png(bytes: &[u8]) -> Option<Vec<u8>> {
    use std::os::windows::process::CommandExt;
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let in_path = std::env::temp_dir().join(format!("pentakuru-thumb-{unique}.img"));
    let out_path = std::env::temp_dir().join(format!("pentakuru-thumb-{unique}.png"));
    std::fs::write(&in_path, bytes).ok()?;
    let script = r#"
Add-Type -AssemblyName System.Drawing
$img = [System.Drawing.Image]::FromFile($env:PK_IN)
$side = [Math]::Min($img.Width, $img.Height)
$x = [int](($img.Width - $side) / 2)
$y = [int](($img.Height - $side) / 2)
$bmp = New-Object System.Drawing.Bitmap(96, 96)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.DrawImage($img, (New-Object System.Drawing.Rectangle(0,0,96,96)), (New-Object System.Drawing.Rectangle($x,$y,$side,$side)), [System.Drawing.GraphicsUnit]::Pixel)
$bmp.Save($env:PK_OUT, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose(); $img.Dispose()
"#;
    let status = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .env("PK_IN", &in_path)
        .env("PK_OUT", &out_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .status()
        .ok();
    let _ = std::fs::remove_file(&in_path);
    let result = if status.is_some_and(|s| s.success()) {
        std::fs::read(&out_path).ok()
    } else {
        None
    };
    let _ = std::fs::remove_file(&out_path);
    result
}

pub fn format_time(seconds: f64) -> String {
    let total = seconds.max(0.0).floor() as u64;
    format!("{}:{:02}", total / 60, total % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_music_and_clip_direct_commands() {
        assert!(matches!(
            parse_direct_command("mở nhạc hợp gu", true, true),
            Some(MediaCommand::MusicTool { ref tool, .. }) if tool == "blend_history_and_taste"
        ));
        assert!(matches!(
            parse_direct_command("mở nhạc cho anh nghe đi", true, false),
            Some(MediaCommand::MusicTool { ref tool, .. }) if tool == "blend_history_and_taste"
        ));
        assert!(matches!(
            parse_direct_command("em mở nhạc giúp anh", true, false),
            Some(MediaCommand::MusicTool { ref tool, .. }) if tool == "blend_history_and_taste"
        ));
        assert!(matches!(
            parse_direct_command("cho anh nghe nhạc đi!", true, false),
            Some(MediaCommand::MusicTool { ref tool, .. }) if tool == "blend_history_and_taste"
        ));
        assert!(matches!(
            parse_direct_command("phát bài Nàng Thơ cho anh nghe", true, false),
            Some(MediaCommand::MusicTool { ref tool, ref arguments, .. })
                if tool == "search_and_stream_music" && arguments["query"] == "nàng thơ"
        ));
        assert!(matches!(
            parse_direct_command("qua bài!", true, true),
            Some(MediaCommand::MusicControl(PlayerAction::Next))
        ));
        assert!(matches!(
            parse_direct_command("tìm video lofi chill cho anh xem", true, true),
            Some(MediaCommand::ClipTool { ref tool, ref arguments, .. })
                if tool == "search_videos" && arguments["query"] == "lofi chill"
        ));
        assert!(matches!(
            parse_direct_command("mở video đề xuất đi", false, true),
            Some(MediaCommand::ClipTool { ref tool, .. }) if tool == "get_recommendations"
        ));
        assert!(matches!(
            parse_direct_command("đóng video", false, true),
            Some(MediaCommand::ClipClose)
        ));
        assert!(parse_direct_command("@note viết về mở nhạc", true, true).is_none());
        assert!(parse_direct_command("viết bài về mở nhạc cho anh", true, true).is_none());
        assert!(parse_direct_command("tại sao không mở nhạc được", true, true).is_none());
        assert!(matches!(
            parse_direct_command("tìm bài mới acoustic cho anh", true, false),
            Some(MediaCommand::MusicTool { ref tool, ref arguments, .. })
                if tool == "find_and_ingest_tracks" && arguments["query"] == "acoustic"
        ));
        assert!(matches!(
            parse_direct_command("em mở bài khác được không bài gì vui vẻ tí xíu", true, false),
            Some(MediaCommand::MusicTool { ref tool, ref arguments, .. })
                if tool == "find_and_ingest_tracks"
                    && arguments["query"].as_str().is_some_and(|q| q.contains("vui vẻ"))
        ));
    }

    #[test]
    fn playlist_end_detection_and_append_discovered_tracks() {
        let ctx = egui::Context::default();
        let mut st = MiniMusicState {
            tracks: vec![
                Track {
                    video_id: "a1".into(),
                    title: "Track 1".into(),
                    artist: "Artist X".into(),
                    ..Default::default()
                },
                Track {
                    video_id: "a2".into(),
                    title: "Track 2".into(),
                    artist: "Artist Y".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert!(!st.hit_playlist_end());
        st.current_index = 1;
        assert!(st.hit_playlist_end());

        let added = st.append_discovered_tracks(
            vec![
                Track {
                    video_id: "a2".into(),
                    title: "Dup".into(),
                    ..Default::default()
                },
                Track {
                    video_id: "b3".into(),
                    title: "New Discovery".into(),
                    ..Default::default()
                },
            ],
            false,
            &ctx,
        );
        assert_eq!(added, 1);
        assert_eq!(st.tracks.len(), 3);
        assert!(!st.hit_playlist_end());
    }

    #[test]
    fn extracts_and_strips_ai_media_tags() {
        let (clean, cmd) = extract_ai_media_tags(
            "Anh nghe list nhạc nhẹ này nhé.\n[[MCP_MUSIC:blend|chill nhẹ nhàng]]",
        );
        assert_eq!(clean, "Anh nghe list nhạc nhẹ này nhé.");
        assert!(matches!(
            cmd,
            Some(MediaCommand::MusicTool { ref tool, ref arguments, .. })
                if tool == "blend_history_and_taste" && arguments["focus_query"] == "chill nhẹ nhàng"
        ));

        let (clean_clip, clip_cmd) =
            extract_ai_media_tags("Mình mở clip AI ngay.\n[[MCP_CLIP:search|AI truth]]");
        assert_eq!(clean_clip, "Mình mở clip AI ngay.");
        assert!(matches!(
            clip_cmd,
            Some(MediaCommand::ClipTool { ref tool, ref arguments, .. })
                if tool == "search_videos" && arguments["query"] == "AI truth"
        ));
    }

    #[test]
    fn extracts_videos_from_clip_mcp_payloads() {
        let recs = json!({
            "recommendations": [
                {"videoId": "N1P8dhu6PmY", "title": "AI Truth", "author": "DOAC", "lengthSeconds": 120}
            ]
        });
        assert_eq!(extract_videos(&recs)[0].video_id, "N1P8dhu6PmY");

        let search = json!({
            "results": [
                {"type": "channel", "author": "Ignore"},
                {"type": "video", "videoId": "0zqOEniTZH4", "title": "Lofi", "author": "Bui"}
            ]
        });
        let list = extract_videos(&search);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].video_id, "0zqOEniTZH4");
    }

    #[test]
    fn builds_jpeg_fallback_urls_for_ytimg_webp() {
        let candidates = thumbnail_candidate_urls(
            "https://i.ytimg.com/vi_webp/oWENAdVkHRk/hq720.webp?sqp=-oaymwEc",
        );
        assert_eq!(
            candidates.first().map(String::as_str),
            Some("https://i.ytimg.com/vi/oWENAdVkHRk/hq720.jpg")
        );
        assert!(
            candidates
                .iter()
                .any(|u| u == "https://i.ytimg.com/vi/oWENAdVkHRk/hqdefault.jpg")
        );
    }

    #[test]
    #[ignore = "Calls live Penta Music and Penta Clip MCP servers using env keys"]
    fn live_mcp_smoke_test() {
        let music_key =
            std::env::var("PENTAKURU_MUSIC_MCP_KEY").expect("Set PENTAKURU_MUSIC_MCP_KEY");
        let clip_key = std::env::var("PENTAKURU_CLIP_MCP_KEY").expect("Set PENTAKURU_CLIP_MCP_KEY");

        let music_status = test_music_mcp(integrations::DEFAULT_MUSIC_MCP_URL, &music_key)
            .expect("Music MCP test failed");
        println!("MUSIC_STATUS={music_status}");
        let music_res = call_music_mcp(
            integrations::DEFAULT_MUSIC_MCP_URL,
            &music_key,
            "blend_history_and_taste",
            json!({"limit": 3, "remote_play": false}),
        )
        .expect("Music blend failed");
        assert!(!music_res.tracks.is_empty());
        let first_track = &music_res.tracks[0];
        println!(
            "MUSIC_TRACK_0={} | {} | {}",
            first_track.title, first_track.artist, first_track.stream_url
        );
        if !first_track.thumbnail.is_empty() {
            let png = fetch_thumbnail_png(&first_track.thumbnail).expect("Thumbnail PNG decode");
            assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
            println!("MUSIC_THUMB_PNG_BYTES={}", png.len());
        }

        let clip_status = test_clip_mcp(integrations::DEFAULT_CLIP_MCP_URL, &clip_key)
            .expect("Clip MCP test failed");
        println!("CLIP_STATUS={clip_status}");
        let clip_recs = call_clip_mcp(
            integrations::DEFAULT_CLIP_MCP_URL,
            &clip_key,
            "get_recommendations",
            json!({"limit": 2}),
        )
        .expect("Clip recommendations failed");
        assert!(!clip_recs.videos.is_empty());
        println!(
            "CLIP_REC_0={} | {} | {}",
            clip_recs.videos[0].video_id, clip_recs.videos[0].title, clip_recs.videos[0].url
        );
        let clip_search = call_clip_mcp(
            integrations::DEFAULT_CLIP_MCP_URL,
            &clip_key,
            "search_videos",
            json!({"query": "lofi chill", "type": "video", "limit": 2}),
        )
        .expect("Clip search failed");
        assert!(!clip_search.videos.is_empty());
        println!(
            "CLIP_SEARCH_0={} | {}",
            clip_search.videos[0].video_id, clip_search.videos[0].title
        );

        #[cfg(windows)]
        {
            let ctx = egui::Context::default();
            let mut engine = AudioEngine::default();
            engine
                .play(&first_track.stream_url, 0.0, &ctx)
                .expect("AudioEngine play");
            let deadline = Instant::now() + Duration::from_secs(12);
            let mut started = PlaybackSnapshot::default();
            while Instant::now() < deadline {
                let s = engine.snapshot();
                if s.state == PlaybackState::Playing && s.duration_secs > 1.0 {
                    started = s;
                    break;
                }
                thread::sleep(Duration::from_millis(120));
            }
            assert_eq!(
                started.state,
                PlaybackState::Playing,
                "AudioEngine did not reach Playing state: {started:?}"
            );
            assert!(started.duration_secs > 1.0);

            engine.pause();
            assert_eq!(engine.snapshot().state, PlaybackState::Paused);
            thread::sleep(Duration::from_millis(350));
            assert_eq!(engine.snapshot().state, PlaybackState::Paused);

            engine.resume();
            let resume_deadline = Instant::now() + Duration::from_secs(4);
            while Instant::now() < resume_deadline {
                if engine.snapshot().state == PlaybackState::Playing {
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }
            assert_eq!(engine.snapshot().state, PlaybackState::Playing);

            engine.seek((started.duration_secs - 1.5).max(0.0));
            let end_deadline = Instant::now() + Duration::from_secs(6);
            let mut ended = false;
            while Instant::now() < end_deadline {
                if engine.snapshot().ended {
                    ended = true;
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }
            engine.stop();
            assert!(ended, "AudioEngine did not detect end of track");
            println!("AUDIO_ENGINE_LIVE=ok dur={:.1}", started.duration_secs);
        }
    }

    #[test]
    fn poll_advances_on_ended_and_requests_discovery_at_end() {
        let ctx = egui::Context::default();
        let mut st = MiniMusicState {
            open: true,
            tracks: vec![
                Track {
                    video_id: "t1".into(),
                    title: "One".into(),
                    artist: "A".into(),
                    ..Default::default()
                },
                Track {
                    video_id: "t2".into(),
                    title: "Two".into(),
                    artist: "B".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        {
            let mut snap = st.engine.snapshot.lock().unwrap();
            snap.state = PlaybackState::Paused;
            snap.duration_secs = 200.0;
            snap.position_secs = 200.0;
            snap.ended = true;
        }
        st.poll(&ctx);
        assert_eq!(st.current_index, 1);
        assert!(st.take_discover_request().is_none());

        {
            let mut snap = st.engine.snapshot.lock().unwrap();
            snap.state = PlaybackState::Paused;
            snap.duration_secs = 180.0;
            snap.position_secs = 180.0;
            snap.ended = true;
        }
        st.poll(&ctx);
        assert!(st.take_discover_request().is_some());
    }
}
