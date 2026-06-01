# Kindergarten Speaking Competition

A kiosk app for a kindergarten English speaking competition. A child draws a random picture card, describes what they see in English aloud, and the app transcribes their speech (Alibaba Paraformer) and generates an AI image (DashScope `qwen-image-2.0-pro`) based on their description. The original card and the AI-generated image are shown side by side so everyone can compare the two.

---

## Adding Pictures to the Gallery

There are two ways to provide the gallery of picture cards.

**Option A — Custom folder (recommended for large sets)**

Open the in-app settings (click the gear icon ⚙ or press **Ctrl+S**), go to **Gallery folder**, and point it to any folder that contains your PNG/JPG/WebP images.

**Option B — Drop files into the default gallery folder**

The app creates a `gallery` folder inside its data directory on first launch. Drop your images there and they will appear automatically.

Default locations:

| Platform | Path |
|----------|------|
| macOS | `~/Library/Application Support/com.kindergarten.speaking/gallery` |
| Windows | `%APPDATA%\com.kindergarten.speaking\gallery` |

Images should be **landscape orientation** — 16:9 looks best at competition resolution.

---

## Running a Round

1. **Start** — launch the app, it opens in presentation mode.
2. **Draw a Card** — tap the button; the app picks a random image from the gallery (avoiding recent repeats).
3. **Child speaks** — the child taps the microphone button and describes the picture in English; tap again to stop.
4. **Make it** — tap the button to submit the transcript to DashScope for image generation (~10–30 s).
5. **Compare** — the screen splits to show the original card alongside the AI-generated image.
6. **Save / Next Child** — tap **Save** to write a PNG snapshot to disk, then **Next Child** to reset for the next contestant.

The gear icon ⚙ (or **Ctrl+S**) opens operator settings at any time: child/round label, gallery folder, snapshots folder, and fullscreen toggle.

---

## Snapshots

After each round you can save a side-by-side PNG. Snapshots are written to:

- The app data `snapshots` subfolder by default.
- Or a custom folder set in Settings → **Snapshot folder**.

Default locations follow the same pattern as the gallery folder above (replace `gallery` with `snapshots`).

---

## Internet Required

Both the speech-to-text (Alibaba Paraformer) and the image generation (DashScope) are cloud calls. **The venue must have reliable internet access.** The app checks connectivity on launch and shows a warning if the DashScope endpoint is unreachable.

---

## API Key Security (IMPORTANT)

The DashScope API key is embedded (obfuscated) into the binary at **build time** from the `DASHSCOPE_API_KEY` environment variable. It is **never committed to source control**.

Before distributing the app:

1. **Rotate the key** — generate a fresh key in the [Alibaba Cloud / Bailian console](https://bailian.console.aliyun.com/).
2. **Set a spending cap** on the key so runaway requests cannot incur unexpected charges.
3. Keep the key secret — do not share the built binary with untrusted parties.

To build locally with a key:

```bash
# macOS — production build
DASHSCOPE_API_KEY=sk-... npm run tauri build

# macOS — local dev
DASHSCOPE_API_KEY=sk-... npm run tauri dev

# Windows PowerShell — production build
$env:DASHSCOPE_API_KEY="sk-..."; npm run tauri build
```

---

## Enabling the Image Model

`qwen-image-2.0-pro` must be **enabled / subscribed** for your Alibaba account in [Model Studio (Bailian)](https://bailian.console.aliyun.com/) before it can be used.

If you want to switch to a different model, edit the constants at the top of `src-tauri/src/image_gen.rs`:

```rust
const MODEL: &str = "qwen-image-2.0-pro"; // synchronous model (USE_ASYNC = false)
const USE_ASYNC: bool = false;
```

Async models (e.g. `qwen-image` or `wanx2.1-t2i-plus`) require polling for the result — set `USE_ASYNC = true` and update `MODEL` accordingly.

---

## Building Installers via GitHub Actions CI

1. Push this repository to GitHub.
2. Add a repository secret named `DASHSCOPE_API_KEY` (the rotated, spending-capped key) under **Settings → Secrets and variables → Actions**.
3. Push a version tag to trigger a release build:

   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

   Or go to **Actions → Release → Run workflow** to trigger it manually.

4. The workflow builds a macOS universal `.dmg` and a Windows NSIS `.exe` and attaches them to a **draft Release**. Review and publish the draft when ready.

---

## Building the .dmg Locally on macOS

```bash
DASHSCOPE_API_KEY=sk-... npm run tauri build
```

The `.dmg` will be at:

```
src-tauri/target/release/bundle/dmg/Kindergarten Speaking_0.1.0_universal.dmg
```

---

## Developer Quick Start

```bash
npm install
DASHSCOPE_API_KEY=sk-... npm run tauri dev   # hot-reload dev build
cargo build --manifest-path src-tauri/Cargo.toml  # Rust-only check
npm run build                                # frontend-only check
```

Node 20+ and a stable Rust toolchain are required.

---

## Android (APK)

### Getting the APK

The Android APK is built by the `android` job in GitHub Actions — **no local Android SDK is needed**.

- **Tag push**: push a `v*` tag (e.g. `git tag v0.1.0 && git push origin v0.1.0`). The workflow creates a draft release with the APK attached alongside the desktop installers.
- **Manual run**: go to **Actions → Release → Run workflow**. The APK is always uploaded as a run artifact named `android-apk` regardless of whether a release tag is present.

Download the APK from the run's **Artifacts** section (or from the draft release) and sideload it on the device.

> **Sideloading**: The APK is **debug-signed** — suitable for competition testing. On the Android device, enable **Settings → Install unknown apps** for your file manager or browser, then open the downloaded APK to install. For a Play Store / production release you must configure a release keystore; see [Tauri's Android signing docs](https://v2.tauri.app/distribute/sign/android/).

### Gallery on Android

The gallery is **bundled into the APK**: the four sample PNGs in `src-tauri/gallery/` (lion, fish, puppy, sun) are included as app resources at build time.

To use your own competition pictures:

1. Replace the files in `src-tauri/gallery/` with your own PNG/JPG/WebP images (landscape/16:9 recommended).
2. Rebuild (push a tag or trigger the workflow). The new images will be bundled automatically.

This bundled `gallery/` folder is also the **default gallery on desktop** now — the app reads from it first (via the Tauri resource directory) before falling back to the writable app-data folder.

### Known Caveat — Microphone on Android (needs device testing)

The Android manifest is patched by CI to include both `RECORD_AUDIO` and `INTERNET` permissions. However, microphone access from a WebView also requires the WebView to grant `getUserMedia` at the OS level at runtime. If speech recognition does not work on-device:

- The generated `MainActivity` (Kotlin) may need an `onPermissionRequest` override that calls `request.grant(request.resources)` to forward the WebView's permission request to the OS.
- This **can only be verified and fixed on a real device** — it is a pending item that requires a CI-built APK and a physical Android phone or tablet.

Internet access is required on Android for the same reason as desktop (Alibaba Paraformer ASR + DashScope image generation).
