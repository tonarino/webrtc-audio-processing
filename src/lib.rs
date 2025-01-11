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
        frame: &mut Vec<Vec<f32>>,
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
        frame: &mut Vec<Vec<f32>>,
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
        if !inner.is_null() {
            Ok(Self { inner })
        } else {
            Err(Error { code })
        }
    }

    fn initialize(&self) {
        unsafe { ffi::initialize(self.inner) }
    }

    fn process_capture_frame(&self, frame: &mut Vec<Vec<f32>>) -> Result<(), Error> {
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

    fn process_render_frame(&self, frame: &mut Vec<Vec<f32>>) -> Result<(), Error> {
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

    fn init_config(num_channels: usize) -> InitializationConfig {
        InitializationConfig {
            num_capture_channels: num_channels,
            num_render_channels: num_channels,
            sample_rate_hz: 48_000,
        }
    }

    // Helper types for test metrics
    #[derive(Debug)]
    #[allow(dead_code)]
    struct ProcessingMetrics {
        original_rms: f32,
        processed_rms: f32,
        max_difference: f32,
    }

    #[derive(Debug)]
    struct ProcessingResults {
        stats: Stats,
        metrics: ProcessingMetrics,
        original_metrics: ProcessingMetrics,
    }

    // Helper function for calculating RMS
    fn calculate_rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|&x| x * x).sum::<f32>() / samples.len() as f32).sqrt()
    }

    // Common test fixtures and helpers
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

        fn generate_frame(&self) -> Vec<f32> {
            vec![0.0f32; self.num_samples * self.num_channels]
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

        fn verify_processing_results(
            &self,
            original: &[f32],
            processed: &[f32],
        ) -> ProcessingMetrics {
            ProcessingMetrics {
                original_rms: calculate_rms(original),
                processed_rms: calculate_rms(processed),
                max_difference: original
                    .iter()
                    .zip(processed)
                    .map(|(a, b)| (a - b).abs())
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap(),
            }
        }

        fn process_frames_with_metrics(
            &mut self,
            iterations: usize,
            config: Option<Config>,
        ) -> ProcessingResults {
            if let Some(cfg) = config {
                self.processor.set_config(cfg);
            }

            let mut render_frame = self.generate_sine_frame(440.0);
            let mut capture_frame = render_frame.clone();
            let original_metrics = self.verify_processing_results(&render_frame, &capture_frame);

            for _ in 0..iterations {
                self.processor.process_render_frame(&mut render_frame).unwrap();
                self.processor.process_capture_frame(&mut capture_frame).unwrap();
            }

            ProcessingResults {
                stats: self.processor.get_stats(),
                metrics: self.verify_processing_results(&render_frame, &capture_frame),
                original_metrics,
            }
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

    /// Tests processing of stereo frames
    #[test]
    fn test_nominal() {
        let config = init_config(2);
        let mut processor = Processor::new(&config).unwrap();

        let mut render_frame = vec![0.0f32; processor.num_samples_per_frame() * 2];
        let mut capture_frame = render_frame.clone();

        processor.process_render_frame(&mut render_frame).unwrap();
        processor.process_capture_frame(&mut capture_frame).unwrap();
    }

    /// Tests processing of stereo frames in multiple threads
    #[test]
    fn test_nominal_threaded() {
        use std::thread;

        let config = init_config(2);
        let processor = Processor::new(&config).unwrap();
        let num_samples = processor.num_samples_per_frame();

        let render_frame = vec![0.0f32; num_samples * 2];
        let capture_frame = render_frame.clone();

        let mut threads = Vec::new();

        for _ in 0..4 {
            let mut processor = processor.clone();
            let mut render = render_frame.clone();
            let mut capture = capture_frame.clone();

            threads.push(thread::spawn(move || {
                processor.process_render_frame(&mut render).unwrap();
                processor.process_capture_frame(&mut capture).unwrap();
            }));
        }

        for thread in threads {
            thread.join().unwrap();
        }
    }

    /// Tests various processor parameter adjustments
    #[test]
    fn test_tweak_processor_params() {
        let config = init_config(2);
        let mut processor = Processor::new(&config).unwrap();

        // Test various parameter adjustments
        processor.set_output_will_be_muted(true);
        processor.set_stream_key_pressed(true);

        // Process some frames to ensure the parameters don't cause issues
        let mut render_frame = vec![0.0f32; processor.num_samples_per_frame() * 2];
        let mut capture_frame = render_frame.clone();

        processor.process_render_frame(&mut render_frame).unwrap();
        processor.process_capture_frame(&mut capture_frame).unwrap();
    }

    /// Tests echo cancellation convergence and signal reduction
    #[test]
    fn test_aec_echo_reduction() {
        let mut ctx = TestContext::new(2);
        let results = ctx.process_frames_with_metrics(
            10,
            Some(Config { echo_canceller: Some(EchoCanceller::default()), ..Default::default() }),
        );

        assert!(
            results.metrics.processed_rms < results.original_metrics.processed_rms,
            "Expected echo reduction, original RMS: {}, processed RMS: {}",
            results.original_metrics.processed_rms,
            results.metrics.processed_rms
        );
    }

    /// Tests muting and key press detection functionality
    #[test]
    fn test_processor_params() {
        let mut ctx = TestContext::new(2);

        // Test muting and key press functionality
        ctx.processor.set_output_will_be_muted(true);
        ctx.processor.set_stream_key_pressed(true);

        // Process some frames to ensure the parameters don't cause issues
        let mut frame = ctx.generate_frame();
        ctx.processor.process_render_frame(&mut frame).unwrap();
        ctx.processor.process_capture_frame(&mut frame).unwrap();
    }

    /// Tests concurrent processing and thread safety
    #[test]
    #[ignore]
    fn test_threaded_processing() {
        use std::thread;
        use std::time::Duration;

        let ctx = TestContext::new(2);
        let processor = ctx.processor.clone();

        // Create test frames
        let render_frame = ctx.generate_frame();
        let capture_frame = ctx.generate_frame();

        // Config thread
        let mut config_processor = processor.clone();
        let config_thread = thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            let config =
                Config { echo_canceller: Some(EchoCanceller::default()), ..Default::default() };
            config_processor.set_config(config);
        });

        // Render thread
        let mut render_processor = processor.clone();
        let render_thread = thread::spawn(move || {
            for _ in 0..100 {
                let mut frame = render_frame.clone();
                render_processor.process_render_frame(&mut frame).unwrap();
                thread::sleep(Duration::from_millis(10));
            }
        });

        let mut capture_processor = processor.clone();
        let capture_thread = thread::spawn(move || {
            for i in 0..100 {
                let mut capture_frame_output = capture_frame.clone();
                capture_processor.process_capture_frame(&mut capture_frame_output).unwrap();

                let stats = capture_processor.get_stats();
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
    /// Tests comprehensive stats reporting functionality
    #[test]
    fn test_stats_reporting() {
        let mut ctx = TestContext::new(1);
        ctx.processor.set_config(Config {
            echo_canceller: Some(EchoCanceller::default()),
            reporting: ReportingConfig {
                enable_voice_detection: true,
                enable_level_estimation: true,
                enable_residual_echo_detector: true,
            },
            ..Default::default()
        });

        let results = ctx.process_frames_with_metrics(10, None);
        assert!(
            results.stats.echo_return_loss.is_some() || results.stats.output_rms_dbfs.is_some()
        );
    }
}
