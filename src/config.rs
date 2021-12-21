use webrtc_audio_processing_sys as ffi;

pub use ffi::InitializationConfig;

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};

/// A level of non-linear suppression during AEC (aka NLP).
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub enum EchoCancellationSuppressionLevel {
    /// Lowest suppression level.
    /// Minimum overdrive exponent = 1.0 (zero suppression).
    Lowest,
    /// Lower suppression level.
    /// Minimum overdrive exponent = 2.0.
    Lower,
    /// Low suppression level.
    /// Minimum overdrive exponent = 3.0.
    Low,
    /// Moderate suppression level.
    /// Minimum overdrive exponent = 6.0.
    Moderate,
    /// Higher suppression level.
    /// Minimum overdrive exponent = 15.0.
    High,
}

impl From<EchoCancellationSuppressionLevel> for ffi::EchoCancellation_SuppressionLevel {
    fn from(other: EchoCancellationSuppressionLevel) -> ffi::EchoCancellation_SuppressionLevel {
        match other {
            EchoCancellationSuppressionLevel::Lowest => {
                ffi::EchoCancellation_SuppressionLevel::LOWEST
            },
            EchoCancellationSuppressionLevel::Lower => {
                ffi::EchoCancellation_SuppressionLevel::LOWER
            },
            EchoCancellationSuppressionLevel::Low => ffi::EchoCancellation_SuppressionLevel::LOW,
            EchoCancellationSuppressionLevel::Moderate => {
                ffi::EchoCancellation_SuppressionLevel::MODERATE
            },
            EchoCancellationSuppressionLevel::High => ffi::EchoCancellation_SuppressionLevel::HIGH,
        }
    }
}

/// Echo cancellation configuration.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub struct EchoCancellation {
    /// Determines the aggressiveness of the suppressor. A higher level trades off
    /// double-talk performance for increased echo suppression.
    pub suppression_level: EchoCancellationSuppressionLevel,

    /// Use to enable the extended filter mode in the AEC, along with robustness
    /// measures around the reported system delays. It comes with a significant
    /// increase in AEC complexity, but is much more robust to unreliable reported
    /// delays.
    pub enable_extended_filter: bool,

    /// Enables delay-agnostic echo cancellation. This feature relies on internally
    /// estimated delays between the process and reverse streams, thus not relying
    /// on reported system delays.
    pub enable_delay_agnostic: bool,

    /// Sets the delay in ms between process_render_frame() receiving a far-end
    /// frame and process_capture_frame() receiving a near-end frame containing
    /// the corresponding echo. You should set this only if you are certain that
    /// the delay will be stable and constant. enable_delay_agnostic will be
    /// ignored when this option is set.
    pub stream_delay_ms: Option<i32>,
}

impl From<EchoCancellation> for ffi::EchoCancellation {
    fn from(other: EchoCancellation) -> ffi::EchoCancellation {
        ffi::EchoCancellation {
            enable: true,
            suppression_level: other.suppression_level.into(),
            enable_extended_filter: other.enable_extended_filter,
            enable_delay_agnostic: other.enable_delay_agnostic,
            stream_delay_ms: other.stream_delay_ms.into(),
        }
    }
}

/// Mode of gain control.
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub enum GainControlMode {
    /// Bring the signal to an appropriate range by applying an adaptive gain
    /// control. The volume is dynamically amplified with a microphone with
    /// small pickup and vice versa.
    AdaptiveDigital,

    /// Unlike ADAPTIVE_DIGITAL, it only compresses (i.e. gradually reduces
    /// gain with increasing level) the input signal when at higher levels.
    /// Use this where the capture signal level is predictable, so that a
    /// known gain can be applied.
    FixedDigital,
}

impl From<GainControlMode> for ffi::GainControl_Mode {
    fn from(other: GainControlMode) -> ffi::GainControl_Mode {
        match other {
            GainControlMode::AdaptiveDigital => ffi::GainControl_Mode::ADAPTIVE_DIGITAL,
            GainControlMode::FixedDigital => ffi::GainControl_Mode::FIXED_DIGITAL,
        }
    }
}

/// Gain control configuration.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub struct GainControl {
    /// Determines what type of gain control is applied.
    pub mode: GainControlMode,

    /// Sets the target peak level (or envelope) of the AGC in dBFs (decibels from
    /// digital full-scale). The convention is to use positive values.
    /// For instance, passing in a value of 3 corresponds to -3 dBFs, or a target
    /// level 3 dB below full-scale. Limited to [0, 31].
    pub target_level_dbfs: i32,

    /// Sets the maximum gain the digital compression stage may apply, in dB. A
    /// higher number corresponds to greater compression, while a value of 0 will
    /// leave the signal uncompressed. Limited to [0, 90].
    pub compression_gain_db: i32,

    /// When enabled, the compression stage will hard limit the signal to the
    /// target level. Otherwise, the signal will be compressed but not limited
    /// above the target level.
    pub enable_limiter: bool,
}

impl From<GainControl> for ffi::GainControl {
    fn from(other: GainControl) -> ffi::GainControl {
        ffi::GainControl {
            enable: true,
            mode: other.mode.into(),
            target_level_dbfs: other.target_level_dbfs,
            compression_gain_db: other.compression_gain_db,
            enable_limiter: other.enable_limiter,
        }
    }
}

/// A level of noise suppression.
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

impl From<NoiseSuppressionLevel> for ffi::NoiseSuppression_SuppressionLevel {
    fn from(other: NoiseSuppressionLevel) -> ffi::NoiseSuppression_SuppressionLevel {
        match other {
            NoiseSuppressionLevel::Low => ffi::NoiseSuppression_SuppressionLevel::LOW,
            NoiseSuppressionLevel::Moderate => ffi::NoiseSuppression_SuppressionLevel::MODERATE,
            NoiseSuppressionLevel::High => ffi::NoiseSuppression_SuppressionLevel::HIGH,
            NoiseSuppressionLevel::VeryHigh => ffi::NoiseSuppression_SuppressionLevel::VERY_HIGH,
        }
    }
}

/// Noise suppression configuration.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub struct NoiseSuppression {
    /// Determines the aggressiveness of the suppression. Increasing the level will
    /// reduce the noise level at the expense of a higher speech distortion.
    pub suppression_level: NoiseSuppressionLevel,
}

impl From<NoiseSuppression> for ffi::NoiseSuppression {
    fn from(other: NoiseSuppression) -> ffi::NoiseSuppression {
        ffi::NoiseSuppression { enable: true, suppression_level: other.suppression_level.into() }
    }
}

/// The sensitivity of the noise detector.
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub enum VoiceDetectionLikelihood {
    /// Even lower detection likelihood.
    VeryLow,
    /// Lower detection likelihood.
    Low,
    /// Moderate detection likelihood.
    Moderate,
    /// Higher detection likelihood.
    High,
}

impl From<VoiceDetectionLikelihood> for ffi::VoiceDetection_DetectionLikelihood {
    fn from(other: VoiceDetectionLikelihood) -> ffi::VoiceDetection_DetectionLikelihood {
        match other {
            VoiceDetectionLikelihood::VeryLow => ffi::VoiceDetection_DetectionLikelihood::VERY_LOW,
            VoiceDetectionLikelihood::Low => ffi::VoiceDetection_DetectionLikelihood::LOW,
            VoiceDetectionLikelihood::Moderate => ffi::VoiceDetection_DetectionLikelihood::MODERATE,
            VoiceDetectionLikelihood::High => ffi::VoiceDetection_DetectionLikelihood::HIGH,
        }
    }
}

/// Voice detection configuration.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub struct VoiceDetection {
    /// Specifies the likelihood that a frame will be declared to contain voice. A
    /// higher value makes it more likely that speech will not be clipped, at the
    /// expense of more noise being detected as voice.
    pub detection_likelihood: VoiceDetectionLikelihood,
}

impl From<VoiceDetection> for ffi::VoiceDetection {
    fn from(other: VoiceDetection) -> ffi::VoiceDetection {
        ffi::VoiceDetection {
            enable: true,
            detection_likelihood: other.detection_likelihood.into(),
        }
    }
}

/// Config that can be used mid-processing.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub struct Config {
    /// Enable and configure AEC (acoustic echo cancellation).
    pub echo_cancellation: Option<EchoCancellation>,

    /// Enable and configure AGC (automatic gain control).
    pub gain_control: Option<GainControl>,

    /// Enable and configure noise suppression.
    pub noise_suppression: Option<NoiseSuppression>,

    /// Enable and configure voice detection.
    pub voice_detection: Option<VoiceDetection>,

    /// Use to enable experimental transient noise suppression.
    #[cfg_attr(feature = "derive_serde", serde(default))]
    pub enable_transient_suppressor: bool,

    /// Use to enable a filtering component which removes DC offset and
    /// low-frequency noise.
    #[cfg_attr(feature = "derive_serde", serde(default))]
    pub enable_high_pass_filter: bool,
}

impl From<Config> for ffi::Config {
    fn from(other: Config) -> ffi::Config {
        let echo_cancellation = if let Some(enabled_value) = other.echo_cancellation {
            enabled_value.into()
        } else {
            ffi::EchoCancellation { enable: false, ..ffi::EchoCancellation::default() }
        };

        let gain_control = if let Some(enabled_value) = other.gain_control {
            enabled_value.into()
        } else {
            ffi::GainControl { enable: false, ..ffi::GainControl::default() }
        };

        let noise_suppression = if let Some(enabled_value) = other.noise_suppression {
            enabled_value.into()
        } else {
            ffi::NoiseSuppression { enable: false, ..ffi::NoiseSuppression::default() }
        };

        let voice_detection = if let Some(enabled_value) = other.voice_detection {
            enabled_value.into()
        } else {
            ffi::VoiceDetection { enable: false, ..ffi::VoiceDetection::default() }
        };

        ffi::Config {
            echo_cancellation,
            gain_control,
            noise_suppression,
            voice_detection,
            enable_transient_suppressor: other.enable_transient_suppressor,
            enable_high_pass_filter: other.enable_high_pass_filter,
        }
    }
}

/// Statistics about the processor state.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub struct Stats {
    /// True if voice is detected in the current frame.
    pub has_voice: Option<bool>,

    /// False if the current frame almost certainly contains no echo and true if it
    /// _might_ contain echo.
    pub has_echo: Option<bool>,

    /// Root mean square (RMS) level in dBFs (decibels from digital full-scale), or
    /// alternately dBov. It is computed over all primary stream frames since the
    /// last call to |get_stats()|. The returned value is constrained to [-127, 0],
    /// where -127 indicates muted.
    pub rms_dbfs: Option<i32>,

    /// Prior speech probability of the current frame averaged over output
    /// channels, internally computed by noise suppressor.
    pub speech_probability: Option<f64>,

    /// RERL = ERL + ERLE
    pub residual_echo_return_loss: Option<f64>,

    /// ERL = 10log_10(P_far / P_echo)
    pub echo_return_loss: Option<f64>,

    /// ERLE = 10log_10(P_echo / P_out)
    pub echo_return_loss_enhancement: Option<f64>,

    /// (Pre non-linear processing suppression) A_NLP = 10log_10(P_echo / P_a)
    pub a_nlp: Option<f64>,

    /// Median of the measured delay in ms. The values are aggregated until the
    /// first call to |get_stats()| and afterwards aggregated and updated every
    /// second.
    pub delay_median_ms: Option<i32>,

    /// Standard deviation of the measured delay in ms. The values are aggregated
    /// until the first call to |get_stats()| and afterwards aggregated and updated
    /// every second.
    pub delay_standard_deviation_ms: Option<i32>,

    /// The fraction of delay estimates that can make the echo cancellation perform
    /// poorly.
    pub delay_fraction_poor_delays: Option<f64>,
}

impl From<ffi::Stats> for Stats {
    fn from(other: ffi::Stats) -> Stats {
        Stats {
            has_voice: other.has_voice.into(),
            has_echo: other.has_echo.into(),
            rms_dbfs: other.rms_dbfs.into(),
            speech_probability: other.speech_probability.into(),
            residual_echo_return_loss: other.residual_echo_return_loss.into(),
            echo_return_loss: other.echo_return_loss.into(),
            echo_return_loss_enhancement: other.echo_return_loss_enhancement.into(),
            a_nlp: other.a_nlp.into(),
            delay_median_ms: other.delay_median_ms.into(),
            delay_standard_deviation_ms: other.delay_standard_deviation_ms.into(),
            delay_fraction_poor_delays: other.delay_fraction_poor_delays.into(),
        }
    }
}
