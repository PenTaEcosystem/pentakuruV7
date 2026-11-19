//! AI providers: detect the provider from an API key, list its models and
//! send chat requests. Runs on worker threads (see `system::Job`).
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Provider {
    OpenAi,
    Gemini,
    Anthropic,
    OpenRouter,
    Groq,
    DeepSeek,
    XAi,
    Ollama,
    Custom,
    #[default]
    #[serde(other)]
    Auto,
}

impl Provider {
    pub const ALL: [Provider; 10] = [
        Provider::Auto,
        Provider::OpenAi,
        Provider::Gemini,
        Provider::Anthropic,
        Provider::OpenRouter,
        Provider::Groq,
        Provider::DeepSeek,
        Provider::XAi,
        Provider::Ollama,
        Provider::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Provider::Auto => "Tự nhận diện từ API key",
            Provider::OpenAi => "OpenAI",
            Provider::Gemini => "Google Gemini",
            Provider::Anthropic => "Anthropic Claude",
            Provider::OpenRouter => "OpenRouter",
            Provider::Groq => "Groq",
            Provider::DeepSeek => "DeepSeek",
            Provider::XAi => "xAI Grok",
            Provider::Ollama => "Ollama (chạy trên máy)",
            Provider::Custom => "Custom · Proxy tương thích OpenAI",
        }
    }

    /// Provider implied by a key prefix.
    pub fn detect(key: &str) -> Option<Provider> {
        let key = key.trim();
        Some(if key.starts_with("sk-ant-") {
            Provider::Anthropic
        } else if key.starts_with("AIza") {
            Provider::Gemini
        } else if key.starts_with("sk-or-") {
            Provider::OpenRouter
        } else if key.starts_with("gsk_") {
            Provider::Groq
        } else if key.starts_with("xai-") {
            Provider::XAi
        } else if key.starts_with("sk-") {
            Provider::OpenAi
        } else {
            return None;
        })
    }

    pub fn default_base(self) -> &'static str {
        match self {
            Provider::Auto | Provider::OpenAi => "https://api.openai.com/v1",
            Provider::Gemini => "https://generativelanguage.googleapis.com/v1beta",
            Provider::Anthropic => "https://api.anthropic.com/v1",
            Provider::OpenRouter => "https://openrouter.ai/api/v1",
            Provider::Groq => "https://api.groq.com/openai/v1",
            Provider::DeepSeek => "https://api.deepseek.com/v1",
            Provider::XAi => "https://api.x.ai/v1",
            Provider::Ollama => "http://localhost:11434/v1",
            Provider::Custom => "http://localhost:8080/v1",
        }
    }

    pub fn needs_key(self) -> bool {
        !matches!(self, Provider::Ollama | Provider::Custom)
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AiSettings {
    pub provider: Provider,
    pub api_key: String,
    /// Empty = provider default.
    pub base_url: String,
    pub model: String,
    /// Last fetched model list (kept so the menu works offline).
    pub models: Vec<String>,
}

impl AiSettings {
    pub fn resolved(&self) -> Option<Provider> {
        match self.provider {
            Provider::Auto => Provider::detect(&self.api_key),
            other => Some(other),
        }
    }

    pub fn base(&self) -> String {
        let base = if self.base_url.trim().is_empty() {
            self.resolved()
                .unwrap_or(Provider::OpenAi)
                .default_base()
                .to_owned()
        } else {
            self.base_url.trim().to_owned()
        };
        base.trim_end_matches('/').to_owned()
    }

    pub fn can_list(&self) -> bool {
        self.resolved()
            .is_some_and(|p| !p.needs_key() || !self.api_key.trim().is_empty())
    }

    pub fn ready(&self) -> bool {
        self.can_list() && !self.model.trim().is_empty()
    }

    /// Short status for the UI: "Gemini · gemini-2.5-flash".
    #[allow(dead_code)]
    pub fn summary(&self) -> String {
        match self.resolved() {
            Some(p) if self.ready() => format!("{} · {}", p.label(), self.model),
            Some(p) => format!("{} · chưa chọn mô hình", p.label()),
            None => "Chưa cấu hình API key".into(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Clone, Default)]
pub struct ImageAttachment {
    pub mime_type: String,
    pub base64_data: String,
    pub file_path: Option<String>,
}

#[derive(Clone)]
pub struct Message {
    pub role: Role,
    pub text: String,
    pub images: Vec<ImageAttachment>,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(120)))
        .build()
        .into()
}

type HttpResult = Result<ureq::http::Response<ureq::Body>, ureq::Error>;

fn read(response: HttpResult) -> Result<Value, String> {
    let mut response = response.map_err(|e| format!("Lỗi mạng: {e}"))?;
    let status = response.status().as_u16();
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("Không đọc được phản hồi: {e}"))?;
    let value: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if status >= 400 {
        let message = value
            .pointer("/error/message")
            .or_else(|| value.get("error"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| text.chars().take(200).collect());
        return Err(match status {
            401 | 403 => format!("API key không hợp lệ hoặc không có quyền ({status}): {message}"),
            429 => format!("Hết hạn mức / gọi quá nhanh (429): {message}"),
            _ => format!("HTTP {status}: {message}"),
        });
    }
    if value.is_null() {
        return Err("Phản hồi không phải JSON".into());
    }
    Ok(value)
}

fn bearer(
    request: ureq::RequestBuilder<ureq::typestate::WithoutBody>,
    key: &str,
) -> ureq::RequestBuilder<ureq::typestate::WithoutBody> {
    if key.trim().is_empty() {
        request
    } else {
        request.header("Authorization", &format!("Bearer {}", key.trim()))
    }
}

/// Fetch available chat models for the configured provider.
pub fn list_models(settings: &AiSettings) -> Result<Vec<String>, String> {
    let provider = settings
        .resolved()
        .ok_or("Không nhận diện được nhà cung cấp từ API key. Hãy chọn thủ công.")?;
    let key = settings.api_key.trim();
    let base = settings.base();
    let agent = agent();
    let mut models: Vec<String> = match provider {
        Provider::Gemini => {
            let value = read(
                agent
                    .get(&format!("{base}/models"))
                    .query("pageSize", "1000")
                    .header("x-goog-api-key", key)
                    .call(),
            )?;
            value["models"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|m| {
                    m["supportedGenerationMethods"]
                        .as_array()
                        .is_some_and(|a| a.iter().any(|x| x == "generateContent"))
                })
                .filter_map(|m| m["name"].as_str())
                .map(|n| n.trim_start_matches("models/").to_owned())
                .collect()
        }
        Provider::Anthropic => {
            let value = read(
                agent
                    .get(&format!("{base}/models"))
                    .query("limit", "1000")
                    .header("x-api-key", key)
                    .header("anthropic-version", "2023-06-01")
                    .call(),
            )?;
            ids(&value)
        }
        _ => {
            let value = read(bearer(agent.get(&format!("{base}/models")), key).call())?;
            ids(&value)
        }
    };
    if provider == Provider::OpenAi {
        const SKIP: [&str; 12] = [
            "embedding",
            "whisper",
            "tts",
            "dall-e",
            "moderation",
            "davinci",
            "babbage",
            "audio",
            "realtime",
            "transcribe",
            "image",
            "search",
        ];
        models.retain(|m| !SKIP.iter().any(|s| m.contains(s)));
    }
    models.retain(|m| !m.contains("embed"));
    models.sort();
    models.dedup();
    if models.is_empty() {
        return Err("Không tìm thấy mô hình chat nào cho key này".into());
    }
    Ok(models)
}

fn ids(value: &Value) -> Vec<String> {
    value["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m["id"].as_str().map(str::to_owned))
        .collect()
}

/// Keep the current model if still offered; otherwise pick a fast, current one.
pub fn choose_model(current: &str, models: &[String], provider: Option<Provider>) -> String {
    if models.iter().any(|m| m == current) {
        return current.to_owned();
    }
    let preferred: &[&str] = match provider {
        Some(Provider::Gemini) => &["gemini-2.5-flash", "flash"],
        Some(Provider::OpenAi) => &["gpt-4.1-mini", "gpt-4o-mini", "mini"],
        Some(Provider::Anthropic) => &["sonnet", "haiku"],
        Some(Provider::Groq) => &["llama-3.3-70b", "llama"],
        Some(Provider::DeepSeek) => &["deepseek-chat"],
        Some(Provider::XAi) => &["grok"],
        _ => &[],
    };
    const UNSTABLE: [&str; 8] = [
        "preview", "exp", "lite", "thinking", "tts", "image", "audio", "vision",
    ];
    let mut newest_first: Vec<&String> = models.iter().collect();
    newest_first.sort_by(|a, b| b.cmp(a));
    for want in preferred {
        if let Some(m) = newest_first
            .iter()
            .find(|m| m.contains(want) && !UNSTABLE.iter().any(|u| m.contains(u)))
        {
            return (*m).clone();
        }
    }
    models.first().cloned().unwrap_or_default()
}

/// Send a conversation and return the assistant reply.
pub fn chat(settings: &AiSettings, system: &str, messages: &[Message]) -> Result<String, String> {
    let provider = settings.resolved().ok_or("Chưa cấu hình API key")?;
    if settings.model.trim().is_empty() {
        return Err("Chưa chọn mô hình trong Cài đặt → AI".into());
    }
    let key = settings.api_key.trim();
    let base = settings.base();
    let model = settings.model.trim();
    let agent = agent();
    match provider {
        Provider::Gemini => {
            let contents: Vec<Value> = messages
                .iter()
                .map(|m| {
                    let mut parts = vec![json!({ "text": m.text })];
                    for img in &m.images {
                        parts.push(json!({
                            "inline_data": {
                                "mime_type": img.mime_type,
                                "data": img.base64_data,
                            }
                        }));
                    }
                    json!({
                        "role": if m.role == Role::User { "user" } else { "model" },
                        "parts": parts,
                    })
                })
                .collect();
            let body = json!({
                "systemInstruction": { "parts": [{ "text": system }] },
                "contents": contents,
            });

            let call_gemini = |target_model: &str| -> Result<String, String> {
                let value = read(
                    agent
                        .post(&format!("{base}/models/{target_model}:generateContent"))
                        .header("x-goog-api-key", key)
                        .header("Content-Type", "application/json")
                        .send(body.to_string()),
                )?;
                let text: String = value
                    .pointer("/candidates/0/content/parts")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|p| p["text"].as_str())
                    .collect();
                if text.is_empty() {
                    let reason = value
                        .pointer("/promptFeedback/blockReason")
                        .and_then(Value::as_str)
                        .unwrap_or("phản hồi trống");
                    return Err(format!("Gemini không trả lời: {reason}"));
                }
                Ok(text)
            };

            // Thử gọi model hiện tại
            match call_gemini(model) {
                Ok(res) => Ok(res),
                Err(err) => {
                    let is_503_or_overload = err.contains("503")
                        || err.contains("demand")
                        || err.contains("UNAVAILABLE")
                        || err.contains("overloaded");

                    if is_503_or_overload && model != "gemini-2.5-flash" {
                        // Tự động chuyển fallback sang model chuẩn ổn định gemini-2.5-flash
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        match call_gemini("gemini-2.5-flash") {
                            Ok(res) => Ok(res),
                            Err(_) => {
                                // Thử thêm 1 lần nữa sau 1s
                                std::thread::sleep(std::time::Duration::from_millis(1000));
                                call_gemini("gemini-2.5-flash").map_err(|_| {
                                    format!("Máy chủ Google Gemini đang quá tải tạm thời (HTTP 503). Vui lòng thử lại sau vài giây hoặc chọn model khác trong Cài đặt (Alt + Q). Chi tiết: {err}")
                                })
                            }
                        }
                    } else if is_503_or_overload {
                        // Chính model gemini-2.5-flash bị 503, thử lại sau 1s
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                        call_gemini(model).map_err(|_| {
                            "Máy chủ Google Gemini đang quá tải tạm thời (HTTP 503). Spikes in demand là lỗi tạm thời của Google. Vui lòng bấm thử lại sau giây lát.".to_string()
                        })
                    } else {
                        Err(err)
                    }
                }
            }
        }
        Provider::Anthropic => {
            let anthropic_messages: Vec<Value> = messages
                .iter()
                .map(|m| {
                    if m.images.is_empty() {
                        json!({
                            "role": if m.role == Role::User { "user" } else { "assistant" },
                            "content": m.text,
                        })
                    } else {
                        let mut parts = Vec::new();
                        for img in &m.images {
                            parts.push(json!({
                                "type": "image",
                                "source": {
                                    "type": "base64",
                                    "media_type": img.mime_type,
                                    "data": img.base64_data,
                                }
                            }));
                        }
                        parts.push(json!({ "type": "text", "text": m.text }));
                        json!({
                            "role": if m.role == Role::User { "user" } else { "assistant" },
                            "content": parts,
                        })
                    }
                })
                .collect();
            let body = json!({
                "model": model,
                "max_tokens": 4096,
                "system": system,
                "messages": anthropic_messages,
            });
            let value = read(
                agent
                    .post(&format!("{base}/messages"))
                    .header("x-api-key", key)
                    .header("anthropic-version", "2023-06-01")
                    .header("Content-Type", "application/json")
                    .send(body.to_string()),
            )?;
            let text: String = value["content"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|c| c["type"] == "text")
                .filter_map(|c| c["text"].as_str())
                .collect();
            if text.is_empty() {
                return Err("Claude trả về phản hồi trống".into());
            }
            Ok(text)
        }
        _ => {
            let mut all = vec![json!({ "role": "system", "content": system })];
            all.extend(messages.iter().map(|m| {
                let content: Value = if m.images.is_empty() {
                    json!(m.text)
                } else {
                    let mut parts = vec![json!({ "type": "text", "text": m.text })];
                    for img in &m.images {
                        parts.push(json!({
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:{};base64,{}", img.mime_type, img.base64_data)
                            }
                        }));
                    }
                    json!(parts)
                };
                json!({
                    "role": if m.role == Role::User { "user" } else { "assistant" },
                    "content": content,
                })
            }));
            let body = json!({ "model": model, "messages": all });
            let mut request = agent
                .post(&format!("{base}/chat/completions"))
                .header("Content-Type", "application/json");
            if !key.is_empty() {
                request = request.header("Authorization", &format!("Bearer {key}"));
            }
            if provider == Provider::OpenRouter {
                request = request.header("X-Title", "PentaKuru");
            }
            let value = read(request.send(body.to_string()))?;
            value
                .pointer("/choices/0/message/content")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .filter(|t| !t.is_empty())
                .ok_or_else(|| "Mô hình trả về phản hồi trống".into())
        }
    }
}

/// Chuyển đổi giọng nói thành văn bản (Speech to Text) qua Gemini
pub fn stt(
    settings: &AiSettings,
    audio_base64: &str,
    mime_type: &str,
    action: &str,
) -> Result<(String, String), String> {
    let key = settings.api_key.trim();
    if key.is_empty() {
        return Err("Chưa cấu hình API Key trong PentaKuru".into());
    }

    if settings.resolved() != Some(Provider::Gemini) {
        return Err("Nhận diện giọng nói đồng hồ hiện cần Gemini. Chọn Gemini và API key tương ứng trong cài đặt AI của Penta.".into());
    }

    let prompt = match action {
        "note" => {
            "Hãy nhận diện giọng nói trong file âm thanh này và chuyển thành văn bản tiếng Việt chính xác để lưu ghi chú. Chỉ trả về văn bản, không thêm lời chào hay giải thích."
        }
        "ask" | "chat" => {
            "Hãy nhận diện giọng nói trong file âm thanh này, sau đó trả lời yêu cầu/câu hỏi đó một cách ngắn gọn, súc tích (dưới 80 từ) cho màn hình đồng hồ. Định dạng trả về:\nNÓI: <nội dung người dùng nói>\nTRẢ LỜI: <câu trả lời của bạn>"
        }
        _ => {
            "Chuyển đổi âm thanh trên thành văn bản tiếng Việt chính xác (Speech to Text). Chỉ trả về nội dung người dùng nói, không thêm lời bình."
        }
    };

    let body = json!({
        "contents": [{
            "parts": [
                {
                    "inline_data": {
                        "mime_type": mime_type,
                        "data": audio_base64
                    }
                },
                {
                    "text": prompt
                }
            ]
        }]
    });

    let model = if settings.model.trim().is_empty() {
        "gemini-2.5-flash"
    } else {
        settings.model.trim()
    };

    let base = settings.base();
    let value = read(
        agent()
            .post(&format!("{base}/models/{model}:generateContent"))
            .header("x-goog-api-key", key)
            .header("Content-Type", "application/json")
            .send(body.to_string()),
    )?;

    let text_output: String = value
        .pointer("/candidates/0/content/parts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|p| p["text"].as_str())
        .collect();

    if text_output.is_empty() {
        return Err("Không nhận diện được giọng nói trong âm thanh".into());
    }

    if matches!(action, "ask" | "chat")
        && text_output.contains("NÓI:")
        && text_output.contains("TRẢ LỜI:")
    {
        let parts: Vec<&str> = text_output.split("TRẢ LỜI:").collect();
        let user_speech = parts[0].replace("NÓI:", "").trim().to_string();
        let ai_reply = parts.get(1).unwrap_or(&"").trim().to_string();
        Ok((user_speech, ai_reply))
    } else {
        Ok((text_output.trim().to_string(), String::new()))
    }
}

/// Nhận diện lệnh `@note` / `#note` để AI viết ghi chú & tạo danh sách công việc.
pub fn note_command(prompt: &str) -> Option<&str> {
    let prompt = prompt.trim();
    let end = prompt.find(char::is_whitespace).unwrap_or(prompt.len());
    let token = &prompt[..end];
    ["@note", "#note", "@ghichu", "#ghichu"]
        .iter()
        .any(|prefix| token.eq_ignore_ascii_case(prefix))
        .then(|| prompt[end..].trim())
}

#[derive(Clone, Debug, PartialEq)]
pub struct AiNotePlan {
    pub title: String,
    pub content: String,
    pub tasks: Vec<(String, String, String)>,
    pub folder: Option<String>,
    pub schedule: String,
    pub due_at: String,
    pub interval_mins: u32,
    pub weekly_target: u8,
    pub remind: bool,
    pub next_week_command: String,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct PlannedTaskSpec {
    pub title: String,
    pub note: String,
    pub status: String,
    pub schedule: String,
    pub due_at: String,
    pub interval_mins: u32,
    pub weekly_target: u8,
    pub remind: Option<bool>,
    pub next_week_command: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AiTaskAction {
    CreatePlan {
        #[serde(default)]
        folder: String,
        #[serde(default)]
        title: String,
        #[serde(default)]
        content: String,
        #[serde(default)]
        schedule: String,
        #[serde(default)]
        due_at: String,
        #[serde(default)]
        interval_mins: u32,
        #[serde(default)]
        weekly_target: u8,
        #[serde(default = "default_true")]
        remind: bool,
        #[serde(default)]
        next_week_command: String,
        #[serde(default)]
        tasks: Vec<PlannedTaskSpec>,
    },
    CreateFolder {
        folder: String,
    },
    CancelSchedule {
        #[serde(default)]
        query: String,
    },
    CompleteTask {
        #[serde(default)]
        query: String,
    },
    ConfigureReminders {
        #[serde(default)]
        startup: Option<bool>,
        #[serde(default)]
        interval_mins: Option<u32>,
        #[serde(default)]
        focus_alert_mins: Option<u32>,
        #[serde(default)]
        proactive_rhythm: Option<bool>,
        #[serde(default)]
        paused: Option<bool>,
    },
}

/// Extracts and strips `[[TASK_ACTION:{...}]]` blocks from an AI reply using brace matching.
pub fn extract_ai_task_actions(reply: &str) -> (String, Vec<AiTaskAction>) {
    const PREFIX: &str = "[[TASK_ACTION:";
    let mut cleaned = String::new();
    let mut actions = Vec::new();
    let mut rest = reply;
    while let Some(pos) = rest.find(PREFIX) {
        cleaned.push_str(&rest[..pos]);
        let after_prefix = &rest[pos + PREFIX.len()..];
        let trimmed = after_prefix.trim_start();
        if let Some(json_start_rel) = trimmed.find('{') {
            let json_slice = &trimmed[json_start_rel..];
            let mut depth = 0i32;
            let mut in_str = false;
            let mut esc = false;
            let mut json_end = None;
            for (idx, ch) in json_slice.char_indices() {
                if in_str {
                    if esc {
                        esc = false;
                    } else if ch == '\\' {
                        esc = true;
                    } else if ch == '"' {
                        in_str = false;
                    }
                    continue;
                }
                match ch {
                    '"' => in_str = true,
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            json_end = Some(idx + ch.len_utf8());
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if let Some(end) = json_end {
                let raw_json = &json_slice[..end];
                if let Ok(action) = serde_json::from_str::<AiTaskAction>(raw_json) {
                    actions.push(action);
                }
                let after_json = &json_slice[end..];
                if let Some(close) = after_json.find("]]") {
                    rest = &after_json[close + 2..];
                    continue;
                }
                rest = after_json;
                continue;
            }
        }
        cleaned.push_str(PREFIX);
        rest = after_prefix;
    }
    cleaned.push_str(rest);
    (cleaned.trim().to_owned(), actions)
}

/// Parses direct explicit schedule cancellation phrases for instant offline execution.
pub fn direct_schedule_command(prompt: &str) -> Option<AiTaskAction> {
    let trimmed = prompt.trim().trim_end_matches(['.', '!', '?']).trim();
    let lower = trimmed.to_lowercase();
    if lower.contains('\n')
        || lower.starts_with("vì sao ")
        || lower.starts_with("tại sao ")
        || lower.starts_with("làm sao ")
        || lower.contains('?')
    {
        return None;
    }
    if [
        "dừng nhắc nhở",
        "dừng nhắc việc",
        "tắt nhắc nhở",
        "tắt nhắc việc",
        "tắt hết nhắc nhở",
        "thôi đừng nhắc nữa",
        "đừng nhắc nữa",
        "ngừng nhắc nhở",
        "tạm dừng nhắc nhở",
    ]
    .iter()
    .any(|p| lower == *p || lower.starts_with(&format!("{p} ")))
    {
        return Some(AiTaskAction::ConfigureReminders {
            startup: None,
            interval_mins: None,
            focus_alert_mins: None,
            proactive_rhythm: None,
            paused: Some(true),
        });
    }
    for prefix in [
        "hủy lịch ",
        "huỷ lịch ",
        "ngừng lịch ",
        "ngưng lịch ",
        "xóa lịch ",
        "bỏ lịch ",
        "tắt lịch nhắc ",
        "tắt nhắc lịch ",
    ] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let q = rest
                .trim()
                .trim_end_matches(" đi em")
                .trim_end_matches(" giúp anh")
                .trim_end_matches(" đi")
                .trim();
            if !q.is_empty() {
                return Some(AiTaskAction::CancelSchedule {
                    query: q.to_owned(),
                });
            }
        }
    }
    None
}

/// Phân tích phản hồi của AI từ lệnh `@note` thành nội dung ghi chú và danh sách công việc Kanban.
pub fn parse_ai_note_response(reply: &str, fallback_title: &str) -> AiNotePlan {
    let mut title = String::new();
    let mut folder: Option<String> = None;
    let mut schedule = String::new();
    let mut due_at = String::new();
    let mut interval_mins: u32 = 0;
    let mut weekly_target: u8 = 0;
    let mut remind = false;
    let mut next_week_command = String::new();
    let mut body_lines = Vec::new();
    let mut tasks: Vec<(String, String, String)> = Vec::new();
    let mut in_task_section = false;

    for raw_line in reply.lines() {
        let trimmed = raw_line.trim();
        if trimmed.starts_with("```") {
            continue;
        }
        let upper = trimmed.to_ascii_uppercase();
        if let Some(rest) = upper
            .strip_prefix("FOLDER:")
            .map(|_| &trimmed[7..])
            .or_else(|| {
                trimmed
                    .to_lowercase()
                    .strip_prefix("thư mục:")
                    .map(|_| &trimmed["thư mục:".len()..])
            })
        {
            let clean = rest.trim();
            if !clean.is_empty() {
                folder = Some(clean.to_owned());
            }
            continue;
        }
        if upper.starts_with("SCHEDULE:") {
            let val = trimmed[9..].trim().to_lowercase();
            schedule = match val.as_str() {
                "daily" | "mỗi ngày" | "hang ngay" => "daily".into(),
                "weekly" | "mỗi tuần" | "hang tuan" => "weekly".into(),
                _ => "none".into(),
            };
            if schedule != "none" {
                remind = true;
            }
            continue;
        }
        if upper.starts_with("TIME:") || upper.starts_with("DUE:") {
            let sep = trimmed.find(':').unwrap_or(4);
            let val = trimmed[sep + 1..].trim();
            if !val.is_empty() && val != "none" {
                due_at = val.to_owned();
                remind = true;
            }
            continue;
        }
        if upper.starts_with("INTERVAL:") {
            if let Ok(m) = trimmed[9..]
                .trim()
                .trim_end_matches(['p', 'm'])
                .trim()
                .parse::<u32>()
            {
                interval_mins = m;
                if m > 0 {
                    remind = true;
                }
            }
            continue;
        }
        if upper.starts_with("WEEKLY_TARGET:") {
            if let Ok(n) = trimmed[14..].trim().parse::<u8>() {
                weekly_target = n;
            }
            continue;
        }
        if upper.starts_with("REMIND:") {
            let val = trimmed[7..].trim().to_lowercase();
            remind = matches!(val.as_str(), "true" | "1" | "yes" | "có");
            continue;
        }
        if upper.starts_with("NEXT_WEEK:") {
            let val = trimmed[10..].trim();
            if !val.is_empty() {
                next_week_command = val.to_owned();
            }
            continue;
        }
        if let Some(h1) = trimmed.strip_prefix("# ") {
            if title.is_empty() {
                title = h1.trim().trim_matches('*').trim().to_string();
            }
            in_task_section = false;
            body_lines.push(raw_line.to_string());
            continue;
        }
        if trimmed.starts_with("## ") || trimmed.starts_with("### ") {
            let lower = trimmed.to_lowercase();
            in_task_section = lower.contains("công việc")
                || lower.contains("việc cần làm")
                || lower.contains("danh sách việc")
                || lower.contains("task")
                || lower.contains("todo")
                || lower.contains("hành động");
            body_lines.push(raw_line.to_string());
            continue;
        }

        let mut extracted: Option<(&str, &str)> = None;
        if let Some(rest) = trimmed
            .strip_prefix("- [ ] ")
            .or_else(|| trimmed.strip_prefix("* [ ] "))
            .or_else(|| trimmed.strip_prefix("TASK:"))
            .or_else(|| trimmed.strip_prefix("TODO:"))
        {
            extracted = Some(("todo", rest.trim()));
        } else if let Some(rest) = trimmed
            .strip_prefix("- [~] ")
            .or_else(|| trimmed.strip_prefix("* [~] "))
            .or_else(|| trimmed.strip_prefix("DOING:"))
        {
            extracted = Some(("doing", rest.trim()));
        } else if let Some(rest) = trimmed
            .strip_prefix("- [x] ")
            .or_else(|| trimmed.strip_prefix("- [X] "))
            .or_else(|| trimmed.strip_prefix("* [x] "))
            .or_else(|| trimmed.strip_prefix("DONE:"))
        {
            extracted = Some(("done", rest.trim()));
        } else if in_task_section {
            if let Some(rest) = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
                .or_else(|| trimmed.strip_prefix("• "))
            {
                extracted = Some(("todo", rest.trim()));
            } else if let Some(dot_idx) = trimmed.find(". ")
                && trimmed[..dot_idx].chars().all(|c| c.is_ascii_digit())
                && dot_idx > 0
                && dot_idx <= 3
            {
                extracted = Some(("todo", trimmed[dot_idx + 2..].trim()));
            }
        }

        if let Some((status, item_text)) = extracted {
            let clean_item = item_text.trim_matches('*').trim();
            if !clean_item.is_empty() {
                let (t_title, t_desc) = if let Some((a, b)) = clean_item.split_once('|') {
                    (a.trim().trim_matches('*').trim(), b.trim())
                } else if let Some((a, b)) = clean_item.split_once(" — ") {
                    (a.trim().trim_matches('*').trim(), b.trim())
                } else if let Some((a, b)) = clean_item.split_once(": ") {
                    (a.trim().trim_matches('*').trim(), b.trim())
                } else {
                    (clean_item, "")
                };
                if !t_title.is_empty()
                    && !tasks
                        .iter()
                        .any(|(existing, _, _)| existing.eq_ignore_ascii_case(t_title))
                {
                    tasks.push((t_title.to_string(), t_desc.to_string(), status.to_string()));
                }
                let checkbox = match status {
                    "done" => "[x]",
                    "doing" => "[~]",
                    _ => "[ ]",
                };
                if t_desc.is_empty() {
                    body_lines.push(format!("- {checkbox} {t_title}"));
                } else {
                    body_lines.push(format!("- {checkbox} **{t_title}**: {t_desc}"));
                }
                continue;
            }
        }

        body_lines.push(raw_line.to_string());
    }

    if title.is_empty() {
        title = fallback_title
            .lines()
            .next()
            .unwrap_or("Ghi chú AI")
            .trim_start_matches('#')
            .trim()
            .chars()
            .take(70)
            .collect();
        if title.is_empty() {
            title = "Ghi chú AI".into();
        }
    }

    let mut content = body_lines.join("\n").trim().to_string();
    if !content.starts_with("# ") {
        content = format!("# {title}\n\n{content}");
    }

    AiNotePlan {
        title,
        content,
        tasks,
        folder,
        schedule,
        due_at,
        interval_mins,
        weekly_target,
        remind,
        next_week_command,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_providers_and_picks_models() {
        assert_eq!(Provider::detect("AIzaSyX"), Some(Provider::Gemini));
        assert_eq!(Provider::detect("sk-ant-api03"), Some(Provider::Anthropic));
        assert_eq!(Provider::detect("sk-or-v1"), Some(Provider::OpenRouter));
        assert_eq!(Provider::detect("sk-proj-1"), Some(Provider::OpenAi));
        assert_eq!(Provider::detect("gsk_1"), Some(Provider::Groq));
        assert_eq!(Provider::detect("hello"), None);
        let models: Vec<String> = [
            "gemini-2.0-flash",
            "gemini-2.5-flash",
            "gemini-2.5-flash-lite",
            "gemini-2.5-pro",
        ]
        .map(String::from)
        .into();
        assert_eq!(
            choose_model("", &models, Some(Provider::Gemini)),
            "gemini-2.5-flash"
        );
        assert_eq!(
            choose_model("gemini-2.5-pro", &models, Some(Provider::Gemini)),
            "gemini-2.5-pro"
        );
        let settings = AiSettings {
            api_key: "AIzaX".into(),
            ..Default::default()
        };
        assert_eq!(
            settings.base(),
            "https://generativelanguage.googleapis.com/v1beta"
        );
        assert!(settings.can_list() && !settings.ready());
    }

    #[test]
    fn test_note_command_and_parse_ai_note_response() {
        assert_eq!(
            note_command("  @note Lập kế hoạch tuần "),
            Some("Lập kế hoạch tuần")
        );
        assert_eq!(note_command("#NOTE\nChuẩn bị demo"), Some("Chuẩn bị demo"));
        assert_eq!(note_command("@note"), Some(""));
        assert_eq!(note_command("@notebook"), None);

        let reply = "# Kế hoạch chuẩn bị Demo\n\nNội dung chính của buổi demo.\n\n## Công việc\n- [ ] Chuẩn bị slide: Hoàn thiện 10 trang giới thiệu\n- [~] Kiểm thử hệ thống | Chạy bộ test tự động\n- [x] Gửi lịch họp: Đã gửi cho cả nhóm";
        let plan = parse_ai_note_response(reply, "Fallback");
        assert_eq!(plan.title, "Kế hoạch chuẩn bị Demo");
        assert_eq!(plan.tasks.len(), 3);
        assert_eq!(
            plan.tasks[0],
            (
                "Chuẩn bị slide".into(),
                "Hoàn thiện 10 trang giới thiệu".into(),
                "todo".into()
            )
        );
        assert_eq!(
            plan.tasks[1],
            (
                "Kiểm thử hệ thống".into(),
                "Chạy bộ test tự động".into(),
                "doing".into()
            )
        );
        assert_eq!(
            plan.tasks[2],
            (
                "Gửi lịch họp".into(),
                "Đã gửi cho cả nhóm".into(),
                "done".into()
            )
        );

        let reply_with_tag = "Mình đã lập lịch tập cơ bắp cho anh nhé!\n[[TASK_ACTION:{\"action\":\"create_plan\",\"folder\":\"Sức khỏe/Cơ bắp\",\"title\":\"Lịch tập cơ bắp\",\"schedule\":\"daily\",\"due_at\":\"19:30\",\"interval_mins\":60,\"weekly_target\":6,\"remind\":true,\"next_week_command\":\"Tăng rep tuần mới\",\"tasks\":[{\"title\":\"Hít đất & kéo xà\",\"note\":\"4 hiệp\"}]}]]";
        let (clean, actions) = extract_ai_task_actions(reply_with_tag);
        assert_eq!(clean, "Mình đã lập lịch tập cơ bắp cho anh nhé!");
        assert_eq!(actions.len(), 1);
        assert!(matches!(
            &actions[0],
            AiTaskAction::CreatePlan { folder, schedule, due_at, tasks, .. }
                if folder == "Sức khỏe/Cơ bắp" && schedule == "daily" && due_at == "19:30" && tasks.len() == 1
        ));
        assert!(matches!(
            direct_schedule_command("hủy lịch tập cơ bắp đi em"),
            Some(AiTaskAction::CancelSchedule { query }) if query == "tập cơ bắp"
        ));
        assert!(matches!(
            direct_schedule_command("dừng nhắc nhở"),
            Some(AiTaskAction::ConfigureReminders {
                paused: Some(true),
                ..
            })
        ));
        assert!(direct_schedule_command("làm sao để hủy lịch?").is_none());
    }
}
