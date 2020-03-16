#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

impl Into<Option<bool>> for OptionalBool {
    fn into(self) -> Option<bool> {
        if self.has_value {
            Some(self.value)
        } else {
            None
        }
    }
}

impl From<Option<bool>> for OptionalBool {
    fn from(other: Option<bool>) -> OptionalBool {
        if let Some(value) = other {
            OptionalBool { has_value: true, value }
        } else {
            OptionalBool { has_value: false, value: false }
        }
    }
}

impl Into<Option<i32>> for OptionalInt {
    fn into(self) -> Option<i32> {
        if self.has_value {
            Some(self.value)
        } else {
            None
        }
    }
}

impl From<Option<i32>> for OptionalInt {
    fn from(other: Option<i32>) -> OptionalInt {
        if let Some(value) = other {
            OptionalInt { has_value: true, value }
        } else {
            OptionalInt { has_value: false, value: 0 }
        }
    }
}

impl Into<Option<f64>> for OptionalDouble {
    fn into(self) -> Option<f64> {
        if self.has_value {
            Some(self.value)
        } else {
            None
        }
    }
}

impl From<Option<f64>> for OptionalDouble {
    fn from(other: Option<f64>) -> OptionalDouble {
        if let Some(value) = other {
            OptionalDouble { has_value: true, value }
        } else {
            OptionalDouble { has_value: false, value: 0.0 }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_config_with_all_enabled() -> InitializationConfig {
        InitializationConfig {
            num_capture_channels: 1,
            num_render_channels: 1,
            enable_experimental_agc: true,
            enable_intelligibility_enhancer: true,
        }
    }

    fn config_with_all_enabled() -> Config {
        Config {
            echo_cancellation: EchoCancellation {
                enable: true,
                suppression_level: EchoCancellation_SuppressionLevel::HIGH,
            },
            gain_control: GainControl {
                enable: true,
                target_level_dbfs: 3,
                compression_gain_db: 3,
                enable_limiter: true,
            },
            noise_suppression: NoiseSuppression {
                enable: true,
                suppression_level: NoiseSuppression_SuppressionLevel::HIGH,
            },
            voice_detection: VoiceDetection {
                enable: true,
                detection_likelihood: VoiceDetection_DetectionLikelihood::HIGH,
            },
            enable_extended_filter: true,
            enable_delay_agnostic: true,
            enable_transient_suppressor: true,
            enable_high_pass_filter: true,
        }
    }

    #[test]
    fn test_create_failure() {
        unsafe {
            let config = InitializationConfig::default();
            let mut error = 0;
            let ap = audio_processing_create(&config, &mut error);
            assert!(ap.is_null());
            assert!(!is_success(error));
        }
    }

    #[test]
    fn test_create_delete() {
        unsafe {
            let config = InitializationConfig {
                num_capture_channels: 1,
                num_render_channels: 1,
                ..InitializationConfig::default()
            };
            let mut error = 0;
            let ap = audio_processing_create(&config, &mut error);
            assert!(!ap.is_null());
            assert!(is_success(error));
            audio_processing_delete(ap);
        }
    }

    #[test]
    fn test_config() {
        unsafe {
            let mut error = 0;
            let ap = audio_processing_create(&init_config_with_all_enabled(), &mut error);
            assert!(!ap.is_null());
            assert!(is_success(error));

            let config = Config::default();
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
            let ap = audio_processing_create(&init_config_with_all_enabled(), &mut error);
            assert!(!ap.is_null());
            assert!(is_success(error));

            let config = config_with_all_enabled();
            set_config(ap, &config);

            let mut frame = vec![vec![0f32; NUM_SAMPLES_PER_FRAME as usize]; 1];
            let mut frame_ptr = frame.iter_mut().map(|v| v.as_mut_ptr()).collect::<Vec<*mut f32>>();
            assert!(is_success(process_render_frame(ap, frame_ptr.as_mut_ptr())));
            assert!(is_success(process_capture_frame(ap, frame_ptr.as_mut_ptr())));

            audio_processing_delete(ap);
        }
    }

    #[test]
    fn test_empty_stats() {
        unsafe {
            let config = InitializationConfig {
                num_capture_channels: 1,
                num_render_channels: 1,
                ..InitializationConfig::default()
            };
            let mut error = 0;
            let ap = audio_processing_create(&config, &mut error);
            assert!(!ap.is_null());
            assert!(is_success(error));

            let stats = get_stats(ap);
            println!("Stats:\n{:#?}", stats);
            assert!(!stats.has_voice.has_value);
            assert!(!stats.has_echo.has_value);
            assert!(!stats.rms_dbfs.has_value);
            assert!(!stats.speech_probability.has_value);
            assert!(!stats.residual_echo_return_loss.has_value);
            assert!(!stats.echo_return_loss.has_value);
            assert!(!stats.echo_return_loss_enhancement.has_value);
            assert!(!stats.a_nlp.has_value);
            assert!(!stats.delay_median_ms.has_value);
            assert!(!stats.delay_standard_deviation_ms.has_value);
            assert!(!stats.delay_fraction_poor_delays.has_value);

            audio_processing_delete(ap);
        }
    }

    #[test]
    fn test_some_stats() {
        unsafe {
            let mut error = 0;
            let ap = audio_processing_create(&init_config_with_all_enabled(), &mut error);
            assert!(!ap.is_null());
            assert!(is_success(error));

            let config = config_with_all_enabled();
            set_config(ap, &config);

            let mut frame = vec![vec![0f32; NUM_SAMPLES_PER_FRAME as usize]; 1];
            let mut frame_ptr = frame.iter_mut().map(|v| v.as_mut_ptr()).collect::<Vec<*mut f32>>();
            assert!(is_success(process_render_frame(ap, frame_ptr.as_mut_ptr())));
            assert!(is_success(process_capture_frame(ap, frame_ptr.as_mut_ptr())));
            let stats = get_stats(ap);
            println!("Stats:\n{:#?}", stats);
            assert!(stats.has_voice.has_value);
            assert!(stats.has_echo.has_value);
            assert!(stats.rms_dbfs.has_value);
            assert!(stats.speech_probability.has_value);
            assert!(stats.residual_echo_return_loss.has_value);
            assert!(stats.echo_return_loss.has_value);
            assert!(stats.echo_return_loss_enhancement.has_value);
            assert!(stats.a_nlp.has_value);
            assert!(stats.delay_median_ms.has_value);
            assert!(stats.delay_standard_deviation_ms.has_value);
            assert!(stats.delay_fraction_poor_delays.has_value);

            audio_processing_delete(ap);
        }
    }
}
