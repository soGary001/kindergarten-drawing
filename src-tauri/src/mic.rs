use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::mpsc::UnboundedSender;

pub struct MicHandle {
    stop: Arc<AtomicBool>,
}

impl MicHandle {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// Start capturing the default input device on a dedicated thread.
/// Converts to 16 kHz mono i16 LE and pushes ~3200-byte chunks (~100 ms @ 16 kHz) to `tx`.
pub fn start_capture(tx: UnboundedSender<Vec<u8>>) -> Result<MicHandle, String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();

    // Probe the device/config on this thread so errors surface synchronously.
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or("no default input device")?;
    let config = device
        .default_input_config()
        .map_err(|e| format!("input config: {e}"))?;

    let in_rate = config.sample_rate().0 as f32;
    let channels = config.channels() as usize;
    let sample_format = config.sample_format();

    std::thread::spawn(move || {
        // step > 1 means we skip input samples; < 1 means we duplicate (shouldn't happen
        // for any normal device since mic rates are >= 16 kHz, but handled correctly).
        let step = in_rate / 16_000.0_f32;

        let err_fn = |e: cpal::StreamError| eprintln!("cpal stream error: {e}");

        // Helper that both closures call.  We can't share one closure across the `match`
        // arms because they capture different data types, so we replicate the logic.
        //
        // Each arm captures its own `pos: f32`, `buf: Vec<u8>`, and `tx` clone.
        // pos tracks the fractional position in the INPUT stream:
        //   - a sample at input frame `i` represents time `i / in_rate`.
        //   - we emit output sample `n` when `pos` crosses `n` (integer boundary at output
        //     sample index n, i.e. time n/16000).  After emitting we advance the "next
        //     expected" output index by 1.
        //   - Equivalently: keep `next_out: f32 = 0.0`; after processing each input
        //     frame `i`, if `i >= next_out` emit that frame's sample and set
        //     `next_out += step`.

        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                let tx2 = tx.clone();
                let mut pos = 0.0_f32; // next output boundary in input-sample units
                let mut buf: Vec<u8> = Vec::with_capacity(3200);
                device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let frames = data.len() / channels;
                        for frame_idx in 0..frames {
                            let input_pos = frame_idx as f32;
                            if input_pos >= pos {
                                // Mono: take channel 0
                                let s = data[frame_idx * channels];
                                let sample = (s.clamp(-1.0, 1.0) * 32_767.0) as i16;
                                let le = sample.to_le_bytes();
                                buf.push(le[0]);
                                buf.push(le[1]);
                                pos += step;
                                if buf.len() >= 3200 {
                                    let _ = tx2.send(std::mem::replace(
                                        &mut buf,
                                        Vec::with_capacity(3200),
                                    ));
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let tx2 = tx.clone();
                let mut pos = 0.0_f32;
                let mut buf: Vec<u8> = Vec::with_capacity(3200);
                device.build_input_stream(
                    &config.into(),
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let frames = data.len() / channels;
                        for frame_idx in 0..frames {
                            let input_pos = frame_idx as f32;
                            if input_pos >= pos {
                                let s = data[frame_idx * channels] as f32 / 32_768.0;
                                let sample = (s.clamp(-1.0, 1.0) * 32_767.0) as i16;
                                let le = sample.to_le_bytes();
                                buf.push(le[0]);
                                buf.push(le[1]);
                                pos += step;
                                if buf.len() >= 3200 {
                                    let _ = tx2.send(std::mem::replace(
                                        &mut buf,
                                        Vec::with_capacity(3200),
                                    ));
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
            }
            other => {
                eprintln!("mic: unsupported sample format {other:?}");
                return;
            }
        };

        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("mic: build_input_stream failed: {e}");
                return;
            }
        };

        if let Err(e) = stream.play() {
            eprintln!("mic: stream.play() failed: {e}");
            return;
        }

        // Keep the stream alive until the stop flag is set.
        while !stop_thread.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        drop(stream);
    });

    Ok(MicHandle { stop })
}
