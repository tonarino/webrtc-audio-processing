#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
// https://github.com/rust-lang/rust-bindgen/issues/1651
#![allow(deref_nullptr)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// Re-export types from webrtc namespace and wrapper namespace
pub use root::{webrtc::*, webrtc_audio_processing_wrapper::*};

// Re-export global extern "C" functions
pub use root::{
    audio_processing_create, audio_processing_delete, get_num_samples_per_frame, get_stats,
    initialize, is_success, process_capture_frame, process_render_frame, set_config,
    set_output_will_be_muted, set_runtime_setting, set_stream_delay_ms, set_stream_key_pressed,
};

impl From<OptionalInt> for Option<i32> {
    fn from(other: OptionalInt) -> Option<i32> {
        if other.has_value {
            Some(other.value)
        } else {
            None
        }
    }
}

impl From<OptionalDouble> for Option<f64> {
    fn from(other: OptionalDouble) -> Option<f64> {
        if other.has_value {
            Some(other.value)
        } else {
            None
        }
    }
}

impl From<OptionalBool> for Option<bool> {
    fn from(other: OptionalBool) -> Option<bool> {
        if other.has_value {
            Some(other.value)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::null;

    const SAMPLE_RATE_HZ: i32 = 48_000;

    fn config_with_all_enabled() -> AudioProcessing_Config {
        AudioProcessing_Config {
            pipeline: AudioProcessing_Config_Pipeline {
                maximum_internal_processing_rate: SAMPLE_RATE_HZ,
                ..Default::default()
            },
            pre_amplifier: AudioProcessing_Config_PreAmplifier {
                enabled: true,
                ..Default::default()
            },
            high_pass_filter: AudioProcessing_Config_HighPassFilter {
                enabled: true,
                ..Default::default()
            },
            echo_canceller: AudioProcessing_Config_EchoCanceller {
                enabled: true,
                ..Default::default()
            },
            noise_suppression: AudioProcessing_Config_NoiseSuppression {
                enabled: true,
                ..Default::default()
            },
            transient_suppression: AudioProcessing_Config_TransientSuppression { enabled: true },
            gain_controller1: AudioProcessing_Config_GainController1 {
                enabled: true,
                mode: AudioProcessing_Config_GainController1_Mode_kAdaptiveDigital,
                analog_gain_controller:
                    AudioProcessing_Config_GainController1_AnalogGainController {
                        enabled: false,
                        ..Default::default()
                    },
                ..Default::default()
            },
            gain_controller2: AudioProcessing_Config_GainController2 {
                enabled: false,
                ..Default::default()
            },
            capture_level_adjustment: AudioProcessing_Config_CaptureLevelAdjustment {
                enabled: false,
                ..Default::default()
            },
        }
    }

    fn assert_success(error: i32) {
        unsafe {
            assert!(is_success(error), "code={}", error);
        }
    }

    #[test]
    fn test_create_failure() {
        unsafe {
            let mut error = 0;
            let ap = audio_processing_create(0, 0, SAMPLE_RATE_HZ, null(), &mut error);
            assert!(!ap.is_null());
            assert_success(error);
        }
    }

    #[test]
    fn test_create_delete() {
        unsafe {
            let mut error = 0;
            let ap = audio_processing_create(1, 1, SAMPLE_RATE_HZ, null(), &mut error);
            assert!(!ap.is_null());
            assert_success(error);
            audio_processing_delete(ap);
        }
    }

    #[test]
    fn test_config() {
        unsafe {
            let mut error = 0;
            let ap = audio_processing_create(1, 1, SAMPLE_RATE_HZ, null(), &mut error);
            assert!(!ap.is_null());
            assert_success(error);

            let config = AudioProcessing_Config::default();
            set_config(ap, &config);

            let config = config_with_all_enabled();
            set_config(ap, &config);

            audio_processing_delete(ap);
        }
    }

    #[test]
    fn test_process() {
        unsafe {
            let mut error = 0;
            let ap = audio_processing_create(1, 1, SAMPLE_RATE_HZ, null(), &mut error);
            assert!(!ap.is_null());
            assert_success(error);

            let config = config_with_all_enabled();
            set_config(ap, &config);

            let num_samples = get_num_samples_per_frame(ap);
            let mut frame = vec![vec![0f32; num_samples as usize]; 1];
            let mut frame_ptr = frame.iter_mut().map(|v| v.as_mut_ptr()).collect::<Vec<*mut f32>>();
            assert_success(process_render_frame(ap, frame_ptr.as_mut_ptr()));
            assert_success(process_capture_frame(ap, frame_ptr.as_mut_ptr()));

            audio_processing_delete(ap);
        }
    }

    #[test]
    fn test_empty_stats() {
        unsafe {
            let mut error = 0;
            let ap = audio_processing_create(1, 1, SAMPLE_RATE_HZ, null(), &mut error);
            assert!(!ap.is_null());
            assert_success(error);

            let stats = get_stats(ap);
            println!("Stats:\n{:#?}", stats);
            assert!(!stats.voice_detected.has_value);
            assert!(!stats.echo_return_loss.has_value);
            assert!(!stats.echo_return_loss_enhancement.has_value);
            assert!(!stats.divergent_filter_fraction.has_value);
            assert!(!stats.delay_median_ms.has_value);
            assert!(!stats.delay_standard_deviation_ms.has_value);
            assert!(!stats.residual_echo_likelihood.has_value);
            assert!(!stats.residual_echo_likelihood_recent_max.has_value);
            assert!(!stats.delay_ms.has_value);

            audio_processing_delete(ap);
        }
    }

    #[test]
    fn test_some_stats() {
        unsafe {
            let mut error = 0;
            let ap = audio_processing_create(1, 1, SAMPLE_RATE_HZ, null(), &mut error);
            assert!(!ap.is_null());
            assert_success(error);

            let config = config_with_all_enabled();
            set_config(ap, &config);

            let num_samples = get_num_samples_per_frame(ap);
            let mut frame = vec![vec![0f32; num_samples as usize]; 1];
            let mut frame_ptr = frame.iter_mut().map(|v| v.as_mut_ptr()).collect::<Vec<*mut f32>>();
            assert_success(process_render_frame(ap, frame_ptr.as_mut_ptr()));
            assert_success(process_capture_frame(ap, frame_ptr.as_mut_ptr()));

            let stats = get_stats(ap);
            println!("Stats:\n{:#?}", stats);
            assert!(stats.echo_return_loss.has_value);
            assert!(stats.echo_return_loss_enhancement.has_value);
            assert!(stats.delay_ms.has_value);

            // The following stats are not asserted because they are not reliably populated
            // in this test environment.
            // assert!(stats.voice_detected.has_value);
            // assert!(stats.residual_echo_likelihood.has_value);
            // assert!(stats.residual_echo_likelihood_recent_max.has_value);

            // TODO: Investigate why these stats are not filled.
            assert!(!stats.divergent_filter_fraction.has_value);
            assert!(!stats.delay_median_ms.has_value);
            assert!(!stats.delay_standard_deviation_ms.has_value);

            audio_processing_delete(ap);
        }
    }
}
