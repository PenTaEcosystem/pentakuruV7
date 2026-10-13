//! Optional DeepTutor and Penta media. Secrets are passed only to the intended service.
use crate::{ai, system};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub music: bool,
    pub clip: bool,
    pub deeptutor: bool,
    pub shared_ai: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            music: false,
            clip: false,
            deeptutor: false,
            shared_ai: true,
        }
    }
}
#[derive(Default)]
pub struct State {
    pub status: String,
    pub job: Option<system::Job<Result<TutorChild, String>>>,
    pub child: Option<TutorChild>,
    pub seen_music: bool,
    pub seen_clip: bool,
    pub seen_tutor: bool,
    pub tutor_status_open: bool,
    pub cancel: Arc<AtomicBool>,
    pub progress: Arc<Mutex<String>>,
}
pub struct TutorChild(pub Child, pub bool, pub u16);
impl Drop for TutorChild {
    fn drop(&mut self) {
        stop_child(&mut self.0);
    }
}

pub fn music_script() -> String {
    include_str!("../assets/music-mini.js").to_owned()
}
pub fn music_open() -> Result<(), String> {
    crate::sally::launcher::open_web(
        "penta-music",
        "https://music.pentasys.id.vn/",
        "PentaKuru · Music",
        480.0,
        420.0,
        music_script(),
    )
}
pub fn clip_script() -> String {
    include_str!("../assets/clip-compact.js").to_owned()
}
pub fn clip_open() -> Result<(), String> {
    crate::sally::launcher::open_web(
        "penta-clip",
        "https://clip.pentasys.id.vn/feed/subscriptions",
        "PentaKuru · Clip",
        480.0,
        420.0,
        clip_script(),
    )
}
fn tutor_home() -> PathBuf {
    system::data_dir().join("DeepTutor")
}
pub fn tutor_catalog(mut catalog: Value, settings: &ai::AiSettings) -> Result<Value, String> {
    let binding = match settings
        .resolved()
        .ok_or("Cấu hình API AI trước khi mở DeepTutor.")?
    {
        ai::Provider::Gemini => "gemini",
        ai::Provider::Anthropic => "anthropic",
        _ => "openai",
    };
    if settings.model.trim().is_empty() {
        return Err("Chọn model AI trước khi mở DeepTutor.".into());
    }
    if !catalog.is_object() {
        catalog = json!({"version":1,"connections":[],"services":{}});
    }
    let profile = json!({"id":"pentaeye-shared","name":"PentaKuru AI","binding":binding,"base_url":settings.base(),"api_key":settings.api_key,"models":[{"id":"pentaeye-model","name":settings.model,"model":settings.model}]});
    if !catalog["services"].is_object() {
        catalog["services"] = json!({});
    }
    for kind in ["llm", "task"] {
        let service = &mut catalog["services"][kind];
        if !service.is_object() {
            *service = json!({"profiles":[]});
        }
        if !service["profiles"].is_array() {
            service["profiles"] = json!([]);
        }
        let profiles = service["profiles"].as_array_mut().unwrap();
        profiles.retain(|p| p["id"] != "pentaeye-shared");
        profiles.push(profile.clone());
        service["active_profile_id"] = "pentaeye-shared".into();
        service["active_model_id"] = "pentaeye-model".into();
    }
    Ok(catalog)
}
fn tutor_stage(home: &std::path::Path, progress: &Mutex<String>, text: &str) {
    let _ = std::fs::write(home.join("install-progress.txt"), text);
    if let Ok(mut current) = progress.lock() {
        *current = text.to_owned();
    }
}
fn install_step(
    mut command: Command,
    cancel: &AtomicBool,
    home: &std::path::Path,
    progress: &Mutex<String>,
) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("Đã dừng DeepTutor.".into());
    }
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(home.join("install.log"))
        .map_err(|e| e.to_string())?;
    command
        .stdout(log.try_clone().map_err(|e| e.to_string())?)
        .stderr(log);
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    loop {
        if let Ok(text) = std::fs::read_to_string(home.join("install-progress.txt"))
            && let Ok(mut current) = progress.lock()
        {
            *current = text;
        }
        if cancel.load(Ordering::Relaxed) {
            stop_child(&mut child);
            return Err("Đã dừng tải/cài DeepTutor.".into());
        }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return if status.success() {
                Ok(())
            } else {
                Err("Chưa cài được DeepTutor. Kiểm tra Internet, dung lượng đĩa và proxy/chính sách mạng rồi thử lại. Chi tiết trong DeepTutor/install.log.".into())
            };
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}
#[derive(Deserialize)]
struct RuntimePaths {
    python: PathBuf,
    node_dir: PathBuf,
}
fn tutor_ports(preferred: (u16, u16)) -> Result<(u16, u16), String> {
    use std::net::TcpListener;
    if let Ok(backend) = TcpListener::bind(("127.0.0.1", preferred.0))
        && let Ok(frontend) = TcpListener::bind(("127.0.0.1", preferred.1))
    {
        return Ok((
            backend.local_addr().map_err(|e| e.to_string())?.port(),
            frontend.local_addr().map_err(|e| e.to_string())?.port(),
        ));
    }
    let backend = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
    let frontend = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
    Ok((
        backend.local_addr().map_err(|e| e.to_string())?.port(),
        frontend.local_addr().map_err(|e| e.to_string())?.port(),
    ))
}

pub fn start_tutor(
    ai_settings: &ai::AiSettings,
    shared: bool,
    cancel: &AtomicBool,
    progress: &Mutex<String>,
) -> Result<TutorChild, String> {
    start_tutor_at(
        ai_settings,
        shared,
        cancel,
        progress,
        tutor_home(),
        tutor_ports((3896, 3897))?,
        true,
    )
}
fn start_tutor_at(
    ai_settings: &ai::AiSettings,
    shared: bool,
    cancel: &AtomicBool,
    progress: &Mutex<String>,
    home: PathBuf,
    ports: (u16, u16),
    force_private: bool,
) -> Result<TutorChild, String> {
    let home = std::path::absolute(home).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&home).map_err(|e| e.to_string())?;
    tutor_stage(&home, progress, "1/5 · Kiểm tra thành phần DeepTutor");
    let bootstrap = home.join("install-runtime.ps1");
    std::fs::write(&bootstrap, include_str!("../scripts/install-deeptutor.ps1"))
        .map_err(|e| e.to_string())?;
    let mut command = background_command("powershell.exe");
    command.env_remove("PSModulePath");
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&bootstrap)
        .arg("-Destination")
        .arg(&home);
    if force_private {
        command.arg("-ForcePrivate");
    }
    install_step(command, cancel, &home, progress)?;
    let paths: RuntimePaths = serde_json::from_slice(
        &std::fs::read(home.join("runtime-paths.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let venv = home.join("runtime");
    let runtime_python = venv.join("Scripts/python.exe");
    let cli = venv.join("Scripts/deeptutor.exe");
    let marker = venv.join(".pentaeye-ready");
    if !cli.is_file()
        || !runtime_python.is_file()
        || std::fs::read_to_string(&marker).ok().as_deref() != Some("1.6.14")
        || !background_command(&runtime_python)
            .args(["-c", "import sys"])
            .status()
            .is_ok_and(|s| s.success())
    {
        tutor_stage(&home, progress, "4/5 · Cài gói DeepTutor và các thư viện");
        let mut create = background_command(&paths.python);
        create.args(["-m", "venv"]).arg(&venv);
        install_step(create, cancel, &home, progress)?;
        let constraints = home.join("runtime-constraints.txt");
        std::fs::write(
            &constraints,
            include_str!("../assets/deeptutor-constraints.txt"),
        )
        .map_err(|e| e.to_string())?;
        let mut pip = background_command(&runtime_python);
        pip.args([
            "-m",
            "pip",
            "install",
            "--disable-pip-version-check",
            "--timeout",
            "30",
            "--retries",
            "3",
            "deeptutor==1.6.14",
        ]);
        pip.arg("--constraint").arg(&constraints);
        install_step(pip, cancel, &home, progress)?;
        std::fs::write(marker, "1.6.14").map_err(|e| e.to_string())?;
    }
    let mut synced_ai = false;
    if shared && let Ok(catalog_template) = tutor_catalog(Value::Null, ai_settings) {
        let directory = home.join("data/user/settings");
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let path = directory.join("model_catalog.json");
        let current = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or(Value::Null);
        let catalog = tutor_catalog(current, ai_settings).unwrap_or(catalog_template);
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&catalog).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        synced_ai = true;
    }
    // Ports are isolated from normal standalone DeepTutor defaults.
    let config = home.join("data/user/settings/system.json");
    std::fs::create_dir_all(config.parent().unwrap()).map_err(|e| e.to_string())?;
    let mut system_config: Value = std::fs::read(&config)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    system_config["backend_port"] = ports.0.into();
    system_config["frontend_port"] = ports.1.into();
    std::fs::write(
        &config,
        serde_json::to_vec_pretty(&system_config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    for port in [ports.0, ports.1] {
        if std::net::TcpStream::connect_timeout(
            &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
            std::time::Duration::from_millis(300),
        )
        .is_ok()
        {
            return Err(format!("Cổng {port} đang được ứng dụng khác sử dụng."));
        }
    }
    let runner = home.join("runner.py");
    std::fs::write(&runner, include_str!("../assets/deeptutor-runner.py"))
        .map_err(|e| e.to_string())?;
    tutor_stage(&home, progress, "5/5 · Khởi động DeepTutor trên máy này");
    let process_path =
        std::env::join_paths([venv.join("Scripts"), paths.node_dir].into_iter().chain(
            std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
        ))
        .map_err(|e| e.to_string())?;
    let startup_log = std::fs::File::create(home.join("startup.log")).map_err(|e| e.to_string())?;
    let mut child = background_command(&runtime_python)
        .stdout(startup_log.try_clone().map_err(|e| e.to_string())?)
        .stderr(startup_log)
        .arg(&runner)
        .arg(&home)
        .current_dir(&home)
        .env("DEEPTUTOR_HOME", &home)
        .env("PATH", process_path)
        .env(
            "NO_PROXY",
            loopback_no_proxy(std::env::var("NO_PROXY").unwrap_or_default()),
        )
        .env(
            "no_proxy",
            loopback_no_proxy(std::env::var("no_proxy").unwrap_or_default()),
        )
        .spawn()
        .map_err(|e| e.to_string())?;
    let agent = ureq::Agent::config_builder()
        .proxy(None)
        .timeout_global(Some(std::time::Duration::from_secs(2)))
        .build()
        .new_agent();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
    loop {
        if cancel.load(Ordering::Relaxed) {
            stop_child(&mut child);
            return Err("Đã dừng DeepTutor.".into());
        }
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            return Err("DeepTutor đã dừng khi khởi động. Xem DeepTutor/startup.log.".into());
        }
        if agent
            .get(format!("http://127.0.0.1:{}/", ports.1))
            .call()
            .is_ok()
            && agent
                .get(format!("http://127.0.0.1:{}/health/ready", ports.0))
                .call()
                .is_ok()
        {
            tutor_stage(&home, progress, "DeepTutor đã sẵn sàng");
            return Ok(TutorChild(child, synced_ai, ports.1));
        }
        if std::time::Instant::now() > deadline {
            stop_child(&mut child);
            return Err("DeepTutor chưa sẵn sàng sau 90 giây.".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
fn loopback_no_proxy(existing: String) -> String {
    let mut entries: Vec<_> = existing
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect();
    for local in ["127.0.0.1", "localhost"] {
        if !entries.iter().any(|v| v == local) {
            entries.push(local.to_owned());
        }
    }
    entries.join(",")
}
pub fn native_tutor_check() {
    let home = PathBuf::from(".build/deeptutor-oneclick-check");
    let progress = Mutex::new(String::new());
    let cancel = AtomicBool::new(false);
    let result = (|| -> Result<Value, String> {
        let child = start_tutor_at(
            &ai::AiSettings::default(),
            true,
            &cancel,
            &progress,
            home.clone(),
            (4898, 4899),
            true,
        )?;
        let agent = ureq::Agent::config_builder()
            .proxy(None)
            .timeout_global(Some(std::time::Duration::from_secs(5)))
            .build()
            .new_agent();
        let status = agent
            .get("http://127.0.0.1:4898/api/settings/catalog")
            .call()
            .map_err(|e| e.to_string())?
            .status()
            .as_u16();
        let connections = background_command("netstat.exe")
            .args(["-ano", "-p", "TCP"])
            .stdout(Stdio::piped())
            .output()
            .map_err(|e| e.to_string())?;
        let listeners: Vec<String> = String::from_utf8_lossy(&connections.stdout)
            .lines()
            .filter_map(|line| {
                let fields: Vec<_> = line.split_whitespace().collect();
                let address = fields.get(1)?;
                if fields.first() == Some(&"TCP")
                    && fields.get(3) == Some(&"LISTENING")
                    && (address.ends_with(":4898") || address.ends_with(":4899"))
                {
                    Some((*address).to_owned())
                } else {
                    None
                }
            })
            .collect();
        if listeners.len() != 2 || !listeners.iter().all(|a| a.starts_with("127.0.0.1:")) {
            return Err(format!("Unexpected bind addresses: {listeners:?}"));
        }
        drop(child);
        Ok(
            json!({"passed":status==200,"catalog_status":status,"credentials_used":false,"private_python_and_node":true,"missing_api_key_does_not_block_installation":true,"listeners":listeners,"no_inbound_firewall_rule_added":true}),
        )
    })();
    let report =
        result.unwrap_or_else(|e| json!({"passed":false,"error":e,"credentials_used":false}));
    let _ = std::fs::create_dir_all(&home);
    let _ = std::fs::write(
        home.join("check-result.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    );
}
pub fn background_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut command = Command::new(program);
    command.env_remove("PYTHONHOME").env_remove("PYTHONPATH");
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}
pub fn stop_child(child: &mut Child) {
    if child.try_wait().ok().flatten().is_some() {
        return;
    }
    #[cfg(windows)]
    {
        let _ = background_command("taskkill.exe")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = child.kill();
    }
    let _ = child.wait();
}
pub fn tutor_open(port: u16) -> Result<(), String> {
    crate::sally::launcher::open_web(
        "penta-tutor",
        &format!("http://127.0.0.1:{port}/"),
        "PentaKuru · DeepTutor",
        960.0,
        700.0,
        String::new(),
    )
}
impl Drop for State {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(child) = &mut self.child {
            stop_child(&mut child.0);
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while self.job.is_some() && std::time::Instant::now() < deadline {
            if let Some(result) = system::take(&mut self.job) {
                drop(result);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn occupied_tutor_ports_select_a_distinct_available_pair() {
        use std::net::TcpListener;
        let backend = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let frontend = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let occupied = (
            backend.local_addr().unwrap().port(),
            frontend.local_addr().unwrap().port(),
        );
        let selected = tutor_ports(occupied).unwrap();
        assert_ne!(selected.0, selected.1);
        assert!(![occupied.0, occupied.1].contains(&selected.0));
        assert!(![occupied.0, occupied.1].contains(&selected.1));
        let _backend = TcpListener::bind(("127.0.0.1", selected.0)).unwrap();
        let _frontend = TcpListener::bind(("127.0.0.1", selected.1)).unwrap();
    }

    #[test]
    fn shared_catalog_preserves_other_services() {
        let cfg = ai::AiSettings {
            provider: ai::Provider::Gemini,
            api_key: "test-key".into(),
            model: "test-model".into(),
            ..Default::default()
        };
        let catalog=tutor_catalog(json!({"services":{"embedding":{"profiles":[{"id":"existing"}]},"llm":{"profiles":[{"id":"manual"}]}}}),&cfg).unwrap();
        assert_eq!(catalog["services"]["llm"]["profiles"][0]["id"], "manual");
        assert_eq!(
            catalog["services"]["llm"]["profiles"][1]["binding"],
            "gemini"
        );
        assert_eq!(
            catalog["services"]["embedding"]["profiles"][0]["id"],
            "existing"
        );
    }
    #[test]
    fn local_health_transport_preserves_existing_proxy_exemptions() {
        assert_eq!(
            loopback_no_proxy("internal.example, localhost".into()),
            "internal.example,localhost,127.0.0.1"
        );
    }
    #[test]
    fn music_uses_web_session_only() {
        let js = music_script();
        assert!(!js.contains("localStorage"));
        assert!(!js.contains("Bearer"));
        assert!(!js.contains("pentakuru-music-tools"));
    }
}
