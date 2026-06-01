use serde::Deserialize;

pub fn run_task_msg(task_id: &str) -> String {
    serde_json::json!({
        "header": { "action": "run-task", "task_id": task_id, "streaming": "duplex" },
        "payload": {
            "task_group": "audio", "task": "asr", "function": "recognition",
            "model": "paraformer-realtime-v2",
            "parameters": { "format": "pcm", "sample_rate": 16000, "language_hints": ["en"] },
            "input": {}
        }
    }).to_string()
}

pub fn finish_task_msg(task_id: &str) -> String {
    serde_json::json!({
        "header": { "action": "finish-task", "task_id": task_id, "streaming": "duplex" },
        "payload": { "input": {} }
    }).to_string()
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

#[derive(Deserialize)]
struct Frame {
    header: Header,
    #[serde(default)]
    payload: Option<Payload>,
}
#[derive(Deserialize)]
struct Header {
    event: String,
    #[serde(default)]
    error_message: Option<String>,
}
#[derive(Deserialize)]
struct Payload {
    #[serde(default)]
    output: Option<Output>,
}
#[derive(Deserialize)]
struct Output {
    #[serde(default)]
    sentence: Option<Sentence>,
}
#[derive(Deserialize)]
struct Sentence {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    sentence_end: Option<bool>,
}

pub fn parse_event(json: &str) -> AsrEvent {
    let f: Frame = match serde_json::from_str(json) {
        Ok(f) => f,
        Err(_) => return AsrEvent::Other,
    };
    match f.header.event.as_str() {
        "task-started" => AsrEvent::Started,
        "task-finished" => AsrEvent::Finished,
        "task-failed" => {
            AsrEvent::Failed(f.header.error_message.unwrap_or_else(|| "asr failed".into()))
        }
        "result-generated" => {
            let s = f.payload.and_then(|p| p.output).and_then(|o| o.sentence);
            match s {
                Some(s) => {
                    let text = s.text.unwrap_or_default();
                    if s.sentence_end.unwrap_or(false) {
                        AsrEvent::Final(text)
                    } else {
                        AsrEvent::Partial(text)
                    }
                }
                None => AsrEvent::Other,
            }
        }
        _ => AsrEvent::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_partial_and_final() {
        let p = r#"{"header":{"task_id":"t","event":"result-generated"},"payload":{"output":{"sentence":{"text":"a pink","sentence_end":false}}}}"#;
        assert_eq!(parse_event(p), AsrEvent::Partial("a pink".into()));
        let f = r#"{"header":{"task_id":"t","event":"result-generated"},"payload":{"output":{"sentence":{"text":"a pink pig","sentence_end":true}}}}"#;
        assert_eq!(parse_event(f), AsrEvent::Final("a pink pig".into()));
    }
    #[test]
    fn parses_lifecycle_and_error() {
        assert_eq!(
            parse_event(r#"{"header":{"task_id":"t","event":"task-started"},"payload":{}}"#),
            AsrEvent::Started
        );
        assert_eq!(
            parse_event(r#"{"header":{"task_id":"t","event":"task-finished"},"payload":{}}"#),
            AsrEvent::Finished
        );
        assert_eq!(
            parse_event(
                r#"{"header":{"task_id":"t","event":"task-failed","error_message":"nope"}}"#
            ),
            AsrEvent::Failed("nope".into())
        );
    }
    #[test]
    fn run_task_has_model_and_lang() {
        let m = run_task_msg("abc");
        assert!(m.contains("paraformer-realtime-v2"));
        assert!(m.contains("\"en\""));
        assert!(m.contains("\"action\":\"run-task\""));
    }
}

// Part 2 — WebSocket session

use futures_util::{SinkExt, StreamExt};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};

const WS_URL: &str = "wss://dashscope.aliyuncs.com/api-ws/v1/inference";

pub struct AsrSession {
    pub audio_tx: mpsc::UnboundedSender<Vec<u8>>,
    pub stop_tx: mpsc::UnboundedSender<()>,
}

/// Open the WS, start a task, stream audio from `audio_rx`, emit events to the frontend.
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
                Some(msg) = read.next() => {
                    match msg {
                        Ok(Message::Text(t)) => {
                            match parse_event(&t) {
                                AsrEvent::Partial(s) => { let _ = app2.emit("asr://partial", s); }
                                AsrEvent::Final(s) => { let _ = app2.emit("asr://final", s); }
                                AsrEvent::Finished => { let _ = app2.emit("asr://finished", ()); break; }
                                AsrEvent::Failed(e) => { let _ = app2.emit("asr://error", e); break; }
                                _ => {}
                            }
                        }
                        Ok(Message::Close(_)) | Err(_) => break,
                        _ => {}
                    }
                }
                else => break,
            }
        }
    });

    Ok(AsrSession { audio_tx, stop_tx })
}
