//! This crate is a wrapper around [PulseAudio's repackaging of WebRTC's AudioProcessing module](https://www.freedesktop.org/software/pulseaudio/webrtc-audio-processing/).
//!
//! See `examples/simple.rs` for an example of how to use the library.

#![warn(clippy::all)]
#![warn(missing_docs)]

mod config;
mod stats;

use std::{error, fmt, ptr::null, sync::Arc};
use webrtc_audio_processing_sys as ffi;

pub use config::*;
pub use stats::*;

/// Represents an error inside webrtc::AudioProcessing.
/// See the documentation of [`webrtc::AudioProcessing::Error`](https://cgit.freedesktop.org/pulseaudio/webrtc-audio-processing/tree/webrtc/modules/audio_processing/include/audio_processing.h?id=9def8cf10d3c97640d32f1328535e881288f700f)
/// for further details.
#[derive(Debug)]
pub struct Error {
    /// webrtc::AudioProcessing::Error
    code: i32,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ffi::AudioProcessing::Error code: {}", self.code)
    }
}

impl error::Error for Error {}

impl From<AudioProcessingError> for Error {
    fn from(err: AudioProcessingError) -> Self {
        let code = match err {
            AudioProcessingError::InvalidParameters => -1,
            AudioProcessingError::ConfigValidationFailed => -2,
            AudioProcessingError::CreationFailed => -3,
            AudioProcessingError::InitializationFailed(code) => code,
            AudioProcessingError::UnknownError(code) => code,
            AudioProcessingError::CppException(code) => code,
        };
        Error { code }
    }
}

/// `Processor` provides an access to webrtc's audio processing e.g. echo
/// cancellation and automatic gain control. It can be cloned, and cloned
/// instances share the same underlying processor module. It's the recommended
/// way to run the `Processor` in multi-threaded application.
#[derive(Clone)]
pub struct Processor {
    inner: Arc<AudioProcessing>,
    // TODO: Refactor. It's not necessary to have two frame buffers as
    // `Processor`s are cloned for each thread.
    deinterleaved_capture_frame: Vec<Vec<f32>>,
    deinterleaved_render_frame: Vec<Vec<f32>>,
}

impl Processor {
    /// Creates a new `Processor`. `InitializationConfig` is only used on
    /// instantiation, however new configs can be be passed to `set_config()`
    /// at any time during processing.
    pub fn new(config: &InitializationConfig) -> Result<Self, Error> {
        Self::with_aec3_config(config, None)
    }

    /// Creates a new `Processor` with custom AEC3 configuration.
    pub fn with_aec3_config(
        config: &InitializationConfig,
        aec3_config: Option<EchoCanceller3Config>,
    ) -> Result<Self, Error> {
        if config.num_capture_channels == 0 || config.num_render_channels == 0 {
            return Err(Error { code: -9 }); // kBadNumberChannelsError
        }
        let inner = Arc::new(AudioProcessing::new(config, aec3_config)?);
        let num_samples = inner.num_samples_per_frame();
        Ok(Self {
            inner,
            deinterleaved_capture_frame: vec![
                vec![0f32; num_samples];
                config.num_capture_channels as usize
            ],
            deinterleaved_render_frame: vec![
                vec![0f32; num_samples];
                config.num_render_channels as usize
            ],
        })
    }

    /// Initializes internal states, while retaining all user settings. This should be called before
    /// beginning to process a new audio stream. However, it is not necessary to call before processing
    /// the first stream after creation.
    pub fn initialize(&mut self) {
        self.inner.initialize()
    }

    /// Processes and modifies the audio frame from a capture device by applying
    /// signal processing as specified in the config. `frame` should hold an
    /// interleaved f32 audio frame, with NUM_SAMPLES_PER_FRAME samples.
    pub fn process_capture_frame(&mut self, frame: &mut [f32]) -> Result<(), Error> {
        Self::deinterleave(frame, &mut self.deinterleaved_capture_frame);
        self.inner.process_capture_frame(&mut self.deinterleaved_capture_frame)?;
        Self::interleave(&self.deinterleaved_capture_frame, frame);
        Ok(())
    }

    /// Processes and modifies the audio frame from a capture device by applying
    /// signal processing as specified in the config. `frame` should be a Vec of
    /// length 'num_capture_channels', with each inner Vec representing a channel
    /// with NUM_SAMPLES_PER_FRAME samples.
    pub fn process_capture_frame_noninterleaved(
        &mut self,
        frame: &mut [Vec<f32>],
    ) -> Result<(), Error> {
        self.inner.process_capture_frame(frame)
    }

    /// Processes and optionally modifies the audio frame from a playback device.
    /// `frame` should hold an interleaved `f32` audio frame, with
    /// `NUM_SAMPLES_PER_FRAME` samples.
    pub fn process_render_frame(&mut self, frame: &mut [f32]) -> Result<(), Error> {
        Self::deinterleave(frame, &mut self.deinterleaved_render_frame);
        self.inner.process_render_frame(&mut self.deinterleaved_render_frame)?;
        Self::interleave(&self.deinterleaved_render_frame, frame);
        Ok(())
    }

    /// Processes and optionally modifies the audio frame from a playback device.
    /// `frame` should be a Vec of length 'num_render_channels', with each inner Vec
    /// representing a channel with NUM_SAMPLES_PER_FRAME samples.
    pub fn process_render_frame_noninterleaved(
        &mut self,
        frame: &mut [Vec<f32>],
    ) -> Result<(), Error> {
        self.inner.process_render_frame(frame)
    }

    /// Returns statistics from the last `process_capture_frame()` call.
    pub fn get_stats(&self) -> Stats {
        self.inner.get_stats()
    }

    /// Returns the number of samples per frame based on the sample rate and frame size.
    pub fn num_samples_per_frame(&self) -> usize {
        self.inner.num_samples_per_frame()
    }

    /// Immediately updates the configurations of the internal signal processor.
    /// May be called multiple times after the initialization and during
    /// processing.
    pub fn set_config(&mut self, config: Config) {
        self.inner.set_config(config);
    }

    /// Signals the AEC and AGC that the audio output will be / is muted.
    /// They may use the hint to improve their parameter adaptation.
    pub fn set_output_will_be_muted(&self, muted: bool) {
        self.inner.set_output_will_be_muted(muted);
    }

    /// Sets the delay in milliseconds between `process_render_frame()` receiving a far-end frame
    /// and `process_capture_frame()` receiving a near-end frame containing the corresponding echo.
    ///
    /// This should only be used when the delay is known to be stable and constant. For adaptive
    /// delay estimation, leave this unset and rely on the internal estimator.
    pub fn set_stream_delay_ms(&self, delay: i32) {
        self.inner.set_stream_delay_ms(delay);
    }

    /// Signals the AEC and AGC that the next frame will contain key press sound
    pub fn set_stream_key_pressed(&self, pressed: bool) {
        self.inner.set_stream_key_pressed(pressed);
    }

    /// De-interleaves multi-channel frame `src` into `dst`.
    ///
    /// ```text
    /// e.g. A stereo frame with 3 samples:
    ///
    /// Interleaved
    /// +---+---+---+---+---+---+
    /// |L0 |R0 |L1 |R1 |L2 |R2 |
    /// +---+---+---+---+---+---+
    ///
    /// Non-interleaved
    /// +---+---+---+
    /// |L0 |L1 |L2 |
    /// +---+---+---+
    /// |R0 |R1 |R2 |
    /// +---+---+---+
    /// ```
    fn deinterleave<T: AsMut<[f32]>>(src: &[f32], dst: &mut [T]) {
        let num_channels = dst.len();
        let num_samples = dst[0].as_mut().len();
        assert_eq!(src.len(), num_channels * num_samples);
        for channel_index in 0..num_channels {
            for sample_index in 0..num_samples {
                dst[channel_index].as_mut()[sample_index] =
                    src[num_channels * sample_index + channel_index];
            }
        }
    }

    /// Reverts the `deinterleave` operation.
    fn interleave<T: AsRef<[f32]>>(src: &[T], dst: &mut [f32]) {
        let num_channels = src.len();
        let num_samples = src[0].as_ref().len();
        assert_eq!(dst.len(), num_channels * num_samples);
        for channel_index in 0..num_channels {
            for sample_index in 0..num_samples {
                dst[num_channels * sample_index + channel_index] =
                    src[channel_index].as_ref()[sample_index];
            }
        }
    }
}

/// Minimal wrapper for safe and synchronized ffi.
struct AudioProcessing {
    inner: *mut ffi::AudioProcessing,
}

/// Represents specific errors that can occur during audio processing operations.
#[derive(Debug)]
pub enum AudioProcessingError {
    /// The parameters provided to the audio processor were invalid.
    InvalidParameters,

    /// The configuration validation failed.
    ConfigValidationFailed,

    /// Failed to create the audio processor.
    CreationFailed,

    /// The audio processor initialization failed with the given error code.
    InitializationFailed(i32),

    /// An unknown error occurred with the given error code.
    UnknownError(i32),

    /// A C++ exception was caught with the given error code.
    CppException(i32),
}

impl fmt::Display for AudioProcessingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidParameters => write!(f, "Invalid parameters"),
            Self::ConfigValidationFailed => write!(f, "Config validation failed"),
            Self::CreationFailed => write!(f, "Failed to create audio processor"),
            Self::InitializationFailed(code) => write!(f, "Initialization failed: {}", code),
            Self::UnknownError(code) => write!(f, "Unknown error: {}", code),
            Self::CppException(code) => write!(f, "C++ exception occurred: {}", code),
        }
    }
}

impl error::Error for AudioProcessingError {}

impl AudioProcessing {
    fn new(
        config: &InitializationConfig,
        aec3_config: Option<EchoCanceller3Config>,
    ) -> Result<Self, Error> {
        let aec3_config = if let Some(aec3_config) = aec3_config {
            &aec3_config.into() as *const ffi::EchoCanceller3ConfigOverride
        } else {
            null()
        };

        let mut code = 0;
        let inner = unsafe {
            ffi::audio_processing_create(
                config.num_capture_channels as i32,
                config.num_render_channels as i32,
                config.sample_rate_hz as i32,
                aec3_config,
                &mut code,
            )
        };
        if inner.is_null() || code != 0 {
            Err(Error { code })
        } else {
            Ok(Self { inner })
        }
    }

    fn initialize(&self) {
        unsafe { ffi::initialize(self.inner) }
    }

    fn process_capture_frame(&self, frame: &mut [Vec<f32>]) -> Result<(), Error> {
        let mut frame_ptr = frame.iter_mut().map(|v| v.as_mut_ptr()).collect::<Vec<*mut f32>>();
        unsafe {
            let code = ffi::process_capture_frame(self.inner, frame_ptr.as_mut_ptr());
            if ffi::is_success(code) {
                Ok(())
            } else {
                Err(Error { code })
            }
        }
    }

    fn process_render_frame(&self, frame: &mut [Vec<f32>]) -> Result<(), Error> {
        let mut frame_ptr = frame.iter_mut().map(|v| v.as_mut_ptr()).collect::<Vec<*mut f32>>();
        unsafe {
            let code = ffi::process_render_frame(self.inner, frame_ptr.as_mut_ptr());
            if ffi::is_success(code) {
                Ok(())
            } else {
                Err(Error { code })
            }
        }
    }

    fn get_stats(&self) -> Stats {
        unsafe { ffi::get_stats(self.inner).into() }
    }

    fn num_samples_per_frame(&self) -> usize {
        unsafe { ffi::get_num_samples_per_frame(self.inner) as usize }
    }

    fn set_config(&self, config: Config) {
        unsafe {
            ffi::set_config(self.inner, &config.into());
        }
    }

    fn set_output_will_be_muted(&self, muted: bool) {
        unsafe {
            ffi::set_output_will_be_muted(self.inner, muted);
        }
    }

    fn set_stream_key_pressed(&self, pressed: bool) {
        unsafe {
            ffi::set_stream_key_pressed(self.inner, pressed);
        }
    }

    fn set_stream_delay_ms(&self, delay: i32) {
        unsafe {
            ffi::set_stream_delay_ms(self.inner, delay);
        }
    }
}

impl Drop for AudioProcessing {
    fn drop(&mut self) {
        unsafe {
            ffi::audio_processing_delete(self.inner);
        }
    }
}

// ffi::AudioProcessing provides thread safety with a few exceptions around the concurrent usage of
// its corresponding getters and setters.
unsafe impl Sync for AudioProcessing {}
unsafe impl Send for AudioProcessing {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time::Duration};

    fn init_config(num_channels: usize) -> InitializationConfig {
        InitializationConfig {
            num_capture_channels: num_channels,
            num_render_channels: num_channels,
            sample_rate_hz: 48_000,
        }
    }

    fn sample_stereo_frames(ap: &Processor) -> (Vec<f32>, Vec<f32>) {
        let num_samples_per_frame = ap.num_samples_per_frame();

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

    // Helper function for calculating ERL (Echo Return Loss)
    fn calculate_erl(reference: &[f32], processed: &[f32]) -> f32 {
        // Ensure valid comparison
        assert_eq!(reference.len(), processed.len(), "Signal lengths must match");

        // Sum of squares for both signals
        let reference_power: f32 = reference.iter().map(|x| x * x).sum();
        let processed_power: f32 = processed.iter().map(|x| x * x).sum();

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
        fn new(num_channels: usize) -> Self {
            let config = init_config(num_channels);
            let processor = Processor::new(&config).unwrap();
            let num_samples = processor.num_samples_per_frame();
            Self { processor, num_samples, num_channels }
        }

        fn generate_sine_frame(&self, frequency: f32) -> Vec<f32> {
            let mut frame = Vec::with_capacity(self.num_samples * self.num_channels);
            for i in 0..self.num_samples {
                let sample =
                    (i as f32 * frequency / 48000.0 * 2.0 * std::f32::consts::PI).sin() * 0.5;
                for _ in 0..self.num_channels {
                    frame.push(sample);
                }
            }
            frame
        }

        fn process_frames(&mut self, render: &mut [f32], capture: &mut [f32], iterations: usize) {
            for _ in 0..iterations {
                self.processor.process_render_frame(render).unwrap();
                self.processor.process_capture_frame(capture).unwrap();
            }
        }

        fn measure_echo_reduction(&mut self, render_frame: &[f32], iterations: usize) -> f32 {
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
            render_frame: &[f32],
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
        let config = init_config(0);
        assert!(Processor::new(&config).is_err());
    }

    /// Tests proper resource cleanup on drop
    #[test]
    fn test_create_drop() {
        let config = init_config(1);
        let _p = Processor::new(&config).unwrap();
    }

    /// Tests audio frame interleaving/deinterleaving operations
    #[test]
    fn test_deinterleave_interleave() {
        let num_channels = 2usize;
        let num_samples = 3usize;

        let interleaved = (0..num_channels * num_samples).map(|v| v as f32).collect::<Vec<f32>>();
        let mut deinterleaved = vec![vec![-1f32; num_samples]; num_channels];
        Processor::deinterleave(&interleaved, &mut deinterleaved);
        assert_eq!(vec![vec![0f32, 2f32, 4f32], vec![1f32, 3f32, 5f32]], deinterleaved);

        let mut interleaved_out = vec![-1f32; num_samples * num_channels];
        Processor::interleave(&deinterleaved, &mut interleaved_out);
        assert_eq!(interleaved, interleaved_out);
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
        println!("{:#?}", stats);
    }

    /// Tests in a threaded environment.
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

    /// Tests tweaking processor params outside of config.
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

    /// Measures baseline echo cancellation performance using industry metrics.
    ///
    /// Uses a pure sine wave to create ideal test conditions. Verifies the AEC
    /// achieves at least 18dB ERL.
    #[test]
    fn test_echo_cancellation_effectiveness() {
        let mut context = TestContext::new(1);

        // Configure AEC
        context.processor.set_config(Config {
            echo_canceller: Some(EchoCanceller::default()),
            ..Default::default()
        });

        // Test with pure sine wave
        let render_frame = context.generate_sine_frame(440.0);
        let erle = context.measure_echo_reduction(&render_frame, 100);

        // Verify industry-standard performance
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
        let mut context = TestContext::new(2); // Use stereo
        let render_frame = context.generate_sine_frame(440.0);

        // Measure for Full mode
        context.processor.set_config(Config {
            echo_canceller: Some(EchoCanceller::Full { enforce_high_pass_filtering: true }),
            ..Default::default()
        });
        let full_reduction = context.measure_steady_state_performance(&render_frame, 50, 10);

        // Measure for Mobile mode
        context.processor.set_config(Config {
            echo_canceller: Some(EchoCanceller::Mobile),
            ..Default::default()
        });
        let mobile_reduction = context.measure_steady_state_performance(&render_frame, 50, 10);

        // Verify both modes achieve some echo reduction
        assert!(
            full_reduction > 0.0 && mobile_reduction > 0.0,
            "Both modes should achieve positive echo reduction"
        );
    }

    /// Verifies that unique AEC configurations produce measurably different results.
    ///
    /// This test is used to verify that a AEC3 configuration will apply and output
    /// different results (in this case, 4dB of ERL).
    #[test]
    fn test_aec3_configuration_tuning() {
        // Strong suppression configuration
        let strong_config = Config {
            echo_canceller: Some(EchoCanceller::default()),
            aec3_config: Some(EchoCanceller3Config {
                suppressor: Suppressor {
                    dominant_nearend_detection: DominantNearendDetection {
                        enr_threshold: 0.75,
                        snr_threshold: 20.0,
                        ..Default::default()
                    },
                    high_bands_suppression: HighBandsSuppression {
                        enr_threshold: 0.8,
                        max_gain_during_echo: 0.3,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            }),
            ..Default::default()
        };

        // Light suppression configuration
        let light_config = Config {
            echo_canceller: Some(EchoCanceller::default()),
            aec3_config: Some(EchoCanceller3Config {
                suppressor: Suppressor {
                    dominant_nearend_detection: DominantNearendDetection {
                        enr_threshold: 0.25,
                        snr_threshold: 30.0,
                        ..Default::default()
                    },
                    high_bands_suppression: HighBandsSuppression {
                        enr_threshold: 1.2,
                        max_gain_during_echo: 0.8,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            }),
            ..Default::default()
        };

        let mut context = TestContext::new(2);
        let render_frame = context.generate_sine_frame(440.0);

        // Test strong suppression
        context.processor.set_config(strong_config);
        let strong_reduction = context.measure_steady_state_performance(&render_frame, 50, 10);

        // Test light suppression
        context.processor.set_config(light_config);
        let light_reduction = context.measure_steady_state_performance(&render_frame, 50, 10);

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
        let mut context = TestContext::new(2);
        let render_frame = context.generate_sine_frame(440.0);
        let mut capture_frame = render_frame.clone();

        // Configure initial Full mode
        context.processor.set_config(Config {
            echo_canceller: Some(EchoCanceller::Full { enforce_high_pass_filtering: true }),
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
            echo_canceller: Some(EchoCanceller::Mobile),
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
