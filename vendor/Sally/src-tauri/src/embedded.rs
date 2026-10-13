//! Windows host for the complete Sally UI inside PentaKuru's process.
//! Tauri owns its own Windows UI thread; eframe retains PentaKuru's main thread.
use crate::{commands::AppState, config::AppConfig};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
};
use tauri::{Emitter, Listener, Manager};

pub enum Command {
    Open,
    Disable,
    Shutdown,
    Probe(mpsc::Sender<serde_json::Value>),
    ClosePanel,
    OpenWeb {
        label: &'static str,
        url: String,
        title: String,
        width: f64,
        height: f64,
        script: String,
    },
    CloseWeb(&'static str),
    ProbeWeb {
        label: &'static str,
        script: String,
        reply: mpsc::Sender<Result<serde_json::Value, String>>,
    },
}

pub struct Options {
    pub data_dir: PathBuf,
    pub gemini_key: String,
    pub target_language: String,
    pub test_mode: bool,
}

pub fn run(
    options: Options,
    commands: mpsc::Receiver<Command>,
    visible: Arc<AtomicBool>,
    errors: mpsc::Sender<String>,
) -> Result<(), String> {
    let setup_visible = visible.clone();
    let close_visible = visible.clone();
    // Tauri's code generator resolves the standard config in the supplied
    // directory, ignoring an alternate basename. Assets use the same dist,
    // but the embedded host needs its own identifier, hidden window and title.
    let mut context = tauri::generate_context!();
    context.set_default_window_icon(Some(tauri::image::Image::new_owned(
        include_bytes!("../../../../assets/pentakuru-icon.rgba").to_vec(),
        64,
        64,
    )));
    *context.config_mut() = serde_json::from_str(include_str!("../tauri.pentakuru.conf.json"))
        .map_err(|e| format!("Invalid embedded Sally configuration: {e}"))?;
    let app = crate::builder()
        .any_thread()
        .setup(move |app| {
            let data_dir = if options.test_mode {
                options.data_dir
            } else {
                app.path()
                    .app_config_dir()
                    .ok()
                    .and_then(|dir| {
                        crate::config::read_data_dir_pointer(&dir).or_else(|| {
                            dir.parent().and_then(|parent| {
                                crate::config::read_data_dir_pointer(&parent.join("com.sally.app"))
                            })
                        })
                    })
                    .filter(|dir| dir.is_dir())
                    .unwrap_or(options.data_dir)
            };
            let existing = data_dir.join(".env").is_file();
            let mut cfg = AppConfig::load(data_dir)?;
            // Keep a key entered in Sally. PentaKuru can also use non-Google
            // providers, whose keys must never be passed to Gemini.
            if cfg.api_key.is_empty() && options.gemini_key.starts_with("AIza") {
                cfg.api_key = options.gemini_key;
            }
            if !existing {
                cfg.target_language = options.target_language;
            }
            cfg.readout_enabled = false;
            cfg.save()?;
            *app.state::<AppState>().config.try_lock()? = Some(cfg);
            setup_visible.store(false, Ordering::Relaxed);
            Ok(())
        })
        .on_window_event(move |window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() != "main" {
                    return;
                }
                // Closing the panel must not exit PentaKuru or discard a meeting.
                api.prevent_close();
                let _ = window.hide();
                close_visible.store(false, Ordering::Relaxed);
            }
        })
        .build(context)
        .map_err(|e| format!("Không thể mở giao diện Sally: {e}"))?;

    let mut controls = Some((commands, visible, errors));
    app.run_return(move |app_handle, event| {
        if matches!(event, tauri::RunEvent::Ready) {
            let handle = app_handle.clone();
            let (commands, visible, errors) = controls.take().unwrap();
            std::thread::spawn(move || {
                let mut web_scripts=std::collections::HashMap::<&'static str,String>::new();
                while let Ok(command) = commands.recv() {
                    match command {
                        Command::OpenWeb {label,url,title,width,height,script} => {
                            let result=(|| -> Result<(),String> {
                                let parsed:tauri::Url=url.parse().map_err(|_|"Invalid integration URL")?;
                                let host=parsed.host_str().unwrap_or("").to_owned();
                                if !matches!(host.as_str(),"music.pentasys.id.vn"|"clip.pentasys.id.vn"|"127.0.0.1") {return Err("Integration origin not allowed".into());}
                                if let Some(window)=handle.get_webview_window(label).filter(|window|web_scripts.get(label).is_some_and(|previous|previous!=&script)||(label=="penta-tutor"&&window.url().is_ok_and(|current|current!=parsed))){
                                    window.destroy().map_err(|e|e.to_string())?;
                                    let deadline=std::time::Instant::now()+std::time::Duration::from_secs(2);
                                    while handle.get_webview_window(label).is_some(){if std::time::Instant::now()>deadline{return Err("Khung đang đóng; thử mở lại.".into());}std::thread::sleep(std::time::Duration::from_millis(20));}
                                }
                                if let Some(window)=handle.get_webview_window(label){
                                    window.show().and_then(|_|window.unminimize()).and_then(|_|window.set_focus()).map_err(|e|e.to_string())?;
                                }else{
                                    let scheme=parsed.scheme().to_owned();let port=parsed.port_or_known_default();
                                    let base=if std::env::args().any(|arg|arg=="--web-integration-check"){std::env::current_dir().map_err(|e|e.to_string())?.join(".build/web-integration-data")}else{handle.path().app_data_dir().map_err(|e|e.to_string())?};
                                    let directory=base.join("integrations").join(label);
                                    std::fs::create_dir_all(&directory).map_err(|e|e.to_string())?;
                                    tauri::WebviewWindowBuilder::new(&handle,label,tauri::WebviewUrl::External(parsed))
                                        .title(title).inner_size(width,height).min_inner_size(360.0,260.0).resizable(true)
                                        .always_on_top(label=="penta-music").data_directory(directory)
                                        .initialization_script(&script)
                                        .on_navigation(move|next|next.host_str()==Some(&host)&&next.scheme()==scheme&&next.port_or_known_default()==port)
                                        .build().map_err(|e|e.to_string())?;
                                }
                                web_scripts.insert(label,script);
                                Ok(())
                            })();
                            if let Err(error)=result{let _=errors.send(format!("Integration: {error}"));}
                        }
                        Command::CloseWeb(label)=>{web_scripts.remove(label);if let Some(window)=handle.get_webview_window(label){let _=window.destroy();}}
                        Command::ProbeWeb{label,script,reply}=>{
                            if let Some(window)=handle.get_webview_window(label){
                                let failure=reply.clone();
                                let result=window.with_webview(move|webview|{
                                    // SAFETY: this closure runs on the WebView UI thread; all COM buffers live through ExecuteScript.
                                    let result=unsafe{
                                        let core=webview.controller().CoreWebView2();
                                        core.and_then(|core|{
                                            let js=webview2_com::CoTaskMemPWSTR::from(script.as_str());
                                            core.ExecuteScript(*js.as_ref().as_pcwstr(),&webview2_com::ExecuteScriptCompletedHandler::create(Box::new(move|status,result|{
                                                let parsed=status.map_err(|e|e.to_string()).and_then(|_|serde_json::from_str::<serde_json::Value>(&result).map_err(|e|e.to_string()));
                                                let _=reply.send(parsed);Ok(())
                                            })))
                                        })
                                    };
                                    if let Err(error)=result{let _=failure.send(Err(error.to_string()));}
                                });
                                if let Err(error)=result{let _=errors.send(error.to_string());}
                            }else{let _=reply.send(Err("Web window is closed".into()));}
                        }
                        Command::Open => {
                            if let Some(window) = handle.get_webview_window("main") {
                                let result = window
                                    .show()
                                    .and_then(|_| window.unminimize())
                                    .and_then(|_| window.set_focus());
                                match result {
                                    Ok(()) => visible.store(true, Ordering::Relaxed),
                                    Err(e) => {
                                        let _ = errors.send(e.to_string());
                                    }
                                }
                            }
                        }
                        Command::Disable | Command::Shutdown => {
                            let shutdown = matches!(command, Command::Shutdown);
                            let app = handle.clone();
                            let visible = visible.clone();
                            tauri::async_runtime::block_on(async move {
                                let state = app.state::<AppState>();
                                let session = state.session.lock().await.take();
                                if let Some(mut session) = session {
                                    let _ = session
                                        .control_tx
                                        .send(crate::session::Control::Stop)
                                        .await;
                                    if let Some(done) = session.done_rx.take() {
                                        if let Ok(Ok(review)) = done.await {
                                            *state.last_meeting.lock().await = Some(review);
                                        }
                                    }
                                    let _ = app.emit("pentakuru://disabled", ());
                                }
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = window.hide();
                                }
                                visible.store(false, Ordering::Relaxed);
                                if shutdown {
                                    app.exit(0);
                                }
                            });
                            if shutdown {
                                break;
                            }
                        }
                        Command::ClosePanel => {
                            if let Some(window) = handle.get_webview_window("main") {
                                let _ = window.close();
                            }
                        }
                        Command::Probe(reply) => {
                            let window = handle.get_webview_window("main").unwrap();
                            let pid = std::process::id();
                            let shown = window.is_visible().unwrap_or(false);
                            handle.once("pentakuru://probe", move |event| {
                                if let Ok(mut payload) = serde_json::from_str::<serde_json::Value>(event.payload()) {
                                    payload["pid"] = pid.into();
                                    payload["visible"] = shown.into();
                                    let _ = reply.send(payload);
                                }
                            });
                            let _ = window.eval(r#"(async () => {
                                const invoke = window.__TAURI_INTERNALS__.invoke;
                                const boot = await invoke('get_boot_info');
                                await invoke('plugin:event|emit', {event:'pentakuru://probe', payload:{
                                    titlebar: !!document.querySelector('.titlebar'),
                                    panels: !!document.querySelector('.panels'),
                                    controls: !!document.querySelector('.session-bar'),
                                    microphoneOff: !!document.querySelector('[data-microphone-muted="true"]'),
                                    styles: getComputedStyle(document.querySelector('.app')).backgroundColor,
                                    needsSetup: boot.needs_setup,
                                    noKey: !boot.config.has_api_key
                                }});
                            })()"#);
                        }
                    }
                }
            });
        }
        crate::handle_exit(app_handle, event);
    });
    Ok(())
}
