//! This crate provides config structs for `webrtc-audio-processing` without any FFI and with only
//! minimal dependencies. Handy when you want to configure it from e.g. WASM project.

#![warn(clippy::all)]
#![warn(missing_docs)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The parameters and behavior of the audio processing module are controlled
/// by changing the default values in this `Config` struct.
/// The config is applied by passing the struct to the [`set_config`] method.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct Config {
    /// Sets the properties of the audio processing pipeline.
    pub pipeline: Pipeline,

    /// Enables and configures capture-side pre-amplifier/capture-level adjustment.
    pub capture_amplifier: Option<CaptureAmplifier>,

    /// Enables and configures high pass filter.
    pub high_pass_filter: Option<HighPassFilter>,

    /// Enables and configures acoustic echo cancellation.
    pub echo_canceller: Option<EchoCanceller>,

    /// Enables and configures background noise suppression.
    pub noise_suppression: Option<NoiseSuppression>,

    /// Enables and configures automatic gain control (v1 or v2).
    pub gain_controller: Option<GainController>,
}

/// Sets the properties of the audio processing pipeline.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct Pipeline {
    /// Maximum allowed processing rate used internally. May only be set to
    /// 32000 or 48000 and any differing values will be treated as 48000.
    pub maximum_internal_processing_rate: PipelineProcessingRate,

    /// Allow multi-channel processing of render audio.
    pub multi_channel_render: bool,

    /// Allow multi-channel processing of capture audio when AEC3 is active
    /// or a custom AEC is injected.
    pub multi_channel_capture: bool,

    /// Indicates how to downmix multi-channel capture audio to mono (when
    /// needed).
    pub capture_downmix_method: DownmixMethod,
}

/// Internal processing rate.
#[derive(Debug, Copy, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PipelineProcessingRate {
    /// Limit the rate to 32k Hz.
    Max32000Hz = 32_000,

    /// Limit the rate to 48k Hz.
    #[default]
    Max48000Hz = 48_000,
}

/// Downmix method for multi-channel capture audio.
#[derive(Debug, Copy, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DownmixMethod {
    /// Mix by averaging.
    #[default]
    Average,
    /// Mix by selecting the first channel.
    UseFirstChannel,
}

/// A choice of capture-side pre-amplification/volume adjustment.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CaptureAmplifier {
    /// Use the legacy PreAmplifier.
    PreAmplifier(PreAmplifier),
    /// Use the new CaptureLevelAdjustment.
    CaptureLevelAdjustment(CaptureLevelAdjustment),
}

/// The `PreAmplifier` amplifies the capture signal before any other processing is done.
/// TODO(webrtc:5298): Will be deprecated to use the pre-gain functionality
/// in capture_level_adjustment instead.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct PreAmplifier {
    /// Fixed linear gain multiplier. The default is 1.0 (no effect).
    pub fixed_gain_factor: f32,
}

impl Default for PreAmplifier {
    fn default() -> Self {
        Self { fixed_gain_factor: 1.0 }
    }
}

/// Functionality for general level adjustment in the capture pipeline. This
/// should not be used together with the legacy PreAmplifier functionality.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct CaptureLevelAdjustment {
    /// The `pre_gain_factor` scales the signal before any processing is done.
    pub pre_gain_factor: f32,

    /// The `post_gain_factor` scales the signal after all processing is done.
    pub post_gain_factor: f32,

    /// Analog mic gain emulation.
    pub analog_mic_gain_emulation: Option<AnalogMicGainEmulation>,
}

impl Default for CaptureLevelAdjustment {
    fn default() -> Self {
        Self { pre_gain_factor: 1.0, post_gain_factor: 1.0, analog_mic_gain_emulation: None }
    }
}

/// Analog mic gain emulation for capture level adjustment.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct AnalogMicGainEmulation {
    /// Initial analog gain level to use for the emulated analog gain. Must
    /// be in the range [0...255].
    pub initial_level: u8,
}

impl Default for AnalogMicGainEmulation {
    fn default() -> Self {
        Self { initial_level: 255 }
    }
}

/// HPF (high-pass filter) configuration.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct HighPassFilter {
    /// Whether or not HPF should be applied in the full-band (i.e. 20 – 20,000 Hz).
    pub apply_in_full_band: bool,
}

impl Default for HighPassFilter {
    fn default() -> Self {
        Self { apply_in_full_band: true }
    }
}

/// AEC (acoustic echo cancellation) configuration.
/// Defaults to Full (AEC3) mode with delay estimation (stream_delay unset).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum EchoCanceller {
    /// Use low-complexity AEC implementation that is optimized for mobile.
    Mobile {
        /// Set the delay in ms between process_render_frame() and process_capture_frame().
        /// Mandatory for the Mobile echo canceller variant.
        stream_delay_ms: u16,
    },

    /// Uses the full AEC3 implementation.
    Full {
        /// Set the delay in ms between process_render_frame() and process_capture_frame().
        /// If None, we let the AEC processor try determining it.
        stream_delay_ms: Option<u16>,
    },
}

impl Default for EchoCanceller {
    fn default() -> Self {
        Self::Full { stream_delay_ms: None }
    }
}

/// Enables background noise suppression.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct NoiseSuppression {
    /// Determines the aggressiveness of the suppression. Increasing the level will reduce the
    /// noise level at the expense of a higher speech distortion.
    pub level: NoiseSuppressionLevel,

    /// Analyze the output of the linear AEC instead of the capture frame. Has no effect if echo
    /// cancellation is not enabled.
    pub analyze_linear_aec_output: bool,
}

impl Default for NoiseSuppression {
    fn default() -> Self {
        Self { level: NoiseSuppressionLevel::Moderate, analyze_linear_aec_output: false }
    }
}

/// Noise suppression level.
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum NoiseSuppressionLevel {
    /// Lower suppression level.
    Low,
    /// Moderate suppression level.
    Moderate,
    /// Higher suppression level.
    High,
    /// Even higher suppression level.
    VeryHigh,
}

/// A choice of the gain controller implementation.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum GainController {
    /// Legacy gain controller 1.
    GainController1(GainController1),
    /// New gain controller 2.
    GainController2(GainController2),
}

/// Enables automatic gain control (AGC) functionality.
/// The automatic gain control (AGC) component brings the signal to an
/// appropriate range. This is done by applying a digital gain directly and,
/// in the analog mode, prescribing an analog gain to be applied at the audio
/// HAL.
/// Recommended to be enabled on the client-side.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct GainController1 {
    /// AGC mode.
    pub mode: GainControllerMode,

    /// Sets the target peak level (or envelope) of the AGC in dBFs (decibels
    /// from digital full-scale). The convention is to use positive values. For
    /// instance, passing in a value of 3 corresponds to -3 dBFs, or a target
    /// level 3 dB below full-scale. Limited to [0, 31].
    pub target_level_dbfs: u8,

    /// Sets the maximum gain the digital compression stage may apply, in dB. A
    /// higher number corresponds to greater compression, while a value of 0
    /// will leave the signal uncompressed. Limited to [0, 90].
    ///
    /// For updates after APM setup, the C++ upstream suggests using RuntimeSetting
    /// instead (which is not yet exposed in the Rust wrapper).
    pub compression_gain_db: u8,

    /// When enabled, the compression stage will hard limit the signal to the
    /// target level. Otherwise, the signal will be compressed but not limited
    /// above the target level.
    pub enable_limiter: bool,

    /// Analog gain controller configuration.
    pub analog_gain_controller: Option<AnalogGainController>,
}

impl Default for GainController1 {
    fn default() -> Self {
        Self {
            mode: GainControllerMode::AdaptiveAnalog,
            target_level_dbfs: 3,
            compression_gain_db: 9,
            enable_limiter: true,
            analog_gain_controller: None,
        }
    }
}

/// Gain control mode.
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum GainControllerMode {
    /// Adaptive mode intended for use if an analog volume control is
    /// available on the capture device. It will require the user to provide
    /// coupling between the OS mixer controls and AGC through the
    /// stream_analog_level() functions.
    /// It consists of an analog gain prescription for the audio device and a
    /// digital compression stage.
    AdaptiveAnalog,
    /// Adaptive mode intended for situations in which an analog volume
    /// control is unavailable. It operates in a similar fashion to the
    /// adaptive analog mode, but with scaling instead applied in the digital
    /// domain. As with the analog mode, it additionally uses a digital
    /// compression stage.
    AdaptiveDigital,
    /// Fixed mode which enables only the digital compression stage also used
    /// by the two adaptive modes.
    /// It is distinguished from the adaptive modes by considering only a
    /// short time-window of the input signal. It applies a fixed gain
    /// through most of the input level range, and compresses (gradually
    /// reduces gain with increasing level) the input signal at higher
    /// levels. This mode is preferred on embedded devices where the capture
    /// signal level is predictable, so that a known gain can be applied.
    FixedDigital,
}

/// Enables the analog gain controller functionality.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct AnalogGainController {
    /// TODO(bugs.webrtc.org/7494): Will be deprecated.
    pub startup_min_volume: i32,
    /// Lowest analog microphone level that will be applied in response to
    /// clipping.
    pub clipped_level_min: i32,
    /// If true, an adaptive digital gain is applied.
    pub enable_digital_adaptive: bool,
    /// Amount the microphone level is lowered with every clipping event.
    /// Limited to (0, 255].
    pub clipped_level_step: i32,
    /// Proportion of clipped samples required to declare a clipping event.
    /// Limited to (0.f, 1.f).
    pub clipped_ratio_threshold: f32,
    /// Time in frames to wait after a clipping event before checking again.
    /// Limited to values higher than 0.
    pub clipped_wait_frames: i32,
    /// Clipping predictor.
    pub clipping_predictor: Option<ClippingPredictor>,
}

impl Default for AnalogGainController {
    fn default() -> Self {
        Self {
            startup_min_volume: 0,
            clipped_level_min: 70,
            enable_digital_adaptive: true,
            clipped_level_step: 15,
            clipped_ratio_threshold: 0.1,
            clipped_wait_frames: 300,
            clipping_predictor: None,
        }
    }
}

/// Enables clipping prediction functionality.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct ClippingPredictor {
    /// Mode.
    pub mode: ClippingPredictorMode,
    /// Number of frames in the sliding analysis window.
    pub window_length: i32,
    /// Number of frames in the sliding reference window.
    pub reference_window_length: i32,
    /// Reference window delay (unit: number of frames).
    pub reference_window_delay: i32,
    /// Clipping prediction threshold (dBFS).
    pub clipping_threshold: f32,
    /// Crest factor drop threshold (dB).
    pub crest_factor_margin: f32,
    /// If true, the recommended clipped level step is used to modify the
    /// analog gain. Otherwise, the predictor runs without affecting the
    /// analog gain.
    pub use_predicted_step: bool,
}

impl Default for ClippingPredictor {
    fn default() -> Self {
        Self {
            mode: ClippingPredictorMode::ClippingEventPrediction,
            window_length: 5,
            reference_window_length: 5,
            reference_window_delay: 5,
            clipping_threshold: -1.0,
            crest_factor_margin: 3.0,
            use_predicted_step: true,
        }
    }
}

/// Clipping predictor mode.
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ClippingPredictorMode {
    /// Clipping event prediction mode with fixed step estimation.
    ClippingEventPrediction,
    /// Clipped peak estimation mode with adaptive step estimation.
    AdaptiveStepClippingPeakPrediction,
    /// Clipped peak estimation mode with fixed step estimation.
    FixedStepClippingPeakPrediction,
}

/// Parameters for AGC2, an Automatic Gain Control (AGC) sub-module which
/// replaces the AGC sub-module parameterized by `gain_controller1`.
/// AGC2 brings the captured audio signal to the desired level by combining
/// three different controllers (namely, input volume controller, adaptive
/// digital controller and fixed digital controller) and a limiter.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct GainController2 {
    /// Enables the input volume controller, which adjusts the input
    /// volume applied when the audio is captured (e.g., microphone volume on
    /// a soundcard, input volume on HAL).
    pub input_volume_controller_enabled: bool,
    /// Parameters for the adaptive digital controller, which adjusts and
    /// applies a digital gain after echo cancellation and after noise
    /// suppression.
    pub adaptive_digital: Option<AdaptiveDigital>,
    /// Parameters for the fixed digital controller, which applies a fixed
    /// digital gain after the adaptive digital controller and before the
    /// limiter.
    pub fixed_digital: FixedDigital,
}

/// Parameters for the adaptive digital controller, which adjusts and
/// applies a digital gain after echo cancellation and after noise
/// suppression.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct AdaptiveDigital {
    /// Headroom (dB).
    pub headroom_db: f32,
    /// Max gain (dB).
    pub max_gain_db: f32,
    /// Initial gain (dB).
    pub initial_gain_db: f32,
    /// Max gain change speed (dB/s).
    pub max_gain_change_db_per_second: f32,
    /// Max output noise level (dBFS).
    pub max_output_noise_level_dbfs: f32,
}

impl Default for AdaptiveDigital {
    fn default() -> Self {
        Self {
            headroom_db: 5.0,
            max_gain_db: 50.0,
            initial_gain_db: 15.0,
            max_gain_change_db_per_second: 6.0,
            max_output_noise_level_dbfs: -50.0,
        }
    }
}

/// Parameters for the fixed digital controller, which applies a fixed
/// digital gain after the adaptive digital controller and before the
/// limiter.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct FixedDigital {
    /// By setting `gain_db` to a value greater than zero, the limiter can be
    /// turned into a compressor that first applies a fixed gain.
    pub gain_db: f32,
}

impl Default for FixedDigital {
    fn default() -> Self {
        Self { gain_db: 0.0 }
    }
}
