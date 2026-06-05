use base64::{engine::general_purpose::STANDARD, Engine as _};
use crate::{gallery, image_gen, settings::{self, AppSettings}};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

#[derive(Default)]
pub struct AppState {
    pub recent: Mutex<Vec<usize>>,
    pub mic: Mutex<Option<crate::mic::MicHandle>>,
    pub asr: Mutex<Option<crate::asr::AsrSession>>,
}

fn app_data(app: &AppHandle) -> std::path::PathBuf {
    app.path().app_data_dir().unwrap_or(std::env::temp_dir())
}

fn active_gallery_dir(app: &AppHandle, s: &AppSettings) -> std::path::PathBuf {
    // 1. Operator override (desktop): explicit folder wins.
    if let Some(d) = &s.gallery_dir { return std::path::PathBuf::from(d); }
    // 2. Bundled default gallery (works on desktop AND Android via resource dir).
    if let Ok(res) = app.path().resource_dir() {
        let g = res.join("gallery");
        if !crate::gallery::scan(&g).is_empty() { return g; }
    }
    // 3. Writable fallback in app data (operator can drop files here on desktop).
    let dir = app_data(app).join("gallery");
    let _ = std::fs::create_dir_all(&dir);
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
pub async fn generate_image(app: AppHandle, transcript: String) -> Result<image_gen::GenResult, String> {
    let cache = app_data(&app).join("cache");
    image_gen::generate(&transcript, &cache).await
}

#[tauri::command]
pub async fn edit_image(app: AppHandle, prev_url: String, instruction: String) -> Result<image_gen::GenResult, String> {
    let cache = app_data(&app).join("cache");
    image_gen::edit(&prev_url, &instruction, &cache).await
}

#[tauri::command]
pub async fn asr_start(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    // Block (off the async executor) until mic permission is granted on macOS.
    tokio::task::spawn_blocking(crate::permission::ensure_mic)
        .await
        .map_err(|e| e.to_string())??;
    let key = crate::secret::api_key();
    let session = crate::asr::run_session(app.clone(), key).await?;
    let mic = crate::mic::start_capture(session.audio_tx.clone())?;
    *state.mic.lock().unwrap() = Some(mic);
    *state.asr.lock().unwrap() = Some(session);
    Ok(())
}

#[tauri::command]
pub fn asr_stop(state: State<AppState>) -> Result<(), String> {
    if let Some(m) = state.mic.lock().unwrap().take() { m.stop(); }
    if let Some(s) = state.asr.lock().unwrap().take() { let _ = s.stop_tx.send(()); }
    Ok(())
}

#[tauri::command]
pub fn save_snapshot(app: AppHandle, png_base64: String) -> Result<String, String> {
    use std::io::Write;
    let s = settings::load(&app_data(&app));
    let dir = s.snapshot_dir.map(std::path::PathBuf::from).unwrap_or_else(|| app_data(&app).join("snapshots"));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let raw = png_base64.split(',').last().unwrap_or(&png_base64);
    let bytes = STANDARD.decode(raw).map_err(|e| format!("bad base64: {e}"))?;
    let path = dir.join(format!("snapshot_{}.png", uuid::Uuid::new_v4()));
    let mut f = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    f.write_all(&bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn check_connectivity() -> bool {
    reqwest::Client::new()
        .get("https://vercel-proxy-plum-eight.vercel.app")
        .timeout(std::time::Duration::from_secs(5))
        .send().await.is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip_unaligned() {
        for s in ["a", "ab", "abc", "abcd", "hello world"] {
            let enc = STANDARD.encode(s.as_bytes());
            assert_eq!(STANDARD.decode(&enc).unwrap(), s.as_bytes());
        }
    }
}
