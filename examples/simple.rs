use webrtc_audio_processing::*;
use webrtc_audio_processing_config::{Config, EchoCanceller};

fn main() {
    let config = InitializationConfig {
        num_capture_channels: 2, // Stereo mic input
        num_render_channels: 2,  // Stereo speaker output
        sample_rate_hz: 48_000,  // The maximum processing rate
    };

    let mut ap = Processor::new(&config).unwrap();

    let config = Config { echo_canceller: Some(EchoCanceller::default()), ..Default::default() };
    ap.set_config(config);

    // The render_frame is what is sent to the speakers, and
    // capture_frame is audio captured from a microphone.
    let (render_frame, capture_frame) = sample_stereo_frames(&ap);

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
fn sample_stereo_frames(processor: &Processor) -> (Vec<Vec<f32>>, Vec<Vec<f32>>) {
    let num_samples_per_frame = processor.num_samples_per_frame();

    let mut render_frame = vec![vec![]; 2];
    let mut capture_frame = vec![vec![]; 2];
    for i in 0..num_samples_per_frame {
        render_frame[0].push((i as f32 / 40.0).cos() * 0.4);
        render_frame[1].push((i as f32 / 40.0).cos() * 0.2);
        capture_frame[0].push((i as f32 / 20.0).sin() * 0.4 + render_frame[0][i] * 0.2);
        capture_frame[1].push((i as f32 / 20.0).sin() * 0.2 + render_frame[1][i] * 0.2);
    }

    (render_frame, capture_frame)
}
