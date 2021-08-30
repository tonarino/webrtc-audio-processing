use webrtc_audio_processing_sys as ffi;

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};

/// A configuration for initializing a Processor instance.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct InitializationConfig {
    /// Number of the input and output channels for the capture frame.
    pub num_capture_channels: usize,

    /// Number of the input and output channels for the render frame.
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
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub enum PipelineProcessingRate {
    /// Limit the rate to 32k Hz.
    Max32000Hz = 32_000,

    /// Limit the rate to 48k Hz.
    Max48000Hz = 48_000,
}

impl Default for PipelineProcessingRate {
    fn default() -> Self {
        // cf. https://gitlab.freedesktop.org/pulseaudio/webrtc-audio-processing/-/blob/master/webrtc/modules/audio_processing/include/audio_processing.cc#L55
        if cfg!(target_arch = "arm") {
            Self::Max32000Hz
        } else {
            Self::Max48000Hz
        }
    }
}

/// Sets the properties of the audio processing pipeline.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
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
    pub capture_downmix_method: ffi::AudioProcessing_Config_Pipeline_DownmixMethod,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self {
            maximum_internal_processing_rate: PipelineProcessingRate::default(),
            multi_channel_capture: false,
            multi_channel_render: false,
            capture_downmix_method: 0,
        }
    }
}

impl From<Pipeline> for ffi::AudioProcessing_Config_Pipeline {
    fn from(other: Pipeline) -> Self {
        Self {
            maximum_internal_processing_rate: other.maximum_internal_processing_rate as i32,
            multi_channel_capture: other.multi_channel_capture,
            multi_channel_render: other.multi_channel_render,
            capture_downmix_method: other.capture_downmix_method,
        }
    }
}

/// Enabled the pre-amplifier. It amplifies the capture signal
/// before any other processing is done.
/// TODO(webrtc:5298): Will be deprecated to use the pre-gain functionality
/// in capture_level_adjustment instead.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct PreAmplifier {
    /// Fixed linear gain multiplayer. The default is 1.0 (no effect).
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

/// HPF (high-pass fitler) configuration.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct HighPassFilter {
    /// HPF should be applied in the full-band (i.e. 20 – 20,000 Hz).
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
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub enum EchoCanceller {
    /// Uses low-complexity AEC implementation that is optimized for mobile.
    Mobile,

    /// Uses the full AEC3 implementation.
    Full {
        /// Enforce the highpass filter to be on (has no effect for the mobile
        /// mode).
        enforce_high_pass_filtering: bool,
    },
}

impl Default for EchoCanceller {
    fn default() -> Self {
        Self::Full { enforce_high_pass_filtering: true }
    }
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
            EchoCanceller::Full { enforce_high_pass_filtering } => Self {
                enabled: true,
                mobile_mode: false,
                enforce_high_pass_filtering,
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
pub struct InputVolumeController {
    /// Enabled.
    pub enabled: bool,
}

impl Default for InputVolumeController {
    fn default() -> Self {
        Self { enabled: false }
    }
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
/// replaces the AGC sub-module parametrized by `gain_controller1`.
/// AGC2 brings the captured audio signal to the desired level by combining
/// three different controllers (namely, input volume controller, adapative
/// digital controller and fixed digital controller) and a limiter.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
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

impl Default for GainController2 {
    fn default() -> Self {
        Self {
            enabled: false,
            input_volume_controller: InputVolumeController::default(),
            adaptive_digital: AdaptiveDigital::default(),
            fixed_digital: FixedDigital::default(),
        }
    }
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
    pub initial_level: i32,
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
        Self { enabled: other.enabled, initial_level: other.initial_level }
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
    #[serde(default)]
    pub pipeline: Pipeline,

    /// Enables and configures level adjustment in the capture pipeline.
    #[serde(default)]
    pub capture_level_adjustment: Option<CaptureLevelAdjustment>,

    /// Enables and configures high pass filter.
    #[serde(default)]
    pub high_pass_filter: Option<HighPassFilter>,

    /// Enables and configures acoustic echo cancellation.
    #[serde(default)]
    pub echo_canceller: Option<EchoCanceller>,

    /// Enables and configures background noise suppression.
    #[serde(default)]
    pub noise_suppression: Option<NoiseSuppression>,

    /// Enables and configures automatic gain control.
    #[serde(default)]
    pub gain_controller: Option<GainController>,

    /// Enables and configures Gain Controller 2.
    #[serde(default)]
    pub gain_controller2: Option<GainController2>,

    /// Fine-grained AEC3 configuration parameters.
    #[serde(default)]
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
        let transient_suppression = ffi::AudioProcessing_Config_TransientSuppression {
            enabled: false,
            ..Default::default()
        };

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

/// [Highly Experimental] Configurations of internal AEC3 implementation.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct EchoCanceller3Config {
    /// Delay configuration
    pub delay: Delay,

    /// Filter configuration
    pub filter: Filter,

    /// ERLE configuration
    pub erle: Erle,

    /// EP Strength configuration
    pub ep_strength: EpStrength,

    /// Echo Audibility configuration
    pub echo_audibility: EchoAudibility,

    /// Render Levels configuration
    pub render_levels: RenderLevels,

    /// Suppressor configuration
    pub suppressor: Suppressor,

    /// Buffering configuration
    pub buffering: Buffering,

    /// Comfort noise configuration
    pub comfort_noise: ComfortNoise,

    /// Echo Model configuration
    pub echo_model: EchoModel,

    /// Echo Removal Control configuration
    pub echo_removal_control: EchoRemovalControl,

    /// Multi-channel configuration
    pub multi_channel: MultiChannel,
}

impl Default for EchoCanceller3Config {
    fn default() -> Self {
        Self {
            delay: Delay::default(),
            filter: Filter::default(),
            erle: Erle::default(),
            ep_strength: EpStrength::default(),
            echo_audibility: EchoAudibility::default(),
            render_levels: RenderLevels::default(),
            suppressor: Suppressor::default(),
            buffering: Buffering::default(),
            comfort_noise: ComfortNoise { noise_floor_dbfs: -96.03406 },
            echo_model: EchoModel::default(),
            echo_removal_control: EchoRemovalControl {
                has_clock_drift: false,
                linear_and_stable_echo_path: false,
            },
            multi_channel: MultiChannel::default(),
        }
    }
}

impl From<EchoCanceller3Config> for ffi::EchoCanceller3ConfigOverride {
    fn from(other: EchoCanceller3Config) -> Self {
        Self {
            // Buffering
            buffering_excess_render_detection_interval_blocks: other
                .buffering
                .excess_render_detection_interval_blocks,
            buffering_max_allowed_excess_render_blocks: other
                .buffering
                .max_allowed_excess_render_blocks,

            // Delay
            delay_default_delay: other.delay.default_delay,
            delay_down_sampling_factor: other.delay.down_sampling_factor,
            delay_num_filters: other.delay.num_filters,
            delay_delay_headroom_samples: other.delay.delay_headroom_samples,
            delay_hysteresis_limit_blocks: other.delay.hysteresis_limit_blocks,
            delay_fixed_capture_delay_samples: other.delay.fixed_capture_delay_samples,
            delay_estimate_smoothing: other.delay.delay_estimate_smoothing,
            delay_estimate_smoothing_delay_found: other.delay.delay_estimate_smoothing_delay_found,
            delay_candidate_detection_threshold: other.delay.delay_candidate_detection_threshold,
            delay_selection_thresholds_initial: other.delay.delay_selection_thresholds.initial,
            delay_selection_thresholds_converged: other.delay.delay_selection_thresholds.converged,
            delay_use_external_delay_estimator: other.delay.use_external_delay_estimator,
            delay_log_warning_on_delay_changes: other.delay.log_warning_on_delay_changes,
            delay_detect_pre_echo: other.delay.detect_pre_echo,

            // Delay AlignmentMixing
            delay_render_alignment_mixing_downmix: other.delay.render_alignment_mixing.downmix,
            delay_render_alignment_mixing_adaptive_selection: other
                .delay
                .render_alignment_mixing
                .adaptive_selection,
            delay_render_alignment_mixing_activity_power_threshold: other
                .delay
                .render_alignment_mixing
                .activity_power_threshold,
            delay_render_alignment_mixing_prefer_first_two_channels: other
                .delay
                .render_alignment_mixing
                .prefer_first_two_channels,
            delay_capture_alignment_mixing_downmix: other.delay.capture_alignment_mixing.downmix,
            delay_capture_alignment_mixing_adaptive_selection: other
                .delay
                .capture_alignment_mixing
                .adaptive_selection,
            delay_capture_alignment_mixing_activity_power_threshold: other
                .delay
                .capture_alignment_mixing
                .activity_power_threshold,
            delay_capture_alignment_mixing_prefer_first_two_channels: other
                .delay
                .capture_alignment_mixing
                .prefer_first_two_channels,

            // Filter
            filter_refined_length_blocks: other.filter.refined.length_blocks,
            filter_refined_leakage_converged: other.filter.refined.leakage_converged,
            filter_refined_leakage_diverged: other.filter.refined.leakage_diverged,
            filter_refined_error_floor: other.filter.refined.error_floor,
            filter_refined_error_ceil: other.filter.refined.error_ceil,
            filter_refined_noise_gate: other.filter.refined.noise_gate,

            filter_coarse_length_blocks: other.filter.coarse.length_blocks,
            filter_coarse_rate: other.filter.coarse.rate,
            filter_coarse_noise_gate: other.filter.coarse.noise_gate,

            filter_refined_initial_length_blocks: other.filter.refined_initial.length_blocks,
            filter_refined_initial_leakage_converged: other
                .filter
                .refined_initial
                .leakage_converged,
            filter_refined_initial_leakage_diverged: other.filter.refined_initial.leakage_diverged,
            filter_refined_initial_error_floor: other.filter.refined_initial.error_floor,
            filter_refined_initial_error_ceil: other.filter.refined_initial.error_ceil,
            filter_refined_initial_noise_gate: other.filter.refined_initial.noise_gate,

            filter_coarse_initial_length_blocks: other.filter.coarse_initial.length_blocks,
            filter_coarse_initial_rate: other.filter.coarse_initial.rate,
            filter_coarse_initial_noise_gate: other.filter.coarse_initial.noise_gate,

            filter_config_change_duration_blocks: other.filter.config_change_duration_blocks,
            filter_initial_state_seconds: other.filter.initial_state_seconds,
            filter_coarse_reset_hangover_blocks: other.filter.coarse_reset_hangover_blocks,
            filter_conservative_initial_phase: other.filter.conservative_initial_phase,
            filter_enable_coarse_filter_output_usage: other
                .filter
                .enable_coarse_filter_output_usage,
            filter_use_linear_filter: other.filter.use_linear_filter,
            filter_high_pass_filter_echo_reference: other.filter.high_pass_filter_echo_reference,
            filter_export_linear_aec_output: other.filter.export_linear_aec_output,

            // Erle
            erle_min: other.erle.min,
            erle_max_l: other.erle.max_l,
            erle_max_h: other.erle.max_h,
            erle_onset_detection: other.erle.onset_detection,
            erle_num_sections: other.erle.num_sections,
            erle_clamp_quality_estimate_to_zero: other.erle.clamp_quality_estimate_to_zero,
            erle_clamp_quality_estimate_to_one: other.erle.clamp_quality_estimate_to_one,

            // EpStrength
            ep_strength_default_gain: other.ep_strength.default_gain,
            ep_strength_default_len: other.ep_strength.default_len,
            ep_strength_nearend_len: other.ep_strength.nearend_len,
            ep_strength_echo_can_saturate: other.ep_strength.echo_can_saturate,
            ep_strength_bounded_erl: other.ep_strength.bounded_erl,
            ep_strength_erle_onset_compensation_in_dominant_nearend: other
                .ep_strength
                .erle_onset_compensation_in_dominant_nearend,
            ep_strength_use_conservative_tail_frequency_response: other
                .ep_strength
                .use_conservative_tail_frequency_response,

            // EchoAudibility
            echo_audibility_low_render_limit: other.echo_audibility.low_render_limit,
            echo_audibility_normal_render_limit: other.echo_audibility.normal_render_limit,
            echo_audibility_floor_power: other.echo_audibility.floor_power,
            echo_audibility_audibility_threshold_lf: other.echo_audibility.audibility_threshold_lf,
            echo_audibility_audibility_threshold_mf: other.echo_audibility.audibility_threshold_mf,
            echo_audibility_audibility_threshold_hf: other.echo_audibility.audibility_threshold_hf,
            echo_audibility_use_stationarity_properties: other
                .echo_audibility
                .use_stationarity_properties,
            echo_audibility_use_stationarity_properties_at_init: other
                .echo_audibility
                .use_stationarity_properties_at_init,

            // RenderLevels
            render_levels_active_render_limit: other.render_levels.active_render_limit,
            render_levels_poor_excitation_render_limit: other
                .render_levels
                .poor_excitation_render_limit,
            render_levels_poor_excitation_render_limit_ds8: other
                .render_levels
                .poor_excitation_render_limit_ds8,
            render_levels_render_power_gain_db: other.render_levels.render_power_gain_db,

            // EchoRemovalControl
            echo_removal_control_has_clock_drift: other.echo_removal_control.has_clock_drift,
            echo_removal_control_linear_and_stable_echo_path: other
                .echo_removal_control
                .linear_and_stable_echo_path,

            // EchoModel
            echo_model_noise_floor_hold: other.echo_model.noise_floor_hold,
            echo_model_min_noise_floor_power: other.echo_model.min_noise_floor_power,
            echo_model_stationary_gate_slope: other.echo_model.stationary_gate_slope,
            echo_model_noise_gate_power: other.echo_model.noise_gate_power,
            echo_model_noise_gate_slope: other.echo_model.noise_gate_slope,
            echo_model_render_pre_window_size: other.echo_model.render_pre_window_size,
            echo_model_render_post_window_size: other.echo_model.render_post_window_size,
            echo_model_model_reverb_in_nonlinear_mode: other
                .echo_model
                .model_reverb_in_nonlinear_mode,

            // ComfortNoise
            comfort_noise_noise_floor_dbfs: other.comfort_noise.noise_floor_dbfs,

            // Suppressor
            suppressor_nearend_average_blocks: other.suppressor.nearend_average_blocks,

            // Suppressor Normal Tuning
            suppressor_normal_tuning_mask_lf_enr_transparent: other
                .suppressor
                .normal_tuning
                .mask_lf
                .enr_transparent,
            suppressor_normal_tuning_mask_lf_enr_suppress: other
                .suppressor
                .normal_tuning
                .mask_lf
                .enr_suppress,
            suppressor_normal_tuning_mask_lf_emr_transparent: other
                .suppressor
                .normal_tuning
                .mask_lf
                .emr_transparent,
            suppressor_normal_tuning_mask_hf_enr_transparent: other
                .suppressor
                .normal_tuning
                .mask_hf
                .enr_transparent,
            suppressor_normal_tuning_mask_hf_enr_suppress: other
                .suppressor
                .normal_tuning
                .mask_hf
                .enr_suppress,
            suppressor_normal_tuning_mask_hf_emr_transparent: other
                .suppressor
                .normal_tuning
                .mask_hf
                .emr_transparent,
            suppressor_normal_tuning_max_inc_factor: other.suppressor.normal_tuning.max_inc_factor,
            suppressor_normal_tuning_max_dec_factor_lf: other
                .suppressor
                .normal_tuning
                .max_dec_factor_lf,

            // Suppressor Nearend Tuning
            suppressor_nearend_tuning_mask_lf_enr_transparent: other
                .suppressor
                .nearend_tuning
                .mask_lf
                .enr_transparent,
            suppressor_nearend_tuning_mask_lf_enr_suppress: other
                .suppressor
                .nearend_tuning
                .mask_lf
                .enr_suppress,
            suppressor_nearend_tuning_mask_lf_emr_transparent: other
                .suppressor
                .nearend_tuning
                .mask_lf
                .emr_transparent,
            suppressor_nearend_tuning_mask_hf_enr_transparent: other
                .suppressor
                .nearend_tuning
                .mask_hf
                .enr_transparent,
            suppressor_nearend_tuning_mask_hf_enr_suppress: other
                .suppressor
                .nearend_tuning
                .mask_hf
                .enr_suppress,
            suppressor_nearend_tuning_mask_hf_emr_transparent: other
                .suppressor
                .nearend_tuning
                .mask_hf
                .emr_transparent,
            suppressor_nearend_tuning_max_inc_factor: other
                .suppressor
                .nearend_tuning
                .max_inc_factor,
            suppressor_nearend_tuning_max_dec_factor_lf: other
                .suppressor
                .nearend_tuning
                .max_dec_factor_lf,

            // Suppressor Smoothing
            suppressor_lf_smoothing_during_initial_phase: other
                .suppressor
                .lf_smoothing_during_initial_phase,
            suppressor_last_permanent_lf_smoothing_band: other
                .suppressor
                .last_permanent_lf_smoothing_band,
            suppressor_last_lf_smoothing_band: other.suppressor.last_lf_smoothing_band,
            suppressor_last_lf_band: other.suppressor.last_lf_band,
            suppressor_first_hf_band: other.suppressor.first_hf_band,

            // Suppressor DominantNearendDetection
            suppressor_dominant_nearend_detection_enr_threshold: other
                .suppressor
                .dominant_nearend_detection
                .enr_threshold,
            suppressor_dominant_nearend_detection_enr_exit_threshold: other
                .suppressor
                .dominant_nearend_detection
                .enr_exit_threshold,
            suppressor_dominant_nearend_detection_snr_threshold: other
                .suppressor
                .dominant_nearend_detection
                .snr_threshold,
            suppressor_dominant_nearend_detection_hold_duration: other
                .suppressor
                .dominant_nearend_detection
                .hold_duration,
            suppressor_dominant_nearend_detection_trigger_threshold: other
                .suppressor
                .dominant_nearend_detection
                .trigger_threshold,
            suppressor_dominant_nearend_detection_use_during_initial_phase: other
                .suppressor
                .dominant_nearend_detection
                .use_during_initial_phase,
            suppressor_dominant_nearend_detection_use_unbounded_echo_spectrum: other
                .suppressor
                .dominant_nearend_detection
                .use_unbounded_echo_spectrum,

            // Suppressor SubbandNearendDetection
            suppressor_subband_nearend_detection_nearend_average_blocks: other
                .suppressor
                .subband_nearend_detection
                .nearend_average_blocks,
            suppressor_subband_nearend_detection_subband1_low: other
                .suppressor
                .subband_nearend_detection
                .subband1
                .low,
            suppressor_subband_nearend_detection_subband1_high: other
                .suppressor
                .subband_nearend_detection
                .subband1
                .high,
            suppressor_subband_nearend_detection_subband2_low: other
                .suppressor
                .subband_nearend_detection
                .subband2
                .low,
            suppressor_subband_nearend_detection_subband2_high: other
                .suppressor
                .subband_nearend_detection
                .subband2
                .high,
            suppressor_subband_nearend_detection_nearend_threshold: other
                .suppressor
                .subband_nearend_detection
                .nearend_threshold,
            suppressor_subband_nearend_detection_snr_threshold: other
                .suppressor
                .subband_nearend_detection
                .snr_threshold,

            suppressor_use_subband_nearend_detection: other
                .suppressor
                .use_subband_nearend_detection,

            // Suppressor HighBandsSuppression
            suppressor_high_bands_suppression_enr_threshold: other
                .suppressor
                .high_bands_suppression
                .enr_threshold,
            suppressor_high_bands_suppression_max_gain_during_echo: other
                .suppressor
                .high_bands_suppression
                .max_gain_during_echo,
            suppressor_high_bands_suppression_anti_howling_activation_threshold: other
                .suppressor
                .high_bands_suppression
                .anti_howling_activation_threshold,
            suppressor_high_bands_suppression_anti_howling_gain: other
                .suppressor
                .high_bands_suppression
                .anti_howling_gain,

            suppressor_floor_first_increase: other.suppressor.floor_first_increase,
            suppressor_conservative_hf_suppression: other.suppressor.conservative_hf_suppression,

            // MultiChannel
            multi_channel_detect_stereo_content: other.multi_channel.detect_stereo_content,
            multi_channel_stereo_detection_threshold: other
                .multi_channel
                .stereo_detection_threshold,
            multi_channel_stereo_detection_timeout_threshold_seconds: other
                .multi_channel
                .stereo_detection_timeout_threshold_seconds,
            multi_channel_stereo_detection_hysteresis_seconds: other
                .multi_channel
                .stereo_detection_hysteresis_seconds,
        }
    }
}

impl EchoCanceller3Config {
    /// Validates the configuration values.
    /// Returns true if all values are within acceptable ranges.
    pub fn validate(&self) -> bool {
        // TODO: Implement validation logic matching C++
        true
    }
}

/// Buffering configuration
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct Buffering {
    /// Excess render detection interval in blocks
    pub excess_render_detection_interval_blocks: usize,

    /// Maximum allowed excess render blocks
    pub max_allowed_excess_render_blocks: usize,
}

impl Default for Buffering {
    fn default() -> Self {
        Self { excess_render_detection_interval_blocks: 250, max_allowed_excess_render_blocks: 8 }
    }
}

/// Delay configuration
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct Delay {
    /// Initial default delay estimate in blocks
    pub default_delay: usize,

    /// Downsampling factor for delay estimation (must be either 4 or 8)
    pub down_sampling_factor: usize,

    /// Number of filters for delay estimation
    pub num_filters: usize,

    /// Additional headroom for delay estimation in samples
    pub delay_headroom_samples: usize,

    /// Hysteresis for delay changes in blocks
    pub hysteresis_limit_blocks: usize,

    /// Fixed capture delay in samples (0 for adaptive delay estimation)
    pub fixed_capture_delay_samples: usize,

    /// Smoothing factor for delay estimates (0.0-1.0)
    pub delay_estimate_smoothing: f32,

    /// Smoothing factor for delay estimates (0.0-1.0)
    pub delay_estimate_smoothing_delay_found: f32,

    /// Detection threshold for delay candidates (0.0-1.0)
    pub delay_candidate_detection_threshold: f32,

    /// Delay selection thresholds
    pub delay_selection_thresholds: DelaySelectionThresholds,

    /// Whether to use external delay estimator
    pub use_external_delay_estimator: bool,

    /// Whether to log warning on delay changes
    pub log_warning_on_delay_changes: bool,

    /// Whether to detect pre-echo
    pub detect_pre_echo: bool,

    /// Render alignment mixing configuration
    pub render_alignment_mixing: AlignmentMixing,

    /// Capture alignment mixing configuration
    pub capture_alignment_mixing: AlignmentMixing,
}

impl Default for Delay {
    fn default() -> Self {
        Self {
            default_delay: 5,
            down_sampling_factor: 4,
            num_filters: 5,
            delay_headroom_samples: 32,
            hysteresis_limit_blocks: 1,
            fixed_capture_delay_samples: 0,
            delay_estimate_smoothing: 0.7,
            delay_estimate_smoothing_delay_found: 0.7,
            delay_candidate_detection_threshold: 0.2,
            delay_selection_thresholds: Default::default(),
            use_external_delay_estimator: false,
            log_warning_on_delay_changes: false,
            render_alignment_mixing: AlignmentMixing {
                downmix: false,
                adaptive_selection: true,
                activity_power_threshold: 10000.0,
                prefer_first_two_channels: true,
            },
            capture_alignment_mixing: AlignmentMixing {
                downmix: false,
                adaptive_selection: true,
                activity_power_threshold: 10000.0,
                prefer_first_two_channels: false,
            },
            detect_pre_echo: true,
        }
    }
}

/// Delay selection thresholds configuration
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct DelaySelectionThresholds {
    /// Initial threshold
    pub initial: i32,
    /// Converged threshold
    pub converged: i32,
}

impl Default for DelaySelectionThresholds {
    fn default() -> Self {
        Self { initial: 5, converged: 20 }
    }
}

/// Configuration for filter alignment and mixing settings
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct AlignmentMixing {
    /// Whether to downmix the signal
    pub downmix: bool,

    /// Whether to use adaptive channel selection
    pub adaptive_selection: bool,

    /// Power threshold for activity detection
    pub activity_power_threshold: f32,

    /// Whether to prioritize the first two channels in processing
    pub prefer_first_two_channels: bool,
}

impl Default for AlignmentMixing {
    fn default() -> Self {
        Self {
            downmix: false,
            adaptive_selection: true,
            activity_power_threshold: 10000.0,
            prefer_first_two_channels: true,
        }
    }
}

/// Configuration for the main adaptive filter component that models the echo path
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct Filter {
    /// Configuration for the refined filter stage
    pub refined: RefinedConfiguration,

    /// Configuration for the coarse filter stage
    pub coarse: CoarseConfiguration,

    /// Initial configuration for the refined filter
    pub refined_initial: RefinedConfiguration,

    /// Initial configuration for the coarse filter
    pub coarse_initial: CoarseConfiguration,

    /// Duration in blocks for configuration changes to take effect
    pub config_change_duration_blocks: usize,

    /// Duration of initial state in seconds
    pub initial_state_seconds: f32,

    /// Duration in blocks for coarse filter reset hangover
    pub coarse_reset_hangover_blocks: i32,

    /// Whether to use conservative settings during initial phase
    pub conservative_initial_phase: bool,

    /// Whether to enable usage of coarse filter output
    pub enable_coarse_filter_output_usage: bool,

    /// Whether to use linear filtering
    pub use_linear_filter: bool,

    /// Whether to apply high-pass filter to echo reference
    pub high_pass_filter_echo_reference: bool,

    /// Whether to export linear AEC output
    pub export_linear_aec_output: bool,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            refined: RefinedConfiguration::default(),
            coarse: CoarseConfiguration::default(),
            refined_initial: RefinedConfiguration {
                length_blocks: 12,
                leakage_converged: 0.005,
                leakage_diverged: 0.5,
                error_floor: 0.001,
                error_ceil: 2.0,
                noise_gate: 20075344.0,
            },
            coarse_initial: CoarseConfiguration {
                length_blocks: 12,
                rate: 0.9,
                noise_gate: 20075344.0,
            },
            config_change_duration_blocks: 250,
            initial_state_seconds: 2.5,
            coarse_reset_hangover_blocks: 25,
            conservative_initial_phase: false,
            enable_coarse_filter_output_usage: true,
            use_linear_filter: true,
            high_pass_filter_echo_reference: false,
            export_linear_aec_output: false,
        }
    }
}

/// Configuration for the refined filter stage
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct RefinedConfiguration {
    /// Length in blocks
    pub length_blocks: usize,

    /// Leakage when filter is converged
    pub leakage_converged: f32,

    /// Leakage when filter is diverged
    pub leakage_diverged: f32,

    /// Error floor
    pub error_floor: f32,

    /// Error ceiling
    pub error_ceil: f32,

    /// Noise gate threshold
    pub noise_gate: f32,
}

impl Default for RefinedConfiguration {
    fn default() -> Self {
        Self {
            length_blocks: 13,
            leakage_converged: 0.00005,
            leakage_diverged: 0.05,
            error_floor: 0.001,
            error_ceil: 2.0,
            noise_gate: 20075344.0,
        }
    }
}

/// Configuration for the coarse filter stage
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct CoarseConfiguration {
    /// Length in blocks
    pub length_blocks: usize,

    /// Filter adaptation rate
    pub rate: f32,

    /// Noise gate threshold
    pub noise_gate: f32,
}

impl Default for CoarseConfiguration {
    fn default() -> Self {
        Self { length_blocks: 13, rate: 0.7, noise_gate: 20075344.0 }
    }
}

/// ERLE (Echo Return Loss Enhancement) configuration
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct Erle {
    /// Minimum ERLE value
    pub min: f32,

    /// Maximum ERLE for lower frequencies
    pub max_l: f32,

    /// Maximum ERLE for higher frequencies
    pub max_h: f32,

    /// Whether to use onset detection
    pub onset_detection: bool,

    /// Number of frequency sections for ERLE estimation
    pub num_sections: usize,

    /// Whether to clamp quality estimate to zero
    pub clamp_quality_estimate_to_zero: bool,

    /// Whether to clamp quality estimate to one
    pub clamp_quality_estimate_to_one: bool,
}

impl Default for Erle {
    fn default() -> Self {
        Self {
            min: 1.0,
            max_l: 4.0,
            max_h: 1.5,
            onset_detection: true,
            num_sections: 1,
            clamp_quality_estimate_to_zero: true,
            clamp_quality_estimate_to_one: true,
        }
    }
}

/// Echo Path strength configuration.
/// Controls how the system adapts to changes in the echo path.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct EpStrength {
    /// Default gain value
    pub default_gain: f32,

    /// Default echo path strength.
    pub default_len: f32,

    /// Default nearend echo path strength.
    pub nearend_len: f32,

    /// Whether echo can saturate.
    pub echo_can_saturate: bool,

    /// Whether to use bounded ERL.
    pub bounded_erl: bool,

    /// Whether to use ERL onset compensation in dominant nearend.
    pub erle_onset_compensation_in_dominant_nearend: bool,

    /// Whether to use conservative tail frequency response.
    pub use_conservative_tail_frequency_response: bool,
}

impl Default for EpStrength {
    fn default() -> Self {
        Self {
            default_gain: 1.0,
            default_len: 0.83,
            nearend_len: 0.83,
            echo_can_saturate: true,
            bounded_erl: false,
            erle_onset_compensation_in_dominant_nearend: false,
            use_conservative_tail_frequency_response: true,
        }
    }
}

/// Echo audibility configuration.
/// Controls how the system detects and handles audible echo.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct EchoAudibility {
    /// Low render limit for echo detection
    pub low_render_limit: f32,

    /// Normal render limit for echo detection
    pub normal_render_limit: f32,

    /// Floor power for echo detection
    pub floor_power: f32,

    /// Audibility threshold for low frequencies
    pub audibility_threshold_lf: f32,

    /// Audibility threshold for mid frequencies
    pub audibility_threshold_mf: f32,

    /// Audibility threshold for high frequencies
    pub audibility_threshold_hf: f32,

    /// Whether to use stationarity properties.
    pub use_stationarity_properties: bool,

    /// Whether to use stationarity properties at initialization.
    pub use_stationarity_properties_at_init: bool,
}

impl Default for EchoAudibility {
    fn default() -> Self {
        Self {
            low_render_limit: 4.0 * 64.0,
            normal_render_limit: 64.0,
            floor_power: 2.0 * 64.0,
            audibility_threshold_lf: 10.0,
            audibility_threshold_mf: 10.0,
            audibility_threshold_hf: 10.0,
            use_stationarity_properties: false,
            use_stationarity_properties_at_init: false,
        }
    }
}

/// Render levels configuration
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct RenderLevels {
    /// Active render limit
    pub active_render_limit: f32,

    /// Poor excitation render limit
    pub poor_excitation_render_limit: f32,

    /// Poor excitation render limit for downsampled signals
    pub poor_excitation_render_limit_ds8: f32,

    /// Render power gain in dB
    pub render_power_gain_db: f32,
}

impl Default for RenderLevels {
    fn default() -> Self {
        Self {
            active_render_limit: 100.0,
            poor_excitation_render_limit: 150.0,
            poor_excitation_render_limit_ds8: 20.0,
            render_power_gain_db: 0.0,
        }
    }
}

/// Echo removal control configuration
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct EchoRemovalControl {
    /// Whether clock drift is present
    pub has_clock_drift: bool,

    /// Whether echo path is linear and stable
    pub linear_and_stable_echo_path: bool,
}

impl Default for EchoRemovalControl {
    fn default() -> Self {
        Self { has_clock_drift: false, linear_and_stable_echo_path: false }
    }
}

/// Echo model configuration
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct EchoModel {
    /// Noise floor hold time
    pub noise_floor_hold: usize,

    /// Minimum noise floor power
    pub min_noise_floor_power: f32,

    /// Stationary gate slope
    pub stationary_gate_slope: f32,

    /// Noise gate power
    pub noise_gate_power: f32,

    /// Noise gate slope
    pub noise_gate_slope: f32,

    /// Render pre-window size
    pub render_pre_window_size: usize,

    /// Render post-window size
    pub render_post_window_size: usize,

    /// Whether to model reverb in nonlinear mode
    pub model_reverb_in_nonlinear_mode: bool,
}

impl Default for EchoModel {
    fn default() -> Self {
        Self {
            noise_floor_hold: 50,
            min_noise_floor_power: 1638400.0,
            stationary_gate_slope: 10.0,
            noise_gate_power: 27509.42,
            noise_gate_slope: 0.3,
            render_pre_window_size: 1,
            render_post_window_size: 1,
            model_reverb_in_nonlinear_mode: true,
        }
    }
}

/// Comfort noise configuration
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct ComfortNoise {
    /// Noise floor level in dBFS
    pub noise_floor_dbfs: f32,
}

impl Default for ComfortNoise {
    fn default() -> Self {
        Self { noise_floor_dbfs: -96.03406 }
    }
}

/// Configuration for the echo suppressor component, which removes residual echo
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct Suppressor {
    /// Number of blocks to average for nearend detection
    pub nearend_average_blocks: usize,

    /// Tuning parameters for normal operation
    pub normal_tuning: Tuning,

    /// Tuning parameters for nearend speech
    pub nearend_tuning: Tuning,

    /// Whether to use LF smoothing during initial phase
    pub lf_smoothing_during_initial_phase: bool,

    /// Last permanent LF smoothing band
    pub last_permanent_lf_smoothing_band: i32,

    /// Last LF smoothing band
    pub last_lf_smoothing_band: i32,

    /// Last LF band
    pub last_lf_band: i32,

    /// First HF band
    pub first_hf_band: i32,

    /// Configuration for dominant nearend detection
    pub dominant_nearend_detection: DominantNearendDetection,

    /// Configuration for subband-based nearend detection
    pub subband_nearend_detection: SubbandNearendDetection,

    /// Whether to use subband-based nearend detection
    pub use_subband_nearend_detection: bool,

    /// Configuration for high frequency bands suppression
    pub high_bands_suppression: HighBandsSuppression,

    /// Initial floor increase rate
    pub floor_first_increase: f32,

    /// Whether to use conservative high frequency suppression
    pub conservative_hf_suppression: bool,
}

impl Default for Suppressor {
    fn default() -> Self {
        Self {
            nearend_average_blocks: 4,
            normal_tuning: Tuning::default(),
            nearend_tuning: Tuning {
                mask_lf: MaskingThresholds {
                    enr_transparent: 1.09,
                    enr_suppress: 1.1,
                    emr_transparent: 0.3,
                },
                mask_hf: MaskingThresholds {
                    enr_transparent: 0.1,
                    enr_suppress: 0.3,
                    emr_transparent: 0.3,
                },
                max_inc_factor: 2.0,
                max_dec_factor_lf: 0.25,
            },
            lf_smoothing_during_initial_phase: true,
            last_permanent_lf_smoothing_band: 0,
            last_lf_smoothing_band: 5,
            last_lf_band: 5,
            first_hf_band: 8,
            dominant_nearend_detection: DominantNearendDetection::default(),
            subband_nearend_detection: SubbandNearendDetection::default(),
            use_subband_nearend_detection: false,
            high_bands_suppression: HighBandsSuppression::default(),
            floor_first_increase: 0.00001,
            conservative_hf_suppression: false,
        }
    }
}

/// Masking thresholds configuration
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct MaskingThresholds {
    /// Transparent energy ratio threshold
    pub enr_transparent: f32,

    /// Suppression energy ratio threshold
    pub enr_suppress: f32,

    /// Transparent error-to-mask ratio threshold
    pub emr_transparent: f32,
}

impl Default for MaskingThresholds {
    fn default() -> Self {
        Self { enr_transparent: 0.3, enr_suppress: 0.4, emr_transparent: 0.3 }
    }
}

/// Tuning parameters for echo suppression, controlling how aggressively echo is removed
/// in different frequency bands and how quickly the suppression can change
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct Tuning {
    /// Low-frequency masking thresholds
    pub mask_lf: MaskingThresholds,

    /// High-frequency masking thresholds
    pub mask_hf: MaskingThresholds,

    /// Maximum increment factor for gain changes
    pub max_inc_factor: f32,

    /// Maximum decrement factor for low frequencies
    pub max_dec_factor_lf: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            mask_lf: MaskingThresholds {
                enr_transparent: 0.3,
                enr_suppress: 0.4,
                emr_transparent: 0.3,
            },
            mask_hf: MaskingThresholds {
                enr_transparent: 0.07,
                enr_suppress: 0.1,
                emr_transparent: 0.3,
            },
            max_inc_factor: 2.0,
            max_dec_factor_lf: 0.25,
        }
    }
}

/// Configuration for dominant nearend speech detection
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct DominantNearendDetection {
    /// Echo-to-noise ratio threshold
    pub enr_threshold: f32,

    /// Echo-to-noise ratio threshold for exiting detection state
    pub enr_exit_threshold: f32,

    /// Signal-to-noise ratio threshold
    pub snr_threshold: f32,

    /// Duration to hold detection state
    pub hold_duration: i32,

    /// Threshold for triggering detection
    pub trigger_threshold: i32,

    /// Whether to use during initial processing phase
    pub use_during_initial_phase: bool,

    /// Whether to use unbounded echo spectrum
    pub use_unbounded_echo_spectrum: bool,
}

impl Default for DominantNearendDetection {
    fn default() -> Self {
        Self {
            enr_threshold: 0.25,
            enr_exit_threshold: 10.0,
            snr_threshold: 30.0,
            hold_duration: 50,
            trigger_threshold: 12,
            use_during_initial_phase: true,
            use_unbounded_echo_spectrum: true,
        }
    }
}

/// Configuration for subband-based nearend detection
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct SubbandNearendDetection {
    /// Number of blocks to average for nearend detection
    pub nearend_average_blocks: usize,

    /// First subband region configuration
    pub subband1: SubbandRegion,

    /// Second subband region configuration
    pub subband2: SubbandRegion,

    /// Nearend threshold
    pub nearend_threshold: f32,

    /// Signal-to-noise ratio threshold
    pub snr_threshold: f32,
}

impl Default for SubbandNearendDetection {
    fn default() -> Self {
        Self {
            nearend_average_blocks: 1,
            subband1: SubbandRegion::default(),
            subband2: SubbandRegion::default(),
            nearend_threshold: 1.0,
            snr_threshold: 1.0,
        }
    }
}

/// Configuration for a subband frequency region
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct SubbandRegion {
    /// Lower frequency bound of the subband region
    pub low: usize,

    /// Upper frequency bound of the subband region
    pub high: usize,
}

impl Default for SubbandRegion {
    fn default() -> Self {
        Self { low: 1, high: 1 }
    }
}

/// Configuration for high frequency bands suppression
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct HighBandsSuppression {
    /// Echo-to-noise ratio threshold
    pub enr_threshold: f32,

    /// Maximum gain allowed during echo
    pub max_gain_during_echo: f32,

    /// Threshold for anti-howling activation
    pub anti_howling_activation_threshold: f32,

    /// Gain applied for anti-howling
    pub anti_howling_gain: f32,
}

impl Default for HighBandsSuppression {
    fn default() -> Self {
        Self {
            enr_threshold: 1.0,
            max_gain_during_echo: 1.0,
            anti_howling_activation_threshold: 400.0,
            anti_howling_gain: 1.0,
        }
    }
}

/// Multi-channel configuration
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "derive_serde", serde(default))]
pub struct MultiChannel {
    /// Whether to detect stereo content
    pub detect_stereo_content: bool,

    /// Threshold for stereo detection
    pub stereo_detection_threshold: f32,

    /// Timeout threshold for stereo detection in seconds
    pub stereo_detection_timeout_threshold_seconds: i32,

    /// Hysteresis for stereo detection in seconds
    pub stereo_detection_hysteresis_seconds: f32,
}

impl Default for MultiChannel {
    fn default() -> Self {
        Self {
            detect_stereo_content: true,
            stereo_detection_threshold: 0.0,
            stereo_detection_timeout_threshold_seconds: 300,
            stereo_detection_hysteresis_seconds: 2.0,
        }
    }
}
