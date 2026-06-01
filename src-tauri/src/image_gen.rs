use serde::Deserialize;
use crate::{prompt, secret};
use std::path::PathBuf;

const ENDPOINT: &str = "https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation";
const MODEL: &str = "qwen-image-2.0-pro";
const SIZE: &str = "1280*720"; // 16:9

// ── Success shape ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct MmResponse {
    #[serde(default)] output:  Option<MmOutput>,
    #[serde(default)] code:    Option<String>,
    #[serde(default)] message: Option<String>,
}

#[derive(Deserialize)]
struct MmOutput {
    #[serde(default)] choices: Vec<MmChoice>,
}

#[derive(Deserialize)]
struct MmChoice {
    #[serde(default)] message: Option<MmMessage>,
}

#[derive(Deserialize)]
struct MmMessage {
    #[serde(default)] content: Vec<MmContentItem>,
}

/// One element of `content[]`; either a text item or an image item (or both).
#[derive(Deserialize)]
struct MmContentItem {
    #[serde(default)] image: Option<String>,
}

// ── Public parsing helper ─────────────────────────────────────────────────────

/// Extract the first image URL from a multimodal-generation response.
///
/// Success path: `output.choices[*].message.content[*].image` (first non-empty).
/// Error path:   top-level `code` field present → `Err("API error {code}: {message}")`.
pub fn first_image_url(json: &str) -> Result<String, String> {
    let resp: MmResponse = serde_json::from_str(json)
        .map_err(|e| format!("bad image response json: {e}"))?;

    // Top-level API error envelope: {"code":"...","message":"..."}
    if let Some(code) = &resp.code {
        let msg = resp.message.clone().unwrap_or_else(|| code.clone());
        return Err(format!("API error {code}: {msg}"));
    }

    let output = resp.output.ok_or_else(|| "no output in response".to_string())?;

    for choice in output.choices {
        if let Some(msg) = choice.message {
            for item in msg.content {
                if let Some(url) = item.image {
                    if !url.is_empty() {
                        return Ok(url);
                    }
                }
            }
        }
    }

    Err("no image in response".to_string())
}

// ── Main generate function ────────────────────────────────────────────────────

/// Generate an image from the child's transcript, download it, return the local PNG path.
///
/// Uses the **synchronous** `qwen-image-2.0-pro` multimodal-generation endpoint
/// (verified live against the DashScope API; no async header, no polling needed).
pub async fn generate(transcript: &str, cache_dir: &PathBuf) -> Result<String, String> {
    let key = secret::api_key();
    if key.is_empty() {
        return Err("API key missing from build".into());
    }

    let body = serde_json::json!({
        "model": MODEL,
        "input": {
            "messages": [{
                "role": "user",
                "content": [{ "text": prompt::build_prompt(transcript) }]
            }]
        },
        "parameters": { "size": SIZE, "n": 1 }
    });

    let client = reqwest::Client::new();

    let response_text = client
        .post(ENDPOINT)
        .bearer_auth(&key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("network error: {e}"))?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    let image_url = first_image_url(&response_text)?;

    // Download the (time-limited) presigned URL immediately
    let bytes = client
        .get(&image_url)
        .send()
        .await
        .map_err(|e| format!("download error: {e}"))?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    std::fs::create_dir_all(cache_dir).map_err(|e| e.to_string())?;
    let fname = format!("gen_{}.png", uuid::Uuid::new_v4());
    let path = cache_dir.join(&fname);
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "bad path".into())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Verified success shape from a real live call to the multimodal-generation endpoint.
    #[test]
    fn parses_success() {
        let j = r#"{
            "output": {
                "choices": [{
                    "finish_reason": "stop",
                    "message": {
                        "content": [{"image": "https://dashscope-oss.aliyuncs.com/gen.png"}],
                        "role": "assistant"
                    }
                }]
            },
            "usage": {"height": 720, "image_count": 1, "width": 1280},
            "request_id": "r1"
        }"#;
        assert_eq!(
            first_image_url(j).unwrap(),
            "https://dashscope-oss.aliyuncs.com/gen.png"
        );
    }

    /// Top-level error envelope — no `output` field.
    #[test]
    fn errors_on_top_level_error() {
        let j = r#"{"code":"InvalidParameter","message":"bad","request_id":"r2"}"#;
        let err = first_image_url(j).unwrap_err();
        assert!(err.contains("InvalidParameter"), "got: {err}");
        assert!(err.contains("bad"), "got: {err}");
    }

    /// Content array has a text-only item first; parser must skip it and return the image URL.
    #[test]
    fn picks_first_image_when_text_item_present() {
        let j = r#"{
            "output": {
                "choices": [{
                    "message": {
                        "content": [
                            {"text": "a description"},
                            {"image": "https://x/i.png"}
                        ]
                    }
                }]
            },
            "request_id": "r3"
        }"#;
        assert_eq!(first_image_url(j).unwrap(), "https://x/i.png");
    }

    /// No choices at all → Err.
    #[test]
    fn errors_when_no_choices() {
        let j = r#"{"output":{"choices":[]},"request_id":"r4"}"#;
        assert!(first_image_url(j).is_err());
    }

    /// Live smoke test — requires DASHSCOPE_API_KEY env var and real network access.
    /// Verified working against the multimodal-generation endpoint (qwen-image-2.0-pro).
    #[tokio::test]
    #[ignore]
    async fn live_generate_smoke() {
        let dir = std::env::temp_dir().join("kg_live_img_test");
        let _ = std::fs::remove_dir_all(&dir);
        let res = generate("a happy yellow duck", &dir).await;
        match &res {
            Ok(path) => {
                let meta = std::fs::metadata(path).expect("file should exist");
                assert!(
                    meta.len() > 1000,
                    "image file suspiciously small: {} bytes",
                    meta.len()
                );
                println!("LIVE OK: {} ({} bytes)", path, meta.len());
            }
            Err(e) => panic!("LIVE FAIL: {e}"),
        }
    }
}
