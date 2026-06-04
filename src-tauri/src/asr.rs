use futures_util::{SinkExt, StreamExt};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};

const WS_URL: &str = "wss://dashscope.aliyuncs.com/api-ws/v1/inference";
const MODEL: &str = "paraformer-realtime-v2";

pub fn run_task_msg(task_id: &str) -> String {
    serde_json::json!({
        "header": { "action": "run-task", "task_id": task_id, "streaming": "duplex" },
        "payload": {
            "task_group": "audio", "task": "asr", "function": "recognition",
            "model": MODEL,
            "parameters": { "format": "pcm", "sample_rate": 16000, "language_hints": ["en"] },
            "input": {}
        }
    })
    .to_string()
}

pub fn finish_task_msg(task_id: &str) -> String {
    serde_json::json!({
        "header": { "action": "finish-task", "task_id": task_id, "streaming": "duplex" },
        "payload": { "input": {} }
    })
    .to_string()
}

#[derive(Debug, PartialEq)]
pub enum AsrEvent {
    Started,
    Partial(String),
    Final(String),
    Finished,
    Failed(String),
    Other,
}

pub fn parse_event(json: &str) -> AsrEvent {
    let v: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return AsrEvent::Other,
    };
    match v.pointer("/header/event").and_then(|x| x.as_str()).unwrap_or("") {
        "task-started" => AsrEvent::Started,
        "task-finished" => AsrEvent::Finished,
        "task-failed" => AsrEvent::Failed(
            v.pointer("/header/error_message")
                .and_then(|x| x.as_str())
                .unwrap_or("asr failed")
                .to_string(),
        ),
        "result-generated" => {
            let s = v.pointer("/payload/output/sentence");
            let text = s
                .and_then(|x| x.get("text"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let end = s
                .and_then(|x| x.get("sentence_end"))
                .and_then(|x| x.as_bool())
                .unwrap_or(false);
            if end {
                AsrEvent::Final(text)
            } else {
                AsrEvent::Partial(text)
            }
        }
        _ => AsrEvent::Other,
    }
}

pub struct AsrSession {
    pub audio_tx: mpsc::UnboundedSender<Vec<u8>>,
    pub stop_tx: mpsc::UnboundedSender<()>,
}

/// Open the Paraformer real-time WebSocket, stream audio, emit transcript events to the frontend:
/// `asr://partial` (in-progress sentence), `asr://final` (finished sentence), `asr://error`,
/// `asr://finished`.
pub async fn run_session(app: AppHandle, api_key: String) -> Result<AsrSession, String> {
    let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let (stop_tx, mut stop_rx) = mpsc::unbounded_channel::<()>();

    let mut req = WS_URL.into_client_request().map_err(|e| e.to_string())?;
    req.headers_mut()
        .insert("Authorization", format!("Bearer {api_key}").parse().unwrap());
    req.headers_mut()
        .insert("X-DashScope-DataInspection", "enable".parse().unwrap());

    let (ws, _) = tokio_tungstenite::connect_async(req)
        .await
        .map_err(|e| format!("ws connect: {e}"))?;
    let (mut write, mut read) = ws.split();

    let task_id = uuid::Uuid::new_v4().simple().to_string();
    write
        .send(Message::Text(run_task_msg(&task_id)))
        .await
        .map_err(|e| e.to_string())?;

    let app2 = app.clone();
    let finish_id = task_id.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(chunk) = audio_rx.recv() => {
                    if write.send(Message::Binary(chunk)).await.is_err() { break; }
                }
                Some(_) = stop_rx.recv() => {
                    let _ = write.send(Message::Text(finish_task_msg(&finish_id))).await;
                }
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(t))) => match parse_event(&t) {
                            AsrEvent::Partial(s) => { let _ = app2.emit("asr://partial", s); }
                            AsrEvent::Final(s) => { let _ = app2.emit("asr://final", s); }
                            AsrEvent::Finished => { let _ = app2.emit("asr://finished", ()); break; }
                            AsrEvent::Failed(e) => { let _ = app2.emit("asr://error", e); break; }
                            _ => {}
                        },
                        Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                        _ => {}
                    }
                }
                else => break,
            }
        }
    });

    Ok(AsrSession { audio_tx, stop_tx })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_partial_and_final() {
        let p = r#"{"header":{"event":"result-generated"},"payload":{"output":{"sentence":{"text":"a pink","sentence_end":false}}}}"#;
        assert_eq!(parse_event(p), AsrEvent::Partial("a pink".into()));
        let f = r#"{"header":{"event":"result-generated"},"payload":{"output":{"sentence":{"text":"a pink pig","sentence_end":true}}}}"#;
        assert_eq!(parse_event(f), AsrEvent::Final("a pink pig".into()));
    }

    #[test]
    fn run_task_has_model_and_lang() {
        let m = run_task_msg("abc");
        assert!(m.contains("paraformer-realtime-v2"));
        assert!(m.contains("\"en\""));
    }

    // Live streaming smoke test against the real Paraformer WS.
    // Needs a 16kHz mono 16-bit WAV at /tmp/sp.wav and the key built in.
    // Run: DASHSCOPE_API_KEY=sk-... cargo test asr::tests::live_stream -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn live_stream() {
        let key = crate::secret::api_key();
        assert!(key.starts_with("sk-"), "build with DASHSCOPE_API_KEY");
        let wav = std::fs::read("/tmp/sp.wav").expect("need /tmp/sp.wav (16k mono)");
        let pcm = &wav[44..];

        let mut req = WS_URL.into_client_request().unwrap();
        req.headers_mut().insert("Authorization", format!("Bearer {key}").parse().unwrap());
        req.headers_mut().insert("X-DashScope-DataInspection", "enable".parse().unwrap());
        let (ws, _) = tokio_tungstenite::connect_async(req).await.expect("ws connect");
        let (mut write, mut read) = ws.split();
        let id = uuid::Uuid::new_v4().simple().to_string();
        write.send(Message::Text(run_task_msg(&id))).await.unwrap();

        // wait for task-started
        loop {
            match read.next().await {
                Some(Ok(Message::Text(t))) => {
                    if parse_event(&t) == AsrEvent::Started { break; }
                }
                other => panic!("expected task-started, got {other:?}"),
            }
        }
        // stream PCM ~100ms chunks
        for ch in pcm.chunks(3200) {
            write.send(Message::Binary(ch.to_vec())).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        }
        write.send(Message::Text(finish_task_msg(&id))).await.unwrap();

        let mut finals = String::new();
        loop {
            match read.next().await {
                Some(Ok(Message::Text(t))) => match parse_event(&t) {
                    AsrEvent::Final(s) => { finals.push_str(&s); finals.push(' '); }
                    AsrEvent::Finished => break,
                    AsrEvent::Failed(e) => panic!("asr failed: {e}"),
                    _ => {}
                },
                Some(Ok(_)) => {}
                _ => break,
            }
        }
        println!("LIVE ASR FINALS: {}", finals.trim());
        assert!(!finals.trim().is_empty(), "no transcript returned");
    }
}
