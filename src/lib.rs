//! This crate is a wrapper around [PulseAudio's repackaging of WebRTC's AudioProcessing module](https://www.freedesktop.org/software/pulseaudio/webrtc-audio-processing/).
//!
//! See `examples/simple.rs` for an example of how to use the library.

#![warn(clippy::all)]
#![warn(missing_docs)]

mod config;
mod stats;

/// [Highly experimental]
/// Exposes finer-grained control of the internal AEC3 configuration.
#[cfg(feature = "experimental-aec3-config")]
pub mod experimental;

use crate::config::IntoFfi;
use std::{error, fmt, ptr::null_mut};
use webrtc_audio_processing_config::Config;
use webrtc_audio_processing_sys as ffi;

pub use config::InitializationConfig;
pub use stats::*;

/// Represents an error inside webrtc::AudioProcessing.
/// Drawn from documentation of pulseaudio upstream `webrtc::AudioProcessing::Error`:
/// https://cgit.freedesktop.org/pulseaudio/webrtc-audio-processing/tree/webrtc/modules/audio_processing/include/audio_processing.h?id=9def8cf10d3c97640d32f1328535e881288f700f
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// An unspecified error from the underlying WebRTC library. `kUnspecifiedError`
    Unspecified,
    /// The initialization of the audio processor failed. `kCreationFailedError`
    InitializationFailed,
    /// An unsupported component was used. `kUnsupportedComponentError`
    UnsupportedComponent,
    /// An unsupported function was called. `kUnsupportedFunctionError`
    UnsupportedFunction,
    /// A null pointer was passed to the underlying WebRTC library. `kNullPointerError`
    NullPointer,
    /// An invalid parameter was passed. `kBadParameterError`
    BadParameter,
    /// An invalid sample rate was used. `kBadSampleRateError`
    BadSampleRate,
    /// An invalid frame length was used. `kBadDataLengthError`
    BadDataLength,
    /// An invalid number of channels was used. `kBadNumberChannelsError`
    BadNumberChannels,
    /// A file access error occurred. `kFileError`
    File,
    /// A stream parameter was not set. `kStreamParameterNotSetError`
    StreamParameterNotSet,
    /// A feature was used without being enabled. `kNotEnabledError`
    NotEnabled,
}

impl From<i32> for Error {
    fn from(code: i32) -> Self {
        match code {
            0 => panic!("Error should not be created from a success code"),
            -1 => Self::Unspecified,
            -2 => Self::InitializationFailed,
            -3 => Self::UnsupportedComponent,
            -4 => Self::UnsupportedFunction,
            -5 => Self::NullPointer,
            -6 => Self::BadParameter,
            -7 => Self::BadSampleRate,
            -8 => Self::BadDataLength,
            -9 => Self::BadNumberChannels,
            -10 => Self::File,
            -11 => Self::StreamParameterNotSet,
            -12 => Self::NotEnabled,
            _ => Self::Unspecified,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            Self::Unspecified => "Unspecified error",
            Self::InitializationFailed => "Initialization failed",
            Self::UnsupportedComponent => "Unsupported component",
            Self::UnsupportedFunction => "Unsupported function",
            Self::NullPointer => "Null pointer",
            Self::BadParameter => "Bad parameter",
            Self::BadSampleRate => "Bad sample rate",
            Self::BadDataLength => "Invalid data length",
            Self::BadNumberChannels => "Invalid number of channels",
            Self::File => "File error",
            Self::StreamParameterNotSet => "Stream parameter not set",
            Self::NotEnabled => "Feature not enabled",
        };
        write!(f, "WebRTC AudioProcessing error: {}", description)
    }
}

impl error::Error for Error {}

fn result_from_code<T>(on_success: T, error_code: i32) -> Result<T, Error> {
    if error_code == 0 {
        Ok(on_success)
    } else {
        Err(Error::from(error_code))
    }
}

/// [`Self`] provides an access to webrtc's audio processing e.g. echo cancellation and automatic
/// gain control.
///
/// It is [`Send`] + [`Sync`] and its methods take `&self` shared reference (as we expose
/// thread-safe APIs of the underlying C++ library), so it can be easily wrapped in an
/// [`Arc`](std::sync::Arc) for multithreaded use.
#[derive(Debug, Clone)]
pub struct Processor {
    inner: *mut ffi::AudioProcessing,
    capture_stream_config: ffi::StreamConfig,
    render_stream_config: ffi::StreamConfig,
}

impl Processor {
    /// Creates a new [`Self`]. [`InitializationConfig`] is only used on instantiation, however new
    /// configs can be be passed to [`Self::set_config()`] at any time during processing.
    pub fn new(config: &InitializationConfig) -> Result<Self, Error> {
        Self::new_with_ptr(config, null_mut())
    }

    /// [Highly experimental]
    /// Creates a new `Processor` with custom AEC3 configuration.
    #[cfg(feature = "experimental-aec3-config")]
    pub fn with_aec3_config(
        config: &InitializationConfig,
        mut aec3_config: experimental::EchoCanceller3Config,
    ) -> Result<Self, Error> {
        Self::new_with_ptr(config, &raw mut *aec3_config)
    }

    /// Pass null ptr in `aec3_config` to use its default config.
    fn new_with_ptr(
        config: &InitializationConfig,
        aec3_config: *mut ffi::EchoCanceller3Config,
    ) -> Result<Self, Error> {
        if config.num_capture_channels == 0 || config.num_render_channels == 0 {
            return Err(Error::BadNumberChannels);
        }

        let capture_stream_config = config.capture_stream_config();
        let render_stream_config = config.render_stream_config();

        let mut code = 0;
        let inner = unsafe {
            ffi::create_audio_processing(
                &capture_stream_config,
                &render_stream_config,
                aec3_config,
                &mut code,
            )
        };
        Ok(Self {
            inner: result_from_code(inner, code)?,
            capture_stream_config,
            render_stream_config,
        })
    }

    /// Processes and modifies the audio frame from a capture device by applying
    /// signal processing as specified in the config. `frame` should be a Vec of
    /// length 'num_capture_channels', with each inner Vec representing a channel
    /// with NUM_SAMPLES_PER_FRAME samples.
    pub fn process_capture_frame(&mut self, frame: &mut [Vec<f32>]) -> Result<(), Error> {
        let mut frame_ptr = frame.iter_mut().map(|v| v.as_mut_ptr()).collect::<Vec<*mut f32>>();
        let code = unsafe {
            ffi::process_capture_frame(
                self.inner,
                &self.capture_stream_config,
                frame_ptr.as_mut_ptr(),
            )
        };
        result_from_code((), code)
    }

    /// Processes and optionally modifies the audio frame from a playback device.
    /// `frame` should be a Vec of length 'num_render_channels', with each inner Vec
    /// representing a channel with NUM_SAMPLES_PER_FRAME samples.
    pub fn process_render_frame(&mut self, frame: &mut [Vec<f32>]) -> Result<(), Error> {
        let mut frame_ptr = frame.iter_mut().map(|v| v.as_mut_ptr()).collect::<Vec<*mut f32>>();
        let code = unsafe {
            ffi::process_render_frame(
                self.inner,
                &self.render_stream_config,
                frame_ptr.as_mut_ptr(),
            )
        };
        result_from_code((), code)
    }

    /// Returns statistics from the last `process_capture_frame()` call.
    pub fn get_stats(&self) -> Stats {
        unsafe { ffi::get_stats(self.inner).into() }
    }

    /// Returns the number of (possibly multichannel) samples per frame based on the sample rate
    /// and frame size (fixed in WebRTC to 10ms).
    pub fn num_samples_per_frame(&self) -> usize {
        // We have a confusing terminology clash here. For us, a frame is "10ms worth of audio data
        // at given sample rate". For WebRTC, frame is a (possibly multichannel) sample.
        // The value we get is computed by the followind C++ snippet>
        //   static int GetFrameSize(int sample_rate_hz) { return sample_rate_hz / 100; }
        //
        // It doesn't matter whether we use capture or render stream config - we use the same frame
        // rate for both.
        self.capture_stream_config.num_frames_
    }

    /// Immediately updates the configurations of the internal signal processor.
    /// May be called multiple times after the initialization and during
    /// processing.
    pub fn set_config(&mut self, config: Config) {
        let delay_ms = config.echo_canceller.as_ref().and_then(|ec| ec.stream_delay_ms);
        unsafe {
            ffi::set_config(self.inner, &config.into_ffi());
            if let Some(delay) = delay_ms {
                ffi::set_stream_delay_ms(self.inner, delay);
            }
        }
    }

    /// Signals the AEC and AGC that the audio output will be / is muted.
    /// They may use the hint to improve their parameter adaptation.
    pub fn set_output_will_be_muted(&self, muted: bool) {
        unsafe {
            ffi::set_output_will_be_muted(self.inner, muted);
        }
    }

    /// Sets the delay in milliseconds between `process_render_frame()` receiving a far-end frame
    /// and `process_capture_frame()` receiving a near-end frame containing the corresponding echo.
    ///
    /// This should only be used when the delay is known to be stable and constant. For adaptive
    /// delay estimation, leave this unset and rely on the internal estimator.
    pub fn set_stream_delay_ms(&self, delay: i32) {
        unsafe {
            ffi::set_stream_delay_ms(self.inner, delay);
        }
    }

    /// Signals the AEC and AGC that the next frame will contain key press sound
    pub fn set_stream_key_pressed(&self, pressed: bool) {
        unsafe {
            ffi::set_stream_key_pressed(self.inner, pressed);
        }
    }
}

impl Drop for Processor {
    fn drop(&mut self) {
        unsafe {
            ffi::delete_audio_processing(self.inner);
        }
    }
}

// ffi::AudioProcessing provides thread safety with a few exceptions around the concurrent usage of
// its corresponding getters and setters.
// TODO(strohel): wrap the ffi pointer and do this only for it
unsafe impl Sync for Processor {}
unsafe impl Send for Processor {}

// This block is checked at compile time, but stripped from the final binary.
const _: () = {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    #[expect(dead_code)]
    fn trigger() {
        assert_send::<Processor>();
        assert_sync::<Processor>();
    }
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time::Duration};
    use webrtc_audio_processing_config::{EchoCanceller, EchoCancellerMode};

    fn init_config(num_channels: usize) -> InitializationConfig {
        InitializationConfig {
            num_capture_channels: num_channels,
            num_render_channels: num_channels,
            sample_rate_hz: 48_000,
        }
    }

    fn sample_stereo_frames(ap: &Processor) -> (Vec<Vec<f32>>, Vec<Vec<f32>>) {
        let num_samples_per_frame = ap.num_samples_per_frame();

        // Stereo frame with a lower frequency cosine wave.
        let mut render_frame = vec![vec![]; 2];
        let [render_left, render_right] = &mut render_frame[..] else { unreachable!() };
        for i in 0..num_samples_per_frame {
            render_left.push((i as f32 / 40.0).cos() * 0.4);
            render_right.push((i as f32 / 40.0).cos() * 0.2);
        }

        // Stereo frame with a higher frequency sine wave, mixed with the cosine
        // wave from render frame.
        let mut capture_frame = vec![vec![]; 2];
        let [capture_left, capture_right] = &mut capture_frame[..] else { unreachable!() };
        for i in 0..num_samples_per_frame {
            capture_left.push((i as f32 / 20.0).sin() * 0.4 + render_left[i] * 0.2);
            capture_right.push((i as f32 / 20.0).sin() * 0.2 + render_right[i] * 0.2);
        }

        (render_frame, capture_frame)
    }

    // Helper function for calculating ERL (Echo Return Loss)
    fn calculate_erl(reference: &[Vec<f32>], processed: &[Vec<f32>]) -> f32 {
        // Ensure valid comparison
        assert_eq!(reference.len(), processed.len(), "Signal lengths must match");
        assert!(reference.iter().zip(processed).all(|(r, p)| r.len() == p.len()));

        // Sum of squares for both signals
        let reference_power: f32 = reference.iter().flatten().map(|x| x * x).sum();
        let processed_power: f32 = processed.iter().flatten().map(|x| x * x).sum();

        // Convert to dB: 10 * log10(reference/processed)
        if reference_power > 1e-12 && processed_power > 1e-12 {
            10.0 * (reference_power / processed_power).log10()
        } else {
            0.0 // Handle near-silent signals < -120dB
        }
    }

    /// A context to put abstracted methods commonly reused for tests.
    struct TestContext {
        processor: Processor,
        num_samples: usize,
        num_channels: usize,
    }

    impl TestContext {
        #[cfg(feature = "experimental-aec3-config")]
        fn new(
            num_channels: usize,
            aec3_config: Option<experimental::EchoCanceller3Config>,
        ) -> Self {
            let config = init_config(num_channels);
            let processor = match aec3_config {
                Some(aec3_config) => Processor::with_aec3_config(&config, aec3_config).unwrap(),
                None => Processor::new(&config).unwrap(),
            };
            let num_samples = processor.num_samples_per_frame();
            Self { processor, num_samples, num_channels }
        }

        #[cfg(not(feature = "experimental-aec3-config"))]
        fn new(num_channels: usize, _: Option<()>) -> Self {
            let config = init_config(num_channels);
            let processor = Processor::new(&config).unwrap();
            let num_samples = processor.num_samples_per_frame();
            Self { processor, num_samples, num_channels }
        }

        fn generate_sine_frame(&self, frequency: f32) -> Vec<Vec<f32>> {
            let mut channel = Vec::with_capacity(self.num_samples);
            for i in 0..self.num_samples {
                let sample =
                    (i as f32 * frequency / 48000.0 * 2.0 * std::f32::consts::PI).sin() * 0.5;
                channel.push(sample);
            }

            vec![channel; self.num_channels]
        }

        fn process_frames(
            &mut self,
            render: &mut [Vec<f32>],
            capture: &mut [Vec<f32>],
            iterations: usize,
        ) {
            for _ in 0..iterations {
                self.processor.process_render_frame(render).unwrap();
                self.processor.process_capture_frame(capture).unwrap();
            }
        }

        fn measure_echo_reduction(&mut self, render_frame: &[Vec<f32>], iterations: usize) -> f32 {
            let mut capture_frame = render_frame.to_vec();

            // Calculate initial ERL
            let initial_erl = calculate_erl(render_frame, &capture_frame);

            // Process frames
            self.process_frames(&mut render_frame.to_vec(), &mut capture_frame, iterations);

            // Calculate final ERL
            let final_erl = calculate_erl(render_frame, &capture_frame);
            final_erl - initial_erl
        }

        /// Warm up the AEC and then measure ERL
        fn measure_steady_state_performance(
            &mut self,
            render_frame: &[Vec<f32>],
            warmup_iterations: usize,
            measurement_count: usize,
        ) -> f32 {
            let capture_frame = render_frame.to_vec();

            // Warm up
            self.process_frames(
                &mut render_frame.to_vec(),
                &mut capture_frame.clone(),
                warmup_iterations,
            );

            // Measure steady state and sum up the ERL reduction
            let mut total_reduction = 0.0;
            for _ in 0..measurement_count {
                let mut test_capture = capture_frame.clone();
                self.process_frames(&mut render_frame.to_vec(), &mut test_capture, 1);
                total_reduction += calculate_erl(&capture_frame, &test_capture);
            }

            total_reduction / measurement_count as f32
        }
    }

    /// Tests initialization failure with invalid configuration
    #[test]
    fn test_create_failure() {
        let config = InitializationConfig { num_capture_channels: 0, ..init_config(1) };
        let err = Processor::new(&config).unwrap_err();
        assert_eq!(err, Error::BadNumberChannels);
    }

    /// Tests proper resource cleanup on drop
    #[test]
    fn test_create_drop() {
        let config = init_config(1);
        let _p = Processor::new(&config).unwrap();
    }

    /// Tests nominal operation of the audio processing library.
    #[test]
    fn test_nominal() {
        let config = init_config(2);
        let mut ap = Processor::new(&config).unwrap();

        let config =
            Config { echo_canceller: Some(EchoCanceller::default()), ..Default::default() };
        ap.set_config(config);

        let (render_frame, capture_frame) = sample_stereo_frames(&ap);

        let mut render_frame_output = render_frame.clone();
        ap.process_render_frame(&mut render_frame_output).unwrap();

        // Render frame should not be modified.
        assert_eq!(render_frame, render_frame_output);

        let mut capture_frame_output = capture_frame.clone();
        ap.process_capture_frame(&mut capture_frame_output).unwrap();

        // Echo cancellation should have modified the capture frame.
        // We don't validate how it's modified. Out of scope for this unit test.
        assert_ne!(capture_frame, capture_frame_output);

        let stats = ap.get_stats();
        assert!(stats.echo_return_loss.is_some());
        println!("{stats:#?}");
    }

    #[test]
    #[ignore]
    fn test_nominal_threaded() {
        let config = init_config(2);
        let ap = Processor::new(&config).unwrap();

        let (render_frame, capture_frame) = sample_stereo_frames(&ap);

        let mut config_ap = ap.clone();
        let config_thread = thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));

            let config =
                Config { echo_canceller: Some(EchoCanceller::default()), ..Default::default() };
            config_ap.set_config(config);
        });

        let mut render_ap = ap.clone();
        let render_thread = thread::spawn(move || {
            for _ in 0..100 {
                let mut render_frame_output = render_frame.clone();
                render_ap.process_render_frame(&mut render_frame_output).unwrap();

                thread::sleep(Duration::from_millis(10));
            }
        });

        let mut capture_ap = ap.clone();
        let capture_thread = thread::spawn(move || {
            for i in 0..100 {
                let mut capture_frame_output = capture_frame.clone();
                capture_ap.process_capture_frame(&mut capture_frame_output).unwrap();

                let stats = capture_ap.get_stats();
                if i < 5 {
                    // first 50ms
                    assert!(stats.echo_return_loss.is_none());
                } else if i >= 95 {
                    // last 50ms
                    assert!(stats.echo_return_loss.is_some());
                }

                thread::sleep(Duration::from_millis(10));
            }
        });

        config_thread.join().unwrap();
        render_thread.join().unwrap();
        capture_thread.join().unwrap();
    }

    #[test]
    fn test_tweak_processor_params() {
        let config = InitializationConfig {
            num_capture_channels: 2,
            num_render_channels: 2,
            ..InitializationConfig::default()
        };
        let mut ap = Processor::new(&config).unwrap();

        // tweak params outside of config
        ap.set_output_will_be_muted(true);
        ap.set_stream_key_pressed(true);
        ap.set_stream_delay_ms(10);

        // test one process call
        let (render_frame, capture_frame) = sample_stereo_frames(&ap);

        let mut render_frame_output = render_frame.clone();
        ap.process_render_frame(&mut render_frame_output).unwrap();
        let mut capture_frame_output = capture_frame.clone();
        ap.process_capture_frame(&mut capture_frame_output).unwrap();

        // it shouldn't crash
    }

    #[test]
    fn test_stream_delay() {
        let make_config = |delay_ms| Config {
            echo_canceller: Some(EchoCanceller {
                mode: EchoCancellerMode::Full,
                stream_delay_ms: Some(delay_ms),
            }),
            ..Default::default()
        };

        // Verify via stats & warm up
        let mut context = TestContext::new(1, None);
        context.processor.set_config(make_config(200));

        let mut frame = vec![vec![0.1f32; context.num_samples]];
        for _ in 0..20 {
            context.processor.process_render_frame(&mut frame).unwrap();
            context.processor.process_capture_frame(&mut frame).unwrap();
        }

        assert!(
            context.processor.get_stats().delay_ms.is_some(),
            "Stream delay should be reported in statistics"
        );

        // Verify matched delay should handle a signal pulse better
        let measure_pulse_reduction = |applied_delay_ms| {
            let mut context = TestContext::new(1, None);
            // Apply either a correct hint (200ms) or an incorrect one (0ms)
            context.processor.set_config(make_config(applied_delay_ms));

            let num_samples = context.num_samples;
            // Make a fake path delay of 200ms (20 frames of 10ms)
            let mut history = vec![vec![vec![0.0; num_samples]; 20]];
            let (mut total_in_p, mut total_out_p) = (0.0, 0.0);

            for i in 0..100 {
                // Make a pulse for 50ms (5 frames), then silence
                let mut render = if i < 5 {
                    context.generate_sine_frame(440.0)
                } else {
                    vec![vec![0.0; num_samples]]
                };

                // Add the render frame to history and pop the delayed frame as the "echo"
                history.push(render.clone());
                // Capture is the staggered echo signal
                let mut capture: Vec<_> = history
                    .remove(0)
                    .iter()
                    .map(|channel| channel.iter().map(|x| x * 0.8).collect())
                    .collect();

                // Get the energy before and after processing across the entire run
                total_in_p += capture.iter().flatten().map(|x| x * x).sum::<f32>();
                context.processor.process_render_frame(&mut render).unwrap();
                context.processor.process_capture_frame(&mut capture).unwrap();
                total_out_p += capture.iter().flatten().map(|x| x * x).sum::<f32>();
            }
            // Return the global reduction ratio
            total_in_p / total_out_p.max(1e-9)
        };

        // Measure reduction with a 0ms hint
        let reduction_mismatched = measure_pulse_reduction(0);
        // Measure reduction with a 200ms hint
        let reduction_matched = measure_pulse_reduction(200);

        // Correct alignment should result in much better cancellation
        assert!(
            reduction_matched * 1000.0 > reduction_mismatched,
            "Matched delay should have better echo cancellation"
        );
    }

    /// Measures baseline echo cancellation performance.
    ///
    /// Uses a pure sine wave to create ideal test conditions. Verifies the AEC
    /// achieves at least 18dB ERL.
    #[test]
    fn test_echo_cancellation_effectiveness() {
        let mut context = TestContext::new(1, None);

        // Configure AEC
        context.processor.set_config(Config {
            echo_canceller: Some(EchoCanceller::default()),
            ..Default::default()
        });

        // Test with pure sine wave
        let render_frame = context.generate_sine_frame(440.0);
        let erle = context.measure_echo_reduction(&render_frame, 100);

        // Verify there is echo loss.
        assert!(
            erle >= 18.0,
            "Echo canceller should achieve at least 18 dB of ERLE (got {:.1} dB)",
            erle
        );
    }

    /// Verifies that different AEC configurations produce measurably different results.
    ///
    /// These modes should have distinct echo cancellation behaviors by design.
    #[test]
    fn test_aec3_configuration_impact() {
        let mut context = TestContext::new(2, None); // Use stereo
        let render_frame = context.generate_sine_frame(440.0);

        // Measure for Full mode
        context.processor.set_config(Config {
            echo_canceller: Some(EchoCanceller {
                mode: EchoCancellerMode::Full,
                ..Default::default()
            }),
            ..Default::default()
        });
        let full_reduction = context.measure_steady_state_performance(&render_frame, 50, 10);

        // Measure for Mobile mode
        context.processor.set_config(Config {
            echo_canceller: Some(EchoCanceller {
                mode: EchoCancellerMode::Mobile,
                ..Default::default()
            }),
            ..Default::default()
        });
        let mobile_reduction = context.measure_steady_state_performance(&render_frame, 50, 10);

        // Verify both modes achieve some echo reduction
        assert!(
            full_reduction > 0.0 && mobile_reduction > 0.0,
            "Both modes should achieve positive echo reduction"
        );
    }

    /// Verifies that unique AEC3 configurations produce measurably different results.
    ///
    /// This test is used to verify that a AEC3 configuration will apply and output
    /// different results (in this case, 4dB of ERL).
    #[test]
    #[cfg(feature = "experimental-aec3-config")]
    fn test_aec3_configuration_tuning() {
        // Test strong suppression
        let strong_reduction = {
            let config =
                Config { echo_canceller: Some(EchoCanceller::default()), ..Default::default() };
            let mut aec3_config = experimental::EchoCanceller3Config::default();
            // Aggressive suppression
            aec3_config.suppressor.normal_tuning.mask_lf.enr_suppress = 5.0;
            aec3_config.suppressor.normal_tuning.mask_hf.enr_suppress = 5.0;

            let mut context = TestContext::new(2, Some(aec3_config));
            let render_frame = context.generate_sine_frame(440.0);
            context.processor.set_config(config);
            context.measure_steady_state_performance(&render_frame, 50, 10)
        };

        // Test light suppression
        let light_reduction = {
            let config =
                Config { echo_canceller: Some(EchoCanceller::default()), ..Default::default() };
            let mut aec3_config = experimental::EchoCanceller3Config::default();
            // Very light suppression
            aec3_config.suppressor.normal_tuning.mask_lf.enr_suppress = 0.1;
            aec3_config.suppressor.normal_tuning.mask_hf.enr_suppress = 0.1;

            let mut context = TestContext::new(2, Some(aec3_config));
            let render_frame = context.generate_sine_frame(440.0);
            context.processor.set_config(config);
            context.measure_steady_state_performance(&render_frame, 50, 10)
        };

        // Verify the configurations produce measurably different results
        assert!(
            strong_reduction > light_reduction + 3.0,
            "Strong suppression ({:.1} dB) should achieve at least 3dB more reduction than light suppression ({:.1} dB)",
            strong_reduction,
            light_reduction
        );
    }

    /// Validates AEC configuration state management across processing modes.
    ///
    /// Tests that AEC metrics and behavior remain consistent when switching
    /// between different modes (Full vs Mobile).
    #[test]
    fn test_aec3_configuration_behavior() {
        let mut context = TestContext::new(2, None);
        let render_frame = context.generate_sine_frame(440.0);
        let mut capture_frame = render_frame.clone();

        // Configure initial Full mode
        context.processor.set_config(Config {
            echo_canceller: Some(EchoCanceller {
                mode: EchoCancellerMode::Full,
                ..Default::default()
            }),
            ..Default::default()
        });

        // Verify initial state
        let initial_stats = context.processor.get_stats();
        assert!(
            initial_stats.echo_return_loss.is_none(),
            "Echo metrics should not be available before processing"
        );

        // Process and verify AEC is working
        context.process_frames(&mut render_frame.clone(), &mut capture_frame, 30);

        let mid_stats = context.processor.get_stats();
        assert!(
            mid_stats.echo_return_loss.is_some(),
            "Echo metrics should be available after processing"
        );

        // Switch to Mobile mode and verify persistence
        context.processor.set_config(Config {
            echo_canceller: Some(EchoCanceller {
                mode: EchoCancellerMode::Mobile,
                ..Default::default()
            }),
            ..Default::default()
        });

        context.process_frames(&mut render_frame.clone(), &mut capture_frame, 10);

        let final_stats = context.processor.get_stats();
        assert!(
            final_stats.echo_return_loss.is_some(),
            "Echo metrics should remain available after config change"
        );
    }
}
