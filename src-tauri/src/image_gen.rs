use std::path::PathBuf;
use std::time::Duration;

const ENDPOINT: &str =
    "https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation";
const MODEL: &str = "qwen-image-2.0-pro";
const SIZE: &str = "1280*720"; // 16:9

/// Generate an image directly from DashScope (embedded key), download it, return local path.
pub async fn generate(transcript: &str, cache_dir: &PathBuf) -> Result<String, String> {
    let key = crate::secret::api_key();
    if key.is_empty() {
        return Err("API key missing from build".into());
    }
    let prompt = crate::prompt::build_prompt(transcript);
    let body = serde_json::json!({
        "model": MODEL,
        "input": { "messages": [{ "role": "user", "content": [{ "text": prompt }] }] },
        "parameters": { "size": SIZE, "n": 1 }
    });
    let client = reqwest::Client::new();
    let resp = client
        .post(ENDPOINT)
        .bearer_auth(&key)
        .json(&body)
        .timeout(Duration::from_secs(90))
        .send()
        .await
        .map_err(|e| format!("network error: {e}"))?;
    let text = resp.text().await.map_err(|e| e.to_string())?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("bad json: {e}"))?;
    if let Some(code) = v.get("code").and_then(|x| x.as_str()) {
        return Err(format!(
            "{code}: {}",
            v.get("message").and_then(|m| m.as_str()).unwrap_or("")
        ));
    }
    let url = v
        .pointer("/output/choices/0/message/content")
        .and_then(|c| c.as_array())
        .and_then(|a| a.iter().find_map(|it| it.get("image").and_then(|x| x.as_str())))
        .ok_or("no image url in response")?;
    let bytes = client
        .get(url)
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
    path.to_str().map(|s| s.to_string()).ok_or_else(|| "bad path".into())
}
