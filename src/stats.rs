use webrtc_audio_processing_sys as ffi;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Statistics about the processor state.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Stats {
    /// AEC stats: ERL = 10log_10(P_far / P_echo)
    pub echo_return_loss: Option<f64>,
    /// AEC stats: ERLE = 10log_10(P_echo / P_out)
    pub echo_return_loss_enhancement: Option<f64>,

    /// Residual echo detector likelihood.
    #[cfg(feature = "bundled")]
    pub residual_echo_likelihood: Option<f64>,
    /// Maximum residual echo likelihood from the last time period.
    #[cfg(feature = "bundled")]
    pub residual_echo_likelihood_recent_max: Option<f64>,

    /// The instantaneous delay estimate produced in the AEC. The unit is in milliseconds and the
    /// value is the instantaneous value at the time of the call to
    /// [`Processor::get_stats()`](crate::Processor::get_stats()).
    pub delay_ms: Option<u32>,
}

impl From<ffi::Stats> for Stats {
    fn from(other: ffi::Stats) -> Self {
        Self {
            echo_return_loss: other.echo_return_loss.into(),
            echo_return_loss_enhancement: other.echo_return_loss_enhancement.into(),
            #[cfg(feature = "bundled")]
            residual_echo_likelihood: other.residual_echo_likelihood.into(),
            #[cfg(feature = "bundled")]
            residual_echo_likelihood_recent_max: other.residual_echo_likelihood_recent_max.into(),
            delay_ms: Option::<i32>::from(other.delay_ms).map(|v| v as u32),
        }
    }
}
