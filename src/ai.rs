//! AI providers: detect the provider from an API key, list its models and
//! send chat requests. Runs on worker threads (see `system::Job`).
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Provider {
    #[default]
    Auto,
    OpenAi,
    Gemini,
    Anthropic,
    OpenRouter,
    Groq,
    DeepSeek,
    XAi,
    UiTars,
    Ollama,
    Custom,
}

impl Provider {
    pub const ALL: [Provider; 11] = [
        Provider::Auto,
        Provider::OpenAi,
        Provider::Gemini,
        Provider::Anthropic,
        Provider::OpenRouter,
        Provider::Groq,
        Provider::DeepSeek,
        Provider::XAi,
        Provider::UiTars,
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
            Provider::UiTars => "UI-TARS (ByteDance GUI Agent)",
            Provider::Ollama => "Ollama (chạy trên máy)",
            Provider::Custom => "Tương thích OpenAI (tự nhập URL)",
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
            Provider::UiTars => "https://api.siliconflow.cn/v1",
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

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(default)]
pub struct UiTarsSettings {
    pub enabled: bool,
    pub use_shared_key: bool,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

impl Default for UiTarsSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            use_shared_key: true,
            api_key: String::new(),
            base_url: "https://api.siliconflow.cn/v1".to_string(),
            model: "ByteDance/UI-TARS-72B-DPO".to_string(),
        }
    }
}

impl UiTarsSettings {
    pub fn resolved_key<'a>(&'a self, shared_key: &'a str) -> &'a str {
        if self.use_shared_key && !shared_key.trim().is_empty() {
            shared_key.trim()
        } else {
            self.api_key.trim()
        }
    }

    pub fn resolved_base(&self) -> String {
        let b = if self.base_url.trim().is_empty() {
            "https://api.siliconflow.cn/v1"
        } else {
            self.base_url.trim()
        };
        b.trim_end_matches('/').to_owned()
    }

    pub fn resolved_model(&self) -> &str {
        if self.model.trim().is_empty() {
            "ByteDance/UI-TARS-72B-DPO"
        } else {
            self.model.trim()
        }
    }

    #[allow(dead_code)]
    pub fn ready(&self, shared_key: &str) -> bool {
        let key = self.resolved_key(shared_key);
        let base = self.resolved_base();
        if base.contains("localhost") || base.contains("127.0.0.1") {
            true
        } else {
            !key.is_empty()
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
        Some(Provider::UiTars) => &["UI-TARS", "ui-tars", "72B", "7B"],
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

/// Test connection to the UI-TARS server.
#[allow(dead_code)]
pub fn test_ui_tars_connection(
    settings: &UiTarsSettings,
    shared_key: &str,
) -> Result<String, String> {
    let key = settings.resolved_key(shared_key);
    let base = settings.resolved_base();
    let model = settings.resolved_model();
    let agent = agent();

    // Thử truy vấn danh sách model trước
    let mut req = agent.get(&format!("{base}/models"));
    if !key.is_empty() {
        req = req.header("Authorization", &format!("Bearer {key}"));
    }
    match read(req.call()) {
        Ok(val) => {
            let model_count = val["data"].as_array().map(|a| a.len()).unwrap_or(0);
            if model_count > 0 {
                Ok(format!(
                    "Kết nối thành công! Server phản hồi tốt ({model_count} mô hình khả dụng)."
                ))
            } else {
                Ok("Kết nối thành công! Server UI-TARS phản hồi tốt.".into())
            }
        }
        Err(e) => {
            // Nếu endpoint /models không mở hoặc trả về lỗi, thử ping nhẹ chat/completions
            let body = json!({
                "model": model,
                "messages": [{"role": "user", "content": "ping"}],
                "max_tokens": 1
            });
            let mut chat_req = agent
                .post(&format!("{base}/chat/completions"))
                .header("Content-Type", "application/json");
            if !key.is_empty() {
                chat_req = chat_req.header("Authorization", &format!("Bearer {key}"));
            }
            match read(chat_req.send(body.to_string())) {
                Ok(_) => Ok("Kết nối thành công! UI-TARS đã sẵn sàng tiếp nhận lệnh GUI.".into()),
                Err(err_msg) => {
                    let err_str = err_msg.to_string();
                    if err_str.contains("10061") || err_str.to_lowercase().contains("refused") {
                        Err(format!(
                            "Máy chủ tại {base} đang tắt hoặc chưa chạy trên máy (Lỗi 10061: Connection refused).\nHãy mở ứng dụng UI-TARS Desktop hoặc chuyển sang chế độ 'Dùng AI hệ thống (Gemini)' để không cần server ngoài."
                        ))
                    } else {
                        Err(format!("Lỗi kết nối tới {base}: {e}"))
                    }
                }
            }
        }
    }
}

/// Test connection to GUI Agent (either native System AI or external UI-TARS server).
#[allow(dead_code)]
pub fn test_gui_agent_connection(
    settings: &UiTarsSettings,
    ai_settings: &AiSettings,
) -> Result<String, String> {
    if settings.use_shared_key {
        if !ai_settings.ready() {
            return Err("Chưa cấu hình API Key ở mục 'Cấu hình Trợ lý AI' phía trên. Hãy nhập API Key và chọn mô hình trước.".into());
        }
        let ping_msg = [Message {
            role: Role::User,
            text: "ping".into(),
            images: Vec::new(),
        }];
        match chat(ai_settings, "ping", &ping_msg) {
            Ok(_) => {
                let p_name = ai_settings.resolved().map(|p| p.label()).unwrap_or("AI");
                Ok(format!(
                    "Kết nối thành công tới {} ({})! Đã sẵn sàng làm GUI Agent phân tích màn hình & điều khiển giao diện.",
                    p_name, ai_settings.model
                ))
            }
            Err(e) => Err(format!("Lỗi kết nối tới AI hệ thống: {e}")),
        }
    } else {
        test_ui_tars_connection(settings, &ai_settings.api_key)
    }
}

/// Send GUI Agent command to the UI-TARS endpoint.
pub fn call_ui_tars(
    settings: &UiTarsSettings,
    shared_key: &str,
    system_prompt: &str,
    messages: &[Message],
) -> Result<String, String> {
    let key = settings.resolved_key(shared_key);
    let base = settings.resolved_base();
    let model = settings.resolved_model();
    let agent = agent();

    let mut all = vec![json!({ "role": "system", "content": system_prompt })];
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

    let body = json!({
        "model": model,
        "messages": all,
        "temperature": 0.0,
    });

    let mut request = agent
        .post(&format!("{base}/chat/completions"))
        .header("Content-Type", "application/json");
    if !key.is_empty() {
        request = request.header("Authorization", &format!("Bearer {key}"));
    }

    let value = read(request.send(body.to_string()))?;
    value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| "Mô hình UI-TARS trả về phản hồi trống".into())
}

/// Trích xuất tọa độ hành động (Action: click) từ phản hồi của mô hình UI-TARS hoặc Gemini.
pub fn parse_ui_tars_action(text: &str) -> Option<(i32, i32)> {
    let lower = text.to_lowercase();

    // Mẫu 1: point=[x, y] hoặc point: [x, y] hoặc point:[x, y] hoặc point = [x, y]
    for key in [
        "point=[",
        "point: [",
        "point:[",
        "point = [",
        "tọa độ [",
        "toạ độ [",
    ] {
        if let Some(idx) = lower.find(key) {
            let after = &lower[idx + key.len()..];
            if let Some(end) = after.find(']') {
                let nums: Vec<&str> = after[..end].split(',').map(str::trim).collect();
                if nums.len() >= 2
                    && let (Ok(x), Ok(y)) = (nums[0].parse::<f32>(), nums[1].parse::<f32>())
                {
                    return Some(scale_coords(x as i32, y as i32));
                }
            }
        }
    }

    // Mẫu 2: click(...)
    if let Some(idx) = lower.find("click(") {
        let after = &lower[idx + 6..];
        if let Some(end) = after.find(')') {
            let sub = &after[..end];
            let clean = sub.replace("point=", "").replace(['[', ']'], "");
            let nums: Vec<&str> = clean.split(',').map(str::trim).collect();
            if nums.len() >= 2
                && let (Ok(x), Ok(y)) = (nums[0].parse::<f32>(), nums[1].parse::<f32>())
            {
                return Some(scale_coords(x as i32, y as i32));
            }
        }
    }

    // Mẫu 3: click at (x, y) hoặc click tại (x, y) hoặc click at [x, y] hoặc click tại [x, y]
    for prefix in [
        "click at (",
        "click tại (",
        "click tai (",
        "click at [",
        "click tại [",
        "click tai [",
        "click [",
    ] {
        if let Some(idx) = lower.find(prefix) {
            let after = &lower[idx + prefix.len()..];
            let close_char = if prefix.ends_with('[') { ']' } else { ')' };
            if let Some(end) = after.find(close_char) {
                let sub = &after[..end];
                let nums: Vec<&str> = sub.split(',').map(str::trim).collect();
                if nums.len() >= 2
                    && let (Ok(x), Ok(y)) = (nums[0].parse::<f32>(), nums[1].parse::<f32>())
                {
                    return Some(scale_coords(x as i32, y as i32));
                }
            }
        }
    }

    None
}

#[derive(Clone, Debug, PartialEq)]
pub enum UiAgentAction {
    Click { x: i32, y: i32 },
    DoubleClick { x: i32, y: i32 },
    RightClick { x: i32, y: i32 },
    Type { text: String },
    KeyPress { key: String },
    Scroll { amount: i32 },
    Finished { message: String },
    Unknown { raw: String },
}

/// Recognize a complete mode token, including mixed case and newline separators.
pub fn ui_command(prompt: &str) -> Option<&str> {
    let prompt = prompt.trim();
    let end = prompt.find(char::is_whitespace).unwrap_or(prompt.len());
    let token = &prompt[..end];
    ["@ui", "#ui", "@tars", "#tars"]
        .iter()
        .any(|prefix| token.eq_ignore_ascii_case(prefix))
        .then(|| prompt[end..].trim())
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiAgentStep {
    pub thought: String,
    pub action: UiAgentAction,
    pub description: String,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct UiAgentStepRecord {
    pub step_number: usize,
    pub thought: String,
    pub action: UiAgentAction,
    pub description: String,
    pub timestamp: std::time::Instant,
    pub screenshot_path: Option<std::path::PathBuf>,
}

#[derive(Clone, Debug, Default)]
pub struct UiAgentState {
    pub is_active: bool,
    pub goal: String,
    pub current_step: usize,
    pub steps_history: Vec<UiAgentStepRecord>,
    pub waiting_delay_until: Option<std::time::Instant>,
    pub last_status_message: String,
}

impl UiAgentState {
    pub fn new(goal: String) -> Self {
        Self {
            is_active: true,
            goal,
            current_step: 1,
            steps_history: Vec::new(),
            waiting_delay_until: None,
            last_status_message: "Bắt đầu quan sát màn hình máy tính...".into(),
        }
    }
}

/// Phân tích phản hồi từng bước của mô hình AI theo chuẩn Thought & Action của UI-TARS.
pub fn parse_ui_agent_step(text: &str) -> UiAgentStep {
    let mut thought = String::new();
    let mut action_str = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_lowercase();
        if lower.starts_with("thought:") || lower.starts_with("suy nghĩ:") {
            thought = trimmed
                .trim_start_matches("thought:")
                .trim_start_matches("Thought:")
                .trim_start_matches("suy nghĩ:")
                .trim_start_matches("Suy nghĩ:")
                .trim()
                .to_string();
        } else if lower.starts_with("action:") || lower.starts_with("hành động:") {
            action_str = trimmed
                .trim_start_matches("action:")
                .trim_start_matches("Action:")
                .trim_start_matches("hành động:")
                .trim_start_matches("Hành động:")
                .trim()
                .to_string();
        } else if thought.is_empty()
            && !lower.contains("click")
            && !lower.contains("type")
            && !lower.contains("press")
            && !lower.contains("finished")
        {
            thought.push_str(trimmed);
            thought.push(' ');
        }
    }

    if action_str.is_empty() {
        action_str = text.to_string();
    }
    thought = thought.trim().to_string();

    let lower_action = action_str.to_lowercase();

    // 1. Finished action
    if lower_action.contains("finished")
        || lower_action.contains("finish(")
        || lower_action.contains("hoàn thành")
    {
        let msg = if let Some(idx) = lower_action.find("message=") {
            let sub = &action_str[idx + 8..];
            sub.trim_matches(|c| c == '(' || c == ')' || c == '"' || c == '\'')
                .to_string()
        } else {
            "Mục tiêu đã hoàn tất.".to_string()
        };
        return UiAgentStep {
            thought,
            action: UiAgentAction::Finished {
                message: msg.clone(),
            },
            description: format!("Hoàn thành: {msg}"),
        };
    }

    // 2. Type text action: type(content="...") hoặc type(text="...")
    if let Some(idx) = lower_action.find("type(") {
        let sub = &action_str[idx + 5..];
        if let Some(end) = sub.find(')') {
            let mut content = sub[..end].trim().to_string();
            if content.starts_with("content=") {
                content = content.trim_start_matches("content=").trim().to_string();
            } else if content.starts_with("text=") {
                content = content.trim_start_matches("text=").trim().to_string();
            }
            let clean_text = content.trim_matches(|c| c == '"' || c == '\'').to_string();
            return UiAgentStep {
                thought,
                action: UiAgentAction::Type {
                    text: clean_text.clone(),
                },
                description: format!("Gõ chữ: \"{clean_text}\""),
            };
        }
    }

    // 3. Key Press action: press(key="...") hoặc press("...")
    if let Some(idx) = lower_action.find("press(") {
        let sub = &action_str[idx + 6..];
        if let Some(end) = sub.find(')') {
            let mut key = sub[..end].trim().to_string();
            if key.starts_with("key=") {
                key = key.trim_start_matches("key=").trim().to_string();
            }
            let clean_key = key.trim_matches(|c| c == '"' || c == '\'').to_string();
            return UiAgentStep {
                thought,
                action: UiAgentAction::KeyPress {
                    key: clean_key.clone(),
                },
                description: format!("Nhấn phím: {clean_key}"),
            };
        }
    }

    // 4. Double click
    if lower_action.contains("double_click")
        && let Some((x, y)) = parse_ui_tars_action(&action_str)
    {
        return UiAgentStep {
            thought,
            action: UiAgentAction::DoubleClick { x, y },
            description: format!("Nhấp đúp chuột tại ({x}, {y})"),
        };
    }

    // 5. Right click
    if lower_action.contains("right_click")
        && let Some((x, y)) = parse_ui_tars_action(&action_str)
    {
        return UiAgentStep {
            thought,
            action: UiAgentAction::RightClick { x, y },
            description: format!("Nhấp chuột phải tại ({x}, {y})"),
        };
    }

    // 6. Scroll
    if lower_action.contains("scroll") {
        let amt = if lower_action.contains("up") { 3 } else { -3 };
        let dir = if amt > 0 { "lên" } else { "xuống" };
        return UiAgentStep {
            thought,
            action: UiAgentAction::Scroll { amount: amt },
            description: format!("Cuộn màn hình {dir}"),
        };
    }

    // 7. Regular Click
    if let Some((x, y)) = parse_ui_tars_action(&action_str) {
        return UiAgentStep {
            thought,
            action: UiAgentAction::Click { x, y },
            description: format!("Nhấp chuột tại ({x}, {y})"),
        };
    }

    UiAgentStep {
        thought,
        action: UiAgentAction::Unknown {
            raw: action_str.clone(),
        },
        description: action_str,
    }
}

fn scale_coords(mut x: i32, mut y: i32) -> (i32, i32) {
    #[cfg(target_os = "windows")]
    let (screen_w, screen_h) = unsafe {
        (
            windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
                windows_sys::Win32::UI::WindowsAndMessaging::SM_CXSCREEN,
            ),
            windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
                windows_sys::Win32::UI::WindowsAndMessaging::SM_CYSCREEN,
            ),
        )
    };
    #[cfg(not(target_os = "windows"))]
    let (screen_w, screen_h) = (1920, 1080);

    // Tọa độ chuẩn hóa [0, 1000] của UI-TARS
    if (0..=1000).contains(&x) && (0..=1000).contains(&y) && screen_w > 0 && screen_h > 0 {
        x = (x as f32 * screen_w as f32 / 1000.0) as i32;
        y = (y as f32 * screen_h as f32 / 1000.0) as i32;
    }
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_mode_requires_a_complete_token_and_accepts_whitespace() {
        assert_eq!(
            ui_command("  @Ui\nmở trình duyệt  "),
            Some("mở trình duyệt")
        );
        assert_eq!(ui_command("@UI\tclick"), Some("click"));
        assert_eq!(ui_command("#TARS mục tiêu"), Some("mục tiêu"));
        assert_eq!(ui_command("@ui"), Some(""));
        for prompt in ["@uikit", "@ui-example", "x @ui mở", ""] {
            assert_eq!(ui_command(prompt), None);
        }
    }

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
    fn test_ui_tars_settings() {
        let mut tars = UiTarsSettings::default();
        assert_eq!(tars.resolved_key("shared-key-123"), "shared-key-123");
        tars.use_shared_key = false;
        tars.api_key = "custom-key".into();
        assert_eq!(tars.resolved_key("shared-key-123"), "custom-key");
        assert_eq!(tars.resolved_base(), "https://api.siliconflow.cn/v1");
        assert_eq!(tars.resolved_model(), "ByteDance/UI-TARS-72B-DPO");
    }

    #[test]
    fn test_parse_ui_tars_action() {
        let action1 = "Action: click(point=[500, 500])";
        assert!(parse_ui_tars_action(action1).is_some());

        let action2 = "Thought: I need to click the login button\nAction: click(100, 200)";
        assert!(parse_ui_tars_action(action2).is_some());

        let action3 = "Tôi sẽ click tại (300, 450) để mở cửa sổ";
        assert!(parse_ui_tars_action(action3).is_some());

        let action4 = "Action: click at (250, 600)";
        assert!(parse_ui_tars_action(action4).is_some());

        let no_action = "Here is some general text without clicks.";
        assert!(parse_ui_tars_action(no_action).is_none());
    }

    #[test]
    fn test_parse_ui_agent_step() {
        let text1 = "Thought: Cần mở Chrome\nAction: click(point=[500, 500])";
        let step1 = parse_ui_agent_step(text1);
        assert_eq!(step1.thought, "Cần mở Chrome");
        assert!(matches!(step1.action, UiAgentAction::Click { .. }));

        let text2 = "Thought: Gõ từ khóa tìm kiếm\nAction: type(content=\"thời tiết hôm nay\")";
        let step2 = parse_ui_agent_step(text2);
        assert_eq!(step2.thought, "Gõ từ khóa tìm kiếm");
        assert_eq!(
            step2.action,
            UiAgentAction::Type {
                text: "thời tiết hôm nay".into()
            }
        );

        let text3 = "Thought: Đã xong việc\nAction: finished(message=\"Hoàn thành\")";
        let step3 = parse_ui_agent_step(text3);
        assert!(matches!(step3.action, UiAgentAction::Finished { .. }));
    }
}
