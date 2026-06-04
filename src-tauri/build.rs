fn main() {
    // Link AVFoundation on macOS so we can request microphone permission at runtime.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-lib=framework=AVFoundation");
    }
    tauri_build::build();
}
