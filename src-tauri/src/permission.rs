// Request microphone permission on macOS via AVFoundation BEFORE starting audio capture.
// Without this, an ad-hoc-signed app prompts repeatedly and cpal reads an empty buffer
// because capture starts before the user has granted access.

#[cfg(target_os = "macos")]
pub fn ensure_mic() -> Result<(), String> {
    use block2::RcBlock;
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, Bool};
    use objc2_foundation::NSString;
    use std::sync::mpsc;
    use std::time::Duration;

    unsafe {
        let cls = AnyClass::get(c"AVCaptureDevice").ok_or("AVCaptureDevice unavailable")?;
        // AVMediaTypeAudio == @"soun"
        let media = NSString::from_str("soun");
        // AVAuthorizationStatus: 0 NotDetermined, 1 Restricted, 2 Denied, 3 Authorized
        let status: isize = msg_send![cls, authorizationStatusForMediaType: &*media];
        match status {
            3 => Ok(()),
            0 => {
                let (tx, rx) = mpsc::channel::<bool>();
                let handler = RcBlock::new(move |granted: Bool| {
                    let _ = tx.send(granted.as_bool());
                });
                let _: () = msg_send![
                    cls,
                    requestAccessForMediaType: &*media,
                    completionHandler: &*handler,
                ];
                match rx.recv_timeout(Duration::from_secs(120)) {
                    Ok(true) => Ok(()),
                    Ok(false) => Err("麦克风权限被拒绝 / microphone permission denied".into()),
                    Err(_) => Err("等待麦克风授权超时 / microphone permission timed out".into()),
                }
            }
            _ => Err(
                "麦克风被禁用：请到 系统设置 → 隐私与安全性 → 麦克风 里打开本应用 / \
                 Microphone disabled — enable this app in System Settings → Privacy → Microphone"
                    .into(),
            ),
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn ensure_mic() -> Result<(), String> {
    Ok(())
}
