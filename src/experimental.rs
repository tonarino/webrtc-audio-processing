use std::ops::{Deref, DerefMut};
use webrtc_audio_processing_sys as ffi;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// [Highly experimental]
/// Exposes a finer-grained control of the internal AEC3 configuration.
/// It's minimally documented and highly experimental, and we don't yet provide Rust-idiomatic API.
/// If you want to create a new instance of [`EchoCanceller3Config`], and only modify
/// some of the fields you are interested in, you need to do in the following way:
///
/// ```
/// use webrtc_audio_processing::experimental::EchoCanceller3Config;
/// let mut aec3_config = EchoCanceller3Config::default();
/// // Alternatively:
/// let mut aec3_config = EchoCanceller3Config::multichannel_default();
/// aec3_config.suppressor.dominant_nearend_detection.enr_threshold = 0.25;
/// aec3_config.suppressor.dominant_nearend_detection.snr_threshold = 30.0;
/// assert!(aec3_config.validate());
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct EchoCanceller3Config(ffi::EchoCanceller3Config);

impl EchoCanceller3Config {
    /// Create default AEC3 config for multichannel audio processor.
    /// See also [`EchoCanceller3Config::default()`].
    pub fn multichannel_default() -> Self {
        Self(unsafe { ffi::create_multichannel_aec3_config() })
    }

    /// Checks and updates the config parameters to lie within (mostly) reasonable ranges.
    /// Returns true if and only of the config did not need to be changed.
    pub fn validate(&mut self) -> bool {
        unsafe { ffi::validate_aec3_config(&raw mut self.0) }
    }
}

impl Default for EchoCanceller3Config {
    /// Create default single-channel AEC3 config.
    /// See also [`choCanceller3Config::multichannel_default()`].
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
    /// After mutating the internals of the struct, the users are responsible for calling
    /// [`Self::validate()`] before passing it to [`crate::Processor`], or function calls may fail.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// [Highly experimental] Expose all of the inner structs of the EchoCanceller3Config.
// These do not have Default implementations and other ergonomic Rust APIs.
pub use ffi::{
    EchoCanceller3Config_Buffering, EchoCanceller3Config_ComfortNoise, EchoCanceller3Config_Delay,
    EchoCanceller3Config_Delay_AlignmentMixing,
    EchoCanceller3Config_Delay_DelaySelectionThresholds, EchoCanceller3Config_EchoAudibility,
    EchoCanceller3Config_EchoModel, EchoCanceller3Config_EchoRemovalControl,
    EchoCanceller3Config_EpStrength, EchoCanceller3Config_Erle, EchoCanceller3Config_Filter,
    EchoCanceller3Config_Filter_CoarseConfiguration,
    EchoCanceller3Config_Filter_RefinedConfiguration, EchoCanceller3Config_MultiChannel,
    EchoCanceller3Config_RenderLevels, EchoCanceller3Config_Suppressor,
    EchoCanceller3Config_Suppressor_DominantNearendDetection,
    EchoCanceller3Config_Suppressor_HighBandsSuppression,
    EchoCanceller3Config_Suppressor_MaskingThresholds,
    EchoCanceller3Config_Suppressor_SubbandNearendDetection,
    EchoCanceller3Config_Suppressor_SubbandNearendDetection_SubbandRegion,
    EchoCanceller3Config_Suppressor_Tuning,
};

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

    #[test]
    fn test_aec3_config_validation() {
        let mut aec3_config = EchoCanceller3Config::default();
        assert!(aec3_config.validate(), "Default config should be valid");

        aec3_config.erle.min = 5.0;
        aec3_config.erle.max_l = 4.0;
        assert!(!aec3_config.validate(), "Config with min ERLE > max ERLE should be invalid");
    }
}
