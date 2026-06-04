use std::time::Duration;

const ENDPOINT: &str =
    "https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation";
const MODEL: &str = "qwen3-asr-flash";

/// Wrap raw PCM16 mono little-endian samples in a 44-byte WAV header.
pub fn wav_from_pcm16le(pcm: &[u8], sample_rate: u32) -> Vec<u8> {
    let data_len = pcm.len() as u32;
    let byte_rate = sample_rate * 2; // mono * 16-bit
    let mut w = Vec::with_capacity(44 + pcm.len());
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data_len).to_le_bytes());
    w.extend_from_slice(b"WAVE");
    w.extend_from_slice(b"fmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());   // PCM
    w.extend_from_slice(&1u16.to_le_bytes());   // mono
    w.extend_from_slice(&sample_rate.to_le_bytes());
    w.extend_from_slice(&byte_rate.to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());   // block align
    w.extend_from_slice(&16u16.to_le_bytes());  // bits/sample
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data_len.to_le_bytes());
    w.extend_from_slice(pcm);
    w
}

/// Transcribe recorded PCM (16k mono LE) directly via DashScope qwen3-asr-flash (embedded key).
pub async fn transcribe(pcm: Vec<u8>) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    if pcm.len() < 3200 {
        return Err("没听到声音，请再说一次 / no audio captured".into());
    }
    let key = crate::secret::api_key();
    if key.is_empty() {
        return Err("API key missing from build".into());
    }
    let wav = wav_from_pcm16le(&pcm, 16000);
    let audio = format!("data:audio/wav;base64,{}", STANDARD.encode(&wav));
    let body = serde_json::json!({
        "model": MODEL,
        "input": { "messages": [{ "role": "user", "content": [{ "audio": audio }] }] }
    });
    let client = reqwest::Client::new();
    let resp = client
        .post(ENDPOINT)
        .bearer_auth(&key)
        .json(&body)
        .timeout(Duration::from_secs(60))
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
    Ok(v
        .pointer("/output/choices/0/message/content")
        .and_then(|c| c.as_array())
        .and_then(|a| a.iter().find_map(|it| it.get("text").and_then(|x| x.as_str())))
        .unwrap_or("")
        .trim()
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wav_header_is_44_bytes_and_sized() {
        let pcm = vec![0u8; 320];
        let w = wav_from_pcm16le(&pcm, 16000);
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(&w[8..12], b"WAVE");
        assert_eq!(w.len(), 44 + 320);
        // data chunk size little-endian == 320
        assert_eq!(u32::from_le_bytes([w[40],w[41],w[42],w[43]]), 320);
    }
}
