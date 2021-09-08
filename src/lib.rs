//! This crate is a wrapper around [PulseAudio's repackaging of WebRTC's AudioProcessing module](https://www.freedesktop.org/software/pulseaudio/webrtc-audio-processing/).
//!
//! See `examples/simple.rs` for an example of how to use the library.

#![warn(clippy::all)]
#![warn(missing_docs)]

mod config;
mod stats;

use std::{error, fmt, sync::Arc};
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
        let inner = Arc::new(AudioProcessing::new(config)?);
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

impl AudioProcessing {
    fn new(config: &InitializationConfig) -> Result<Self, Error> {
        let mut code = 0;
        let inner = unsafe {
            ffi::audio_processing_create(
                config.num_capture_channels as i32,
                config.num_render_channels as i32,
                config.sample_rate_hz as i32,
                &mut code,
            )
        };
        if !inner.is_null() {
            Ok(Self { inner })
        } else {
            Err(Error { code })
        }
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

    #[test]
    fn test_create_failure() {
        let config = init_config(0);
        assert!(Processor::new(&config).is_err());
    }

    #[test]
    fn test_create_drop() {
        let config = init_config(1);
        let _p = Processor::new(&config).unwrap();
    }

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
}
