// This example loops the microphone input back to the speakers, while applying echo cancellation,
// creating an experience similar to Karaoke microphones. It uses PortAudio as an interface to the
// underlying audio devices.
use anyhow::Error;
use ctrlc;
use portaudio;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use webrtc_audio_processing::*;

// The highest sample rate that webrtc-audio-processing supports.
const SAMPLE_RATE: f64 = 48_000.0;

// webrtc-audio-processing expects a 10ms chunk for each process call.
const FRAMES_PER_BUFFER: u32 = 480;

fn create_processor(
    num_capture_channels: usize,
    num_render_channels: usize,
) -> Result<Processor, Error> {
    let mut processor = Processor::new(&InitializationConfig {
        num_capture_channels,
        num_render_channels,
        sample_rate_hz: SAMPLE_RATE as u32,
    })?;

    // The default AEC configuration enables HPF, too.
    let config = Config { echo_canceller: Some(EchoCanceller::default()), ..Config::default() };
    processor.set_config(config);

    Ok(processor)
}

fn wait_ctrlc() -> Result<(), Error> {
    let running = Arc::new(AtomicBool::new(true));

    ctrlc::set_handler({
        let running = running.clone();
        move || {
            running.store(false, Ordering::SeqCst);
        }
    })?;

    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(10));
    }

    Ok(())
}

fn main() -> Result<(), Error> {
    // Monoral microphone.
    let input_channels = 1;
    // Monoral speaker.
    let output_channels = 1;

    let mut processor = create_processor(input_channels, output_channels)?;

    let pa = portaudio::PortAudio::new()?;

    let stream_settings = pa.default_duplex_stream_settings(
        input_channels as i32,
        output_channels as i32,
        SAMPLE_RATE,
        FRAMES_PER_BUFFER,
    )?;

    // Memory allocation should not happen inside the audio loop.
    let mut processed = vec![0f32; FRAMES_PER_BUFFER as usize * input_channels as usize];

    let mut stream = pa.open_non_blocking_stream(
        stream_settings,
        move |portaudio::DuplexStreamCallbackArgs { in_buffer, mut out_buffer, frames, .. }| {
            assert_eq!(frames as u32, FRAMES_PER_BUFFER);

            processed.copy_from_slice(&in_buffer);
            processor.process_capture_frame(&mut processed).unwrap();

            // Play back the processed audio capture.
            out_buffer.copy_from_slice(&processed);
            processor.process_render_frame(&mut out_buffer).unwrap();

            portaudio::Continue
        },
    )?;

    stream.start()?;

    wait_ctrlc()?;

    Ok(())
}
