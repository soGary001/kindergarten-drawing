use serde::Deserialize;

#[derive(Deserialize)]
struct ImageResponse {
    #[serde(default)] output: Option<ImageOutput>,
    #[serde(default)] code: Option<String>,
    #[serde(default)] message: Option<String>,
}
#[derive(Deserialize)]
struct ImageOutput {
    #[serde(default)] task_id: Option<String>,
    #[serde(default)] task_status: Option<String>,
    #[serde(default)] results: Option<Vec<ImageResult>>,
    #[serde(default)] message: Option<String>,
}
#[derive(Deserialize)]
struct ImageResult { #[serde(default)] url: Option<String> }

/// Extract the first image URL from a DashScope image-synthesis response.
/// Handles both top-level error envelopes and output-nested shapes.
pub fn first_image_url(json: &str) -> Result<String, String> {
    let resp: ImageResponse = serde_json::from_str(json)
        .map_err(|e| format!("bad image response json: {e}"))?;
    // Top-level API errors (no output field): {"code":"...","message":"..."}
    if let Some(code) = &resp.code {
        let msg = resp.message.unwrap_or_else(|| code.clone());
        return Err(format!("API error {code}: {msg}"));
    }
    let output = resp.output.ok_or_else(|| "no output in response".to_string())?;
    if let Some(status) = &output.task_status {
        if status != "SUCCEEDED" {
            return Err(output.message.unwrap_or_else(|| format!("image task status: {status}")));
        }
    }
    output.results
        .and_then(|r| r.into_iter().find_map(|x| x.url))
        .ok_or_else(|| "no image url in response".to_string())
}

/// Extract the task_id from a DashScope async submit response.
fn task_id_from(json: &str) -> Result<String, String> {
    let resp: ImageResponse = serde_json::from_str(json)
        .map_err(|e| format!("bad submit response json: {e}"))?;
    if let Some(code) = &resp.code {
        let msg = resp.message.unwrap_or_else(|| code.clone());
        return Err(format!("API error {code}: {msg}"));
    }
    resp.output
        .and_then(|o| o.task_id)
        .ok_or_else(|| "no task_id in submit response".to_string())
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
    #[test]
    fn errors_on_top_level_api_error() {
        let j = r#"{"request_id":"abc","code":"InvalidParameter","message":"url error"}"#;
        let err = first_image_url(j).unwrap_err();
        assert!(err.contains("InvalidParameter"), "got: {err}");
        assert!(err.contains("url error"), "got: {err}");
    }
    #[test]
    fn parses_task_id_from_submit() {
        let j = r#"{"output":{"task_id":"abc-123","task_status":"PENDING"},"request_id":"r"}"#;
        assert_eq!(task_id_from(j).unwrap(), "abc-123");
    }

    #[tokio::test]
    #[ignore]
    async fn live_generate_smoke() {
        let dir = std::env::temp_dir().join("kg_live_img_test");
        let _ = std::fs::remove_dir_all(&dir);
        let res = generate("a happy yellow duck", &dir).await;
        match &res {
            Ok(path) => {
                let meta = std::fs::metadata(path).expect("file should exist");
                assert!(meta.len() > 1000, "image file suspiciously small: {} bytes", meta.len());
                println!("LIVE OK: {} ({} bytes)", path, meta.len());
            }
            Err(e) => panic!("LIVE FAIL: {e}"),
        }
    }
}

use crate::{prompt, secret};
use std::path::PathBuf;

const ENDPOINT: &str = "https://dashscope.aliyuncs.com/api/v1/services/aigc/text2image/image-synthesis";
const TASK_ENDPOINT: &str = "https://dashscope.aliyuncs.com/api/v1/tasks";
// wanx2.1-t2i-plus is the recommended async text-to-image model available on this account.
// qwen-image-2.0-pro (synchronous) returns "url error" on this account/region.
const MODEL: &str = "wanx2.1-t2i-plus";
const SIZE: &str = "1280*720"; // 16:9, within wanx2.1-t2i-plus limits (512–1440)

/// Generate an image from the child's transcript, download it, return the local file path.
/// Uses async task submission + polling (required by wanx2.1-t2i-plus).
pub async fn generate(transcript: &str, cache_dir: &PathBuf) -> Result<String, String> {
    let key = secret::api_key();
    if key.is_empty() { return Err("API key missing from build".into()); }
    let body = serde_json::json!({
        "model": MODEL,
        "input": { "prompt": prompt::build_prompt(transcript) },
        "parameters": { "size": SIZE, "n": 1, "prompt_extend": false, "watermark": false }
    });
    let client = reqwest::Client::new();

    // Submit async task
    let submit_text = client.post(ENDPOINT)
        .bearer_auth(&key)
        .header("X-DashScope-Async", "enable")
        .json(&body)
        .send().await.map_err(|e| format!("network error: {e}"))?
        .text().await.map_err(|e| e.to_string())?;
    let task_id = task_id_from(&submit_text)?;

    // Poll until SUCCEEDED (or FAILED)
    let url = poll_task(&client, &key, &task_id).await?;

    // Download to cache
    let bytes = client.get(&url).send().await.map_err(|e| e.to_string())?
        .bytes().await.map_err(|e| e.to_string())?;
    std::fs::create_dir_all(cache_dir).map_err(|e| e.to_string())?;
    let fname = format!("gen_{}.png", uuid::Uuid::new_v4());
    let path = cache_dir.join(&fname);
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    path.to_str().map(|s| s.to_string()).ok_or_else(|| "bad path".into())
}

/// Poll the task endpoint every 2 seconds until the task is no longer PENDING/RUNNING.
async fn poll_task(client: &reqwest::Client, key: &str, task_id: &str) -> Result<String, String> {
    let url = format!("{TASK_ENDPOINT}/{task_id}");
    for _ in 0..60 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let text = client.get(&url)
            .bearer_auth(key)
            .send().await.map_err(|e| format!("poll error: {e}"))?
            .text().await.map_err(|e| e.to_string())?;
        match task_status_from(&text)? {
            TaskPollResult::Pending => continue,
            TaskPollResult::Done(image_url) => return Ok(image_url),
        }
    }
    Err("image generation timed out after 120 seconds".to_string())
}

enum TaskPollResult { Pending, Done(String) }

fn task_status_from(json: &str) -> Result<TaskPollResult, String> {
    let resp: ImageResponse = serde_json::from_str(json)
        .map_err(|e| format!("bad poll response json: {e}"))?;
    if let Some(code) = &resp.code {
        let msg = resp.message.unwrap_or_else(|| code.clone());
        return Err(format!("API error {code}: {msg}"));
    }
    let output = resp.output.ok_or_else(|| "no output in poll response".to_string())?;
    match output.task_status.as_deref() {
        Some("PENDING") | Some("RUNNING") => Ok(TaskPollResult::Pending),
        Some("SUCCEEDED") => {
            let url = output.results
                .and_then(|r| r.into_iter().find_map(|x| x.url))
                .ok_or_else(|| "SUCCEEDED but no image url".to_string())?;
            Ok(TaskPollResult::Done(url))
        }
        Some(status) => {
            let msg = output.message.unwrap_or_else(|| format!("task status: {status}"));
            Err(msg)
        }
        None => Err("missing task_status in poll response".to_string()),
    }
}
