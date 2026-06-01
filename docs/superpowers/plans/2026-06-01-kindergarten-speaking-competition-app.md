# Kindergarten English Speaking Competition App — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Tauri 2 desktop app (macOS `.dmg` + Windows `.exe`) for a live kindergarten English speaking competition: a child draws a random picture, describes it in English by speaking, the app transcribes the speech and generates an AI image from their words, then shows original vs. generated side by side.

**Architecture:** Tauri 2 with a Rust backend (holds the obfuscated DashScope key, makes all network calls) and an HTML/CSS/vanilla-TS frontend (Vite). The frontend never sees the key — it invokes Rust commands and listens to Rust events. Speech uses DashScope Paraformer real-time ASR over WebSocket; images use DashScope `qwen-image-2.0-pro` (synchronous). Visual style is Memphis "Bubblegum Pop".

**Tech Stack:** Tauri 2, Rust (tokio, reqwest, tokio-tungstenite, serde, rand, image), TypeScript + Vite (vanilla, no framework), CSS animations, GitHub Actions (`tauri-apps/tauri-action`).

**Reference spec:** `docs/superpowers/specs/2026-06-01-kindergarten-speaking-competition-design.md`

---

## Verified DashScope API shapes (use these exactly)

**Image (synchronous — `qwen-image-2.0-pro`):**
- `POST https://dashscope.aliyuncs.com/api/v1/services/aigc/text2image/image-synthesis`
- Headers: `Content-Type: application/json`, `Authorization: Bearer <key>`. **No** `X-DashScope-Async` header (sync).
- Body: `{"model":"qwen-image-2.0-pro","input":{"prompt":"..."},"parameters":{"size":"1664*928","n":1,"prompt_extend":false,"watermark":false}}`
- Response: `{"output":{"task_status":"SUCCEEDED","results":[{"url":"https://...","orig_prompt":"...","actual_prompt":"..."}]},"usage":{...},"request_id":"..."}`
- Image URL expires in 24h — download immediately.

**Speech (Paraformer real-time, WebSocket):**
- `wss://dashscope.aliyuncs.com/api-ws/v1/inference`
- Connection headers: `Authorization: Bearer <key>`, `X-DashScope-DataInspection: enable`
- run-task: `{"header":{"action":"run-task","task_id":"<32hex>","streaming":"duplex"},"payload":{"task_group":"audio","task":"asr","function":"recognition","model":"paraformer-realtime-v2","parameters":{"format":"pcm","sample_rate":16000,"language_hints":["en"]},"input":{}}}`
- Then send audio as **binary** WS frames (PCM16 mono 16kHz, ~100ms chunks).
- finish-task: `{"header":{"action":"finish-task","task_id":"<same>","streaming":"duplex"},"payload":{"input":{}}}`
- Server events (text frames): `task-started`, `result-generated` (`payload.output.sentence.text`, `payload.output.sentence.sentence_end`), `task-finished`, `task-failed` (`header.error_message`).

---

## File Structure

```
kindergarten-drawing/
├─ package.json                      # Vite + Tauri CLI scripts
├─ index.html                        # app entry
├─ vite.config.ts
├─ tsconfig.json
├─ src/                              # FRONTEND (renderer)
│  ├─ main.ts                        # bootstrap + state machine
│  ├─ state.ts                       # AppState machine (screens)
│  ├─ api.ts                         # typed wrappers over Tauri invoke/events
│  ├─ audio.ts                       # mic capture → PCM16 16k chunks
│  ├─ screens/
│  │  ├─ idle.ts
│  │  ├─ draw.ts
│  │  ├─ describe.ts
│  │  ├─ generating.ts
│  │  ├─ compare.ts
│  │  ├─ settings.ts
│  │  └─ errorOverlay.ts
│  ├─ snapshot.ts                    # canvas composite → PNG bytes
│  └─ styles/
│     ├─ tokens.css                  # palette, fonts, shapes
│     ├─ components.css              # buttons, pills, panels
│     └─ animations.css
├─ public/fonts/                     # Baloo2, Fredoka (bundled, offline)
├─ public/gallery/                   # default bundled pictures (samples)
└─ src-tauri/                        # BACKEND (Rust)
   ├─ Cargo.toml
   ├─ build.rs                       # obfuscates DASHSCOPE_API_KEY at build time
   ├─ tauri.conf.json
   └─ src/
      ├─ main.rs                     # Tauri builder, command registration
      ├─ secret.rs                   # deobfuscate embedded key
      ├─ xor.rs                      # pure xor helper (unit-tested)
      ├─ settings.rs                 # load/save AppSettings JSON
      ├─ gallery.rs                  # scan folder, draw_random
      ├─ prompt.rs                   # build image prompt (literal + style)
      ├─ image_gen.rs                # qwen-image-2.0-pro client + parse + download
      ├─ asr.rs                      # Paraformer message build/parse + WS task
      └─ commands.rs                 # Tauri command handlers
```

---

## Phase 0 — Scaffold

### Task 0.1: Initialize Tauri 2 project

**Files:**
- Create: whole scaffold via CLI

- [ ] **Step 1: Scaffold**

Run in the project root (`/Users/sogary/Documents/GitHub/kindergarten-drawing`):
```bash
npm create tauri-app@latest . -- --template vanilla-ts --manager npm --identifier com.kindergarten.speaking --yes
npm install
```
If the directory-not-empty prompt blocks (because `docs/` and `.git` exist), scaffold in a temp dir and move files in:
```bash
npm create tauri-app@latest kg-tmp -- --template vanilla-ts --manager npm --identifier com.kindergarten.speaking --yes
cp -R kg-tmp/. . && rm -rf kg-tmp && npm install
```

- [ ] **Step 2: Verify the dev app runs**

Run: `npm run tauri dev`
Expected: a desktop window opens showing the default Tauri+Vite page. Close it (Ctrl-C).

- [ ] **Step 3: Add Rust deps**

Edit `src-tauri/Cargo.toml` `[dependencies]` to add:
```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "stream", "rustls-tls"], default-features = false }
tokio-tungstenite = { version = "0.24", features = ["rustls-tls-webpki-roots"] }
futures-util = "0.3"
rand = "0.8"
uuid = { version = "1", features = ["v4"] }
image = "0.25"
tauri-plugin-dialog = "2"
```
Add to `[build-dependencies]` (create the section if missing): *(none needed; build.rs uses std only).*

Run: `cd src-tauri && cargo build && cd ..`
Expected: compiles (may be slow first time).

- [ ] **Step 4: Commit**
```bash
git add -A && git commit -m "chore: scaffold Tauri 2 vanilla-ts app"
```

### Task 0.2: Configure window as fullscreen kiosk

**Files:**
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Set window + product config**

In `src-tauri/tauri.conf.json`, set `productName` to `"Kindergarten Speaking"`, and the `app.windows[0]` object to:
```json
{
  "title": "Kindergarten Speaking Competition",
  "width": 1280,
  "height": 800,
  "resizable": true,
  "fullscreen": false,
  "center": true
}
```
(Fullscreen is toggled at runtime from settings, not forced at launch, so dev is easy.)

- [ ] **Step 2: Verify**

Run: `npm run tauri dev`
Expected: window titled "Kindergarten Speaking Competition" opens. Close it.

- [ ] **Step 3: Commit**
```bash
git add -A && git commit -m "chore: configure app window"
```

---

## Phase 1 — Rust core: key, settings, gallery

### Task 1.1: XOR helper (pure, unit-tested)

**Files:**
- Create: `src-tauri/src/xor.rs`
- Test: in `src-tauri/src/xor.rs` (`#[cfg(test)]`)

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/xor.rs`:
```rust
pub const XOR_KEY: &[u8] = b"kg-memphis-2026-salt-do-not-reuse";

/// XOR each byte of `data` with a repeating key. Symmetric: applying twice restores input.
pub fn xor_bytes(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, b)| b ^ key[i % key.len()])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_restores_original() {
        let secret = b"sk-abc123-EXAMPLE";
        let obf = xor_bytes(secret, XOR_KEY);
        assert_ne!(&obf[..], &secret[..], "obfuscated must differ from plaintext");
        let back = xor_bytes(&obf, XOR_KEY);
        assert_eq!(&back[..], &secret[..]);
    }
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cd src-tauri && cargo test xor::tests::roundtrip_restores_original && cd ..`
Expected: PASS. (This is a pure function; test+impl land together.)

- [ ] **Step 3: Register module**

In `src-tauri/src/main.rs`, add near the top (after any existing `mod` lines): `mod xor;`

- [ ] **Step 4: Commit**
```bash
git add -A && git commit -m "feat: add xor helper for key obfuscation"
```

### Task 1.2: build.rs embeds the obfuscated key

**Files:**
- Create: `src-tauri/build.rs` (or modify existing if scaffold created one)
- Create: `src-tauri/src/secret.rs`

- [ ] **Step 1: Write build.rs**

Tauri's scaffold creates a `src-tauri/build.rs` containing `fn main() { tauri_build::build() }`. Replace it with:
```rust
use std::{env, fs, path::Path};

// Must match xor::XOR_KEY
const XOR_KEY: &[u8] = b"kg-memphis-2026-salt-do-not-reuse";

fn main() {
    // Build-time only: read key from env (local export or CI secret). Never committed.
    let key = env::var("DASHSCOPE_API_KEY").unwrap_or_default();
    let obf: Vec<u8> = key
        .as_bytes()
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ XOR_KEY[i % XOR_KEY.len()])
        .collect();
    let arr = obf.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(",");
    let out = env::var("OUT_DIR").unwrap();
    fs::write(
        Path::new(&out).join("embedded_key.rs"),
        format!("pub const OBFUSCATED_KEY: &[u8] = &[{arr}];\n"),
    )
    .unwrap();
    println!("cargo:rerun-if-env-changed=DASHSCOPE_API_KEY");

    tauri_build::build();
}
```

- [ ] **Step 2: Write secret.rs**

Create `src-tauri/src/secret.rs`:
```rust
use crate::xor::{xor_bytes, XOR_KEY};

include!(concat!(env!("OUT_DIR"), "/embedded_key.rs"));

/// Reassemble the API key at runtime from the obfuscated bytes embedded at build time.
pub fn api_key() -> String {
    String::from_utf8(xor_bytes(OBFUSCATED_KEY, XOR_KEY)).unwrap_or_default()
}
```

- [ ] **Step 3: Register module**

In `src-tauri/src/main.rs` add: `mod secret;`

- [ ] **Step 4: Verify it builds with a key**

Run: `cd src-tauri && DASHSCOPE_API_KEY=sk-test-123 cargo build 2>&1 | tail -5 && cd ..`
Expected: builds successfully. (We can't assert the key value without exposing it; the xor roundtrip test already proves correctness.)

- [ ] **Step 5: Commit**
```bash
git add -A && git commit -m "feat: embed obfuscated DashScope key at build time"
```

### Task 1.3: Settings (load/save JSON in app data dir)

**Files:**
- Create: `src-tauri/src/settings.rs`

- [ ] **Step 1: Write settings.rs with a unit test for defaults + serde**
```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppSettings {
    pub gallery_dir: Option<String>,   // None => bundled default
    pub snapshot_dir: Option<String>,  // None => app data /snapshots
    pub fullscreen: bool,
    pub child_label: String,           // e.g. "Leo · Round 5"
}

impl Default for AppSettings {
    fn default() -> Self {
        Self { gallery_dir: None, snapshot_dir: None, fullscreen: false, child_label: String::new() }
    }
}

pub fn settings_path(app_data: &PathBuf) -> PathBuf {
    app_data.join("settings.json")
}

pub fn load(app_data: &PathBuf) -> AppSettings {
    let p = settings_path(app_data);
    std::fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(app_data: &PathBuf, s: &AppSettings) -> Result<(), String> {
    std::fs::create_dir_all(app_data).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(settings_path(app_data), json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_then_roundtrip() {
        let dir = std::env::temp_dir().join("kg_settings_test");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(load(&dir), AppSettings::default());
        let mut s = AppSettings::default();
        s.fullscreen = true;
        s.child_label = "Leo".into();
        save(&dir, &s).unwrap();
        assert_eq!(load(&dir), s);
    }
}
```

- [ ] **Step 2: Run test**

Run: `cd src-tauri && cargo test settings:: && cd ..`
Expected: PASS.

- [ ] **Step 3: Register module** — add `mod settings;` to `main.rs`.

- [ ] **Step 4: Commit**
```bash
git add -A && git commit -m "feat: app settings load/save"
```

### Task 1.4: Gallery (scan + random draw)

**Files:**
- Create: `src-tauri/src/gallery.rs`

- [ ] **Step 1: Write gallery.rs with unit tests for filtering + draw logic**
```rust
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ImageMeta {
    pub id: String,    // file name
    pub path: String,  // absolute path
}

const EXTS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

pub fn scan(dir: &Path) -> Vec<ImageMeta> {
    let mut out = vec![];
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            let is_img = p.extension()
                .and_then(|x| x.to_str())
                .map(|x| EXTS.contains(&x.to_lowercase().as_str()))
                .unwrap_or(false);
            if is_img {
                if let (Some(name), Some(path)) = (p.file_name().and_then(|n| n.to_str()), p.to_str()) {
                    out.push(ImageMeta { id: name.to_string(), path: path.to_string() });
                }
            }
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// Indices of `all` that are NOT in `recent` (so the same card isn't drawn twice in a row).
/// Falls back to ALL indices if everything is recent.
pub fn candidate_indices(all_len: usize, recent: &[usize]) -> Vec<usize> {
    let avail: Vec<usize> = (0..all_len).filter(|i| !recent.contains(i)).collect();
    if avail.is_empty() { (0..all_len).collect() } else { avail }
}

pub fn draw_random(all: &[ImageMeta], recent: &[usize]) -> Option<(usize, ImageMeta)> {
    use rand::seq::SliceRandom;
    let cands = candidate_indices(all.len(), recent);
    let idx = *cands.choose(&mut rand::thread_rng())?;
    Some((idx, all[idx].clone()))
}

#[allow(dead_code)]
pub fn default_dir(resource_dir: &Path) -> PathBuf { resource_dir.join("gallery") }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn candidates_exclude_recent() {
        assert_eq!(candidate_indices(5, &[1, 3]), vec![0, 2, 4]);
    }
    #[test]
    fn candidates_fallback_when_all_recent() {
        assert_eq!(candidate_indices(3, &[0, 1, 2]), vec![0, 1, 2]);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test gallery:: && cd ..`
Expected: PASS (2 tests).

- [ ] **Step 3: Register module** — add `mod gallery;` to `main.rs`.

- [ ] **Step 4: Commit**
```bash
git add -A && git commit -m "feat: gallery scan and random lucky-draw logic"
```

---

## Phase 2 — Image generation (`qwen-image-2.0-pro`)

### Task 2.1: Prompt builder

**Files:**
- Create: `src-tauri/src/prompt.rs`

- [ ] **Step 1: Write prompt.rs with tests**
```rust
const STYLE_SUFFIX: &str =
    "cute children's book illustration, soft pastel colors, friendly, simple, clean background";

/// Child's literal words + a fixed hidden style suffix. Content comes only from the child.
pub fn build_prompt(transcript: &str) -> String {
    let t = transcript.trim();
    if t.is_empty() { STYLE_SUFFIX.to_string() } else { format!("{t}, {STYLE_SUFFIX}") }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn appends_suffix() {
        assert_eq!(build_prompt("a pink pig in mud"),
            "a pink pig in mud, cute children's book illustration, soft pastel colors, friendly, simple, clean background");
    }
    #[test]
    fn empty_is_suffix_only() {
        assert_eq!(build_prompt("   "), STYLE_SUFFIX);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test prompt:: && cd ..`
Expected: PASS.

- [ ] **Step 3: Register module** — add `mod prompt;` to `main.rs`.

- [ ] **Step 4: Commit**
```bash
git add -A && git commit -m "feat: image prompt builder"
```

### Task 2.2: Image client — parse response (TDD) then call + download

**Files:**
- Create: `src-tauri/src/image_gen.rs`

- [ ] **Step 1: Write the failing test for response parsing**

Create `src-tauri/src/image_gen.rs`:
```rust
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
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cd src-tauri && cargo test image_gen::tests && cd ..`
Expected: PASS (2 tests).

- [ ] **Step 3: Add the network + download function (manual-verified)**

Append to `src-tauri/src/image_gen.rs`:
```rust
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
```

- [ ] **Step 4: Verify it compiles**

Run: `cd src-tauri && cargo build 2>&1 | tail -3 && cd ..`
Expected: compiles. (Live API call is verified end-to-end in Task 4.5.)

- [ ] **Step 5: Register module + commit** — add `mod image_gen;` to `main.rs`.
```bash
git add -A && git commit -m "feat: qwen-image-2.0-pro client with response parsing"
```

---

## Phase 3 — Speech (Paraformer real-time ASR)

### Task 3.1: ASR message build/parse (TDD)

**Files:**
- Create: `src-tauri/src/asr.rs`

- [ ] **Step 1: Write asr.rs message helpers with tests**
```rust
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
struct Frame { header: Header, #[serde(default)] payload: Option<Payload> }
#[derive(Deserialize)]
struct Header { event: String, #[serde(default)] error_message: Option<String> }
#[derive(Deserialize)]
struct Payload { #[serde(default)] output: Option<Output> }
#[derive(Deserialize)]
struct Output { #[serde(default)] sentence: Option<Sentence> }
#[derive(Deserialize)]
struct Sentence { #[serde(default)] text: Option<String>, #[serde(default)] sentence_end: Option<bool> }

pub fn parse_event(json: &str) -> AsrEvent {
    let f: Frame = match serde_json::from_str(json) { Ok(f) => f, Err(_) => return AsrEvent::Other };
    match f.header.event.as_str() {
        "task-started" => AsrEvent::Started,
        "task-finished" => AsrEvent::Finished,
        "task-failed" => AsrEvent::Failed(f.header.error_message.unwrap_or_else(|| "asr failed".into())),
        "result-generated" => {
            let s = f.payload.and_then(|p| p.output).and_then(|o| o.sentence);
            match s {
                Some(s) => {
                    let text = s.text.unwrap_or_default();
                    if s.sentence_end.unwrap_or(false) { AsrEvent::Final(text) } else { AsrEvent::Partial(text) }
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
        assert_eq!(parse_event(r#"{"header":{"task_id":"t","event":"task-started"},"payload":{}}"#), AsrEvent::Started);
        assert_eq!(parse_event(r#"{"header":{"task_id":"t","event":"task-finished"},"payload":{}}"#), AsrEvent::Finished);
        assert_eq!(parse_event(r#"{"header":{"task_id":"t","event":"task-failed","error_message":"nope"}}"#), AsrEvent::Failed("nope".into()));
    }
    #[test]
    fn run_task_has_model_and_lang() {
        let m = run_task_msg("abc");
        assert!(m.contains("paraformer-realtime-v2"));
        assert!(m.contains("\"en\""));
        assert!(m.contains("\"action\":\"run-task\""));
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test asr::tests && cd ..`
Expected: PASS (3 tests).

- [ ] **Step 3: Register module + commit** — add `mod asr;` to `main.rs`.
```bash
git add -A && git commit -m "feat: Paraformer ASR message build/parse"
```

### Task 3.2: ASR WebSocket session (streams audio, emits transcript events)

**Files:**
- Modify: `src-tauri/src/asr.rs`
- Modify: `src-tauri/src/main.rs` (shared session state)

Design: The frontend sends PCM16 chunks via a Tauri command `asr_send_audio(bytes)`. A single background task owns the WebSocket. We use a `tokio::sync::mpsc` channel: the command pushes audio bytes into the channel; the WS task forwards them as binary frames and forwards server events to the frontend via `app.emit`.

- [ ] **Step 1: Add the session struct + start/stop to asr.rs**

Append to `src-tauri/src/asr.rs`:
```rust
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
    req.headers_mut().insert("Authorization", format!("Bearer {api_key}").parse().unwrap());
    req.headers_mut().insert("X-DashScope-DataInspection", "enable".parse().unwrap());

    let (ws, _) = tokio_tungstenite::connect_async(req).await.map_err(|e| format!("ws connect: {e}"))?;
    let (mut write, mut read) = ws.split();

    let task_id = uuid::Uuid::new_v4().simple().to_string();
    write.send(Message::Text(run_task_msg(&task_id))).await.map_err(|e| e.to_string())?;

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
```

- [ ] **Step 2: Verify compile**

Run: `cd src-tauri && cargo build 2>&1 | tail -5 && cd ..`
Expected: compiles. (If `Emitter` import errors, confirm Tauri 2 `tauri::Emitter` trait is in scope.)

- [ ] **Step 3: Commit**
```bash
git add -A && git commit -m "feat: Paraformer WebSocket streaming session"
```

---

## Phase 4 — Commands, frontend shell, screens

### Task 4.1: Tauri commands + state wiring

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Write commands.rs**
```rust
use crate::{asr, gallery, image_gen, secret, settings::{self, AppSettings}};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

#[derive(Default)]
pub struct AppState {
    pub recent: Mutex<Vec<usize>>,
    pub asr: Mutex<Option<asr::AsrSession>>,
}

fn app_data(app: &AppHandle) -> std::path::PathBuf {
    app.path().app_data_dir().unwrap_or(std::env::temp_dir())
}

fn active_gallery_dir(app: &AppHandle, s: &AppSettings) -> std::path::PathBuf {
    if let Some(d) = &s.gallery_dir { return std::path::PathBuf::from(d); }
    // bundled default under resources
    app.path().resource_dir().map(|r| r.join("gallery")).unwrap_or_else(|_| std::path::PathBuf::from("public/gallery"))
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
pub async fn generate_image(app: AppHandle, transcript: String) -> Result<String, String> {
    let cache = app_data(&app).join("cache");
    image_gen::generate(&transcript, &cache).await
}

#[tauri::command]
pub async fn asr_start(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let key = secret::api_key();
    let session = asr::run_session(app.clone(), key).await?;
    *state.asr.lock().unwrap() = Some(session);
    Ok(())
}

#[tauri::command]
pub fn asr_send_audio(state: State<AppState>, chunk: Vec<u8>) -> Result<(), String> {
    if let Some(s) = state.asr.lock().unwrap().as_ref() {
        s.audio_tx.send(chunk).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn asr_stop(state: State<AppState>) -> Result<(), String> {
    if let Some(s) = state.asr.lock().unwrap().as_ref() { let _ = s.stop_tx.send(()); }
    Ok(())
}

#[tauri::command]
pub fn save_snapshot(app: AppHandle, png_base64: String) -> Result<String, String> {
    use std::io::Write;
    let s = settings::load(&app_data(&app));
    let dir = s.snapshot_dir.map(std::path::PathBuf::from).unwrap_or_else(|| app_data(&app).join("snapshots"));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let raw = png_base64.split(',').last().unwrap_or(&png_base64);
    let bytes = base64_decode(raw)?;
    let path = dir.join(format!("snapshot_{}.png", uuid::Uuid::new_v4()));
    let mut f = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    f.write_all(&bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

// minimal base64 decode (avoid extra dep)
fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut rev = [255u8; 256];
    for (i, &c) in T.iter().enumerate() { rev[c as usize] = i as u8; }
    let clean: Vec<u8> = s.bytes().filter(|&b| b != b'=' && !b.is_ascii_whitespace()).collect();
    let mut out = vec![];
    for chunk in clean.chunks(4) {
        let mut buf = 0u32; let mut bits = 0;
        for &c in chunk { let v = rev[c as usize]; if v == 255 { return Err("bad base64".into()); } buf = (buf << 6) | v as u32; bits += 6; }
        bits -= bits % 8;
        for i in (0..bits).step_by(8).rev() { out.push((buf >> i) as u8); }
    }
    Ok(out)
}

#[tauri::command]
pub async fn check_connectivity() -> bool {
    reqwest::Client::new().get("https://dashscope.aliyuncs.com").timeout(std::time::Duration::from_secs(5)).send().await.is_ok()
}
```

- [ ] **Step 2: Wire into main.rs**

Replace `src-tauri/src/main.rs` body so it declares all modules and registers commands:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod xor; mod secret; mod settings; mod gallery; mod prompt; mod image_gen; mod asr; mod commands;
use commands::*;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_settings, set_settings, list_gallery, draw_random,
            generate_image, asr_start, asr_send_audio, asr_stop,
            save_snapshot, check_connectivity
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Verify compile**

Run: `cd src-tauri && cargo build 2>&1 | tail -5 && cd ..`
Expected: compiles.

- [ ] **Step 4: Commit**
```bash
git add -A && git commit -m "feat: tauri commands and app state wiring"
```

### Task 4.2: Frontend — design tokens & base CSS

**Files:**
- Create: `src/styles/tokens.css`, `src/styles/components.css`, `src/styles/animations.css`
- Add fonts: `public/fonts/` (Baloo2, Fredoka woff2)

- [ ] **Step 1: Add bundled fonts**

Download Baloo 2 (700, 800) and Fredoka (500, 600) woff2 files into `public/fonts/`. (From Google Fonts; commit the woff2 files so the app is offline-safe.)

- [ ] **Step 2: Write tokens.css**
```css
@font-face { font-family:'Baloo 2'; src:url('/fonts/Baloo2-Bold.woff2') format('woff2'); font-weight:700; }
@font-face { font-family:'Baloo 2'; src:url('/fonts/Baloo2-ExtraBold.woff2') format('woff2'); font-weight:800; }
@font-face { font-family:'Fredoka'; src:url('/fonts/Fredoka-Medium.woff2') format('woff2'); font-weight:500; }
@font-face { font-family:'Fredoka'; src:url('/fonts/Fredoka-SemiBold.woff2') format('woff2'); font-weight:600; }

:root{
  --bg:#fdeef2; --pink:#ff7eb6; --mint:#a0e7e5; --yellow:#ffd23f; --lav:#c3b6f7; --ink:#2b2b3a;
  --shadow-pink:#ffd1e3; --shadow-mint:#b8ecd8; --shadow-yellow:#e0a800; --shadow-lav:#e7d6ff;
  --radius:24px; --font-display:'Baloo 2',system-ui,sans-serif; --font-body:'Fredoka',system-ui,sans-serif;
}
*{box-sizing:border-box;margin:0;padding:0}
html,body,#app{height:100%}
body{font-family:var(--font-body);background:var(--bg);color:var(--ink);overflow:hidden}
h1,h2,.display{font-family:var(--font-display);font-weight:800}
```

- [ ] **Step 3: Write components.css** (buttons, pills, panels, scattered shapes)
```css
.btn{font-family:var(--font-display);font-weight:800;font-size:28px;padding:18px 48px;border:none;
  border-radius:999px;background:var(--yellow);color:var(--ink);box-shadow:0 7px 0 var(--shadow-yellow);
  cursor:pointer;transition:transform .1s}
.btn:active{transform:translateY(4px);box-shadow:0 3px 0 var(--shadow-yellow)}
.btn.pink{background:var(--pink);color:#fff;box-shadow:0 7px 0 #d85a93}
.btn.mint{background:var(--mint);color:#11625e;box-shadow:0 7px 0 #6fc7c4}
.pill{font-family:var(--font-display);font-weight:800;font-size:18px;padding:8px 20px;border-radius:999px;background:var(--pink);color:#fff}
.panel{background:#fff;border-radius:var(--radius);padding:14px;box-shadow:0 6px 0 var(--shadow-pink)}
.panel.gen{box-shadow:0 6px 0 var(--shadow-mint)}
.shape{position:fixed;z-index:0;pointer-events:none;opacity:.9}
.screen{position:relative;z-index:1;height:100%;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:24px;padding:32px}
.hidden{display:none!important}
```

- [ ] **Step 4: Write animations.css**
```css
@keyframes floaty{0%,100%{transform:translateY(0)}50%{transform:translateY(-14px)}}
@keyframes pop{0%{transform:scale(.6);opacity:0}70%{transform:scale(1.08)}100%{transform:scale(1);opacity:1}}
@keyframes spin{to{transform:rotate(360deg)}}
@keyframes flip{0%{transform:rotateY(0)}100%{transform:rotateY(360deg)}}
.floaty{animation:floaty 4s ease-in-out infinite}
.pop{animation:pop .45s cubic-bezier(.2,1.4,.4,1) both}
.spin{animation:spin 1.1s linear infinite}
```

- [ ] **Step 5: Verify** — these are loaded in Task 4.3. Commit now:
```bash
git add -A && git commit -m "feat: Memphis Bubblegum Pop design system + fonts"
```

### Task 4.3: Frontend — API layer, state machine, app shell

**Files:**
- Create: `src/api.ts`, `src/state.ts`, `src/main.ts`
- Modify: `index.html`

- [ ] **Step 1: index.html**
```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Kindergarten Speaking Competition</title>
    <link rel="stylesheet" href="/src/styles/tokens.css" />
    <link rel="stylesheet" href="/src/styles/components.css" />
    <link rel="stylesheet" href="/src/styles/animations.css" />
  </head>
  <body>
    <div id="shapes"></div>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 2: api.ts (typed wrappers)**
```ts
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface ImageMeta { id: string; path: string; }
export interface AppSettings { gallery_dir: string|null; snapshot_dir: string|null; fullscreen: boolean; child_label: string; }

export const api = {
  getSettings: () => invoke<AppSettings>('get_settings'),
  setSettings: (s: AppSettings) => invoke<void>('set_settings', { new: s }),
  listGallery: () => invoke<ImageMeta[]>('list_gallery'),
  drawRandom: () => invoke<ImageMeta>('draw_random'),
  generateImage: (transcript: string) => invoke<string>('generate_image', { transcript }),
  asrStart: () => invoke<void>('asr_start'),
  asrSendAudio: (chunk: number[]) => invoke<void>('asr_send_audio', { chunk }),
  asrStop: () => invoke<void>('asr_stop'),
  saveSnapshot: (pngBase64: string) => invoke<string>('save_snapshot', { pngBase64 }),
  checkConnectivity: () => invoke<boolean>('check_connectivity'),
};

export function onEvent<T>(name: string, cb: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(name, (e) => cb(e.payload));
}

// Convert a local file path to a webview-loadable URL.
import { convertFileSrc } from '@tauri-apps/api/core';
export const fileUrl = (p: string) => convertFileSrc(p);
```
(If `@tauri-apps/api` is not installed: `npm install @tauri-apps/api @tauri-apps/plugin-dialog`.)

- [ ] **Step 3: state.ts (screen state machine)**
```ts
export type Screen = 'idle'|'draw'|'describe'|'generating'|'compare';
export interface Round { picture?: { id:string; path:string }; transcript:string; generatedPath?:string; }

type Render = (root: HTMLElement, ctx: AppCtx) => void;
export interface AppCtx {
  go: (s: Screen) => void;
  round: Round;
  resetRound: () => void;
  showError: (msg: string) => void;
}

export class App {
  private screen: Screen = 'idle';
  round: Round = { transcript: '' };
  constructor(private root: HTMLElement, private screens: Record<Screen, Render>, private onError:(m:string)=>void) {}
  go(s: Screen) { this.screen = s; this.render(); }
  resetRound() { this.round = { transcript: '' }; }
  showError(m: string) { this.onError(m); }
  render() {
    this.root.innerHTML = '';
    this.screens[this.screen](this.root, this);
  }
}
```

- [ ] **Step 4: main.ts (bootstrap, scatter shapes, error overlay, fullscreen from settings)**
```ts
import { App, type Screen } from './state';
import { api } from './api';
import { renderIdle } from './screens/idle';
import { renderDraw } from './screens/draw';
import { renderDescribe } from './screens/describe';
import { renderGenerating } from './screens/generating';
import { renderCompare } from './screens/compare';
import { mountSettings } from './screens/settings';
import { showError } from './screens/errorOverlay';

function scatterShapes() {
  const host = document.getElementById('shapes')!;
  const svgs = [
    `<svg width="80" height="80"><circle cx="40" cy="40" r="26" fill="#a0e7e5"/></svg>`,
    `<svg width="80" height="80"><polygon points="40,6 72,68 8,68" fill="#c3b6f7"/></svg>`,
    `<svg width="90" height="40"><path d="M5 30 Q20 5 35 30 T70 30" stroke="#ffd23f" stroke-width="8" fill="none"/></svg>`,
    `<svg width="60" height="60"><rect x="10" y="10" width="40" height="40" rx="8" fill="#ff7eb6" transform="rotate(18 30 30)"/></svg>`,
  ];
  const spots = [[5,8],[88,12],[3,70],[90,75],[48,4],[70,88]];
  spots.forEach(([x,y],i)=>{ const d=document.createElement('div'); d.className='shape floaty';
    d.style.left=x+'vw'; d.style.top=y+'vh'; d.style.animationDelay=(i*0.4)+'s';
    d.innerHTML=svgs[i%svgs.length]; host.appendChild(d); });
}

async function boot() {
  scatterShapes();
  const root = document.getElementById('app')!;
  const app = new App(root, {
    idle: renderIdle, draw: renderDraw, describe: renderDescribe,
    generating: renderGenerating, compare: renderCompare,
  } as Record<Screen, any>, (m)=>showError(m, ()=>app.go('idle')));

  mountSettings(app);                  // hidden gear / "S" key
  const s = await api.getSettings();
  if (s.fullscreen) { /* applied via tauri window in settings.ts */ }
  if (!(await api.checkConnectivity())) showError("No internet — speech & image need a connection 🌐", ()=>app.go('idle'));
  app.go('idle');
}
boot();
```

- [ ] **Step 5: Verify compile** (screens not yet created will fail TS import — create stubs in next tasks). For now ensure Rust+Vite build pipeline is intact by running `npm run build 2>&1 | tail -5` AFTER screen tasks. Skip running here.

- [ ] **Step 6: Commit**
```bash
git add -A && git commit -m "feat: frontend api layer, state machine, app shell"
```

### Task 4.4: Idle screen

**Files:**
- Create: `src/screens/idle.ts`

- [ ] **Step 1: Write idle.ts**
```ts
import type { App } from '../state';
export function renderIdle(root: HTMLElement, app: App) {
  const el = document.createElement('div');
  el.className = 'screen';
  el.innerHTML = `
    <div class="display pop" style="font-size:88px">🎨🗣️✨</div>
    <h1 class="pop" style="font-size:56px;color:var(--pink);text-align:center">English Speaking Stars</h1>
    <p class="display" style="font-size:24px;color:var(--lav)">Draw a picture · Describe it · Watch the magic!</p>
    <button class="btn pink floaty" id="start" style="margin-top:20px">Start! 🚀</button>`;
  root.appendChild(el);
  el.querySelector<HTMLButtonElement>('#start')!.onclick = () => { app.resetRound(); app.go('draw'); };
}
```

- [ ] **Step 2: Commit**
```bash
git add -A && git commit -m "feat: idle/welcome screen"
```

### Task 4.5: Draw screen (lucky draw)

**Files:**
- Create: `src/screens/draw.ts`

- [ ] **Step 1: Write draw.ts**
```ts
import type { App } from '../state';
import { api, fileUrl } from '../api';

export function renderDraw(root: HTMLElement, app: App) {
  const el = document.createElement('div'); el.className = 'screen';
  el.innerHTML = `
    <h1 style="color:var(--pink);font-size:44px">Lucky Draw 🎴</h1>
    <div id="card" class="panel" style="width:62vw;aspect-ratio:16/9;display:flex;align-items:center;justify-content:center;font-size:120px">🎴</div>
    <div style="display:flex;gap:18px">
      <button class="btn" id="draw">Draw a Card!</button>
      <button class="btn mint hidden" id="use">Use this one ✅</button>
      <button class="btn pink hidden" id="again">Draw again 🔄</button>
    </div>`;
  root.appendChild(el);
  const card = el.querySelector<HTMLDivElement>('#card')!;
  const drawBtn = el.querySelector<HTMLButtonElement>('#draw')!;
  const useBtn = el.querySelector<HTMLButtonElement>('#use')!;
  const againBtn = el.querySelector<HTMLButtonElement>('#again')!;

  async function doDraw() {
    card.classList.add('spin');
    try {
      const pic = await api.drawRandom();
      setTimeout(() => {
        card.classList.remove('spin');
        card.style.fontSize = '0';
        card.innerHTML = `<img src="${fileUrl(pic.path)}" style="width:100%;height:100%;object-fit:cover;border-radius:16px" class="pop"/>`;
        app.round.picture = pic;
        drawBtn.classList.add('hidden'); useBtn.classList.remove('hidden'); againBtn.classList.remove('hidden');
      }, 700);
    } catch (e) { card.classList.remove('spin'); app.showError(String(e)); }
  }
  drawBtn.onclick = doDraw;
  againBtn.onclick = doDraw;
  useBtn.onclick = () => app.go('describe');
}
```

- [ ] **Step 2: Verify gallery wiring live**

Put 2–3 test images in `public/gallery/`. Run `npm run tauri dev`, click Start → Draw a Card. Expected: an image appears. (This verifies `draw_random` + `list_gallery` + `convertFileSrc` end-to-end.)

- [ ] **Step 3: Commit**
```bash
git add -A && git commit -m "feat: lucky draw screen with random gallery pick"
```

### Task 4.6: audio.ts (mic → PCM16 16k) + Describe screen

**Files:**
- Create: `src/audio.ts`, `src/screens/describe.ts`

- [ ] **Step 1: Write audio.ts**
```ts
// Captures mic, resamples to 16kHz mono PCM16, calls onChunk with byte arrays (~100ms).
export class MicStreamer {
  private ctx?: AudioContext; private stream?: MediaStream; private node?: ScriptProcessorNode; private src?: MediaStreamAudioSourceNode;
  constructor(private onChunk: (bytes: number[]) => void) {}
  async start() {
    this.stream = await navigator.mediaDevices.getUserMedia({ audio: { channelCount: 1, echoCancellation: true, noiseSuppression: true } });
    this.ctx = new AudioContext();
    this.src = this.ctx.createMediaStreamSource(this.stream);
    this.node = this.ctx.createScriptProcessor(4096, 1, 1);
    const inRate = this.ctx.sampleRate;
    this.src.connect(this.node); this.node.connect(this.ctx.destination);
    this.node.onaudioprocess = (e) => {
      const input = e.inputBuffer.getChannelData(0);
      const down = downsample(input, inRate, 16000);
      const pcm = floatToPCM16(down);
      this.onChunk(Array.from(new Uint8Array(pcm.buffer)));
    };
  }
  async stop() {
    this.node?.disconnect(); this.src?.disconnect();
    this.stream?.getTracks().forEach(t => t.stop());
    await this.ctx?.close();
  }
}
function downsample(buf: Float32Array, inRate: number, outRate: number): Float32Array {
  if (outRate >= inRate) return buf;
  const ratio = inRate / outRate; const len = Math.floor(buf.length / ratio); const out = new Float32Array(len);
  for (let i=0;i<len;i++) out[i] = buf[Math.floor(i*ratio)];
  return out;
}
function floatToPCM16(buf: Float32Array): Int16Array {
  const out = new Int16Array(buf.length);
  for (let i=0;i<buf.length;i++){ const s=Math.max(-1,Math.min(1,buf[i])); out[i]= s<0 ? s*0x8000 : s*0x7fff; }
  return out;
}
```

- [ ] **Step 2: Write describe.ts**
```ts
import type { App } from '../state';
import { api, fileUrl, onEvent } from '../api';
import { MicStreamer } from '../audio';

export function renderDescribe(root: HTMLElement, app: App) {
  const el = document.createElement('div'); el.className = 'screen';
  const picUrl = app.round.picture ? fileUrl(app.round.picture.path) : '';
  el.innerHTML = `
    <div class="panel" style="width:58vw;aspect-ratio:16/9"><img src="${picUrl}" style="width:100%;height:100%;object-fit:cover;border-radius:16px"/></div>
    <div id="transcript" class="display" style="font-size:30px;min-height:44px;color:var(--ink);text-align:center;max-width:70vw">🎤 Tap the mic and tell me what you see!</div>
    <div style="display:flex;gap:18px">
      <button class="btn pink" id="mic">🎤 Start Talking</button>
      <button class="btn mint hidden" id="gen">Make it! ✨</button>
    </div>`;
  root.appendChild(el);
  const tEl = el.querySelector<HTMLDivElement>('#transcript')!;
  const mic = el.querySelector<HTMLButtonElement>('#mic')!;
  const gen = el.querySelector<HTMLButtonElement>('#gen')!;

  let streamer: MicStreamer | null = null;
  let recording = false;
  let finalText = '';
  const unlisten: Array<Promise<any>> = [];

  unlisten.push(onEvent<string>('asr://partial', (s) => { tEl.textContent = finalText + ' ' + s; }));
  unlisten.push(onEvent<string>('asr://final', (s) => { finalText = (finalText + ' ' + s).trim(); tEl.textContent = finalText; app.round.transcript = finalText; }));
  unlisten.push(onEvent<string>('asr://error', (e) => app.showError(e)));

  mic.onclick = async () => {
    if (!recording) {
      try {
        finalText=''; app.round.transcript='';
        await api.asrStart();
        streamer = new MicStreamer((bytes)=>api.asrSendAudio(bytes));
        await streamer.start();
        recording = true; mic.textContent = '⏹ Stop'; mic.classList.add('mint');
      } catch (e) { app.showError(String(e)); }
    } else {
      await streamer?.stop(); await api.asrStop();
      recording = false; mic.textContent = '🎤 Talk again';
      gen.classList.remove('hidden');
    }
  };
  gen.onclick = () => { unlisten.forEach(u=>u.then(f=>f())); app.go('generating'); };
}
```

- [ ] **Step 3: Verify ASR live**

Run `npm run tauri dev`, go to Describe, tap mic, speak English. Expected: live partial text appears, finalizes on pause. (Verifies Paraformer end-to-end. Requires internet + a valid key built in: run dev with `DASHSCOPE_API_KEY=<key> npm run tauri dev`.)

- [ ] **Step 4: Commit**
```bash
git add -A && git commit -m "feat: mic capture + describe screen with live transcript"
```

### Task 4.7: Generating screen (calls image API)

**Files:**
- Create: `src/screens/generating.ts`

- [ ] **Step 1: Write generating.ts**
```ts
import type { App } from '../state';
import { api } from '../api';
export function renderGenerating(root: HTMLElement, app: App) {
  const el = document.createElement('div'); el.className = 'screen';
  el.innerHTML = `
    <div class="display spin" style="font-size:90px">🎨</div>
    <h1 style="color:var(--pink);font-size:44px">AI is painting…</h1>
    <p class="display" style="font-size:24px;color:var(--lav)">"${app.round.transcript || '...'}"</p>`;
  root.appendChild(el);
  (async () => {
    try {
      const path = await api.generateImage(app.round.transcript);
      app.round.generatedPath = path;
      app.go('compare');
    } catch (e) { app.showError(String(e)); }
  })();
}
```

- [ ] **Step 2: Verify image gen live** (covered in Task 4.8 end-to-end). Commit:
```bash
git add -A && git commit -m "feat: generating screen invoking qwen-image"
```

### Task 4.8: Compare screen + snapshot

**Files:**
- Create: `src/screens/compare.ts`, `src/snapshot.ts`

- [ ] **Step 1: Write snapshot.ts (canvas composite → base64 PNG)**
```ts
// Compose original + generated side by side with the transcript caption into one PNG.
export async function composeSnapshot(origUrl: string, genUrl: string, caption: string, label: string): Promise<string> {
  const [a, b] = await Promise.all([loadImg(origUrl), loadImg(genUrl)]);
  const W = 1600, H = 760, pad = 30, gap = 30;
  const cw = (W - pad*2 - gap) / 2, ch = cw * 9/16;
  const c = document.createElement('canvas'); c.width = W; c.height = H;
  const ctx = c.getContext('2d')!;
  ctx.fillStyle = '#fdeef2'; ctx.fillRect(0,0,W,H);
  drawCover(ctx, a, pad, 80, cw, ch);
  drawCover(ctx, b, pad+cw+gap, 80, cw, ch);
  ctx.fillStyle = '#ff7eb6'; ctx.font = '800 40px "Baloo 2", sans-serif'; ctx.textAlign='center';
  ctx.fillText('English Speaking Stars ⭐', W/2, 56);
  ctx.fillStyle = '#2b2b3a'; ctx.font = '500 30px "Fredoka", sans-serif';
  ctx.fillText(`"${caption}"`, W/2, 80+ch+50);
  if (label) { ctx.fillStyle='#c3b6f7'; ctx.font='600 26px "Fredoka",sans-serif'; ctx.fillText(label, W/2, 80+ch+90); }
  return c.toDataURL('image/png');
}
function loadImg(src: string){ return new Promise<HTMLImageElement>((res,rej)=>{const i=new Image(); i.crossOrigin='anonymous'; i.onload=()=>res(i); i.onerror=rej; i.src=src;}); }
function drawCover(ctx:CanvasRenderingContext2D,img:HTMLImageElement,x:number,y:number,w:number,h:number){
  const r=Math.max(w/img.width,h/img.height); const iw=img.width*r, ih=img.height*r;
  ctx.save(); ctx.beginPath(); roundRect(ctx,x,y,w,h,16); ctx.clip();
  ctx.drawImage(img, x+(w-iw)/2, y+(h-ih)/2, iw, ih); ctx.restore();
}
function roundRect(ctx:CanvasRenderingContext2D,x:number,y:number,w:number,h:number,r:number){
  ctx.moveTo(x+r,y); ctx.arcTo(x+w,y,x+w,y+h,r); ctx.arcTo(x+w,y+h,x,y+h,r); ctx.arcTo(x,y+h,x,y,r); ctx.arcTo(x,y,x+w,y,r); ctx.closePath();
}
```

- [ ] **Step 2: Write compare.ts**
```ts
import type { App } from '../state';
import { api, fileUrl } from '../api';
import { composeSnapshot } from '../snapshot';

export function renderCompare(root: HTMLElement, app: App) {
  const orig = app.round.picture ? fileUrl(app.round.picture.path) : '';
  const gen = app.round.generatedPath ? fileUrl(app.round.generatedPath) : '';
  const el = document.createElement('div'); el.className = 'screen';
  el.innerHTML = `
    <h1 style="color:var(--pink);font-size:40px">Look what you made! 🎉</h1>
    <div style="display:flex;gap:22px;align-items:center;width:88vw">
      <div class="panel" style="flex:1"><div class="display" style="text-align:center;color:var(--pink);margin-bottom:6px">THE PICTURE</div>
        <img src="${orig}" style="width:100%;aspect-ratio:16/9;object-fit:cover;border-radius:14px"/></div>
      <div class="display" style="font-size:40px;color:var(--pink)">VS</div>
      <div class="panel gen" style="flex:1"><div class="display" style="text-align:center;color:#11625e;margin-bottom:6px">YOUR AI IMAGE ✨</div>
        <img src="${gen}" class="pop" style="width:100%;aspect-ratio:16/9;object-fit:cover;border-radius:14px"/></div>
    </div>
    <p class="display" style="font-size:24px;color:var(--lav)">"${app.round.transcript}"</p>
    <div style="display:flex;gap:18px">
      <button class="btn mint" id="save">💾 Save</button>
      <button class="btn pink" id="next">Next Child ➡️</button>
    </div>`;
  root.appendChild(el);
  el.querySelector<HTMLButtonElement>('#save')!.onclick = async (e) => {
    const btn = e.currentTarget as HTMLButtonElement;
    try {
      const s = await api.getSettings();
      const png = await composeSnapshot(orig, gen, app.round.transcript, s.child_label);
      await api.saveSnapshot(png);
      btn.textContent = '✅ Saved';
    } catch (err) { app.showError(String(err)); }
  };
  el.querySelector<HTMLButtonElement>('#next')!.onclick = () => { app.resetRound(); app.go('draw'); };
}
```

- [ ] **Step 3: Full end-to-end verification**

Run `DASHSCOPE_API_KEY=<valid-key> npm run tauri dev`. Complete a full round: Start → Draw → Talk ("a pink pig in the mud") → Make it → see generated image in Compare → Save. Expected: generated image appears within ~30s; Save writes a PNG (check the app data `snapshots/` folder). This verifies image_gen + snapshot + the whole flow.

- [ ] **Step 4: Commit**
```bash
git add -A && git commit -m "feat: compare screen + snapshot compositing"
```

### Task 4.9: Settings panel + error overlay

**Files:**
- Create: `src/screens/settings.ts`, `src/screens/errorOverlay.ts`

- [ ] **Step 1: errorOverlay.ts**
```ts
export function showError(msg: string, onRetry: () => void) {
  document.querySelector('#err')?.remove();
  const o = document.createElement('div'); o.id='err';
  o.style.cssText='position:fixed;inset:0;z-index:50;background:rgba(253,238,242,.96);display:flex;flex-direction:column;align-items:center;justify-content:center;gap:20px';
  o.innerHTML = `<div style="font-size:80px">🐣</div>
    <h1 class="display" style="color:var(--pink);font-size:40px">Oops, let's try again!</h1>
    <p class="display" style="font-size:22px;color:var(--lav);max-width:60vw;text-align:center">${msg}</p>
    <button class="btn pink" id="retry">Try Again 🔄</button>`;
  document.body.appendChild(o);
  o.querySelector<HTMLButtonElement>('#retry')!.onclick = () => { o.remove(); onRetry(); };
}
```

- [ ] **Step 2: settings.ts (hidden gear + "S" key; folder pickers + fullscreen)**
```ts
import type { App } from '../state';
import { api } from '../api';
import { open } from '@tauri-apps/plugin-dialog';
import { getCurrentWindow } from '@tauri-apps/api/window';

export function mountSettings(app: App) {
  const gear = document.createElement('button');
  gear.textContent = '⚙️';
  gear.style.cssText='position:fixed;top:10px;right:12px;z-index:40;background:none;border:none;font-size:26px;opacity:.5;cursor:pointer';
  document.body.appendChild(gear);
  const openPanel = async () => {
    const s = await api.getSettings();
    const p = document.createElement('div'); p.id='settings';
    p.style.cssText='position:fixed;inset:0;z-index:45;background:rgba(0,0,0,.4);display:flex;align-items:center;justify-content:center';
    p.innerHTML = `<div class="panel" style="width:520px;display:flex;flex-direction:column;gap:14px">
      <h2 style="color:var(--pink)">Operator Settings</h2>
      <label class="display">Child / Round label</label>
      <input id="label" value="${s.child_label}" style="font-size:20px;padding:8px;border-radius:10px;border:2px solid var(--lav)"/>
      <button class="btn mint" id="gallery">📁 Gallery folder</button>
      <div id="gpath" style="font-size:14px;color:#888">${s.gallery_dir ?? '(bundled default)'}</div>
      <button class="btn mint" id="snap">📁 Snapshot folder</button>
      <div id="spath" style="font-size:14px;color:#888">${s.snapshot_dir ?? '(default)'}</div>
      <label class="display"><input type="checkbox" id="fs" ${s.fullscreen?'checked':''}/> Fullscreen</label>
      <div style="display:flex;gap:10px"><button class="btn pink" id="ok">Save</button><button class="btn" id="cancel">Close</button></div>
    </div>`;
    document.body.appendChild(p);
    const next = { ...s };
    p.querySelector<HTMLButtonElement>('#gallery')!.onclick = async () => { const d = await open({ directory:true }); if (d) { next.gallery_dir = d as string; p.querySelector('#gpath')!.textContent = d as string; } };
    p.querySelector<HTMLButtonElement>('#snap')!.onclick = async () => { const d = await open({ directory:true }); if (d) { next.snapshot_dir = d as string; p.querySelector('#spath')!.textContent = d as string; } };
    p.querySelector<HTMLButtonElement>('#cancel')!.onclick = () => p.remove();
    p.querySelector<HTMLButtonElement>('#ok')!.onclick = async () => {
      next.child_label = p.querySelector<HTMLInputElement>('#label')!.value;
      next.fullscreen = p.querySelector<HTMLInputElement>('#fs')!.checked;
      await api.setSettings(next);
      await getCurrentWindow().setFullscreen(next.fullscreen);
      p.remove();
    };
  };
  gear.onclick = openPanel;
  window.addEventListener('keydown', (e)=>{ if (e.key.toLowerCase()==='s' && e.ctrlKey) openPanel(); });
}
```
(Install dialog plugin JS: `npm install @tauri-apps/plugin-dialog @tauri-apps/api`.)

- [ ] **Step 3: Verify build + settings**

Run `npm run tauri dev`. Click ⚙️, set a child label and pick a gallery folder, Save. Re-open to confirm persistence. Toggle fullscreen.

- [ ] **Step 4: Commit**
```bash
git add -A && git commit -m "feat: operator settings panel + kid-friendly error overlay"
```

---

## Phase 5 — Capabilities & permissions

### Task 5.1: Tauri capabilities (dialog, fs read for chosen folders, window fullscreen)

**Files:**
- Modify: `src-tauri/capabilities/default.json`

- [ ] **Step 1: Grant needed permissions**

In `src-tauri/capabilities/default.json`, ensure `permissions` includes:
```json
"core:default",
"dialog:default",
"core:window:allow-set-fullscreen",
"core:webview:default"
```
And confirm `assetProtocol` is enabled in `tauri.conf.json` `app.security`:
```json
"security": { "assetProtocol": { "enable": true, "scope": ["**"] }, "csp": null }
```
(`assetProtocol` is required for `convertFileSrc` to load gallery + generated images.)

- [ ] **Step 2: Verify** — images load in dev (already confirmed in 4.5/4.8). Commit:
```bash
git add -A && git commit -m "chore: tauri capabilities for dialog, fullscreen, asset protocol"
```

### Task 5.2: Bundle default gallery as a resource

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Add: `public/gallery/*` (the provided sample pictures)

- [ ] **Step 1: Add the resource**

Place the 5 sample pictures (and any others) in `public/gallery/`. In `tauri.conf.json` `bundle`, add:
```json
"resources": ["../public/gallery/*"]
```
And confirm `commands.rs::active_gallery_dir` resolves `resource_dir()/gallery` when no custom folder is set. (Dev uses `public/gallery`; built app uses the bundled resource.)

- [ ] **Step 2: Commit**
```bash
git add -A && git commit -m "chore: bundle default gallery pictures as a resource"
```

---

## Phase 6 — Packaging & CI

### Task 6.1: macOS .dmg build

**Files:**
- Modify: `src-tauri/tauri.conf.json` (`bundle.targets`, icons)

- [ ] **Step 1: Configure bundle**

In `tauri.conf.json` set:
```json
"bundle": { "active": true, "targets": ["dmg", "nsis"], "icon": ["icons/32x32.png","icons/128x128.png","icons/icon.icns","icons/icon.ico"], "resources": ["../public/gallery/*"] }
```
Generate icons from a 1024px PNG: `npm run tauri icon path/to/logo.png`.

- [ ] **Step 2: Build the .dmg locally**

Run: `DASHSCOPE_API_KEY=<valid-key> npm run tauri build`
Expected: a `.dmg` appears under `src-tauri/target/release/bundle/dmg/`. Install it on a clean Mac and complete one full round to confirm the embedded key works.

- [ ] **Step 3: Commit**
```bash
git add -A && git commit -m "chore: macOS dmg + Windows nsis bundle config and icons"
```

### Task 6.2: GitHub Actions CI (builds .dmg + .exe with key from secret)

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Create the workflow**
```yaml
name: Release
on:
  push:
    tags: ['v*']
  workflow_dispatch:

jobs:
  build:
    permissions:
      contents: write
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: macos-latest
            args: '--target universal-apple-darwin'
          - platform: windows-latest
            args: ''
    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20 }
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.platform == 'macos-latest' && 'aarch64-apple-darwin,x86_64-apple-darwin' || '' }}
      - name: Install deps
        run: npm install
      - uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          DASHSCOPE_API_KEY: ${{ secrets.DASHSCOPE_API_KEY }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'Kindergarten Speaking ${{ github.ref_name }}'
          releaseDraft: true
          args: ${{ matrix.args }}
```

- [ ] **Step 2: Configure the secret + push**

Create a private GitHub repo, push the code, then in repo Settings → Secrets → Actions add `DASHSCOPE_API_KEY` with the **rotated** capped key. Push a tag:
```bash
git remote add origin <repo-url>   # if not already
git push -u origin main
git tag v0.1.0 && git push origin v0.1.0
```
Expected: the workflow runs and produces a draft Release with a `.dmg` and a `.exe`. Download the `.exe`, run on a Windows machine, complete one round.

- [ ] **Step 3: Commit (workflow file)**
```bash
git add -A && git commit -m "ci: GitHub Actions release workflow for dmg + exe"
```

### Task 6.3: Operator README

**Files:**
- Create: `README.md`

- [ ] **Step 1: Write a short operator guide**

Cover: what the app does; how to run the round flow; how to swap gallery pictures (⚙️ → Gallery folder, or drop files in the bundled folder); where snapshots are saved; the **internet requirement**; the **rotate-key + spending-cap** security note; how to cut a release (push a `vX.Y.Z` tag → CI builds installers).

- [ ] **Step 2: Commit**
```bash
git add -A && git commit -m "docs: operator README"
```

---

## Self-Review (completed during planning)

**Spec coverage check:**
- Tauri app, dmg+exe → Phase 0, 6 ✓
- Embedded obfuscated key → Tasks 1.1, 1.2, 6.2 (CI secret) ✓
- Spending-cap/rotate reminder → README (6.3) ✓
- Speak + Paraformer transcribe → Tasks 3.1, 3.2, 4.6 ✓
- qwen-image-2.0-pro generation → Tasks 2.1, 2.2, 4.7 ✓ (corrected to **synchronous** call)
- Pre-loaded gallery + random lucky draw, swappable folder → Tasks 1.4, 4.5, 5.2, settings ✓
- Single station fullscreen → 0.2, settings.ts ✓
- Compare display only + save snapshot → Tasks 4.8, snapshot.ts ✓
- No hint on picture → draw/describe screens show image only ✓
- Literal + style suffix prompt → Task 2.1 ✓
- Memphis Bubblegum Pop + bundled fonts → Tasks 4.2, screens ✓
- Kid-friendly errors + connectivity check → Tasks 4.1, 4.9 ✓
- Operator settings (gallery/snapshot folder, fullscreen, label) → Task 4.9 ✓

**Type consistency:** Command names match between `commands.rs`, `main.rs::generate_handler!`, and `api.ts` (`get_settings`, `set_settings`, `list_gallery`, `draw_random`, `generate_image`, `asr_start`, `asr_send_audio`, `asr_stop`, `save_snapshot`, `check_connectivity`). Event names match between `asr.rs` emit and `describe.ts` listen (`asr://partial`, `asr://final`, `asr://error`, `asr://finished`). `AppSettings` fields match Rust struct ↔ `api.ts` interface. `set_settings` arg is `new` in both Rust and `api.ts`. `save_snapshot` arg is `pngBase64`→`png_base64` (Tauri camelCase↔snake mapping).

**Note on the one cross-cutting risk:** the synchronous `qwen-image-2.0-pro` HTTP call may take 20–40s; reqwest has no default timeout (good), but if the venue network is slow, consider adding a generous explicit timeout later. Logged here rather than silently assumed.
```
