use webrtc_audio_processing::*;

fn main() {
    let config = InitializationConfig {
        num_capture_channels: 2, // Stereo mic input
        num_render_channels: 2,  // Stereo speaker output
        ..InitializationConfig::default()
    };

    let mut ap = Processor::new(&config).unwrap();

    let config = Config {
        echo_cancellation: Some(EchoCancellation {
            suppression_level: EchoCancellationSuppressionLevel::High,
            enable_delay_agnostic: false,
            enable_extended_filter: false,
            stream_delay_ms: None,
        }),
        ..Config::default()
    };
    ap.set_config(config);

    // The render_frame is what is sent to the speakers, and
    // capture_frame is audio captured from a microphone.
    let (render_frame, capture_frame) = sample_stereo_frames();

    let mut render_frame_output = render_frame.clone();
    ap.process_render_frame(&mut render_frame_output).unwrap();

    assert_eq!(render_frame, render_frame_output, "render_frame should not be modified.");

    let mut capture_frame_output = capture_frame.clone();
    ap.process_capture_frame(&mut capture_frame_output).unwrap();

    assert_ne!(
        capture_frame, capture_frame_output,
        "Echo cancellation should have modified capture_frame."
    );

    // capture_frame_output is now ready to send to a remote peer.
    println!("Successfully processed a render and capture frame through WebRTC!");
}

/// Generate example stereo frames that simulates a situation where the
/// microphone (capture) would be picking up the speaker (render) output.
fn sample_stereo_frames() -> (Vec<f32>, Vec<f32>) {
    let num_samples_per_frame = NUM_SAMPLES_PER_FRAME as usize;

    let mut render_frame = Vec::with_capacity(num_samples_per_frame * 2);
    let mut capture_frame = Vec::with_capacity(num_samples_per_frame * 2);
    for i in 0..num_samples_per_frame {
        render_frame.push((i as f32 / 40.0).cos() * 0.4);
        render_frame.push((i as f32 / 40.0).cos() * 0.2);
        capture_frame.push((i as f32 / 20.0).sin() * 0.4 + render_frame[i * 2] * 0.2);
        capture_frame.push((i as f32 / 20.0).sin() * 0.2 + render_frame[i * 2 + 1] * 0.2);
    }

    (render_frame, capture_frame)
}
