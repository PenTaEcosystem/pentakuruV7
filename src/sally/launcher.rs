//! Hosts the original Sally frontend and services in PentaKuru's executable.
#[cfg(all(windows, feature = "sally-full"))]
mod host {
    use sally_source::embedded::{Command, Options};
    use std::sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc,
    };

    struct Host {
        tx: mpsc::Sender<Command>,
        errors: mpsc::Receiver<String>,
        visible: Arc<AtomicBool>,
        finished: Arc<AtomicBool>,
        disabled: bool,
    }
    static HOST: OnceLock<Mutex<Option<Host>>> = OnceLock::new();

    fn ensure_host(api_key: &str, target_lang: &str) -> Result<(), String> {
        let mut guard = HOST
            .get_or_init(|| Mutex::new(None))
            .lock()
            .map_err(|e| e.to_string())?;
        if guard.is_none() {
            let (tx, rx) = mpsc::channel();
            let (error_tx, errors) = mpsc::channel();
            let visible = Arc::new(AtomicBool::new(false));
            let finished = Arc::new(AtomicBool::new(false));
            let thread_visible = visible.clone();
            let thread_finished = finished.clone();
            let options = Options {
                data_dir: if check_mode() {
                    std::env::current_dir()
                        .unwrap()
                        .join(".build/sally-integration-data")
                } else {
                    crate::system::data_dir().join("Sally")
                },
                gemini_key: api_key.trim().to_owned(),
                target_language: language_name(target_lang).to_owned(),
                test_mode: check_mode(),
            };
            std::thread::Builder::new()
                .name("pentakuru-sally-ui".into())
                .spawn(move || {
                    let error_sink = error_tx.clone();
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        sally_source::embedded::run(options, rx, thread_visible.clone(), error_tx)
                    }));
                    match result {
                        Ok(Ok(())) => {},
                        Ok(Err(error)) => { let _ = error_sink.send(error); },
                        Err(_) => { let _ = error_sink.send("Không thể khởi tạo Sally. Kiểm tra WebView2 Runtime và quyền truy cập thư mục dữ liệu.".into()); },
                    }
                    thread_visible.store(false, Ordering::Relaxed);
                    thread_finished.store(true, Ordering::Release);
                })
                .map_err(|e| e.to_string())?;
            *guard = Some(Host {
                tx,
                errors,
                visible,
                finished,
                disabled: false,
            });
        }
        let host = guard.as_mut().unwrap();
        host.disabled = false;
        Ok(())
    }

    pub fn open_or_focus_sally(api_key: &str, target_lang: &str) -> Result<(), String> {
        ensure_host(api_key, target_lang)?;
        send(Command::Open)
    }
    pub fn open_web(
        label: &'static str,
        url: &str,
        title: &str,
        width: f64,
        height: f64,
        script: String,
    ) -> Result<(), String> {
        ensure_host("", "Tiếng Việt")?;
        send(Command::OpenWeb {
            label,
            url: url.into(),
            title: title.into(),
            width,
            height,
            script,
        })
    }
    pub fn close_web(label: &'static str) -> Result<(), String> {
        if HOST
            .get()
            .and_then(|h| h.lock().ok())
            .is_some_and(|h| h.is_some())
        {
            send(Command::CloseWeb(label))
        } else {
            Ok(())
        }
    }

    fn language_name(value: &str) -> &str {
        match value {
            "Tiếng Việt" => "Vietnamese",
            "日本語" => "Japanese",
            "中文" => "Chinese",
            "한국어" => "Korean",
            "Français" => "French",
            "Deutsch" => "German",
            other => other,
        }
    }

    fn check_mode() -> bool {
        std::env::args().any(|a| a == "--sally-integration-check" || a == "--web-integration-check")
    }
    pub fn web_integration_check(ctx: eframe::egui::Context) {
        std::thread::spawn(move || {
            let result = (|| -> Result<serde_json::Value, String> {
                open_web(
                    "penta-music",
                    "https://music.pentasys.id.vn/",
                    "PentaKuru · Music",
                    480.0,
                    340.0,
                    crate::integrations::music_script(),
                )?;
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(45);
                let report = loop {
                    let (tx, rx) = mpsc::channel();
                    send(Command::ProbeWeb{label:"penta-music",script:"({origin:location.origin,web:document.documentElement.dataset.pentakuruWeb==='true',audio:!!document.getElementById('audio-engine'),search:!!document.getElementById('topbar-search-input')&&document.getElementById('topbar-search-input').getClientRects().length>0,width:innerWidth,document_width:document.documentElement.scrollWidth,native_login:typeof window.openAuthModal==='function',background:getComputedStyle(document.body).backgroundColor})".into(),reply:tx})?;
                    if let Ok(Ok(value)) = rx.recv_timeout(std::time::Duration::from_secs(4))
                        && value["web"] == true
                        && value["search"] == true
                        && value["audio"] == true
                        && value["native_login"] == true
                    {
                        break value;
                    }
                    if std::time::Instant::now() > deadline {
                        return Err("Music chưa tải được trang web/search trong 45 giây.".into());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                };
                let (tx, rx) = mpsc::channel();
                send(Command::ProbeWeb{label:"penta-music",script:"(()=>{const input=document.getElementById('topbar-search-input');input.focus();return {focused:document.activeElement===input,visible:input.getClientRects().length>0};})()".into(),reply:tx})?;
                let interaction = rx
                    .recv_timeout(std::time::Duration::from_secs(4))
                    .map_err(|e| e.to_string())??;
                if interaction["focused"] != true || interaction["visible"] != true {
                    return Err(format!("Music search không nhận focus: {interaction}"));
                }
                close_web("penta-music")?;
                open_web(
                    "penta-clip",
                    "https://clip.pentasys.id.vn/feed/subscriptions",
                    "PentaKuru · Clip",
                    480.0,
                    420.0,
                    crate::integrations::clip_script(),
                )?;
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(40);
                let clip = loop {
                    let (tx, rx) = mpsc::channel();
                    send(Command::ProbeWeb{label:"penta-clip",script:"({origin:location.origin,web:document.documentElement.dataset.pentakuruWeb==='true',width:innerWidth,document_width:document.documentElement.scrollWidth,ready:document.readyState==='complete',search:!!document.querySelector('input[name=\"q\"]')})".into(),reply:tx})?;
                    if let Ok(Ok(value)) = rx.recv_timeout(std::time::Duration::from_secs(4))
                        && value["ready"] == true
                        && value["web"] == true
                        && value["origin"] == "https://clip.pentasys.id.vn"
                    {
                        break value;
                    }
                    if std::time::Instant::now() > deadline {
                        return Err("Clip chưa tải được.".into());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                };
                close_web("penta-clip")?;
                Ok(
                    serde_json::json!({"music":report,"interaction":interaction,"clip":clip,"no_credentials_used":true}),
                )
            })();
            let report = match result {
                Ok(report) => serde_json::json!({"passed":true,"report":report}),
                Err(error) => serde_json::json!({"passed":false,"error":error}),
            };
            let _ = std::fs::create_dir_all(".build");
            let _ = std::fs::write(
                ".build/web-integration-check.json",
                serde_json::to_vec_pretty(&report).unwrap(),
            );
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
        });
    }

    pub fn integration_check(ctx: eframe::egui::Context) {
        std::thread::spawn(move || {
            let result = check_lifecycle();
            let report = match result {
                Ok(value) => value,
                Err(error) => serde_json::json!({"passed": false, "error": error}),
            };
            let _ = std::fs::create_dir_all(".build");
            let _ = std::fs::write(
                ".build/sally-integration-check.json",
                serde_json::to_string_pretty(&report).unwrap(),
            );
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
        });
    }

    fn send(command: Command) -> Result<(), String> {
        HOST.get()
            .unwrap()
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .tx
            .send(command)
            .map_err(|e| e.to_string())
    }

    fn probe() -> Result<serde_json::Value, String> {
        let (tx, rx) = mpsc::channel();
        send(Command::Probe(tx))?;
        rx.recv_timeout(std::time::Duration::from_secs(3))
            .map_err(|e| format!("WebView probe: {e}"))
    }

    fn check_lifecycle() -> Result<serde_json::Value, String> {
        open_or_focus_sally("", "Tiếng Việt")?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let opened = loop {
            if let Some(error) = take_error() {
                return Err(error);
            }
            if is_sally_running()
                && let Ok(report) = probe()
                && report["titlebar"] == true
                && report["panels"] == true
                && report["controls"] == true
                && report["microphoneOff"] == true
            {
                break report;
            }
            if std::time::Instant::now() > deadline {
                return Err("Sally UI did not become ready".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        };
        send(Command::ClosePanel)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while is_sally_running() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let hidden = probe()?;
        open_or_focus_sally("", "Tiếng Việt")?;
        let reopened = probe()?;
        terminate_sally()?;
        let disabled = probe()?;
        let passed = opened["pid"] == std::process::id()
            && hidden["visible"] == false
            && reopened["visible"] == true
            && disabled["visible"] == false
            && opened["noKey"] == true;
        if !passed {
            return Err(format!(
                "Lifecycle failed: {opened}, {hidden}, {reopened}, {disabled}"
            ));
        }
        Ok(
            serde_json::json!({"passed": true, "opened": opened, "hidden": hidden, "reopened": reopened, "disabled": disabled}),
        )
    }

    pub fn is_sally_running() -> bool {
        HOST.get().and_then(|h| h.lock().ok()).is_some_and(|h| {
            h.as_ref()
                .is_some_and(|h| h.visible.load(Ordering::Relaxed))
        })
    }

    pub fn terminate_sally() -> Result<(), String> {
        if let Some(cell) = HOST.get() {
            let mut guard = cell.lock().map_err(|e| e.to_string())?;
            if let Some(host) = guard.as_mut()
                && !host.disabled
            {
                host.tx.send(Command::Disable).map_err(|e| e.to_string())?;
                host.disabled = true;
            }
        }
        Ok(())
    }

    pub fn take_error() -> Option<String> {
        HOST.get()?.lock().ok()?.as_ref()?.errors.try_recv().ok()
    }

    pub fn shutdown() {
        let finished = HOST.get().and_then(|h| h.lock().ok()).and_then(|h| {
            h.as_ref().map(|host| {
                let _ = host.tx.send(Command::Shutdown);
                host.finished.clone()
            })
        });
        if let Some(finished) = finished {
            // Allow Sally to flush the recording and recovery log before exit.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while !finished.load(Ordering::Acquire) && std::time::Instant::now() < deadline {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
    }
}

#[cfg(not(all(windows, feature = "sally-full")))]
mod host {
    pub fn web_integration_check(_: eframe::egui::Context) {}
    pub fn integration_check(_: eframe::egui::Context) {}
    pub fn open_or_focus_sally(_: &str, _: &str) -> Result<(), String> {
        Err("Giao diện Sally đầy đủ yêu cầu bản Windows có tính năng sally-full.".into())
    }
    pub fn open_web(
        _: &'static str,
        _: &str,
        _: &str,
        _: f64,
        _: f64,
        _: String,
    ) -> Result<(), String> {
        Err("Khung tích hợp cần bản Windows có WebView2/sally-full.".into())
    }
    pub fn close_web(_: &'static str) -> Result<(), String> {
        Ok(())
    }
    pub fn is_sally_running() -> bool {
        false
    }
    pub fn terminate_sally() -> Result<(), String> {
        Ok(())
    }
    pub fn take_error() -> Option<String> {
        None
    }
    pub fn shutdown() {}
}

pub use host::*;
