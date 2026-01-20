// This example loops the microphone input back to the speakers, while applying echo cancellation,
// creating an experience similar to Karaoke microphones. It uses PortAudio as an interface to the
// underlying audio devices.
// Optionally, a config file can be provided to override the default settings
use anyhow::Error;
use serde::Deserialize;
use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use structopt::StructOpt;
use webrtc_audio_processing::*;

// The highest sample rate that webrtc-audio-processing supports.
const SAMPLE_RATE: f64 = 48_000.0;

// webrtc-audio-processing expects a 10ms chunk for each process call.
const FRAMES_PER_BUFFER: u32 = 480;

#[derive(Debug, StructOpt)]
struct Args {
    #[structopt(short, long, default_value = "examples/aec-configs/config.json5")]
    config_file: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct AppConfig {
    num_capture_channels: usize,
    num_render_channels: usize,
    #[serde(default)]
    config: Config,
    #[serde(default)]
    aec3: EchoCanceller3Config,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            num_capture_channels: 1,
            num_render_channels: 1,
            config: Config::default(),
            aec3: EchoCanceller3Config::default(),
        }
    }
}

impl AppConfig {
    fn from_file_or_defaults(path: &PathBuf) -> Result<Self, Error> {
        // Load custom config if it exists, otherwise use defaults
        if path.exists() {
            let content = fs::read_to_string(path)?;
            Ok(json5::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }
}

fn create_processor(config: &AppConfig) -> Result<Processor, Error> {
    let mut processor = Processor::with_aec3_config(
        &InitializationConfig {
            num_capture_channels: config.num_capture_channels,
            num_render_channels: config.num_render_channels,
            sample_rate_hz: SAMPLE_RATE as u32,
        },
        Some(config.aec3.clone()),
    )?;

    processor.set_config(config.config.clone());
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
    let args = Args::from_args();
    let config = AppConfig::from_file_or_defaults(&args.config_file)?;

    assert_eq!(config.num_capture_channels, 1, "Capture channels must be 1");
    assert!(
        config.num_render_channels == 1 || config.num_render_channels == 2,
        "Render channels must be 1 or 2"
    );

    let mut processor = create_processor(&config)?;

    let pa = portaudio::PortAudio::new()?;

    let stream_settings = pa.default_duplex_stream_settings(
        config.num_capture_channels as i32,
        config.num_render_channels as i32,
        SAMPLE_RATE,
        FRAMES_PER_BUFFER,
    )?;

    // Memory allocation should not happen inside the audio loop
    let mut processed = vec![0f32; FRAMES_PER_BUFFER as usize * config.num_capture_channels];
    let mut interleave_buffer = vec![0f32; FRAMES_PER_BUFFER as usize * config.num_render_channels];
    let output_channels = config.num_render_channels;

    let mut stream = pa.open_non_blocking_stream(
        stream_settings,
        move |portaudio::DuplexStreamCallbackArgs { in_buffer, out_buffer, frames, .. }| {
            assert_eq!(frames as u32, FRAMES_PER_BUFFER);

            processed.copy_from_slice(in_buffer);
            processor.process_capture_frame(&mut processed).unwrap();

            // Play back the processed audio capture.
            out_buffer.copy_from_slice(&processed);
            processor.process_render_frame(out_buffer).unwrap();
            // Handle mono to mono/stereo conversion (assuming stereo output)
            if output_channels == 1 {
                out_buffer.copy_from_slice(&processed);
            } else {
                for i in 0..frames {
                    interleave_buffer[i * 2] = processed[i];
                    interleave_buffer[i * 2 + 1] = processed[i];
                }
                out_buffer.copy_from_slice(&interleave_buffer);
            }

            portaudio::Continue
        },
    )?;

    stream.start()?;

    wait_ctrlc()?;

    Ok(())
}
