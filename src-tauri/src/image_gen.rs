use serde::Serialize;
use std::path::PathBuf;
use std::time::Duration;

const ENDPOINT: &str =
    "https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation";
const GEN_MODEL: &str = "qwen-image-2.0-pro";
const EDIT_MODEL: &str = "qwen-image-edit";
const SIZE: &str = "1280*720"; // 16:9

#[derive(Serialize)]
pub struct GenResult {
    pub path: String, // local cached PNG, for display
    pub url: String,  // remote OSS url, used as the base for the next edit
}

/// POST the body, parse the image url, download it, return {path, url}.
async fn call_and_download(body: serde_json::Value, cache_dir: &PathBuf) -> Result<GenResult, String> {
    let key = crate::secret::api_key();
    if key.is_empty() {
        return Err("API key missing from build".into());
    }
    let client = reqwest::Client::new();
    let resp = client
        .post(ENDPOINT)
        .bearer_auth(&key)
        .json(&body)
        .timeout(Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| format!("network error: {e}"))?;
    let text = resp.text().await.map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("bad json: {e}"))?;
    if let Some(code) = v.get("code").and_then(|x| x.as_str()) {
        return Err(format!("{code}: {}", v.get("message").and_then(|m| m.as_str()).unwrap_or("")));
    }
    let url = v
        .pointer("/output/choices/0/message/content")
        .and_then(|c| c.as_array())
        .and_then(|a| a.iter().find_map(|it| it.get("image").and_then(|x| x.as_str())))
        .ok_or("no image url in response")?
        .to_string();
    let bytes = client
        .get(&url)
        .timeout(Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    std::fs::create_dir_all(cache_dir).map_err(|e| e.to_string())?;
    let path = cache_dir.join(format!("gen_{}.png", uuid::Uuid::new_v4()));
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(GenResult {
        path: path.to_str().ok_or("bad path")?.to_string(),
        url,
    })
}

/// First generation: text -> image.
pub async fn generate(transcript: &str, cache_dir: &PathBuf) -> Result<GenResult, String> {
    let prompt = crate::prompt::build_prompt(transcript);
    let body = serde_json::json!({
        "model": GEN_MODEL,
        "input": { "messages": [{ "role": "user", "content": [{ "text": prompt }] }] },
        "parameters": { "size": SIZE, "n": 1 }
    });
    call_and_download(body, cache_dir).await
}

/// Edit an existing image (by its remote url) with the child's new instruction.
pub async fn edit(prev_url: &str, instruction: &str, cache_dir: &PathBuf) -> Result<GenResult, String> {
    let instr = format!(
        "{}（保持原图的可爱儿童绘本插画风格 / keep the cute children's book illustration style）",
        instruction.trim()
    );
    let body = serde_json::json!({
        "model": EDIT_MODEL,
        "input": { "messages": [{ "role": "user", "content": [{ "image": prev_url }, { "text": instr }] }] }
    });
    call_and_download(body, cache_dir).await
}
