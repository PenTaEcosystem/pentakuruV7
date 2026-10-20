//! Explicit desktop test against our own disposable Win32 editor.
#![cfg(windows)]
#![allow(dead_code)]

// Exercise the production clipboard implementation without configuring an AI account.
mod ai {
    #[derive(Default)]
    pub struct AiSettings {
        pub model: String,
    }
    impl AiSettings {
        pub fn ready(&self) -> bool {
            false
        }
    }
    pub enum Role {
        User,
    }
    pub struct Message {
        pub role: Role,
        pub text: String,
        pub images: Vec<()>,
    }
    pub fn chat(_: &AiSettings, _: &str, _: &[Message]) -> Result<String, String> {
        panic!("The native spell test must never call an AI provider")
    }
}
#[path = "../src/spell.rs"]
mod spell;

#[test]
#[ignore = "Requires an interactive Windows desktop; uses only a disposable editor"]
fn selected_text_is_replaced_and_stale_clipboard_is_rejected() {
    use std::{sync::mpsc, thread, time::Duration};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    let input = "chào bạn , đây là pentakuru .";
    let (tx, rx) = mpsc::channel();
    let worker = thread::spawn(move || unsafe {
        let class: Vec<u16> = "EDIT\0".encode_utf16().collect();
        let text: Vec<u16> = input.encode_utf16().chain(Some(0)).collect();
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            text.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE | 0x0004, /* ES_MULTILINE */
            120,
            120,
            520,
            220,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null(),
        );
        assert!(!hwnd.is_null());
        SetForegroundWindow(hwnd);
        SetFocus(hwnd);
        SendMessageW(hwnd, 0x00B1 /* EM_SETSEL */, 0, -1);
        tx.send(hwnd as isize).unwrap();
        let mut message: MSG = std::mem::zeroed();
        while IsWindow(hwnd) != 0 {
            while PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            thread::sleep(Duration::from_millis(5));
        }
    });
    let target = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    struct Editor(isize, Option<thread::JoinHandle<()>>);
    impl Drop for Editor {
        fn drop(&mut self) {
            unsafe {
                PostMessageW(self.0 as _, WM_CLOSE, 0, 0);
            }
            if let Some(worker) = self.1.take() {
                worker.join().unwrap();
            }
        }
    }
    let _editor = Editor(target, Some(worker));
    thread::sleep(Duration::from_millis(150));
    let selected = spell::capture_selected_text_windows(target, 0).unwrap();
    assert_eq!(selected, input);
    let result = spell::fix_text(
        &spell::SpellSettings::default(),
        &ai::AiSettings::default(),
        &selected,
    );
    assert!(result.changed);
    assert_eq!(result.fixed, "Chào bạn, đây là PentaKuru.");
    spell::paste_text_windows(&result.fixed, target).unwrap();
    thread::sleep(Duration::from_millis(150));
    let mut text = vec![0u16; 256];
    let count = unsafe { GetWindowTextW(target as _, text.as_mut_ptr(), text.len() as i32) };
    assert_eq!(
        String::from_utf16(&text[..count as usize]).unwrap(),
        result.fixed
    );
    unsafe {
        SendMessageW(target as _, 0x00B1 /* EM_SETSEL */, 0, 0);
    }
    assert!(
        spell::capture_selected_text_windows(target, 0).is_err(),
        "An empty selection must not reuse the previous clipboard text"
    );
    assert!(spell::paste_text_windows("test", 0).is_err());
}
