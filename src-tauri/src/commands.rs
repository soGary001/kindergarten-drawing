use crate::{asr, gallery, image_gen, secret, settings::{self, AppSettings}};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

#[derive(Default)]
pub struct AppState {
    pub recent: Mutex<Vec<usize>>,
    pub asr: Mutex<Option<asr::AsrSession>>,
}

fn app_data(app: &AppHandle) -> std::path::PathBuf {
    app.path().app_data_dir().unwrap_or(std::env::temp_dir())
}

fn active_gallery_dir(app: &AppHandle, s: &AppSettings) -> std::path::PathBuf {
    if let Some(d) = &s.gallery_dir { return std::path::PathBuf::from(d); }
    let dir = app_data(app).join("gallery");
    let _ = std::fs::create_dir_all(&dir); // ensure it exists for the operator to fill
    dir
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> AppSettings { settings::load(&app_data(&app)) }

#[tauri::command]
pub fn set_settings(app: AppHandle, new: AppSettings) -> Result<(), String> {
    settings::save(&app_data(&app), &new)
}

#[tauri::command]
pub fn list_gallery(app: AppHandle) -> Vec<gallery::ImageMeta> {
    let s = settings::load(&app_data(&app));
    gallery::scan(&active_gallery_dir(&app, &s))
}

#[tauri::command]
pub fn draw_random(app: AppHandle, state: State<AppState>) -> Result<gallery::ImageMeta, String> {
    let s = settings::load(&app_data(&app));
    let all = gallery::scan(&active_gallery_dir(&app, &s));
    if all.is_empty() { return Err("gallery is empty".into()); }
    let mut recent = state.recent.lock().unwrap();
    let (idx, meta) = gallery::draw_random(&all, &recent).ok_or("draw failed")?;
    recent.push(idx);
    if recent.len() > 3 { recent.remove(0); }
    Ok(meta)
}

#[tauri::command]
pub async fn generate_image(app: AppHandle, transcript: String) -> Result<String, String> {
    let cache = app_data(&app).join("cache");
    image_gen::generate(&transcript, &cache).await
}

#[tauri::command]
pub async fn asr_start(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let key = secret::api_key();
    let session = asr::run_session(app.clone(), key).await?;
    *state.asr.lock().unwrap() = Some(session);
    Ok(())
}

#[tauri::command]
pub fn asr_send_audio(state: State<AppState>, chunk: Vec<u8>) -> Result<(), String> {
    if let Some(s) = state.asr.lock().unwrap().as_ref() {
        s.audio_tx.send(chunk).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn asr_stop(state: State<AppState>) -> Result<(), String> {
    if let Some(s) = state.asr.lock().unwrap().as_ref() { let _ = s.stop_tx.send(()); }
    Ok(())
}

#[tauri::command]
pub fn save_snapshot(app: AppHandle, png_base64: String) -> Result<String, String> {
    use std::io::Write;
    let s = settings::load(&app_data(&app));
    let dir = s.snapshot_dir.map(std::path::PathBuf::from).unwrap_or_else(|| app_data(&app).join("snapshots"));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let raw = png_base64.split(',').last().unwrap_or(&png_base64);
    let bytes = base64_decode(raw)?;
    let path = dir.join(format!("snapshot_{}.png", uuid::Uuid::new_v4()));
    let mut f = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    f.write_all(&bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut rev = [255u8; 256];
    for (i, &c) in T.iter().enumerate() { rev[c as usize] = i as u8; }
    let clean: Vec<u8> = s.bytes().filter(|&b| b != b'=' && !b.is_ascii_whitespace()).collect();
    let mut out = vec![];
    for chunk in clean.chunks(4) {
        let mut buf = 0u32; let mut bits = 0;
        for &c in chunk { let v = rev[c as usize]; if v == 255 { return Err("bad base64".into()); } buf = (buf << 6) | v as u32; bits += 6; }
        bits -= bits % 8;
        for i in (0..bits).step_by(8).rev() { out.push((buf >> i) as u8); }
    }
    Ok(out)
}

#[tauri::command]
pub async fn check_connectivity() -> bool {
    reqwest::Client::new().get("https://dashscope.aliyuncs.com").timeout(std::time::Duration::from_secs(5)).send().await.is_ok()
}
