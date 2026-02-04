//! This crate is a wrapper around [PulseAudio's repackaging of WebRTC's AudioProcessing module](https://www.freedesktop.org/software/pulseaudio/webrtc-audio-processing/).
//!
//! See `examples/simple.rs` for an example of how to use the library.
//! The [`Processor`] struct is the entrypoint to the functionality it provides.

#![warn(clippy::all)]
#![warn(missing_docs)]

/// Configuration structs for [`Processor`]. The top-level struct [`Config`] is also reexported from
/// the root of this crate, but the nested structs are only accessible through [`config`].
pub mod config;
mod stats;

/// [Highly experimental]
/// Exposes finer-grained control of the internal AEC3 configuration.
#[cfg(feature = "experimental-aec3-config")]
pub mod experimental;

use crate::config::{EchoCanceller, IntoFfi};
use std::{
    convert::TryFrom,
    error, fmt,
    ptr::null_mut,
    sync::atomic::{AtomicU32, Ordering},
};
use webrtc_audio_processing_sys::{self as ffi, StreamConfig};

pub use config::Config;
pub use stats::*;

/// Represents an error inside webrtc::AudioProcessing.
/// Drawn from documentation of pulseaudio upstream `webrtc::AudioProcessing::Error`:
/// <https://cgit.freedesktop.org/pulseaudio/webrtc-audio-processing/tree/webrtc/modules/audio_processing/include/audio_processing.h?id=9def8cf10d3c97640d32f1328535e881288f700f>
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

/// [`Processor`] provides an access to WebRTC's audio processing, e.g. echo cancellation, noise
/// removal, automatic gain control and so on.
///
/// It is [`Send`] + [`Sync`] and its methods take `&self` shared reference (as we expose
/// thread-safe APIs of the underlying C++ library).
/// As a practical example, one can wrap this type in an [`Arc`](std::sync::Arc) to share ownership
/// between capture and render threads.
///
/// Functionality in the C++ library that we don't yet expose:
/// - getting the current config (can only return the FFI, but we can compare against user-supplied
///   one)
#[derive(Debug)]
pub struct Processor {
    inner: AudioProcessingPtr,
    sample_rate_hz: u32,
    /// The stream_delay extracted from config. Underlying C++ library wants us to set this before
    /// every process_capture_frame() call in many cases (Full AEC3 with custom delay, Mobile AECM).
    ///
    /// If the value cannot be exactly represented as i32 (e.g. u32::MAX), it denotes _not set_.
    stream_delay_ms: AtomicU32,
}

impl Processor {
    /// Creates a new [`Processor`]. Detailed general configuration can be be passed to
    /// [`Self::set_config()`] at any time during processing.
    pub fn new(sample_rate_hz: u32) -> Result<Self, Error> {
        Self::new_with_ptr(sample_rate_hz, null_mut())
    }

    /// [Highly experimental]
    /// Creates a new [`Processor`] with custom AEC3 configuration. The AEC3 configuration needs to
    /// be valid, otherwise this returns [`Error::BadParameter`].
    ///
    /// Note that passing the AEC3 configuration:
    /// - force-enables the echo canceller submodule and hardcodes its variant to
    ///   [`config::EchoCanceller::Full`] no matter the contents of [Config::echo_canceller]; it is
    ///   strongly recommended to still set that field accordingly.
    /// - disables the internal logic to use different AEC3 default config based on whether the
    ///   audio stream is truly multichannel (though multichannel detection is still used for other
    ///   functionality). You can use [`experimental::EchoCanceller3Config::multichannel_default()`]
    ///
    /// To change the AEC3 configuration at runtime, the [`Processor`] needs to be currently
    /// recreated. This limitation could be eventually lifted, see issue #77.
    #[cfg(feature = "experimental-aec3-config")]
    pub fn with_aec3_config(
        sample_rate_hz: u32,
        mut aec3_config: experimental::EchoCanceller3Config,
    ) -> Result<Self, Error> {
        Self::new_with_ptr(sample_rate_hz, &raw mut *aec3_config)
    }

    /// Pass null ptr in `aec3_config` to use its default config.
    fn new_with_ptr(
        sample_rate_hz: u32,
        aec3_config: *mut ffi::EchoCanceller3Config,
    ) -> Result<Self, Error> {
        let mut code = 0;
        let inner = unsafe { ffi::create_audio_processing(aec3_config, &mut code) };
        // First check the error code.
        let inner = result_from_code(inner, code)?;
        // Then check for null. This shouldn't normally happen, but can in corner cases like when
        // linking to webrtc-audio-processing built with WEBRTC_EXCLUDE_AUDIO_PROCESSING_MODULE.
        if inner.is_null() {
            return Err(Error::NullPointer);
        }
        let inner = AudioProcessingPtr(inner);

        // u32::MAX to denote stream_delay_ms not (yet) set.
        Ok(Self { inner, sample_rate_hz, stream_delay_ms: AtomicU32::new(u32::MAX) })
    }

    /// Processes and modifies the audio frame from a capture device by applying
    /// signal processing as specified in the config.
    ///
    /// `frame` is a non-interleaved audio frame data: mutable iterator/Vec/array/slice of
    /// channels, which are Vecs/arrays/slices of [`f32`] samples.
    ///
    /// Processor dynamically adapts to the number of channels in `frame` (at the cost of partial
    /// reinitialization).
    ///
    /// # Panics
    /// Panics if the number of samples doesn't match 10ms @ `sample_rate_hz` passed to constructor.
    pub fn process_capture_frame<F, Ch>(&self, frame: F) -> Result<(), Error>
    where
        F: IntoIterator<Item = Ch>,
        Ch: AsMut<[f32]>,
    {
        let frame_ptr = as_mut_ptrs(frame, self.num_samples_per_frame());
        let capture_stream_config = StreamConfig::new(self.sample_rate_hz, frame_ptr.len());

        // If we want a custom stream_delay with Full AEC3, we need to set it before every
        // process_capture_frame() call, otherwise the delay estimator kicks in.
        //
        // The mobile AECM requires stream_delay to be set before every single
        // process_capture_frame() call: we guarantee it on type level in `config::EchoCanceller`.
        //
        // If the value in `self.stream_delay_ms` cannot be represented as i32, it denotes not set.
        let stream_delay_ms = i32::try_from(self.stream_delay_ms.load(Ordering::Relaxed)).ok();
        let code = unsafe {
            if let Some(stream_delay_ms) = stream_delay_ms {
                ffi::set_stream_delay_ms(*self.inner, stream_delay_ms);
            }

            ffi::process_capture_frame(*self.inner, &capture_stream_config, frame_ptr.as_ptr())
        };
        result_from_code((), code)
    }

    /// Processes and optionally modifies the audio frame destined to a playback device.
    /// See [`Self::analyze_render_frame()`] if modification of the stream is not needed/desired.
    ///
    /// `frame` is a non-interleaved audio frame data: mutable iterator/Vec/array/slice of
    /// channels, which are Vecs/arrays/slices of [`f32`] samples.
    ///
    /// Processor dynamically adapts to the number of channels in `frame` (at the cost of partial
    /// reinitialization).
    ///
    /// # Panics
    /// Panics if the number of samples doesn't match 10ms @ `sample_rate_hz` passed to constructor.
    pub fn process_render_frame<F, Ch>(&self, frame: F) -> Result<(), Error>
    where
        F: IntoIterator<Item = Ch>,
        Ch: AsMut<[f32]>,
    {
        let frame_ptr = as_mut_ptrs(frame, self.num_samples_per_frame());
        let render_stream_config = StreamConfig::new(self.sample_rate_hz, frame_ptr.len());
        let code = unsafe {
            ffi::process_render_frame(*self.inner, &render_stream_config, frame_ptr.as_ptr())
        };
        result_from_code((), code)
    }

    /// Analyzes the audio frame destined to playback device without modifying it.
    /// Similar to [`Self::process_render_frame()`], but doesn't modify the frame and takes an
    /// immutable reference to it.
    ///
    /// `frame` is a non-interleaved audio frame data: mutable iterator/Vec/array/slice of
    /// channels, which are Vecs/arrays/slices of [`f32`] samples.
    ///
    /// Processor dynamically adapts to the number of channels in `frame` (at the cost of partial
    /// reinitialization).
    ///
    /// # Panics
    /// Panics if the number of samples doesn't match 10ms @ `sample_rate_hz` passed to constructor.
    pub fn analyze_render_frame<F, Ch>(&self, frame: F) -> Result<(), Error>
    where
        F: IntoIterator<Item = Ch>,
        Ch: AsRef<[f32]>,
    {
        let frame_ptr = as_const_ptrs(frame, self.num_samples_per_frame());
        let render_stream_config = StreamConfig::new(self.sample_rate_hz, frame_ptr.len());
        let code = unsafe {
            ffi::analyze_render_frame(*self.inner, &render_stream_config, frame_ptr.as_ptr())
        };
        result_from_code((), code)
    }

    /// Returns statistics from the last `process_capture_frame()` call.
    pub fn get_stats(&self) -> Stats {
        unsafe { ffi::get_stats(*self.inner).into() }
    }

    /// Returns the number of (possibly multichannel) samples per frame based on the sample rate
    /// and frame size (fixed in WebRTC to 10ms).
    pub fn num_samples_per_frame(&self) -> usize {
        // We mimic the C++ code here (bindgen doesn't support inline methods anyway)
        // ```cpp
        //   static int GetFrameSize(int sample_rate_hz) { return sample_rate_hz / 100; }
        // ```
        self.sample_rate_hz as usize / 100
    }

    /// Immediately updates the configurations of the internal signal processor.
    /// May be called multiple times after the initialization and during processing.
    /// Internally acquires both capture and render locks (is fully serialized w.r.t.
    /// process_{render,capture}_frame()).
    ///
    /// Only submodules whose config has changed are reinitialized. If the passed config is the same
    /// as current config, the overhead is only the locking and doing some comparisons.
    pub fn set_config(&self, config: Config) {
        // Extract the stream delay to our cache (it is a runtime concept for AEC, not a config).
        let stream_delay_ms_opt = match config.echo_canceller {
            Some(EchoCanceller::Full { stream_delay_ms }) => stream_delay_ms,
            Some(EchoCanceller::Mobile { stream_delay_ms }) => Some(stream_delay_ms),
            None => None,
        };
        // Convert optional u16 value into u32, mapping None to u32::MAX (meaning not set).
        let stream_delay_ms = stream_delay_ms_opt.map_or(u32::MAX, u32::from);
        self.stream_delay_ms.store(stream_delay_ms, Ordering::Relaxed);

        unsafe {
            ffi::set_config(*self.inner, &config.into_ffi());
        }
    }

    /// Reinitialize the processor: drop all state (like the estimated echo parameters), but retain
    /// the configuration. Acquires internally both capture and render locks.
    pub fn reinitialize(&self) {
        unsafe {
            ffi::initialize(*self.inner);
        }
    }

    /// Signals the AEC and AGC that the audio output will be / is muted.
    /// They may use the hint to improve their parameter adaptation.
    pub fn set_output_will_be_muted(&self, muted: bool) {
        unsafe {
            ffi::set_output_will_be_muted(*self.inner, muted);
        }
    }

    /// Signals the AEC and AGC that the next frame will contain key press sound
    pub fn set_stream_key_pressed(&self, pressed: bool) {
        unsafe {
            ffi::set_stream_key_pressed(*self.inner, pressed);
        }
    }
}

impl Drop for Processor {
    fn drop(&mut self) {
        unsafe {
            ffi::delete_audio_processing(*self.inner);
        }
    }
}

/// Wrap the raw FFI pointer so that we can unsafe impl Send, Sync only for it and not for the whole
/// [`Processor`] struct. That way Rust type-checks the other [`Processor`] fields.
#[derive(Debug)]
struct AudioProcessingPtr(*mut ffi::AudioProcessing);

impl std::ops::Deref for AudioProcessingPtr {
    type Target = *mut ffi::AudioProcessing;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// ffi::AudioProcessing provides thread safety, at least for the subset of its public API that we
// expose. Relevant pointers in its source code:
// https://gitlab.freedesktop.org/pulseaudio/webrtc-audio-processing/-/blob/v2.1/webrtc/api/audio/audio_processing.h?ref_type=tags#L74-80
// https://gitlab.freedesktop.org/pulseaudio/webrtc-audio-processing/-/blob/v2.1/webrtc/modules/audio_processing/audio_processing_impl.h?ref_type=tags#L352-354
unsafe impl Sync for AudioProcessingPtr {}
unsafe impl Send for AudioProcessingPtr {}

/// Collect a non-interleaved mutable frame (iterator/vec/array/slice of vecs/arrays/slices) into
/// a Vec of mut channel pointers suitable for passing to FFI.
///
/// # Panics
/// Panics if the number of samples doesn't match expectation.
fn as_mut_ptrs<F, Ch>(frame: F, num_samples: usize) -> Vec<*mut f32>
where
    F: IntoIterator<Item = Ch>,
    Ch: AsMut<[f32]>,
{
    frame
        .into_iter()
        .map(|mut channel| {
            let slice = channel.as_mut();
            assert_eq!(slice.len(), num_samples, "number of samples doesn't match expectation");
            slice.as_mut_ptr()
        })
        .collect()
}

/// Collect a non-interleaved immutable frame (iterator/vec/array/slice of vecs/arrays/slices) into
/// a Vec of const channel pointers suitable for passing to FFI.
///
/// # Panics
/// Panics if the number of samples doesn't match expectation.
fn as_const_ptrs<F, Ch>(frame: F, num_samples: usize) -> Vec<*const f32>
where
    F: IntoIterator<Item = Ch>,
    Ch: AsRef<[f32]>,
{
    frame
        .into_iter()
        .map(|channel| {
            let slice = channel.as_ref();
            assert_eq!(slice.len(), num_samples, "number of samples doesn't match expectation");
            slice.as_ptr()
        })
        .collect()
}

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
    use super::{config::EchoCanceller, *};
    use std::{sync::Arc, thread, time::Duration};

    const SAMPLE_RATE_HZ: u32 = 48_000;

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
            let processor = match aec3_config {
                Some(aec3_config) => {
                    Processor::with_aec3_config(SAMPLE_RATE_HZ, aec3_config).unwrap()
                },
                None => Processor::new(SAMPLE_RATE_HZ).unwrap(),
            };
            let num_samples = processor.num_samples_per_frame();
            Self { processor, num_samples, num_channels }
        }

        #[cfg(not(feature = "experimental-aec3-config"))]
        fn new(num_channels: usize, _: Option<()>) -> Self {
            let processor = Processor::new(SAMPLE_RATE_HZ).unwrap();
            let num_samples = processor.num_samples_per_frame();
            Self { processor, num_samples, num_channels }
        }

        /// For multichannel samples `per_channel_offset` determines the phase offset of each
        /// subsequent channel. Used to create true stereo samples. Useful range is 0..1.
        fn generate_sine_frame(&self, frequency: f32, per_channel_offset: f32) -> Vec<Vec<f32>> {
            let mut frame = Vec::with_capacity(self.num_channels);
            for ch_nr in 0..self.num_channels {
                let mut channel = Vec::with_capacity(self.num_samples);
                for i in 0..self.num_samples {
                    let phase = i as f32 * frequency / 48000.0;
                    let offset = ch_nr as f32 * per_channel_offset;
                    let sample = ((phase + offset) * 2.0 * std::f32::consts::PI).sin() * 0.5;
                    channel.push(sample);
                }

                frame.push(channel);
            }

            frame
        }

        fn process_frames(
            &mut self,
            render: &mut [Vec<f32>],
            capture: &mut [Vec<f32>],
            iterations: usize,
        ) {
            for _ in 0..iterations {
                self.processor.process_render_frame(&mut *render).unwrap();
                self.processor.process_capture_frame(&mut *capture).unwrap();
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

    /// Tests proper resource cleanup on drop
    #[test]
    fn test_create_drop() {
        let _p = Processor::new(SAMPLE_RATE_HZ).unwrap();
    }

    /// Tests nominal operation of the audio processing library.
    #[test]
    fn test_nominal() {
        let ap = Processor::new(SAMPLE_RATE_HZ).unwrap();

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

        let render_frame_immutable = render_frame.clone();
        ap.analyze_render_frame(&render_frame_immutable).unwrap();

        // Immutable render frame really shouldn't be modified. In safe Rust that wouldn't be
        // possible, but we use FFI and unsafe {}, so better test that.
        assert_eq!(render_frame, render_frame_immutable);

        let stats = ap.get_stats();
        assert!(stats.echo_return_loss.is_some());
        println!("{stats:#?}");
    }

    #[test]
    fn test_process_signatures() {
        const NUM_SAMPLES: usize = 480;

        let ap = Processor::new(SAMPLE_RATE_HZ).unwrap();
        assert_eq!(ap.num_samples_per_frame(), NUM_SAMPLES);

        // Iterator of Vecs
        #[expect(clippy::useless_vec)]
        let mut vector = vec![vec![0.0; NUM_SAMPLES]];
        ap.process_capture_frame(vector.iter_mut()).unwrap();

        // Vec of arrays
        let mut vector = vec![[0.0; NUM_SAMPLES]];
        ap.process_render_frame(&mut vector).unwrap();

        // array of slices
        let mut channel = vec![0.0; NUM_SAMPLES];
        let mut array = [&mut channel[..]];
        ap.process_capture_frame(&mut array).unwrap();

        // slice of Vecs
        let channel = vec![0.0; NUM_SAMPLES];
        let slice = &mut [channel][..];
        ap.process_render_frame(slice).unwrap();
    }

    #[test]
    fn test_zero_channels() {
        let ap = Processor::new(SAMPLE_RATE_HZ).unwrap();
        let mut frame: Vec<Vec<f32>> = vec![];

        assert_eq!(ap.process_capture_frame(&mut frame), Err(Error::BadNumberChannels));
        assert_eq!(ap.process_render_frame(&mut frame), Err(Error::BadNumberChannels));
        assert_eq!(ap.analyze_render_frame(&frame), Err(Error::BadNumberChannels));
    }

    #[test]
    // The test consistently fails on MacOS, probably because it is sensitive to timing and
    // thead::sleep() which is notoriously imprecise on macs.
    #[cfg_attr(target_os = "macos", ignore)]
    fn test_nominal_threaded() {
        let ap = Arc::new(Processor::new(SAMPLE_RATE_HZ).unwrap());

        let (render_frame, capture_frame) = sample_stereo_frames(&ap);

        let config_ap = Arc::clone(&ap);
        let config_thread = thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));

            let config =
                Config { echo_canceller: Some(EchoCanceller::default()), ..Default::default() };
            config_ap.set_config(config);
        });

        let render_ap = Arc::clone(&ap);
        let render_thread = thread::spawn(move || {
            for _ in 0..100 {
                let mut render_frame_output = render_frame.clone();
                render_ap.process_render_frame(&mut render_frame_output).unwrap();

                thread::sleep(Duration::from_millis(10));
            }
        });

        let capture_ap = Arc::clone(&ap);
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
        let ap = Processor::new(SAMPLE_RATE_HZ).unwrap();

        // Add some runtime events.
        ap.set_output_will_be_muted(true);
        ap.set_stream_key_pressed(true);

        // Set non-default stream_delay
        let config = Config {
            echo_canceller: Some(EchoCanceller::Full { stream_delay_ms: Some(10) }),
            ..Config::default()
        };
        ap.set_config(config);

        // test one process call
        let (render_frame, capture_frame) = sample_stereo_frames(&ap);

        let mut render_frame_output = render_frame.clone();
        ap.process_render_frame(&mut render_frame_output).unwrap();
        let mut capture_frame_output = capture_frame.clone();
        ap.process_capture_frame(&mut capture_frame_output).unwrap();

        // Change the number of channels from 2 to 4, processor should adapt.
        let mut four_channel_frame = vec![vec![0.0; ap.num_samples_per_frame()]; 4];
        ap.process_render_frame(&mut four_channel_frame).unwrap();
        ap.process_capture_frame(&mut four_channel_frame).unwrap();

        // Reinitialize the processor.
        ap.reinitialize();
        ap.analyze_render_frame(&four_channel_frame).unwrap();

        // it shouldn't crash
    }

    #[test]
    fn test_stream_delay() {
        let make_config = |delay_ms| Config {
            echo_canceller: Some(EchoCanceller::Full { stream_delay_ms: delay_ms }),
            ..Default::default()
        };

        // Verify via stats & warm up
        let context = TestContext::new(1, None);
        context.processor.set_config(make_config(Some(200)));

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
            let context = TestContext::new(1, None);
            // Apply either a correct hint (200ms) or an incorrect one (0ms)
            context.processor.set_config(make_config(applied_delay_ms));

            let num_samples = context.num_samples;
            // Make a fake path delay of 200ms (20 frames of 10ms)
            let mut history = vec![vec![vec![0.0; num_samples]]; 20];
            let (mut total_in_p, mut total_out_p) = (0.0, 0.0);

            for i in 0..100 {
                // Make a pulse for 50ms (5 frames), then silence
                let mut render = if i < 5 {
                    context.generate_sine_frame(440.0, 0.0)
                } else {
                    vec![vec![0.0; num_samples]]
                };

                // Add the render frame to history and pop the delayed frame as the "echo"
                history.push(render.clone());
                // Capture is the staggered echo signal
                let mut capture: Vec<_> = history.remove(0);
                for sample in capture.iter_mut().flatten() {
                    *sample *= 0.8;
                }

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
        let reduction_mismatched = measure_pulse_reduction(Some(0));
        let reduction_adaptive = measure_pulse_reduction(None);
        // Measure reduction with a 200ms hint
        let reduction_matched = measure_pulse_reduction(Some(200));

        // Correct alignment should result in much better cancellation
        assert!(
            reduction_matched > reduction_mismatched * 1000.0,
            "Matched delay should have better echo cancellation"
        );
        // Adaptive should be better than outright wrong (in practice it is only slightly)
        assert!(reduction_adaptive > reduction_mismatched);
        // And correctly matched should be better than adaptive
        assert!(reduction_matched > reduction_adaptive);
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
        let render_frame = context.generate_sine_frame(440.0, 0.0);
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
        let render_frame = context.generate_sine_frame(440.0, 0.0);

        // Measure for Full mode (the default)
        context.processor.set_config(Config {
            echo_canceller: Some(EchoCanceller::default()),
            ..Default::default()
        });
        let full_reduction = context.measure_steady_state_performance(&render_frame, 50, 10);

        // Measure for Mobile mode
        context.processor.set_config(Config {
            echo_canceller: Some(EchoCanceller::Mobile { stream_delay_ms: 0 }),
            ..Default::default()
        });
        let mobile_reduction = context.measure_steady_state_performance(&render_frame, 50, 10);

        // Verify both modes achieve some echo reduction
        assert!(
            full_reduction > 0.0 && mobile_reduction > 0.0,
            "Both modes should achieve positive echo reduction"
        );
    }

    /// Verifies AEC3 config setting with mono signal.
    #[test]
    #[cfg(feature = "experimental-aec3-config")]
    fn test_aec3_configuration_tuning_mono() {
        test_aec3_configuration_tuning(1, 0.0);
    }

    /// Verifies AEC3 config setting with fake stereo signal (both channels have the same content).
    #[test]
    #[cfg(feature = "experimental-aec3-config")]
    fn test_aec3_configuration_tuning_fake_stereo() {
        test_aec3_configuration_tuning(2, 0.0);
    }

    /// Verifies AEC3 config setting with true stereo signal (left/right channels different).
    #[test]
    #[cfg(feature = "experimental-aec3-config")]
    fn test_aec3_configuration_tuning_true_stereo() {
        test_aec3_configuration_tuning(2, 0.1);
    }

    /// Verifies that unique AEC3 configurations produce measurably different results.
    ///
    /// This test is used to verify that a AEC3 configuration will apply and output
    /// different results (in this case, 4dB of ERL).
    ///
    /// It is parametrized by
    /// - the number of channels (significant because of multichannel config variant/detection)
    /// - offset of the sine wave of individual channels (e.g. true vs. fake stereo)
    #[cfg(feature = "experimental-aec3-config")]
    fn test_aec3_configuration_tuning(num_channels: usize, sample_frame_signal_offset: f32) {
        // Enable multichannel processing, otherwise AEC simply downmixes.
        let pipeline = config::Pipeline {
            multi_channel_render: num_channels > 1,
            multi_channel_capture: num_channels > 1,
            ..Default::default()
        };
        // Shorten the "true stereo" detection from the default 2 seconds. AEC3 starts with the
        // single-channel config by default and only switches to multichannel with hysteresis.
        // Our warmup interval is only 0.5 secs of audio (50 frames).
        let base_aec3_config = {
            let mut mutable = experimental::EchoCanceller3Config::default();
            mutable.multi_channel.stereo_detection_hysteresis_seconds = 0.1;
            mutable
        };

        // Test strong suppression
        let strong_reduction = {
            let config = Config {
                pipeline,
                echo_canceller: Some(EchoCanceller::default()),
                ..Default::default()
            };
            let mut aec3_config = base_aec3_config;
            // Aggressive suppression
            aec3_config.suppressor.normal_tuning.mask_lf.enr_suppress = 5.0;
            aec3_config.suppressor.normal_tuning.mask_hf.enr_suppress = 5.0;

            let mut context = TestContext::new(num_channels, Some(aec3_config));
            let render_frame = context.generate_sine_frame(440.0, sample_frame_signal_offset);
            context.processor.set_config(config);
            context.measure_steady_state_performance(&render_frame, 50, 10)
        };

        // Test light suppression
        let light_reduction = {
            let config = Config {
                pipeline,
                echo_canceller: Some(EchoCanceller::default()),
                ..Default::default()
            };
            let mut aec3_config = base_aec3_config;
            // Very light suppression
            aec3_config.suppressor.normal_tuning.mask_lf.enr_suppress = 0.1;
            aec3_config.suppressor.normal_tuning.mask_hf.enr_suppress = 0.1;

            let mut context = TestContext::new(num_channels, Some(aec3_config));
            let render_frame = context.generate_sine_frame(440.0, sample_frame_signal_offset);
            context.processor.set_config(config);
            context.measure_steady_state_performance(&render_frame, 50, 10)
        };

        // Verify the configurations produce measurably different results
        assert!(
            strong_reduction > light_reduction + 3.0,
            "Strong suppression ({:.1} dB) should achieve at least 3dB more reduction than light \
             suppression ({:.1} dB)",
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
        let render_frame = context.generate_sine_frame(440.0, 0.0);
        let mut capture_frame = render_frame.clone();

        // Configure initial Full mode (default)
        context.processor.set_config(Config {
            echo_canceller: Some(EchoCanceller::default()),
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
            echo_canceller: Some(EchoCanceller::Mobile { stream_delay_ms: 0 }),
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
