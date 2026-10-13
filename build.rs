fn main() {
    println!("cargo:rerun-if-changed=assets/pentakuru.manifest");
    println!("cargo:rerun-if-changed=assets/pentakuru-icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var_os("CARGO_FEATURE_SALLY_FULL").is_some()
    {
        // Sally's dialog plugin imports TaskDialogIndirect, which requires
        // Common Controls v6. The resource on the Sally dependency's binary
        // does not propagate to this parent executable (or its test harness).
        tauri_winres::WindowsResource::new()
            .set_manifest_file("assets/pentakuru.manifest")
            .set_icon_with_id("assets/pentakuru-icon.ico", "32512")
            .set("ProductName", "PentaKuru")
            .set(
                "FileDescription",
                "PentaKuru Office, Music, Video and DeepTutor",
            )
            .compile()
            .expect("Cannot build PentaKuru's Windows manifest");
    }
}
