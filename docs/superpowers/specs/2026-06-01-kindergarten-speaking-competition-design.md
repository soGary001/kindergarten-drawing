# Kindergarten English Speaking Competition — Design Spec

**Date:** 2026-06-01
**Status:** Approved (design phase)

## Overview

A desktop app for a live international kindergarten English speaking competition. A
single station drives a big screen (projector/TV), operated by a teacher. Children take
turns: each draws a random picture, describes it out loud in English, the app transcribes
the speech and generates an AI image from the child's words, then displays the **original
picture and the AI-generated image side by side** for comparison.

The experience is designed to be cute, eye-catching, and high-energy ("dopamine") for a
children's exhibition setting.

## Goals

- Let a child draw a random picture, describe it in English by speaking, and see an AI
  image generated live from their description.
- Show original vs. generated side by side for an exciting "reveal" moment.
- Ship as a self-contained `.dmg` (macOS) and `.exe` (Windows) that can be copied to other
  computers and run by double-clicking.
- **Do not leak the API key**, even though the key is embedded in the distributed app.

## Non-Goals (YAGNI)

- No scoring/judging UI, no results database. Comparison display only (plus optional
  snapshot saving).
- No multi-device networking. Single independent station.
- No accounts, no cloud sync, no analytics.
- No offline image generation (cloud APIs require internet — accepted).

## Decisions (locked during brainstorming)

| Topic | Decision |
|---|---|
| Key security | **Embed + obfuscate** in a compiled Rust binary, with a spending-capped key. No proxy server. |
| Speech input | Child **speaks**; app transcribes via Alibaba **Paraformer real-time ASR**. |
| Picture source | **Pre-loaded gallery**, random "lucky draw" (抽卡). Swappable folder, no rebuild. |
| Run mode | **Single station + big screen**, teacher-operated, fullscreen kiosk. |
| Scoring/saving | **Comparison display only** + optional "Save Snapshot" composite PNG. |
| Platforms | **Both `.dmg` and `.exe`**. |
| Desktop framework | **Tauri 2** (Rust backend + webview frontend). |
| Windows build | **GitHub Actions CI** builds both `.dmg` and `.exe` on release. |
| Card hint | **None** — pure picture, child describes entirely on their own. |
| Prompt handling | **Literal transcript + fixed hidden style suffix** (fair + polished). |
| Visual style | **Memphis, "Bubblegum Pop" palette** (pink base; mint, sunny-yellow, lavender accents). |
| AI provider | Alibaba **DashScope / Bailian** for both ASR and image generation. |
| Image model | `qwen-image-2.0-pro`, 16:9 aspect. |

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│ Tauri App                                                 │
│                                                           │
│  ┌─────────────────────┐      ┌──────────────────────┐    │
│  │ Frontend (webview)  │ IPC  │ Rust backend (core)   │    │
│  │ HTML/CSS/JS (Vite)  │◄────►│ - holds OBFUSCATED key │   │
│  │ - 5 screens         │ cmds │ - DashScope ASR (WS)   │   │
│  │ - mic capture       │      │ - image gen (HTTP)     │   │
│  │ - Memphis UI/anim   │      │ - gallery/file IO      │   │
│  │ - NEVER sees key    │      │ - snapshot compositing │   │
│  └─────────────────────┘      └──────────────────────┘    │
└─────────────────────────────────────────────────────────┘
                                   │ HTTPS / WSS
                                   ▼
                    DashScope / Bailian (dashscope.aliyuncs.com)
                    - Paraformer real-time ASR (WebSocket)
                    - qwen-image-2.0-pro (async task API)
```

**Boundary:** The frontend is the kid-facing UI and knows nothing about the key or the API.
It invokes Rust commands (`transcribe`, `generate_image`, `list_gallery`, `draw_random`,
`save_snapshot`, `get_settings`, `set_settings`). All secrets and network access live in
Rust.

### Tauri commands (interface)

- `list_gallery() -> [ImageMeta]` — list images in the active gallery folder.
- `draw_random(exclude_recent) -> ImageMeta` — pick a random picture for the lucky draw.
- `start_transcription()` / `stop_transcription()` — begin/end mic streaming; transcript
  chunks are pushed to the frontend via Tauri events (`transcript://partial`,
  `transcript://final`).
- `generate_image(prompt) -> GeneratedImage` — submit prompt (literal + style suffix) to
  `qwen-image-2.0-pro`, poll to completion, download, return a local path/URL.
- `save_snapshot(original, generated, transcript, label) -> path` — composite PNG.
- `get_settings()` / `set_settings(...)` — gallery folder, snapshot folder, fullscreen,
  child name/round label.
- `check_connectivity() -> bool` — on-launch network check.

## Security Model

- The DashScope key is **compiled into the Rust binary**, stored **obfuscated** (byte-split
  + XOR with a per-build constant, reassembled at runtime). Never plaintext on disk, never
  sent to the frontend.
- A Rust binary is significantly harder to extract a key from than an Electron asar (which
  unzips trivially) — this is the main reason Tauri was chosen over Electron.
- **Defense in depth (operator action items):**
  - Set a **spending cap** on the key in the Bailian console.
  - **Rotate** the key that was shared in plaintext during brainstorming.
- The key location in the source is isolated to a single module so swapping/rotating it is a
  one-line change before a rebuild.

> ⚠️ Honest limitation: an embedded key in a distributed binary is *recoverable* by a
> determined attacker. This approach raises the bar and bounds the blast radius (spending
> cap) — it does not make extraction impossible. Acceptable for a small internal event per
> the chosen tradeoff.

## User Flow (5 screens)

1. **Idle / Welcome** — competition title, animated Memphis shapes, large "Start" button.
2. **Lucky Draw 🎴** — child taps; cards shuffle/flip animation; lands on a **random**
   picture from the gallery. Pure image, no hint words. Buttons: "Use this picture" / "Draw
   again" (operator-gated).
3. **Describe 🎤** — picture shown large; child taps mic and speaks English; **live
   transcript** appears in real time. "Re-record" available. Operator confirms transcript,
   then taps "Generate".
4. **Generating ✨** — playful "AI is painting…" loader animation while the image task runs
   (~10–30s).
5. **Compare** — original (left) **VS** AI image (right) at 16:9, transcript shown beneath.
   Buttons: **Save Snapshot** (composite PNG to snapshots folder) and **Next Child** (reset
   to Lucky Draw).

## Speech + Image Generation

### Speech (Paraformer real-time ASR)
- Frontend captures mic via `getUserMedia` + records PCM/audio chunks.
- Chunks are streamed to Rust, which maintains a WebSocket to DashScope Paraformer
  (`wss://dashscope.aliyuncs.com/...`), language = English.
- Partial + final transcripts are emitted back to the frontend as Tauri events for live
  display.
- Chosen over the browser Web Speech API because the latter routes audio to Google servers
  (unreliable / likely blocked in mainland China).

### Image generation (qwen-image-2.0-pro)
- Prompt = **child's literal transcript** + a **fixed hidden style suffix** (e.g. "cute
  children's book illustration, soft colors, friendly"). Content comes only from the child;
  the suffix only standardizes look (keeps competition fair while results stay polished).
- Request **16:9** size to match the landscape gallery pictures.
- DashScope image synthesis is async: submit task → poll status → fetch result URL → Rust
  downloads the image to a local temp/cache path → returns path to frontend.

### Error handling (kid-friendly)
- No internet / API error / no speech detected → gentle full-screen message ("Oops, let's
  try again! 🐣") with a big retry button. Never show raw errors or stack traces.
- On-launch connectivity check warns the operator if the venue network is down.

## Gallery, Settings, Offline

- **Gallery:** ships with a default image set (the provided samples). Loaded from a folder
  the operator can change at runtime — dropping new images in makes them available to the
  draw immediately, no rebuild. Supports common formats (PNG/JPG/WebP).
- **Operator settings (hidden):** corner gear icon or keyboard shortcut opens a small panel:
  choose gallery folder, choose snapshot folder, toggle fullscreen, set the on-screen child
  name / round label, open snapshots folder.
- **Offline reality:** UI works offline, but speech + image generation require internet
  (cloud APIs). Venue must provide connectivity.

## Visual Design

- **Style:** Memphis design — pastel background with scattered geometric shapes (circles,
  triangles, squiggles, dots, zigzags), playful and energetic, suitable for a children's
  exhibition.
- **Palette ("Bubblegum Pop"):** soft pink base (`#fdeef2`), accents in mint (`#a0e7e5`),
  sunny yellow (`#ffd23f`), coral pink (`#ff7eb6`), lavender (`#c3b6f7`).
- **Typography:** chunky rounded display fonts (Baloo 2 / Fredoka), **bundled locally** (no
  CDN dependency, works offline).
- **Feel:** big touch targets, bouncy/pop animations, drop-shadow "sticker" buttons. Cute,
  eye-catching, dopamine-rich.

## Packaging & CI

- **macOS:** `tauri build` produces `.dmg` locally.
- **Windows:** **GitHub Actions** workflow (Tauri's cross-platform build matrix) produces the
  `.exe` (and `.dmg`) on release/tag. Private repo acceptable.
- Output is a self-contained installer that runs on a clean machine by double-clicking.

## Open Questions / To Confirm During Implementation

- Exact DashScope API shapes for `qwen-image-2.0-pro` (model id string, size param, async
  task endpoints) and Paraformer real-time WebSocket protocol — verify against current
  Bailian docs at implementation time.
- Audio format/sample-rate Paraframer expects, and the most reliable way to stream webview
  audio chunks across the Tauri IPC bridge.
- Snapshot composite layout (side-by-side + transcript caption + branding).
```
