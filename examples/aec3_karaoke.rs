use anyhow::Error;
use ctrlc;
use portaudio;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use webrtc_audio_processing::*;

const SAMPLE_RATE: f64 = 48_000.0;
const FRAMES_PER_BUFFER: u32 = 480;

fn create_processor(
    num_capture_channels: usize,
    num_render_channels: usize,
) -> Result<Processor, Error> {
    let aec3_config = EchoCanceller3Config {
        delay: Delay {
            default_delay: 4,
            down_sampling_factor: 4,
            num_filters: 5,
            delay_headroom_samples: 32,
            hysteresis_limit_blocks: 1,
            fixed_capture_delay_samples: 0,
            delay_estimate_smoothing: 0.7,
            delay_candidate_detection_threshold: 0.2,
            delay_selection_thresholds: DelaySelectionThresholds { initial: 5, converged: 20 },
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
        },
        filter: Filter {
            refined: RefinedConfiguration {
                length_blocks: 48,
                leakage_converged: 0.8,
                leakage_diverged: 0.05,
                error_floor: 0.001,
                error_ceil: 2.0,
                noise_gate: 20075344.0,
            },
            coarse: CoarseConfiguration { length_blocks: 48, rate: 0.6, noise_gate: 20075344.0 },
            refined_initial: RefinedConfiguration::default(),
            coarse_initial: CoarseConfiguration::default(),
            config_change_duration_blocks: 250,
            initial_state_seconds: 2.5,
            conservative_initial_phase: false,
            enable_coarse_filter_output_usage: true,
            use_linear_filter: true,
            export_linear_aec_output: false,
        },
        erle: Erle {
            min: 2.0,
            max_l: 12.0,
            max_h: 12.0,
            onset_detection: true,
            num_sections: 1,
            clamp_quality_estimate_to_zero: true,
            clamp_quality_estimate_to_one: true,
        },
        ep_strength: EpStrength::default(),
        echo_audibility: EchoAudibility {
            use_stationarity_properties: true,
            use_stationarity_properties_at_init: true,
            low_render_limit: 4.0,
            normal_render_limit: 64.0,
            floor_power: 128.0,
            audibility_threshold_lf: 10.0,
            audibility_threshold_mf: 10.0,
            audibility_threshold_hf: 10.0,
        },
        render_levels: RenderLevels {
            active_render_limit: 20.0,
            poor_excitation_render_limit: 0.1,
            poor_excitation_render_limit_ds8: 20.0,
            render_power_gain_db: 0.0,
        },
        suppressor: Suppressor {
            nearend_average_blocks: 4,
            normal_tuning: Tuning {
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
            },
            nearend_tuning: Tuning::default(),
            dominant_nearend_detection: Default::default(),
            subband_nearend_detection: Default::default(),
            use_subband_nearend_detection: true,
            high_bands_suppression: Default::default(),
            floor_first_increase: 0.00001,
        },
        buffering: Buffering::default(),
        comfort_noise: ComfortNoise { noise_floor_dbfs: -96.03406 },
        echo_model: EchoModel::default(),
        echo_removal_control: EchoRemovalControl {
            has_clock_drift: false,
            linear_and_stable_echo_path: false,
        },
    };

    let mut processor = Processor::with_aec3_config(
        &InitializationConfig { num_capture_channels, num_render_channels, sample_rate_hz: 48_000 },
        Some(aec3_config),
    )?;

    let config = Config {
        echo_canceller: Some(EchoCanceller::Full { enforce_high_pass_filtering: true }),
        ..Config::default()
    };
    processor.set_config(config);

    Ok(processor)
}

fn wait_ctrlc() -> Result<(), Error> {
    let running = Arc::new(AtomicBool::new(true));

    ctrlc::set_handler({
        let running = running.clone();
        move || {
            running.store(false, Ordering::SeqCst);
        }
    })?;

    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(10));
    }

    Ok(())
}

fn main() -> Result<(), Error> {
    let input_channels = 1;
    let output_channels = 1;

    let mut processor = create_processor(input_channels, output_channels)?;

    let pa = portaudio::PortAudio::new()?;

    let stream_settings = pa.default_duplex_stream_settings(
        input_channels as i32,
        output_channels as i32,
        SAMPLE_RATE,
        FRAMES_PER_BUFFER,
    )?;

    let mut processed = vec![0f32; FRAMES_PER_BUFFER as usize * input_channels];
    let mut output_buffer = vec![0f32; FRAMES_PER_BUFFER as usize * output_channels];

    let mut stream = pa.open_non_blocking_stream(
        stream_settings,
        move |portaudio::DuplexStreamCallbackArgs { in_buffer, mut out_buffer, frames, .. }| {
            assert_eq!(frames as u32, FRAMES_PER_BUFFER);

            processed.copy_from_slice(&in_buffer);
            processor.process_capture_frame(&mut processed).unwrap();

            if output_channels == 1 {
                out_buffer.copy_from_slice(&processed);
            } else {
                for i in 0..frames {
                    output_buffer[i * 2] = processed[i];
                    output_buffer[i * 2 + 1] = processed[i];
                }
                out_buffer.copy_from_slice(&output_buffer);
            }

            processor.process_render_frame(&mut out_buffer).unwrap();

            portaudio::Continue
        },
    )?;

    stream.start()?;

    wait_ctrlc()?;

    Ok(())
}
