use std::{env, fs, path::Path};

// Must match xor::XOR_KEY
const XOR_KEY: &[u8] = b"kg-memphis-2026-salt-do-not-reuse";

fn main() {
    // Build-time only: read key from env (local export or CI secret). Never committed.
    let key = env::var("DASHSCOPE_API_KEY").unwrap_or_default();
    let obf: Vec<u8> = key
        .as_bytes()
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ XOR_KEY[i % XOR_KEY.len()])
        .collect();
    let arr = obf.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(",");
    let out = env::var("OUT_DIR").unwrap();
    fs::write(
        Path::new(&out).join("embedded_key.rs"),
        format!("pub const OBFUSCATED_KEY: &[u8] = &[{arr}];\n"),
    )
    .unwrap();
    println!("cargo:rerun-if-env-changed=DASHSCOPE_API_KEY");

    tauri_build::build();
}
