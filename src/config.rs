use webrtc_audio_processing_config as config;
use webrtc_audio_processing_sys as ffi;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A configuration for initializing a Processor instance.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
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

impl InitializationConfig {
    pub(crate) fn capture_stream_config(&self) -> ffi::StreamConfig {
        ffi::StreamConfig::new(self.sample_rate_hz, self.num_capture_channels)
    }

    pub(crate) fn render_stream_config(&self) -> ffi::StreamConfig {
        ffi::StreamConfig::new(self.sample_rate_hz, self.num_render_channels)
    }
}

/// This is the same as the standard [`From`] trait, which we cannot use because of the orphan rule.
pub(crate) trait FromConfig<T>: Sized {
    fn from_config(value: T) -> Self;
}

/// This is the same as the standard [`Into`] trait, which we cannot use because of the orphan rule.
pub(crate) trait IntoFfi<T>: Sized {
    fn into_ffi(self) -> T;
}

/// Implement Into for everything that implements From the other way.
impl<T, U: FromConfig<T>> IntoFfi<U> for T {
    fn into_ffi(self) -> U {
        U::from_config(self)
    }
}

impl FromConfig<config::Config> for ffi::AudioProcessing_Config {
    fn from_config(other: config::Config) -> Self {
        let (pre_amplifier, capture_level_adjustment) = match other.capture_amplifier {
            Some(config::CaptureAmplifier::PreAmplifier(pre_amplifier)) => {
                (Some(pre_amplifier), None)
            },
            Some(config::CaptureAmplifier::CaptureLevelAdjustment(capture_level_adjustment)) => {
                (None, Some(capture_level_adjustment))
            },
            None => (None, None),
        };

        let mut echo_canceller =
            ffi::AudioProcessing_Config_EchoCanceller::from_config(other.echo_canceller);
        echo_canceller.export_linear_aec_output = if let Some(ns) = &other.noise_suppression {
            ns.analyze_linear_aec_output
        } else {
            false
        };

        // Transient suppressor is being deprecated.
        let transient_suppression =
            ffi::AudioProcessing_Config_TransientSuppression { enabled: false };

        let (gain_controller1, gain_controller2) = match other.gain_controller {
            Some(config::GainController::GainController1(v1)) => (Some(v1), None),
            Some(config::GainController::GainController2(v2)) => (None, Some(v2)),
            None => (None, None),
        };

        Self {
            pipeline: other.pipeline.into_ffi(),
            pre_amplifier: pre_amplifier.into_ffi(),
            capture_level_adjustment: capture_level_adjustment.into_ffi(),
            high_pass_filter: other.high_pass_filter.into_ffi(),
            echo_canceller,
            noise_suppression: other.noise_suppression.into_ffi(),
            transient_suppression,
            gain_controller1: gain_controller1.into_ffi(),
            gain_controller2: gain_controller2.into_ffi(),
        }
    }
}

impl FromConfig<config::Pipeline> for ffi::AudioProcessing_Config_Pipeline {
    fn from_config(pipeline: config::Pipeline) -> Self {
        Self {
            maximum_internal_processing_rate: pipeline.maximum_internal_processing_rate as i32,
            multi_channel_render: pipeline.multi_channel_render,
            multi_channel_capture: pipeline.multi_channel_capture,
            capture_downmix_method: pipeline.capture_downmix_method.into_ffi(),
        }
    }
}

impl FromConfig<config::DownmixMethod> for ffi::AudioProcessing_Config_Pipeline_DownmixMethod {
    fn from_config(
        method: config::DownmixMethod,
    ) -> ffi::AudioProcessing_Config_Pipeline_DownmixMethod {
        match method {
            config::DownmixMethod::Average => {
                ffi::AudioProcessing_Config_Pipeline_DownmixMethod_kAverageChannels
            },
            config::DownmixMethod::UseFirstChannel => {
                ffi::AudioProcessing_Config_Pipeline_DownmixMethod_kUseFirstChannel
            },
        }
    }
}

impl FromConfig<Option<config::PreAmplifier>> for ffi::AudioProcessing_Config_PreAmplifier {
    fn from_config(other: Option<config::PreAmplifier>) -> Self {
        let Some(other) = other else { return Self { enabled: false, ..Self::default() } };
        Self { enabled: true, fixed_gain_factor: other.fixed_gain_factor }
    }
}

impl FromConfig<Option<config::CaptureLevelAdjustment>>
    for ffi::AudioProcessing_Config_CaptureLevelAdjustment
{
    fn from_config(other: Option<config::CaptureLevelAdjustment>) -> Self {
        let Some(other) = other else { return Self { enabled: false, ..Self::default() } };
        Self {
            enabled: true,
            pre_gain_factor: other.pre_gain_factor,
            post_gain_factor: other.post_gain_factor,
            analog_mic_gain_emulation: other.analog_mic_gain_emulation.into_ffi(),
        }
    }
}

impl FromConfig<Option<config::AnalogMicGainEmulation>>
    for ffi::AudioProcessing_Config_CaptureLevelAdjustment_AnalogMicGainEmulation
{
    fn from_config(other: Option<config::AnalogMicGainEmulation>) -> Self {
        let Some(other) = other else { return Self { enabled: false, ..Self::default() } };
        Self { enabled: true, initial_level: other.initial_level as i32 }
    }
}

impl FromConfig<Option<config::HighPassFilter>> for ffi::AudioProcessing_Config_HighPassFilter {
    fn from_config(other: Option<config::HighPassFilter>) -> Self {
        let Some(other) = other else { return Self { enabled: false, ..Self::default() } };
        Self { enabled: true, apply_in_full_band: other.apply_in_full_band }
    }
}

impl FromConfig<Option<config::EchoCanceller>> for ffi::AudioProcessing_Config_EchoCanceller {
    fn from_config(other: Option<config::EchoCanceller>) -> Self {
        let Some(other) = other else { return Self { enabled: false, ..Self::default() } };
        // stream_delay_ms is extracted into a runtime variable in Processor::set_config().
        match other {
            config::EchoCanceller::Mobile { stream_delay_ms: _ } => Self {
                enabled: true,
                mobile_mode: true,
                enforce_high_pass_filtering: false,
                export_linear_aec_output: false,
            },
            config::EchoCanceller::Full { stream_delay_ms: _ } => Self {
                enabled: true,
                mobile_mode: false,
                enforce_high_pass_filtering: true,
                // This may be still enabled by FromConfig<Config> for ffi::AudioProcessing_Config
                export_linear_aec_output: false,
            },
        }
    }
}

impl FromConfig<Option<config::NoiseSuppression>> for ffi::AudioProcessing_Config_NoiseSuppression {
    fn from_config(other: Option<config::NoiseSuppression>) -> Self {
        let Some(other) = other else { return Self { enabled: false, ..Self::default() } };
        Self {
            enabled: true,
            level: other.level.into_ffi(),
            analyze_linear_aec_output_when_available: other.analyze_linear_aec_output,
        }
    }
}

impl FromConfig<config::NoiseSuppressionLevel>
    for ffi::AudioProcessing_Config_NoiseSuppression_Level
{
    fn from_config(other: config::NoiseSuppressionLevel) -> Self {
        match other {
            config::NoiseSuppressionLevel::Low => {
                ffi::AudioProcessing_Config_NoiseSuppression_Level_kLow
            },
            config::NoiseSuppressionLevel::Moderate => {
                ffi::AudioProcessing_Config_NoiseSuppression_Level_kModerate
            },
            config::NoiseSuppressionLevel::High => {
                ffi::AudioProcessing_Config_NoiseSuppression_Level_kHigh
            },
            config::NoiseSuppressionLevel::VeryHigh => {
                ffi::AudioProcessing_Config_NoiseSuppression_Level_kVeryHigh
            },
        }
    }
}

impl FromConfig<Option<config::GainController1>> for ffi::AudioProcessing_Config_GainController1 {
    fn from_config(other: Option<config::GainController1>) -> Self {
        let Some(other) = other else { return Self { enabled: false, ..Self::default() } };
        Self {
            enabled: true,
            mode: other.mode.into_ffi(),
            target_level_dbfs: other.target_level_dbfs as i32,
            compression_gain_db: other.compression_gain_db as i32,
            enable_limiter: other.enable_limiter,
            analog_gain_controller: other.analog_gain_controller.into_ffi(),
        }
    }
}

impl FromConfig<config::GainControllerMode> for ffi::AudioProcessing_Config_GainController1_Mode {
    fn from_config(other: config::GainControllerMode) -> Self {
        match other {
            config::GainControllerMode::AdaptiveAnalog => {
                ffi::AudioProcessing_Config_GainController1_Mode_kAdaptiveAnalog
            },
            config::GainControllerMode::AdaptiveDigital => {
                ffi::AudioProcessing_Config_GainController1_Mode_kAdaptiveDigital
            },
            config::GainControllerMode::FixedDigital => {
                ffi::AudioProcessing_Config_GainController1_Mode_kFixedDigital
            },
        }
    }
}

impl FromConfig<Option<config::AnalogGainController>>
    for ffi::AudioProcessing_Config_GainController1_AnalogGainController
{
    fn from_config(other: Option<config::AnalogGainController>) -> Self {
        let Some(other) = other else { return Self { enabled: false, ..Self::default() } };
        Self {
            enabled: true,
            startup_min_volume: other.startup_min_volume,
            clipped_level_min: other.clipped_level_min,
            enable_digital_adaptive: other.enable_digital_adaptive,
            clipped_level_step: other.clipped_level_step,
            clipped_ratio_threshold: other.clipped_ratio_threshold,
            clipped_wait_frames: other.clipped_wait_frames,
            clipping_predictor: other.clipping_predictor.into_ffi(),
        }
    }
}

impl FromConfig<Option<config::ClippingPredictor>>
    for ffi::AudioProcessing_Config_GainController1_AnalogGainController_ClippingPredictor
{
    fn from_config(other: Option<config::ClippingPredictor>) -> Self {
        let Some(other) = other else { return Self { enabled: false, ..Self::default() } };
        Self {
            enabled: true,
            mode: other.mode.into_ffi(),
            window_length: other.window_length,
            reference_window_length: other.reference_window_length,
            reference_window_delay: other.reference_window_delay,
            clipping_threshold: other.clipping_threshold,
            crest_factor_margin: other.crest_factor_margin,
            use_predicted_step: other.use_predicted_step,
        }
    }
}

impl FromConfig<config::ClippingPredictorMode>
    for ffi::AudioProcessing_Config_GainController1_AnalogGainController_ClippingPredictor_Mode
{
    fn from_config(other: config::ClippingPredictorMode) -> Self {
        match other {
            config::ClippingPredictorMode::ClippingEventPrediction => ffi::AudioProcessing_Config_GainController1_AnalogGainController_ClippingPredictor_Mode_kClippingEventPrediction,
            config::ClippingPredictorMode::AdaptiveStepClippingPeakPrediction => ffi::AudioProcessing_Config_GainController1_AnalogGainController_ClippingPredictor_Mode_kAdaptiveStepClippingPeakPrediction,
            config::ClippingPredictorMode::FixedStepClippingPeakPrediction => ffi::AudioProcessing_Config_GainController1_AnalogGainController_ClippingPredictor_Mode_kFixedStepClippingPeakPrediction,
        }
    }
}

impl FromConfig<Option<config::GainController2>> for ffi::AudioProcessing_Config_GainController2 {
    fn from_config(other: Option<config::GainController2>) -> Self {
        let Some(other) = other else { return Self { enabled: false, ..Self::default() } };
        Self {
            enabled: true,
            input_volume_controller:
                ffi::AudioProcessing_Config_GainController2_InputVolumeController {
                    enabled: other.input_volume_controller_enabled,
                },
            adaptive_digital: other.adaptive_digital.into_ffi(),
            fixed_digital: other.fixed_digital.into_ffi(),
        }
    }
}

impl FromConfig<Option<config::AdaptiveDigital>>
    for ffi::AudioProcessing_Config_GainController2_AdaptiveDigital
{
    fn from_config(other: Option<config::AdaptiveDigital>) -> Self {
        let Some(other) = other else { return Self { enabled: false, ..Self::default() } };
        Self {
            enabled: true,
            headroom_db: other.headroom_db,
            max_gain_db: other.max_gain_db,
            initial_gain_db: other.initial_gain_db,
            max_gain_change_db_per_second: other.max_gain_change_db_per_second,
            max_output_noise_level_dbfs: other.max_output_noise_level_dbfs,
        }
    }
}

impl FromConfig<config::FixedDigital> for ffi::AudioProcessing_Config_GainController2_FixedDigital {
    fn from_config(other: config::FixedDigital) -> Self {
        Self { gain_db: other.gain_db }
    }
}
