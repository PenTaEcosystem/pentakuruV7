//! Rebindable shortcuts. On Windows they are registered system-wide with
//! `RegisterHotKey` on a dedicated message thread, so Alt+Z works while you
//! type in any application. Elsewhere they work while PentaKuru has focus.
use eframe::egui::{self, Key, Modifiers};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{Receiver, Sender, channel};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    FixText,
    Settings,
    Search,
    Notes,
    Ai,
    Translate,
}

impl Action {
    pub const ALL: [Action; 6] = [
        Action::FixText,
        Action::Settings,
        Action::Search,
        Action::Notes,
        Action::Ai,
        Action::Translate,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Action::FixText => "Sửa chính tả đoạn đã tô đen",
            Action::Settings => "Cài đặt & phím tắt",
            Action::Search => "Thanh tìm kiếm trong máy",
            Action::Notes => "Ghi chú",
            Action::Ai => "Hỏi AI",
            Action::Translate => "Dịch cuộc họp trực tiếp (Sally)",
        }
    }

    fn id(self) -> i32 {
        Action::ALL.iter().position(|a| *a == self).unwrap_or(0) as i32 + 1
    }

    fn from_id(id: i32) -> Option<Self> {
        Action::ALL.get(usize::try_from(id - 1).ok()?).copied()
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Binding {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    /// `egui::Key::name()`; empty means "not assigned".
    pub key: String,
}

impl Binding {
    pub fn alt(key: Key) -> Self {
        Self {
            alt: true,
            key: key.name().to_owned(),
            ..Default::default()
        }
    }

    pub fn key(&self) -> Option<Key> {
        Key::from_name(&self.key)
    }

    pub fn is_empty(&self) -> bool {
        self.key().is_none()
    }

    pub fn label(&self) -> String {
        if self.is_empty() {
            return "Chưa gán".into();
        }
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        parts.push(&self.key);
        parts.join(" + ")
    }

    /// Build a binding from a key press while recording. Requires Ctrl or Alt
    /// so ordinary typing is never swallowed system-wide.
    pub fn from_press(modifiers: Modifiers, key: Key) -> Option<Self> {
        if !(modifiers.ctrl || modifiers.alt) || vk(key).is_none() {
            return None;
        }
        Some(Self {
            ctrl: modifiers.ctrl,
            alt: modifiers.alt,
            shift: modifiers.shift,
            key: key.name().to_owned(),
        })
    }

    pub fn matches(&self, modifiers: Modifiers, key: Key) -> bool {
        self.key() == Some(key)
            && self.ctrl == modifiers.ctrl
            && self.alt == modifiers.alt
            && self.shift == modifiers.shift
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Bindings {
    pub fix_text: Binding,
    pub settings: Binding,
    pub search: Binding,
    pub notes: Binding,
    pub ai: Binding,
    pub translate: Binding,
}

impl Default for Bindings {
    fn default() -> Self {
        Self {
            fix_text: Binding::alt(Key::Z),
            settings: Binding::alt(Key::Q),
            search: Binding::alt(Key::S),
            notes: Binding::alt(Key::N),
            ai: Binding::alt(Key::A),
            translate: Binding::alt(Key::T),
        }
    }
}

impl Bindings {
    pub fn get(&self, action: Action) -> &Binding {
        match action {
            Action::FixText => &self.fix_text,
            Action::Settings => &self.settings,
            Action::Search => &self.search,
            Action::Notes => &self.notes,
            Action::Ai => &self.ai,
            Action::Translate => &self.translate,
        }
    }

    pub fn get_mut(&mut self, action: Action) -> &mut Binding {
        match action {
            Action::FixText => &mut self.fix_text,
            Action::Settings => &mut self.settings,
            Action::Search => &mut self.search,
            Action::Notes => &mut self.notes,
            Action::Ai => &mut self.ai,
            Action::Translate => &mut self.translate,
        }
    }

    /// Assign `binding` to `action`; any other action using the same combo is
    /// cleared. Returns the action that lost its shortcut, if any.
    pub fn assign(&mut self, action: Action, binding: Binding) -> Option<Action> {
        let mut displaced = None;
        for other in Action::ALL {
            if other != action && *self.get(other) == binding {
                *self.get_mut(other) = Binding::default();
                displaced = Some(other);
            }
        }
        *self.get_mut(action) = binding;
        displaced
    }

    pub fn reset_defaults(&mut self) {
        *self = Self::default();
    }

    /// Try an alternative combination (adding Ctrl or Shift) when conflicted with another app.
    pub fn suggest_alternative(&mut self, action: Action) {
        let current = self.get(action).clone();
        if current.is_empty() {
            return;
        }
        let key_name = current.key;
        let next = if current.alt && !current.ctrl && !current.shift {
            // Alt + Key -> Ctrl + Alt + Key
            Binding {
                ctrl: true,
                alt: true,
                shift: false,
                key: key_name,
            }
        } else if current.alt && current.ctrl && !current.shift {
            // Ctrl + Alt + Key -> Alt + Shift + Key
            Binding {
                ctrl: false,
                alt: true,
                shift: true,
                key: key_name,
            }
        } else if current.alt && current.shift && !current.ctrl {
            // Alt + Shift + Key -> Ctrl + Shift + Key
            Binding {
                ctrl: true,
                alt: false,
                shift: true,
                key: key_name,
            }
        } else {
            // Reset to Alt + Key
            Binding {
                ctrl: false,
                alt: true,
                shift: false,
                key: key_name,
            }
        };
        self.assign(action, next);
    }

    pub fn list(&self) -> Vec<(Action, Binding)> {
        Action::ALL
            .iter()
            .map(|a| (*a, self.get(*a).clone()))
            .filter(|(_, b)| !b.is_empty())
            .collect()
    }

    /// Shortcut pressed inside a PentaKuru window (fallback when global
    /// registration is unavailable or failed).
    pub fn pressed_in(&self, ctx: &egui::Context) -> Vec<Action> {
        ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        repeat: false,
                        modifiers,
                        ..
                    } => Action::ALL
                        .into_iter()
                        .find(|a| self.get(*a).matches(*modifiers, *key)),
                    _ => None,
                })
                .collect()
        })
    }
}

/// Windows virtual-key code for an egui key.
pub fn vk(key: Key) -> Option<u16> {
    let name = key.name();
    let bytes = name.as_bytes();
    if bytes.len() == 1 && (bytes[0].is_ascii_uppercase() || bytes[0].is_ascii_digit()) {
        return Some(u16::from(bytes[0]));
    }
    if let Some(n) = name.strip_prefix('F').and_then(|n| n.parse::<u16>().ok())
        && (1..=24).contains(&n)
    {
        return Some(0x6F + n);
    }
    Some(match key {
        Key::Space => 0x20,
        Key::ArrowLeft => 0x25,
        Key::ArrowUp => 0x26,
        Key::ArrowRight => 0x27,
        Key::ArrowDown => 0x28,
        Key::Insert => 0x2D,
        Key::Delete => 0x2E,
        Key::Home => 0x24,
        Key::End => 0x23,
        Key::PageUp => 0x21,
        Key::PageDown => 0x22,
        Key::Semicolon => 0xBA,
        Key::Equals | Key::Plus => 0xBB,
        Key::Comma => 0xBC,
        Key::Minus => 0xBD,
        Key::Period => 0xBE,
        Key::Slash => 0xBF,
        Key::Backtick => 0xC0,
        Key::OpenBracket => 0xDB,
        Key::Backslash => 0xDC,
        Key::CloseBracket => 0xDD,
        Key::Quote => 0xDE,
        _ => return None,
    })
}

pub enum HotkeyEvent {
    Pressed(Action, isize),
    /// Registration result per action (`Err` holds a readable reason).
    Status(Vec<(Action, Result<(), String>)>),
}

pub struct HotkeyService {
    commands: Sender<Vec<(Action, Binding)>>,
    events: Receiver<HotkeyEvent>,
    #[cfg(target_os = "windows")]
    thread_id: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl HotkeyService {
    #[allow(dead_code)]
    pub const GLOBAL: bool = cfg!(target_os = "windows");

    pub fn start(ctx: egui::Context) -> Self {
        let (commands, command_rx) = channel();
        let (event_tx, events) = channel();
        #[cfg(target_os = "windows")]
        {
            let (ready_tx, ready_rx) = channel();
            let thread_id = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
            windows::spawn(ctx, command_rx, event_tx, thread_id.clone(), ready_tx);
            // Đợi luồng Win32 tạo xong message queue và lưu thread_id thành công
            let _ = ready_rx.recv_timeout(std::time::Duration::from_millis(2000));
            Self {
                commands,
                events,
                thread_id,
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (ctx, command_rx, event_tx);
            Self { commands, events }
        }
    }

    /// Replace all registered shortcuts. An empty list suspends them (used
    /// while recording a new combination).
    pub fn apply(&self, bindings: Vec<(Action, Binding)>) {
        let _ = self.commands.send(bindings);
        #[cfg(target_os = "windows")]
        windows::wake(&self.thread_id);
    }

    pub fn poll(&self) -> Vec<HotkeyEvent> {
        self.events.try_iter().collect()
    }
}

#[cfg(target_os = "windows")]
impl Drop for HotkeyService {
    fn drop(&mut self) {
        windows::quit(&self.thread_id);
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    };
    use windows_sys::Win32::{
        Foundation::GetLastError,
        System::Threading::GetCurrentThreadId,
        UI::{
            Input::KeyboardAndMouse::{
                MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, RegisterHotKey, UnregisterHotKey,
            },
            WindowsAndMessaging::{
                GetMessageW, MSG, PM_NOREMOVE, PeekMessageW, PostThreadMessageW, WM_APP, WM_HOTKEY,
                WM_QUIT, WM_USER,
            },
        },
    };

    const WM_APPLY: u32 = WM_APP + 41;

    pub fn spawn(
        ctx: egui::Context,
        commands: Receiver<Vec<(Action, Binding)>>,
        events: Sender<HotkeyEvent>,
        thread_id: Arc<AtomicU32>,
        ready_tx: Sender<u32>,
    ) {
        let _ = std::thread::Builder::new()
            .name("pentakuru-hotkeys".into())
            .spawn(move || {
                // SAFETY: standard Win32 message loop owned by this thread;
                // MSG is written by the API before it is read.
                unsafe {
                    let mut msg: MSG = std::mem::zeroed();
                    // Create the thread message queue before publishing the id.
                    PeekMessageW(
                        &mut msg,
                        std::ptr::null_mut(),
                        WM_USER,
                        WM_USER,
                        PM_NOREMOVE,
                    );
                    let tid = GetCurrentThreadId();
                    thread_id.store(tid, Ordering::SeqCst);
                    let _ = ready_tx.send(tid);

                    let mut active: Vec<i32> = Vec::new();
                    let apply_latest = |active: &mut Vec<i32>| {
                        let mut latest = None;
                        while let Ok(b) = commands.try_recv() {
                            latest = Some(b);
                        }
                        let Some(bindings) = latest else {
                            return;
                        };
                        for id in active.drain(..) {
                            UnregisterHotKey(std::ptr::null_mut(), id);
                        }
                        let mut status = Vec::new();
                        for (action, binding) in bindings {
                            let Some(code) = binding.key().and_then(vk) else {
                                continue;
                            };
                            let mut modifiers = MOD_NOREPEAT;
                            if binding.alt {
                                modifiers |= MOD_ALT;
                            }
                            if binding.ctrl {
                                modifiers |= MOD_CONTROL;
                            }
                            if binding.shift {
                                modifiers |= MOD_SHIFT;
                            }
                            let id = action.id();
                            if RegisterHotKey(std::ptr::null_mut(), id, modifiers, u32::from(code))
                                != 0
                            {
                                active.push(id);
                                status.push((action, Ok(())));
                            } else {
                                let error = GetLastError();
                                status.push((
                                    action,
                                    Err(if error == 1409 {
                                        "Đã bị ứng dụng khác chiếm (NVIDIA/Unikey/v.v.)".to_owned()
                                    } else {
                                        format!("Lỗi đăng ký ({error})")
                                    }),
                                ));
                            }
                        }
                        let _ = events.send(HotkeyEvent::Status(status));
                        ctx.request_repaint();
                    };
                    apply_latest(&mut active);
                    loop {
                        let result = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
                        if result == 0 || result == -1 {
                            break;
                        }
                        match msg.message {
                            WM_HOTKEY => {
                                if let Some(action) = Action::from_id(msg.wParam as i32) {
                                    let fg = windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() as isize;
                                    let _ = events.send(HotkeyEvent::Pressed(action, fg));
                                    ctx.request_repaint();
                                }
                            }
                            WM_APPLY => apply_latest(&mut active),
                            _ => {}
                        }
                    }
                    for id in active {
                        UnregisterHotKey(std::ptr::null_mut(), id);
                    }
                }
            });
    }

    pub fn wake(thread_id: &AtomicU32) {
        let id = thread_id.load(Ordering::SeqCst);
        if id != 0 {
            // SAFETY: posting a message to a thread id has no memory effects.
            unsafe {
                for _ in 0..10 {
                    if PostThreadMessageW(id, WM_APPLY, 0, 0) != 0 {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            };
        }
    }

    pub fn quit(thread_id: &AtomicU32) {
        let id = thread_id.load(Ordering::SeqCst);
        if id != 0 {
            // SAFETY: see `wake`.
            unsafe { PostThreadMessageW(id, WM_QUIT, 0, 0) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_labels_and_conflicts() {
        let mut b = Bindings::default();
        assert_eq!(b.fix_text.label(), "Alt + Z");
        assert_eq!(b.settings.label(), "Alt + Q");
        assert_eq!(b.translate.label(), "Alt + T");
        assert_eq!(vk(Key::Z), Some(0x5A));
        assert_eq!(vk(Key::F5), Some(0x74));
        assert!(Binding::from_press(Modifiers::NONE, Key::Z).is_none());
        let combo = Binding::from_press(Modifiers::ALT, Key::S).unwrap();
        assert_eq!(b.assign(Action::Ai, combo), Some(Action::Search));
        assert!(b.search.is_empty());
        assert_eq!(b.list().len(), 5);
        for action in Action::ALL {
            assert_eq!(Action::from_id(action.id()), Some(action));
        }
    }
}
