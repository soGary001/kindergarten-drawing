use std::path::PathBuf;

const PROXY: &str = "https://vercel-proxy-plum-eight.vercel.app/api/generate-image";

/// Send the child's transcript (+ style suffix) to the HK proxy, download the resulting image.
pub async fn generate(transcript: &str, cache_dir: &PathBuf) -> Result<String, String> {
    let prompt = crate::prompt::build_prompt(transcript);
    let client = reqwest::Client::new();
    let resp = client
        .post(PROXY)
        .json(&serde_json::json!({ "prompt": prompt }))
        .send().await.map_err(|e| format!("network error: {e}"))?;
    let text = resp.text().await.map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("bad json: {e}"))?;
    if let Some(err) = v.get("error").and_then(|x| x.as_str()) { return Err(err.to_string()); }
    let url = v.get("url").and_then(|x| x.as_str()).ok_or("no url in proxy response")?;
    let bytes = client.get(url).send().await.map_err(|e| e.to_string())?
        .bytes().await.map_err(|e| e.to_string())?;
    std::fs::create_dir_all(cache_dir).map_err(|e| e.to_string())?;
    let path = cache_dir.join(format!("gen_{}.png", uuid::Uuid::new_v4()));
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    path.to_str().map(|s| s.to_string()).ok_or_else(|| "bad path".into())
}

#[cfg(test)]
mod tests {
    // Live test: hits the deployed proxy. Run with: cargo test image_gen::tests::live_proxy_image -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn live_proxy_image() {
        let dir = std::env::temp_dir().join("kg_proxy_img_test");
        let _ = std::fs::remove_dir_all(&dir);
        let p = super::generate("a happy yellow duck", &dir).await.expect("proxy image");
        let meta = std::fs::metadata(&p).expect("file exists");
        assert!(meta.len() > 1000, "image too small");
        println!("LIVE IMG OK: {p} ({} bytes)", meta.len());
    }
}
