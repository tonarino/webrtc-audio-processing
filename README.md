# webrtc-audio-processing
[![Crates.io](https://img.shields.io/crates/v/webrtc-audio-processing.svg)](https://crates.io/crates/webrtc-audio-processing)
[![Docs.rs](https://docs.rs/webrtc-audio-processing/badge.svg)](https://docs.rs/webrtc-audio-processing)
[![Build Status](https://travis-ci.org/tonarino/webrtc-audio-processing.svg?branch=master)](https://travis-ci.org/tonarino/webrtc-audio-processing)
[![dependency status](https://deps.rs/repo/github/tonarino/webrtc-audio-processing/status.svg)](https://deps.rs/repo/github/tonarino/webrtc-audio-processing)

## Example Usage

```rust
use webrtc_audio_processing::*;

fn main() {
    let config = InitializationConfig {
        num_capture_channels: 2, // Stereo mic input
        num_render_channels: 2, // Stereo speaker output
        ..InitializationConfig::default()
    };
    let mut ap = Processor::new(&config).unwrap();

    let config = Config {
        echo_cancellation: EchoCancellation {
            enable: true,
            suppression_level: EchoCancellation_SuppressionLevel::HIGH,
        },
        ..Config::default()
    };
    ap.set_config(&config);

    // The render_frame is what is sent to the speakers, and
    // capture_frame is audio captured from a microphone.
    let (render_frame, capture_frame) = sample_stereo_frames();

    let mut render_frame_output = render_frame.clone();
    ap.process_render_frame(&mut render_frame_output).unwrap();

    // render_frame should not have been modified.
    assert_eq!(render_frame, render_frame_output);

    let mut capture_frame_output = capture_frame.clone();
    ap.process_capture_frame(&mut capture_frame_output).unwrap();

    // Echo cancellation should have modified capture_frame.
    assert_ne!(capture_frame, capture_frame_output);

    // capture_frame_output is now ready to send to a remote peer,
    // hopefully with their voice/sounds cancelled out of the stream.
}

fn sample_stereo_frames() -> (Vec<f32>, Vec<f32>) {
    let num_samples_per_frame = NUM_SAMPLES_PER_FRAME as usize;

    // Stereo frame with a lower frequency cosine wave.
    let mut render_frame = Vec::with_capacity(num_samples_per_frame * 2);
    for i in 0..num_samples_per_frame {
        render_frame.push((i as f32 / 40.0).cos() * 0.4);
        render_frame.push((i as f32 / 40.0).cos() * 0.2);
    }

    // Stereo frame with a higher frequency sine wave, mixed with the cosine
    // wave from render frame.
    let mut capture_frame = Vec::with_capacity(num_samples_per_frame * 2);
    for i in 0..num_samples_per_frame {
        capture_frame.push((i as f32 / 20.0).sin() * 0.4 + render_frame[i * 2] * 0.2);
        capture_frame.push((i as f32 / 20.0).sin() * 0.2 + render_frame[i * 2 + 1] * 0.2);
    }

    (render_frame, capture_frame)
}
```

## Dependencies

## Linux

```sh
sudo apt install libwebrtc-audio-processing-dev
```

## MacOS

Build from source?

## Windows

Build from source?

## Build

```sh
cargo build
```

## Test

```sh
cargo test
```
