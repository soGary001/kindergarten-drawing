use serde::Deserialize;

#[derive(Deserialize)]
struct ImageResponse { output: ImageOutput }
#[derive(Deserialize)]
struct ImageOutput {
    #[serde(default)] task_status: Option<String>,
    #[serde(default)] results: Option<Vec<ImageResult>>,
    #[serde(default)] message: Option<String>,
}
#[derive(Deserialize)]
struct ImageResult { #[serde(default)] url: Option<String> }

/// Extract the first image URL from a DashScope image-synthesis response.
pub fn first_image_url(json: &str) -> Result<String, String> {
    let resp: ImageResponse = serde_json::from_str(json)
        .map_err(|e| format!("bad image response json: {e}"))?;
    if let Some(status) = &resp.output.task_status {
        if status != "SUCCEEDED" {
            return Err(resp.output.message.unwrap_or_else(|| format!("image task status: {status}")));
        }
    }
    resp.output.results
        .and_then(|r| r.into_iter().find_map(|x| x.url))
        .ok_or_else(|| "no image url in response".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_success() {
        let j = r#"{"output":{"task_status":"SUCCEEDED","results":[{"url":"https://x/i.png"}]},"request_id":"r"}"#;
        assert_eq!(first_image_url(j).unwrap(), "https://x/i.png");
    }
    #[test]
    fn errors_on_failure_status() {
        let j = r#"{"output":{"task_status":"FAILED","message":"bad prompt"}}"#;
        assert!(first_image_url(j).unwrap_err().contains("bad prompt"));
    }
}

use crate::{prompt, secret};
use std::path::PathBuf;

const ENDPOINT: &str = "https://dashscope.aliyuncs.com/api/v1/services/aigc/text2image/image-synthesis";
const MODEL: &str = "qwen-image-2.0-pro";
const SIZE: &str = "1664*928"; // ~16:9

/// Generate an image from the child's transcript, download it, return the local file path.
pub async fn generate(transcript: &str, cache_dir: &PathBuf) -> Result<String, String> {
    let key = secret::api_key();
    if key.is_empty() { return Err("API key missing from build".into()); }
    let body = serde_json::json!({
        "model": MODEL,
        "input": { "prompt": prompt::build_prompt(transcript) },
        "parameters": { "size": SIZE, "n": 1, "prompt_extend": false, "watermark": false }
    });
    let client = reqwest::Client::new();
    let resp = client.post(ENDPOINT)
        .bearer_auth(&key)
        .json(&body)
        .send().await.map_err(|e| format!("network error: {e}"))?;
    let text = resp.text().await.map_err(|e| e.to_string())?;
    let url = first_image_url(&text)?;

    // Download to cache
    let bytes = client.get(&url).send().await.map_err(|e| e.to_string())?
        .bytes().await.map_err(|e| e.to_string())?;
    std::fs::create_dir_all(cache_dir).map_err(|e| e.to_string())?;
    let fname = format!("gen_{}.png", uuid::Uuid::new_v4());
    let path = cache_dir.join(&fname);
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    path.to_str().map(|s| s.to_string()).ok_or_else(|| "bad path".into())
}
