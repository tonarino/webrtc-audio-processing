use webrtc_audio_processing_sys as ffi;

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};

/// A configuration for initializing a Processor instance.
#[derive(Debug, Clone, PartialEq)]
pub struct InitializationConfig {
    /// Number of the input and output channels for the capture frame.
    pub num_capture_channels: usize,
    /// Number of the input and output channels for the render frame.
    pub num_render_channels: usize,
    /// Sampling rate of the capture and render frames. Accepts an arbitrary value, but the maximum
    /// internal processing rate is 48000, so the audio quality is capped as such.
    pub sample_rate_hz: u32,
}

/// Audio processing pipeline configuration.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub struct Pipeline {
    /// Maximum allowed processing rate used internally. May only be set to 32000 or 48000 and any
    /// differing values will be treated as 48000. The default rate is currently selected based on
    /// the CPU architecture.
    pub maximum_internal_processing_rate: u32,

    /// Allow multi-channel processing of render audio.
    pub multi_channel_render: bool,

    /// Allow multi-channel processing of capture audio when AEC3 is active.
    pub multi_channel_capture: bool,
}

impl Default for Pipeline {
    fn default() -> Self {
        // cf. https://gitlab.freedesktop.org/pulseaudio/webrtc-audio-processing/-/blob/master/webrtc/modules/audio_processing/include/audio_processing.cc#L55
        let maximum_internal_processing_rate =
            if cfg!(target_arch = "arm") { 32_000 } else { 48_000 };
        Self {
            maximum_internal_processing_rate,
            multi_channel_render: false,
            multi_channel_capture: false,
        }
    }
}

impl From<Pipeline> for ffi::AudioProcessing_Config_Pipeline {
    fn from(other: Pipeline) -> Self {
        Self {
            maximum_internal_processing_rate: other.maximum_internal_processing_rate as i32,
            multi_channel_render: other.multi_channel_render,
            multi_channel_capture: other.multi_channel_capture,
        }
    }
}

/// Pre-amplifier configuration.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub struct PreAmplifier {
    /// Fixed linear gain multiplifier. The default has no effect.
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
pub struct EchoCanceller {
    /// Uses AEC implementation that is optimized for mobile.
    pub mobile_mode: bool,
    /// Export the output of linear AEC for custom processing.
    pub export_linear_aec_output: bool,
    /// Enforce the highpass filter to be on. It has no effect for the mobile mode.
    pub enforce_high_pass_filtering: bool,
}

impl Default for EchoCanceller {
    fn default() -> Self {
        Self {
            mobile_mode: false,
            export_linear_aec_output: false,
            enforce_high_pass_filtering: true,
        }
    }
}

impl From<EchoCanceller> for ffi::AudioProcessing_Config_EchoCanceller {
    fn from(other: EchoCanceller) -> Self {
        Self {
            enabled: true,
            mobile_mode: other.mobile_mode,
            export_linear_aec_output: other.export_linear_aec_output,
            enforce_high_pass_filtering: other.enforce_high_pass_filtering,
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

/// Noise suppression configuration.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub struct NoiseSuppression {
    /// Determines the aggressiveness of the suppression. Increasing the level will reduce the
    /// noise level at the expense of a higher speech distortion.
    pub level: NoiseSuppressionLevel,
    /// Analyze the output of the linear AEC instead of the capture frame. Has no effect if
    /// echo_canceller.export_linear_aec_output is false.
    pub analyze_linear_aec_output_when_available: bool,
}

impl Default for NoiseSuppression {
    fn default() -> Self {
        Self {
            level: NoiseSuppressionLevel::Moderate,
            analyze_linear_aec_output_when_available: false,
        }
    }
}

impl From<NoiseSuppression> for ffi::AudioProcessing_Config_NoiseSuppression {
    fn from(other: NoiseSuppression) -> Self {
        Self {
            enabled: true,
            level: other.level.into(),
            analyze_linear_aec_output_when_available: other
                .analyze_linear_aec_output_when_available,
        }
    }
}

/// Gain control mode.
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub enum GainControllerMode {
    /// Adaptive mode intended for use if an analog volume control is available on the capture
    /// device. It will require the user to provide coupling between the OS mixer controls and AGC
    /// through the stream_analog_level() functions. It consists of an analog gain prescription for
    /// the audio device and a digital compression stage.
    /// TODO: this mode is not supported yet.
    AdaptiveAnalog,
    /// Adaptive mode intended for situations in which an analog volume control is unavailable. It
    /// operates in a similar fashion to the adaptive analog mode, but with scaling instead applied
    /// in the digital domain. As with the analog mode, it additionally uses a digital compression
    /// stage.
    AdaptiveDigital,
    /// Fixed mode which enables only the digital compression stage also used by the two adaptive
    /// modes. It is distinguished from the adaptive modes by considering only a short time-window
    /// of the input signal. It applies a fixed gain through most of the input level range, and
    /// compresses (gradually reduces gain with increasing level) the input signal at higher
    /// levels. This mode is preferred on embedded devices where the capture signal level is
    /// predictable, so that a known gain can be applied.
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

/// AGC (automatic gain control) configuration.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub struct GainController {
    /// AGC mode.
    pub mode: GainControllerMode,

    /// Sets the target peak level (or envelope) of the AGC in dBFs (decibels from digital
    /// full-scale). The convention is to use positive values. For instance, passing in a value of
    /// 3 corresponds to -3 dBFs, or a target level 3 dB below full-scale. Limited to [0, 31].
    pub target_level_dbfs: u32,

    /// Sets the maximum gain the digital compression stage may apply, in dB. A higher number
    /// corresponds to greater compression, while a value of 0 will leave the signal uncompressed.
    /// Limited to [0, 90]. For updates after APM setup, use a RuntimeSetting instead.
    pub compression_gain_db: u32,

    /// When enabled, the compression stage will hard limit the signal to the target level.
    /// Otherwise, the signal will be compressed but not limited above the target level.
    pub enable_limiter: bool,
}

impl Default for GainController {
    fn default() -> Self {
        Self {
            mode: GainControllerMode::AdaptiveDigital,
            target_level_dbfs: 3,
            compression_gain_db: 9,
            enable_limiter: true,
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
            ..Default::default()
        }
    }
}

/// The parameters and behavior of the audio processing module are controlled
/// by changing the default values in this `Config` struct.
/// The config is applied by passing the struct to the [`set_config`] method.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub struct Config {
    /// Sets the properties of the audio processing pipeline.
    pub pipeline: Pipeline,

    /// Enables and configures the pre-amplifier. It amplifies the capture signal before any other
    /// processing is done.
    pub pre_amplifier: Option<PreAmplifier>,

    /// Enables and configures high pass filter.
    pub high_pass_filter: Option<HighPassFilter>,

    /// Enables and configures acoustic echo cancellation.
    pub echo_canceller: Option<EchoCanceller>,

    /// Enables and configures background noise suppression.
    pub noise_suppression: Option<NoiseSuppression>,

    /// Enables transient noise suppression.
    pub enable_transient_suppression: bool,

    /// Enables reporting of [`voice_detected`] in [`Stats`].
    pub enable_voice_detection: bool,

    /// Enables and configures automatic gain control.
    /// TODO: Experiment with and migrate to GainController2.
    pub gain_controller: Option<GainController>,

    /// Enables reporting of [`residual_echo_likelihood`] and
    /// [`residual_echo_likelihood_recent_max`] in [`Stats`].
    pub enable_residual_echo_detector: bool,

    /// Enables reporting of [`output_rms_dbfs`] in [`Stats`].
    pub enable_level_estimation: bool,
}

impl From<Config> for ffi::AudioProcessing_Config {
    fn from(other: Config) -> Self {
        let pre_amplifier = if let Some(config) = other.pre_amplifier {
            config.into()
        } else {
            ffi::AudioProcessing_Config_PreAmplifier { enabled: false, ..Default::default() }
        };

        let high_pass_filter = if let Some(config) = other.high_pass_filter {
            config.into()
        } else {
            ffi::AudioProcessing_Config_HighPassFilter { enabled: false, ..Default::default() }
        };

        let echo_canceller = if let Some(config) = other.echo_canceller {
            config.into()
        } else {
            ffi::AudioProcessing_Config_EchoCanceller { enabled: false, ..Default::default() }
        };

        let noise_suppression = if let Some(config) = other.noise_suppression {
            config.into()
        } else {
            ffi::AudioProcessing_Config_NoiseSuppression { enabled: false, ..Default::default() }
        };

        let transient_suppression = ffi::AudioProcessing_Config_TransientSuppression {
            enabled: other.enable_transient_suppression,
        };

        let voice_detection =
            ffi::AudioProcessing_Config_VoiceDetection { enabled: other.enable_voice_detection };

        let gain_controller1 = if let Some(config) = other.gain_controller {
            config.into()
        } else {
            ffi::AudioProcessing_Config_GainController1 { enabled: false, ..Default::default() }
        };

        let gain_controller2 =
            ffi::AudioProcessing_Config_GainController2 { enabled: false, ..Default::default() };

        let residual_echo_detector = ffi::AudioProcessing_Config_ResidualEchoDetector {
            enabled: other.enable_residual_echo_detector,
        };

        let level_estimation =
            ffi::AudioProcessing_Config_LevelEstimation { enabled: other.enable_level_estimation };

        Self {
            pipeline: other.pipeline.into(),
            pre_amplifier,
            high_pass_filter,
            echo_canceller,
            noise_suppression,
            transient_suppression,
            voice_detection,
            gain_controller1,
            gain_controller2,
            residual_echo_detector,
            level_estimation,
        }
    }
}

/// Statistics about the processor state.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub struct Stats {
    /// The root mean square (RMS) level in dBFS (decibels from digital full-scale) of the last
    /// capture frame, after processing. It is constrained to [-127, 0]. The computation follows:
    /// https://tools.ietf.org/html/rfc6465 with the intent that it can provide the RTP audio level
    /// indication. Only reported if level estimation is enabled in [`Config`].
    pub output_rms_dbfs: Option<i32>,

    /// True if voice is detected in the last capture frame, after processing. It is conservative
    /// in flagging audio as speech, with low likelihood of incorrectly flagging a frame as voice.
    /// Only reported if voice detection is enabled in [`Config`].
    pub voice_detected: Option<bool>,

    /// AEC stats: ERL = 10log_10(P_far / P_echo)
    pub echo_return_loss: Option<f64>,
    /// AEC stats: ERLE = 10log_10(P_echo / P_out)
    pub echo_return_loss_enhancement: Option<f64>,
    /// AEC stats: Fraction of time that the AEC linear filter is divergent, in a 1-second
    /// non-overlapped aggregation window.
    pub divergent_filter_fraction: Option<f64>,

    /// The delay median in milliseconds. The values are aggregated until the first call to
    /// [`get_stats()`] and afterwards aggregated and updated every second.
    pub delay_median_ms: Option<i32>,
    /// The delay standard deviation in milliseconds. The values are aggregated until the first
    /// call to [`get_stats()`] and afterwards aggregated and updated every second.
    pub delay_standard_deviation_ms: Option<i32>,

    /// Residual echo detector likelihood.
    pub residual_echo_likelihood: Option<f64>,
    /// Maximum residual echo likelihood from the last time period.
    pub residual_echo_likelihood_recent_max: Option<f64>,

    /// The instantaneous delay estimate produced in the AEC. The unit is in milliseconds and the
    /// value is the instantaneous value at the time of the call to [`get_stats()`].
    pub delay_ms: Option<i32>,
}

impl From<ffi::Stats> for Stats {
    fn from(other: ffi::Stats) -> Self {
        Self {
            output_rms_dbfs: other.output_rms_dbfs.into(),
            voice_detected: other.voice_detected.into(),
            echo_return_loss: other.echo_return_loss.into(),
            echo_return_loss_enhancement: other.echo_return_loss_enhancement.into(),
            divergent_filter_fraction: other.divergent_filter_fraction.into(),
            delay_median_ms: other.delay_median_ms.into(),
            delay_standard_deviation_ms: other.delay_standard_deviation_ms.into(),
            residual_echo_likelihood: other.residual_echo_likelihood.into(),
            residual_echo_likelihood_recent_max: other.residual_echo_likelihood_recent_max.into(),
            delay_ms: other.delay_ms.into(),
        }
    }
}
