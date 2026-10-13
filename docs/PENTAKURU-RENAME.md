# PentaKuru rename

PentaEye/Penteye is now PentaKuru. The Cargo package and binary are `pentakuru`; native windows, assets, build scripts and installer use the new branding.

Storage IDs, the legacy `Penteye Notes` directory, Tauri identifier, startup registry value, DeepTutor profile IDs and installer AppId remain stable for compatibility. The installer updates an existing startup entry to `PentaKuru.exe` and removes the replaced `PentaEye.exe` in the same application directory. Upgrades reuse the prior installation directory. Existing MIT copyright attribution remains unchanged.

Validation: `cargo fmt --check`, `cargo test --locked` (50 passed), `cargo clippy --locked --all-targets -- -D warnings`. Historical verification JSON files describe earlier PentaEye builds; they are not evidence for the renamed binary.

The repository includes source only. Downloaded runtimes, build outputs, credentials and personal notes are excluded. CI targets Windows and builds the embedded Sally frontend before Rust.
