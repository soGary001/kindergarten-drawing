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
        // Decimation ratio: how many input frames per output (16 kHz) sample.
        // e.g. 48000 -> step 3.0, 44100 -> step ~2.756.
        let step = (in_rate / 16_000.0_f32).max(1.0);

        let err_fn = |e: cpal::StreamError| eprintln!("cpal stream error: {e}");

        // Streaming resampler: `acc` is a bounded accumulator that PERSISTS across
        // callbacks. For each input frame we add 1.0; whenever it reaches `step` we
        // emit one output sample and subtract `step`. This keeps `acc` in [0, step)
        // forever, so it works correctly across callback boundaries (the previous
        // version compared a global position against a per-buffer index, which made
        // every callback after the first emit ~nothing -> NO_VALID_AUDIO_ERROR).
        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                let tx2 = tx.clone();
                let mut acc = 0.0_f32;
                let mut buf: Vec<u8> = Vec::with_capacity(3200);
                device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let frames = data.len() / channels.max(1);
                        for frame_idx in 0..frames {
                            acc += 1.0;
                            if acc >= step {
                                acc -= step;
                                // Mono: take channel 0 of the frame.
                                let s = data[frame_idx * channels];
                                let sample = (s.clamp(-1.0, 1.0) * 32_767.0) as i16;
                                let le = sample.to_le_bytes();
                                buf.push(le[0]);
                                buf.push(le[1]);
                                if buf.len() >= 3200 {
                                    let _ = tx2
                                        .send(std::mem::replace(&mut buf, Vec::with_capacity(3200)));
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
                let mut acc = 0.0_f32;
                let mut buf: Vec<u8> = Vec::with_capacity(3200);
                device.build_input_stream(
                    &config.into(),
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let frames = data.len() / channels.max(1);
                        for frame_idx in 0..frames {
                            acc += 1.0;
                            if acc >= step {
                                acc -= step;
                                // Already i16 mono sample; pass through.
                                let sample = data[frame_idx * channels];
                                let le = sample.to_le_bytes();
                                buf.push(le[0]);
                                buf.push(le[1]);
                                if buf.len() >= 3200 {
                                    let _ = tx2
                                        .send(std::mem::replace(&mut buf, Vec::with_capacity(3200)));
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let tx2 = tx.clone();
                let mut acc = 0.0_f32;
                let mut buf: Vec<u8> = Vec::with_capacity(3200);
                device.build_input_stream(
                    &config.into(),
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        let frames = data.len() / channels.max(1);
                        for frame_idx in 0..frames {
                            acc += 1.0;
                            if acc >= step {
                                acc -= step;
                                // u16 -> centered i16.
                                let sample = (data[frame_idx * channels] as i32 - 32_768) as i16;
                                let le = sample.to_le_bytes();
                                buf.push(le[0]);
                                buf.push(le[1]);
                                if buf.len() >= 3200 {
                                    let _ = tx2
                                        .send(std::mem::replace(&mut buf, Vec::with_capacity(3200)));
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
