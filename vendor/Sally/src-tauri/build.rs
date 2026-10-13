fn main() {
    let mut attributes = tauri_build::Attributes::new();
    if std::env::var_os("CARGO_FEATURE_EMBEDDED").is_some() {
        // The parent eframe executable owns CRT linkage. Tauri's default
        // static-CRT directives must not leak through this library.
        attributes = attributes
            .windows_attributes(tauri_build::WindowsAttributes::new().static_vc_runtime(false));
        // Remove the generated shim if this output directory previously
        // hosted a standalone Tauri build using static CRT linkage.
        let shim =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("msvcrt.lib");
        match std::fs::remove_file(shim) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => panic!("Cannot remove stale generated CRT shim: {e}"),
        }
    }
    tauri_build::try_build(attributes).expect("Sally build configuration failed");
}
