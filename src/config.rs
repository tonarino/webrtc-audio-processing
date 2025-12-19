use std::ops::{Deref, DerefMut};

use webrtc_audio_processing_sys as ffi;

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};

/// A configuration for initializing a Processor instance.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct InitializationConfig {
    /// Number of input channels for the capture frame.
    pub num_capture_channels: usize,

    /// Number of output channels for the render frame.
    pub num_render_channels: usize,

    /// Sampling rate of the capture and render frames. Accepts an arbitrary value, but the maximum
    /// internal processing rate is 48000, so the audio quality is capped as such.
    pub sample_rate_hz: u32,
}

impl Default for InitializationConfig {
    fn default() -> Self {
        Self { num_capture_channels: 1, num_render_channels: 1, sample_rate_hz: 48_000 }
    }
}

/// Internal processing rate.
#[derive(Debug, Copy, Clone, Default, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub enum PipelineProcessingRate {
    /// Limit the rate to 32k Hz.
    Max32000Hz = 32_000,

    /// Limit the rate to 48k Hz.
    #[default]
    Max48000Hz = 48_000,
}

/// Downmix method for multi-channel capture audio.
#[derive(Debug, Copy, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub enum DownmixMethod {
    /// Mix by averaging.
    #[default]
    Average,
    /// Mix by selecting the first channel.
    UseFirstChannel,
}

impl From<DownmixMethod> for ffi::AudioProcessing_Config_Pipeline_DownmixMethod {
    fn from(method: DownmixMethod) -> ffi::AudioProcessing_Config_Pipeline_DownmixMethod {
        match method {
            DownmixMethod::Average => {
                ffi::AudioProcessing_Config_Pipeline_DownmixMethod_kAverageChannels
            },
            DownmixMethod::UseFirstChannel => {
                ffi::AudioProcessing_Config_Pipeline_DownmixMethod_kUseFirstChannel
            },
        }
    }
}

/// Sets the properties of the audio processing pipeline.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
#[derive(Default)]
pub struct Pipeline {
    /// Maximum allowed processing rate used internally. May only be set to
    /// 32000 or 48000 and any differing values will be treated as 48000.
    pub maximum_internal_processing_rate: PipelineProcessingRate,

    /// Allow multi-channel processing of capture audio when AEC3 is active
    /// or a custom AEC is injected.
    pub multi_channel_capture: bool,

    /// Allow multi-channel processing of render audio.
    pub multi_channel_render: bool,

    /// Indicates how to downmix multi-channel capture audio to mono (when
    /// needed).
    pub capture_downmix_method: DownmixMethod,
}

impl From<Pipeline> for ffi::AudioProcessing_Config_Pipeline {
    fn from(pipeline: Pipeline) -> Self {
        Self {
            maximum_internal_processing_rate: pipeline.maximum_internal_processing_rate as i32,
            multi_channel_capture: pipeline.multi_channel_capture,
            multi_channel_render: pipeline.multi_channel_render,
            capture_downmix_method: pipeline.capture_downmix_method.into(),
        }
    }
}

/// The `PreAmplifier` amplifies the capture signal before any other processing is done.
/// TODO(webrtc:5298): Will be deprecated to use the pre-gain functionality
/// in capture_level_adjustment instead.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct PreAmplifier {
    /// Fixed linear gain multiplier. The default is 1.0 (no effect).
    pub fixed_gain_factor: f32,
}

impl Default for PreAmplifier {
    fn default() -> Self {
        Self { fixed_gain_factor: 1.0 }
    }
}

impl From<PreAmplifier> for ffi::AudioProcessing_Config_PreAmplifier {
    fn from(other: PreAmplifier) -> Self {
        Self { enabled: true, fixed_gain_factor: other.fixed_gain_factor }
    }
}

/// HPF (high-pass filter) configuration.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct HighPassFilter {
    /// Whether or not HPF should be applied in the full-band (i.e. 20 – 20,000 Hz).
    pub apply_in_full_band: bool,
}

impl Default for HighPassFilter {
    fn default() -> Self {
        Self { apply_in_full_band: true }
    }
}

impl From<HighPassFilter> for ffi::AudioProcessing_Config_HighPassFilter {
    fn from(other: HighPassFilter) -> Self {
        Self { enabled: true, apply_in_full_band: other.apply_in_full_band }
    }
}

/// AEC (acoustic echo cancellation) configuration.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub enum EchoCanceller {
    /// Uses low-complexity AEC implementation that is optimized for mobile.
    Mobile,

    /// Uses the full AEC3 implementation.
    #[default]
    Full,
}

impl From<EchoCanceller> for ffi::AudioProcessing_Config_EchoCanceller {
    fn from(other: EchoCanceller) -> Self {
        match other {
            EchoCanceller::Mobile => Self {
                enabled: true,
                mobile_mode: true,
                enforce_high_pass_filtering: false,
                export_linear_aec_output: false,
            },
            EchoCanceller::Full => Self {
                enabled: true,
                mobile_mode: false,
                enforce_high_pass_filtering: true,
                export_linear_aec_output: false,
            },
        }
    }
}

/// Noise suppression level.
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
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

impl From<NoiseSuppressionLevel> for ffi::AudioProcessing_Config_NoiseSuppression_Level {
    fn from(other: NoiseSuppressionLevel) -> Self {
        match other {
            NoiseSuppressionLevel::Low => ffi::AudioProcessing_Config_NoiseSuppression_Level_kLow,
            NoiseSuppressionLevel::Moderate => {
                ffi::AudioProcessing_Config_NoiseSuppression_Level_kModerate
            },
            NoiseSuppressionLevel::High => ffi::AudioProcessing_Config_NoiseSuppression_Level_kHigh,
            NoiseSuppressionLevel::VeryHigh => {
                ffi::AudioProcessing_Config_NoiseSuppression_Level_kVeryHigh
            },
        }
    }
}

/// Enables background noise suppression.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
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

impl From<NoiseSuppression> for ffi::AudioProcessing_Config_NoiseSuppression {
    fn from(other: NoiseSuppression) -> Self {
        Self {
            enabled: true,
            level: other.level.into(),
            analyze_linear_aec_output_when_available: other.analyze_linear_aec_output,
        }
    }
}

/// Gain control mode.
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
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

impl From<GainControllerMode> for ffi::AudioProcessing_Config_GainController1_Mode {
    fn from(other: GainControllerMode) -> Self {
        match other {
            GainControllerMode::AdaptiveAnalog => {
                ffi::AudioProcessing_Config_GainController1_Mode_kAdaptiveAnalog
            },
            GainControllerMode::AdaptiveDigital => {
                ffi::AudioProcessing_Config_GainController1_Mode_kAdaptiveDigital
            },
            GainControllerMode::FixedDigital => {
                ffi::AudioProcessing_Config_GainController1_Mode_kFixedDigital
            },
        }
    }
}

/// Clipping predictor mode.
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub enum ClippingPredictorMode {
    /// Clipping event prediction mode with fixed step estimation.
    ClippingEventPrediction,
    /// Clipped peak estimation mode with adaptive step estimation.
    AdaptiveStepClippingPeakPrediction,
    /// Clipped peak estimation mode with fixed step estimation.
    FixedStepClippingPeakPrediction,
}

impl From<ClippingPredictorMode>
    for ffi::AudioProcessing_Config_GainController1_AnalogGainController_ClippingPredictor_Mode
{
    fn from(other: ClippingPredictorMode) -> Self {
        match other {
            ClippingPredictorMode::ClippingEventPrediction => ffi::AudioProcessing_Config_GainController1_AnalogGainController_ClippingPredictor_Mode_kClippingEventPrediction,
            ClippingPredictorMode::AdaptiveStepClippingPeakPrediction => ffi::AudioProcessing_Config_GainController1_AnalogGainController_ClippingPredictor_Mode_kAdaptiveStepClippingPeakPrediction,
            ClippingPredictorMode::FixedStepClippingPeakPrediction => ffi::AudioProcessing_Config_GainController1_AnalogGainController_ClippingPredictor_Mode_kFixedStepClippingPeakPrediction,
        }
    }
}

/// Enables clipping prediction functionality.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct ClippingPredictor {
    /// Enabled.
    pub enabled: bool,
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
            enabled: false,
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

impl From<ClippingPredictor>
    for ffi::AudioProcessing_Config_GainController1_AnalogGainController_ClippingPredictor
{
    fn from(other: ClippingPredictor) -> Self {
        Self {
            enabled: other.enabled,
            mode: other.mode.into(),
            window_length: other.window_length,
            reference_window_length: other.reference_window_length,
            reference_window_delay: other.reference_window_delay,
            clipping_threshold: other.clipping_threshold,
            crest_factor_margin: other.crest_factor_margin,
            use_predicted_step: other.use_predicted_step,
        }
    }
}

/// Enables the analog gain controller functionality.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct AnalogGainController {
    /// Enabled.
    pub enabled: bool,
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
    pub clipping_predictor: ClippingPredictor,
}

impl Default for AnalogGainController {
    fn default() -> Self {
        Self {
            enabled: true,
            startup_min_volume: 0,
            clipped_level_min: 70,
            enable_digital_adaptive: true,
            clipped_level_step: 15,
            clipped_ratio_threshold: 0.1,
            clipped_wait_frames: 300,
            clipping_predictor: ClippingPredictor::default(),
        }
    }
}

impl From<AnalogGainController>
    for ffi::AudioProcessing_Config_GainController1_AnalogGainController
{
    fn from(other: AnalogGainController) -> Self {
        Self {
            enabled: other.enabled,
            startup_min_volume: other.startup_min_volume,
            clipped_level_min: other.clipped_level_min,
            enable_digital_adaptive: other.enable_digital_adaptive,
            clipped_level_step: other.clipped_level_step,
            clipped_ratio_threshold: other.clipped_ratio_threshold,
            clipped_wait_frames: other.clipped_wait_frames,
            clipping_predictor: other.clipping_predictor.into(),
        }
    }
}

/// Enables automatic gain control (AGC) functionality.
/// The automatic gain control (AGC) component brings the signal to an
/// appropriate range. This is done by applying a digital gain directly and,
/// in the analog mode, prescribing an analog gain to be applied at the audio
/// HAL.
/// Recommended to be enabled on the client-side.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct GainController {
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
    /// For updates after APM setup, use a RuntimeSetting instead.
    pub compression_gain_db: u8,

    /// When enabled, the compression stage will hard limit the signal to the
    /// target level. Otherwise, the signal will be compressed but not limited
    /// above the target level.
    pub enable_limiter: bool,

    /// Analog gain controller configuration.
    pub analog_gain_controller: AnalogGainController,
}

impl Default for GainController {
    fn default() -> Self {
        Self {
            mode: GainControllerMode::AdaptiveAnalog,
            target_level_dbfs: 3,
            compression_gain_db: 9,
            enable_limiter: true,
            analog_gain_controller: AnalogGainController::default(),
        }
    }
}

impl From<GainController> for ffi::AudioProcessing_Config_GainController1 {
    fn from(other: GainController) -> Self {
        Self {
            enabled: true,
            mode: other.mode.into(),
            target_level_dbfs: other.target_level_dbfs as i32,
            compression_gain_db: other.compression_gain_db as i32,
            enable_limiter: other.enable_limiter,
            analog_gain_controller: other.analog_gain_controller.into(),
        }
    }
}

/// Parameters for the input volume controller, which adjusts the input
/// volume applied when the audio is captured (e.g., microphone volume on
/// a soundcard, input volume on HAL).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
#[derive(Default)]
pub struct InputVolumeController {
    /// Enabled.
    pub enabled: bool,
}

impl From<InputVolumeController>
    for ffi::AudioProcessing_Config_GainController2_InputVolumeController
{
    fn from(other: InputVolumeController) -> Self {
        Self { enabled: other.enabled }
    }
}

/// Parameters for the adaptive digital controller, which adjusts and
/// applies a digital gain after echo cancellation and after noise
/// suppression.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct AdaptiveDigital {
    /// Enabled.
    pub enabled: bool,
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
            enabled: false,
            headroom_db: 5.0,
            max_gain_db: 50.0,
            initial_gain_db: 15.0,
            max_gain_change_db_per_second: 6.0,
            max_output_noise_level_dbfs: -50.0,
        }
    }
}

impl From<AdaptiveDigital> for ffi::AudioProcessing_Config_GainController2_AdaptiveDigital {
    fn from(other: AdaptiveDigital) -> Self {
        Self {
            enabled: other.enabled,
            headroom_db: other.headroom_db,
            max_gain_db: other.max_gain_db,
            initial_gain_db: other.initial_gain_db,
            max_gain_change_db_per_second: other.max_gain_change_db_per_second,
            max_output_noise_level_dbfs: other.max_output_noise_level_dbfs,
        }
    }
}

/// Parameters for the fixed digital controller, which applies a fixed
/// digital gain after the adaptive digital controller and before the
/// limiter.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
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

impl From<FixedDigital> for ffi::AudioProcessing_Config_GainController2_FixedDigital {
    fn from(other: FixedDigital) -> Self {
        Self { gain_db: other.gain_db }
    }
}

/// Parameters for AGC2, an Automatic Gain Control (AGC) sub-module which
/// replaces the AGC sub-module parameterized by `gain_controller1`.
/// AGC2 brings the captured audio signal to the desired level by combining
/// three different controllers (namely, input volume controller, adaptive
/// digital controller and fixed digital controller) and a limiter.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
#[derive(Default)]
pub struct GainController2 {
    /// AGC2 must be created if and only if `enabled` is true.
    pub enabled: bool,
    /// Parameters for the input volume controller, which adjusts the input
    /// volume applied when the audio is captured (e.g., microphone volume on
    /// a soundcard, input volume on HAL).
    pub input_volume_controller: InputVolumeController,
    /// Parameters for the adaptive digital controller, which adjusts and
    /// applies a digital gain after echo cancellation and after noise
    /// suppression.
    pub adaptive_digital: AdaptiveDigital,
    /// Parameters for the fixed digital controller, which applies a fixed
    /// digital gain after the adaptive digital controller and before the
    /// limiter.
    pub fixed_digital: FixedDigital,
}

impl From<GainController2> for ffi::AudioProcessing_Config_GainController2 {
    fn from(other: GainController2) -> Self {
        Self {
            enabled: other.enabled,
            input_volume_controller: other.input_volume_controller.into(),
            adaptive_digital: other.adaptive_digital.into(),
            fixed_digital: other.fixed_digital.into(),
        }
    }
}

/// Analog mic gain emulation for capture level adjustment.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct AnalogMicGainEmulation {
    /// Enabled.
    pub enabled: bool,
    /// Initial analog gain level to use for the emulated analog gain. Must
    /// be in the range [0...255].
    pub initial_level: u8,
}

impl Default for AnalogMicGainEmulation {
    fn default() -> Self {
        Self { enabled: false, initial_level: 255 }
    }
}

impl From<AnalogMicGainEmulation>
    for ffi::AudioProcessing_Config_CaptureLevelAdjustment_AnalogMicGainEmulation
{
    fn from(other: AnalogMicGainEmulation) -> Self {
        Self { enabled: other.enabled, initial_level: other.initial_level as i32 }
    }
}

/// Functionality for general level adjustment in the capture pipeline. This
/// should not be used together with the legacy PreAmplifier functionality.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct CaptureLevelAdjustment {
    /// The `pre_gain_factor` scales the signal before any processing is done.
    pub pre_gain_factor: f32,

    /// The `post_gain_factor` scales the signal after all processing is done.
    pub post_gain_factor: f32,

    /// Analog mic gain emulation.
    pub analog_mic_gain_emulation: AnalogMicGainEmulation,
}

impl Default for CaptureLevelAdjustment {
    fn default() -> Self {
        Self {
            pre_gain_factor: 1.0,
            post_gain_factor: 1.0,
            analog_mic_gain_emulation: AnalogMicGainEmulation::default(),
        }
    }
}

impl From<CaptureLevelAdjustment> for ffi::AudioProcessing_Config_CaptureLevelAdjustment {
    fn from(other: CaptureLevelAdjustment) -> Self {
        Self {
            enabled: true,
            pre_gain_factor: other.pre_gain_factor,
            post_gain_factor: other.post_gain_factor,
            analog_mic_gain_emulation: other.analog_mic_gain_emulation.into(),
        }
    }
}

/// The parameters and behavior of the audio processing module are controlled
/// by changing the default values in this `Config` struct.
/// The config is applied by passing the struct to the [`set_config`] method.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct Config {
    /// Sets the properties of the audio processing pipeline.
    #[cfg_attr(feature = "derive_serde", serde(default))]
    pub pipeline: Pipeline,

    /// Enables and configures level adjustment in the capture pipeline.
    #[cfg_attr(feature = "derive_serde", serde(default))]
    pub capture_level_adjustment: Option<CaptureLevelAdjustment>,

    /// Enables and configures high pass filter.
    #[cfg_attr(feature = "derive_serde", serde(default))]
    pub high_pass_filter: Option<HighPassFilter>,

    /// Enables and configures acoustic echo cancellation.
    #[cfg_attr(feature = "derive_serde", serde(default))]
    pub echo_canceller: Option<EchoCanceller>,

    /// Enables and configures background noise suppression.
    #[cfg_attr(feature = "derive_serde", serde(default))]
    pub noise_suppression: Option<NoiseSuppression>,

    /// Enables and configures automatic gain control.
    #[cfg_attr(feature = "derive_serde", serde(default))]
    pub gain_controller: Option<GainController>,

    /// Enables and configures Gain Controller 2.
    #[cfg_attr(feature = "derive_serde", serde(default))]
    pub gain_controller2: Option<GainController2>,

    /// Fine-grained AEC3 configuration parameters.
    #[cfg_attr(feature = "derive_serde", serde(default))]
    pub aec3_config: Option<EchoCanceller3Config>,
}

impl From<Config> for ffi::AudioProcessing_Config {
    fn from(other: Config) -> Self {
        // PreAmplifier is being deprecated.
        let pre_amplifier =
            ffi::AudioProcessing_Config_PreAmplifier { enabled: false, ..Default::default() };

        let capture_level_adjustment = if let Some(config) = other.capture_level_adjustment {
            config.into()
        } else {
            ffi::AudioProcessing_Config_CaptureLevelAdjustment {
                enabled: false,
                ..Default::default()
            }
        };

        let high_pass_filter = if let Some(config) = other.high_pass_filter {
            config.into()
        } else {
            ffi::AudioProcessing_Config_HighPassFilter { enabled: false, ..Default::default() }
        };

        let echo_canceller = if let Some(config) = other.echo_canceller {
            let mut echo_canceller = ffi::AudioProcessing_Config_EchoCanceller::from(config);
            echo_canceller.export_linear_aec_output = if let Some(ns) = &other.noise_suppression {
                ns.analyze_linear_aec_output
            } else {
                false
            };
            echo_canceller
        } else {
            ffi::AudioProcessing_Config_EchoCanceller { enabled: false, ..Default::default() }
        };

        let noise_suppression = if let Some(config) = other.noise_suppression {
            config.into()
        } else {
            ffi::AudioProcessing_Config_NoiseSuppression { enabled: false, ..Default::default() }
        };

        // Transient suppressor is being deprecated.
        let transient_suppression =
            ffi::AudioProcessing_Config_TransientSuppression { enabled: false };

        let gain_controller1 = if let Some(config) = other.gain_controller {
            config.into()
        } else {
            ffi::AudioProcessing_Config_GainController1 { enabled: false, ..Default::default() }
        };

        let gain_controller2 = if let Some(config) = other.gain_controller2 {
            config.into()
        } else {
            ffi::AudioProcessing_Config_GainController2 { enabled: false, ..Default::default() }
        };

        Self {
            pipeline: other.pipeline.into(),
            pre_amplifier,
            capture_level_adjustment,
            high_pass_filter,
            echo_canceller,
            noise_suppression,
            transient_suppression,
            gain_controller1,
            gain_controller2,
        }
    }
}

/// [Highly experimental]
/// Exposes a finer-grained control of the internal AEC3 configuration.
/// It's minimally documented and highly experimental, and we don't yet provide Rust-idiomatic API.
/// If you want to create a new instance of `EchoCanceller3Config`, and only modify
/// some of the fields you are interested in, you need to do in the following way:
///
/// ```
/// let mut aec3_config = EchoCanceller3Config::default();
/// aec3_config.suppressor.dominant_nearend_detection.enr_threshold = 0.25;
/// aec3_config.suppressor.dominant_nearend_detection.snr_threshold = 30.0;
/// assert!(aec3_config.validate());
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct EchoCanceller3Config(pub ffi::EchoCanceller3Config);

impl EchoCanceller3Config {
    /// Checks and updates the config parameters to lie within (mostly) reasonable ranges.
    /// Returns true if and only of the config did not need to be changed.
    pub fn validate(&mut self) -> bool {
        unsafe { ffi::validate_aec3_config(&mut self.0 as *mut ffi::EchoCanceller3Config) }
    }
}

impl Default for EchoCanceller3Config {
    fn default() -> Self {
        Self(unsafe { ffi::create_aec3_config() })
    }
}

impl Deref for EchoCanceller3Config {
    type Target = ffi::EchoCanceller3Config;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EchoCanceller3Config {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// [Highly experimental] Expose all of the inner structs of the EchoCanceller3Config.
// These do not have Default implementations and other ergonomic Rust APIs.
pub use ffi::EchoCanceller3Config_Buffering;
pub use ffi::EchoCanceller3Config_ComfortNoise;
pub use ffi::EchoCanceller3Config_Delay;
pub use ffi::EchoCanceller3Config_Delay_AlignmentMixing;
pub use ffi::EchoCanceller3Config_Delay_DelaySelectionThresholds;
pub use ffi::EchoCanceller3Config_EchoAudibility;
pub use ffi::EchoCanceller3Config_EchoModel;
pub use ffi::EchoCanceller3Config_EchoRemovalControl;
pub use ffi::EchoCanceller3Config_EpStrength;
pub use ffi::EchoCanceller3Config_Erle;
pub use ffi::EchoCanceller3Config_Filter;
pub use ffi::EchoCanceller3Config_Filter_CoarseConfiguration;
pub use ffi::EchoCanceller3Config_Filter_RefinedConfiguration;
pub use ffi::EchoCanceller3Config_MultiChannel;
pub use ffi::EchoCanceller3Config_RenderLevels;
pub use ffi::EchoCanceller3Config_Suppressor;
pub use ffi::EchoCanceller3Config_Suppressor_DominantNearendDetection;
pub use ffi::EchoCanceller3Config_Suppressor_HighBandsSuppression;
pub use ffi::EchoCanceller3Config_Suppressor_MaskingThresholds;
pub use ffi::EchoCanceller3Config_Suppressor_SubbandNearendDetection;
pub use ffi::EchoCanceller3Config_Suppressor_SubbandNearendDetection_SubbandRegion;
pub use ffi::EchoCanceller3Config_Suppressor_Tuning;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aec3_config_default() {
        let default_aec3_config = EchoCanceller3Config::default();
        // Check if the default values are pulled from the C/C++ code rather than the rust defaults.
        assert_eq!(8, default_aec3_config.buffering.max_allowed_excess_render_blocks);
        assert!(default_aec3_config.delay.detect_pre_echo);
        assert_eq!(1.0, default_aec3_config.erle.min);
    }
}
