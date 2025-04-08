use webrtc_audio_processing_sys as ffi;

#[cfg(feature = "derive_serde")]
use serde::{Deserialize, Serialize};

/// Statistics about the processor state.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
pub struct Stats {
    /// AEC stats: ERL = 10log_10(P_far / P_echo)
    pub echo_return_loss: Option<f64>,
    /// AEC stats: ERLE = 10log_10(P_echo / P_out)
    pub echo_return_loss_enhancement: Option<f64>,
    /// AEC stats: Fraction of time that the AEC linear filter is divergent, in a 1-second
    /// non-overlapped aggregation window.
    pub divergent_filter_fraction: Option<f64>,

    /// The delay median in milliseconds. The values are aggregated until the first call to
    /// [`get_stats()`] and afterwards aggregated and updated every second.
    pub delay_median_ms: Option<u32>,
    /// The delay standard deviation in milliseconds. The values are aggregated until the first
    /// call to [`get_stats()`] and afterwards aggregated and updated every second.
    pub delay_standard_deviation_ms: Option<u32>,

    /// Residual echo detector likelihood.
    pub residual_echo_likelihood: Option<f64>,
    /// Maximum residual echo likelihood from the last time period.
    pub residual_echo_likelihood_recent_max: Option<f64>,

    /// The instantaneous delay estimate produced in the AEC. The unit is in milliseconds and the
    /// value is the instantaneous value at the time of the call to [`get_stats()`].
    pub delay_ms: Option<u32>,
}

impl From<ffi::Stats> for Stats {
    fn from(other: ffi::Stats) -> Self {
        Self {
            echo_return_loss: other.echo_return_loss.into(),
            echo_return_loss_enhancement: other.echo_return_loss_enhancement.into(),
            divergent_filter_fraction: other.divergent_filter_fraction.into(),
            delay_median_ms: Option::<i32>::from(other.delay_median_ms).map(|v| v as u32),
            delay_standard_deviation_ms: Option::<i32>::from(other.delay_standard_deviation_ms)
                .map(|v| v as u32),
            residual_echo_likelihood: other.residual_echo_likelihood.into(),
            residual_echo_likelihood_recent_max: other.residual_echo_likelihood_recent_max.into(),
            delay_ms: Option::<i32>::from(other.delay_ms).map(|v| v as u32),
        }
    }
}
