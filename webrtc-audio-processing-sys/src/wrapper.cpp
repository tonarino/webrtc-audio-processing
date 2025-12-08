#include "wrapper.hpp"

// These definitions shouldn't affect the implementation of AEC3.
// We are defining them to work around some build-time assertions
// when including the internal header file of echo_canceller3.h
#define WEBRTC_APM_DEBUG_DUMP 0
#define WEBRTC_POSIX
#include "webrtc/modules/audio_processing/aec3/echo_canceller3.h"

#include <algorithm>
#include <memory>
#include <optional>

#define WEBRTC_POSIX

namespace webrtc_audio_processing_wrapper {
namespace {

OptionalDouble from_absl_optional(const std::optional<double>& optional) {
  OptionalDouble rv;
  rv.has_value = optional.has_value();
  rv.value = optional.value_or(0.0);
  return rv;
}

OptionalInt from_absl_optional(const std::optional<int>& optional) {
  OptionalInt rv;
  rv.has_value = optional.has_value();
  rv.value = optional.value_or(0);
  return rv;
}

OptionalBool from_absl_optional(const std::optional<bool>& optional) {
  OptionalBool rv;
  rv.has_value = optional.has_value();
  rv.value = optional.value_or(false);
  return rv;
}

webrtc::EchoCanceller3Config build_aec3_config(const EchoCanceller3ConfigOverride& override) {
    try {
        webrtc::EchoCanceller3Config config;

        // Buffering
        config.buffering.excess_render_detection_interval_blocks =
            override.buffering_excess_render_detection_interval_blocks;
        config.buffering.max_allowed_excess_render_blocks =
            override.buffering_max_allowed_excess_render_blocks;

        // Delay
        config.delay.default_delay = override.delay_default_delay;
        config.delay.down_sampling_factor = override.delay_down_sampling_factor;
        config.delay.num_filters = override.delay_num_filters;
        config.delay.delay_headroom_samples = override.delay_delay_headroom_samples;
        config.delay.hysteresis_limit_blocks = override.delay_hysteresis_limit_blocks;
        config.delay.fixed_capture_delay_samples = override.delay_fixed_capture_delay_samples;
        config.delay.delay_estimate_smoothing = override.delay_estimate_smoothing;
        config.delay.delay_estimate_smoothing_delay_found = override.delay_estimate_smoothing_delay_found;
        config.delay.delay_candidate_detection_threshold = override.delay_candidate_detection_threshold;
        config.delay.delay_selection_thresholds.initial = override.delay_selection_thresholds_initial;
        config.delay.delay_selection_thresholds.converged = override.delay_selection_thresholds_converged;
        config.delay.use_external_delay_estimator = override.delay_use_external_delay_estimator;
        config.delay.log_warning_on_delay_changes = override.delay_log_warning_on_delay_changes;
        config.delay.detect_pre_echo = override.delay_detect_pre_echo;

        // Delay AlignmentMixing
        config.delay.render_alignment_mixing.downmix = override.delay_render_alignment_mixing_downmix;
        config.delay.render_alignment_mixing.adaptive_selection =
            override.delay_render_alignment_mixing_adaptive_selection;
        config.delay.render_alignment_mixing.activity_power_threshold =
            override.delay_render_alignment_mixing_activity_power_threshold;
        config.delay.render_alignment_mixing.prefer_first_two_channels =
            override.delay_render_alignment_mixing_prefer_first_two_channels;
        config.delay.capture_alignment_mixing.downmix = override.delay_capture_alignment_mixing_downmix;
        config.delay.capture_alignment_mixing.adaptive_selection =
            override.delay_capture_alignment_mixing_adaptive_selection;
        config.delay.capture_alignment_mixing.activity_power_threshold =
            override.delay_capture_alignment_mixing_activity_power_threshold;
        config.delay.capture_alignment_mixing.prefer_first_two_channels =
            override.delay_capture_alignment_mixing_prefer_first_two_channels;

        // Filter
        config.filter.refined.length_blocks = override.filter_refined_length_blocks;
        config.filter.refined.leakage_converged = override.filter_refined_leakage_converged;
        config.filter.refined.leakage_diverged = override.filter_refined_leakage_diverged;
        config.filter.refined.error_floor = override.filter_refined_error_floor;
        config.filter.refined.error_ceil = override.filter_refined_error_ceil;
        config.filter.refined.noise_gate = override.filter_refined_noise_gate;

        // Filter (continued)
        config.filter.coarse.length_blocks = override.filter_coarse_length_blocks;
        config.filter.coarse.rate = override.filter_coarse_rate;
        config.filter.coarse.noise_gate = override.filter_coarse_noise_gate;

        config.filter.refined_initial.length_blocks = override.filter_refined_initial_length_blocks;
        config.filter.refined_initial.leakage_converged = override.filter_refined_initial_leakage_converged;
        config.filter.refined_initial.leakage_diverged = override.filter_refined_initial_leakage_diverged;
        config.filter.refined_initial.error_floor = override.filter_refined_initial_error_floor;
        config.filter.refined_initial.error_ceil = override.filter_refined_initial_error_ceil;
        config.filter.refined_initial.noise_gate = override.filter_refined_initial_noise_gate;

        config.filter.coarse_initial.length_blocks = override.filter_coarse_initial_length_blocks;
        config.filter.coarse_initial.rate = override.filter_coarse_initial_rate;
        config.filter.coarse_initial.noise_gate = override.filter_coarse_initial_noise_gate;

        config.filter.config_change_duration_blocks = override.filter_config_change_duration_blocks;
        config.filter.initial_state_seconds = override.filter_initial_state_seconds;
        config.filter.coarse_reset_hangover_blocks = override.filter_coarse_reset_hangover_blocks;
        config.filter.conservative_initial_phase = override.filter_conservative_initial_phase;
        config.filter.enable_coarse_filter_output_usage = override.filter_enable_coarse_filter_output_usage;
        config.filter.use_linear_filter = override.filter_use_linear_filter;
        config.filter.high_pass_filter_echo_reference = override.filter_high_pass_filter_echo_reference;
        config.filter.export_linear_aec_output = override.filter_export_linear_aec_output;

        // Erle
        config.erle.min = override.erle_min;
        config.erle.max_l = override.erle_max_l;
        config.erle.max_h = override.erle_max_h;
        config.erle.onset_detection = override.erle_onset_detection;
        config.erle.num_sections = override.erle_num_sections;
        config.erle.clamp_quality_estimate_to_zero = override.erle_clamp_quality_estimate_to_zero;
        config.erle.clamp_quality_estimate_to_one = override.erle_clamp_quality_estimate_to_one;

        // EpStrength
        config.ep_strength.default_gain = override.ep_strength_default_gain;
        config.ep_strength.default_len = override.ep_strength_default_len;
        config.ep_strength.nearend_len = override.ep_strength_nearend_len;
        config.ep_strength.echo_can_saturate = override.ep_strength_echo_can_saturate;
        config.ep_strength.bounded_erl = override.ep_strength_bounded_erl;
        config.ep_strength.erle_onset_compensation_in_dominant_nearend =
            override.ep_strength_erle_onset_compensation_in_dominant_nearend;
        config.ep_strength.use_conservative_tail_frequency_response =
            override.ep_strength_use_conservative_tail_frequency_response;

        // EchoAudibility
        config.echo_audibility.low_render_limit = override.echo_audibility_low_render_limit;
        config.echo_audibility.normal_render_limit = override.echo_audibility_normal_render_limit;
        config.echo_audibility.floor_power = override.echo_audibility_floor_power;
        config.echo_audibility.audibility_threshold_lf = override.echo_audibility_audibility_threshold_lf;
        config.echo_audibility.audibility_threshold_mf = override.echo_audibility_audibility_threshold_mf;
        config.echo_audibility.audibility_threshold_hf = override.echo_audibility_audibility_threshold_hf;
        config.echo_audibility.use_stationarity_properties = override.echo_audibility_use_stationarity_properties;
        config.echo_audibility.use_stationarity_properties_at_init =
            override.echo_audibility_use_stationarity_properties_at_init;

        // RenderLevels
        config.render_levels.active_render_limit = override.render_levels_active_render_limit;
        config.render_levels.poor_excitation_render_limit = override.render_levels_poor_excitation_render_limit;
        config.render_levels.poor_excitation_render_limit_ds8 =
            override.render_levels_poor_excitation_render_limit_ds8;
        config.render_levels.render_power_gain_db = override.render_levels_render_power_gain_db;

        // EchoRemovalControl
        config.echo_removal_control.has_clock_drift = override.echo_removal_control_has_clock_drift;
        config.echo_removal_control.linear_and_stable_echo_path =
            override.echo_removal_control_linear_and_stable_echo_path;

        // EchoModel
        config.echo_model.noise_floor_hold = override.echo_model_noise_floor_hold;
        config.echo_model.min_noise_floor_power = override.echo_model_min_noise_floor_power;
        config.echo_model.stationary_gate_slope = override.echo_model_stationary_gate_slope;
        config.echo_model.noise_gate_power = override.echo_model_noise_gate_power;
        config.echo_model.noise_gate_slope = override.echo_model_noise_gate_slope;
        config.echo_model.render_pre_window_size = override.echo_model_render_pre_window_size;
        config.echo_model.render_post_window_size = override.echo_model_render_post_window_size;
        config.echo_model.model_reverb_in_nonlinear_mode = override.echo_model_model_reverb_in_nonlinear_mode;

        // ComfortNoise
        config.comfort_noise.noise_floor_dbfs = override.comfort_noise_noise_floor_dbfs;

        // Suppressor
        config.suppressor.nearend_average_blocks = override.suppressor_nearend_average_blocks;

        // Suppressor Normal Tuning
        config.suppressor.normal_tuning.mask_lf.enr_transparent =
            override.suppressor_normal_tuning_mask_lf_enr_transparent;
        config.suppressor.normal_tuning.mask_lf.enr_suppress =
            override.suppressor_normal_tuning_mask_lf_enr_suppress;
        config.suppressor.normal_tuning.mask_lf.emr_transparent =
            override.suppressor_normal_tuning_mask_lf_emr_transparent;
        config.suppressor.normal_tuning.mask_hf.enr_transparent =
            override.suppressor_normal_tuning_mask_hf_enr_transparent;
        config.suppressor.normal_tuning.mask_hf.enr_suppress =
            override.suppressor_normal_tuning_mask_hf_enr_suppress;
        config.suppressor.normal_tuning.mask_hf.emr_transparent =
            override.suppressor_normal_tuning_mask_hf_emr_transparent;
        config.suppressor.normal_tuning.max_inc_factor =
            override.suppressor_normal_tuning_max_inc_factor;
        config.suppressor.normal_tuning.max_dec_factor_lf =
            override.suppressor_normal_tuning_max_dec_factor_lf;

        // Suppressor Nearend Tuning
        config.suppressor.nearend_tuning.mask_lf.enr_transparent =
            override.suppressor_nearend_tuning_mask_lf_enr_transparent;
        config.suppressor.nearend_tuning.mask_lf.enr_suppress =
            override.suppressor_nearend_tuning_mask_lf_enr_suppress;
        config.suppressor.nearend_tuning.mask_lf.emr_transparent =
            override.suppressor_nearend_tuning_mask_lf_emr_transparent;
        config.suppressor.nearend_tuning.mask_hf.enr_transparent =
            override.suppressor_nearend_tuning_mask_hf_enr_transparent;
        config.suppressor.nearend_tuning.mask_hf.enr_suppress =
            override.suppressor_nearend_tuning_mask_hf_enr_suppress;
        config.suppressor.nearend_tuning.mask_hf.emr_transparent =
            override.suppressor_nearend_tuning_mask_hf_emr_transparent;
        config.suppressor.nearend_tuning.max_inc_factor =
            override.suppressor_nearend_tuning_max_inc_factor;
        config.suppressor.nearend_tuning.max_dec_factor_lf =
            override.suppressor_nearend_tuning_max_dec_factor_lf;

        // Suppressor Smoothing
        config.suppressor.floor_first_increase = override.suppressor_floor_first_increase;
        config.suppressor.lf_smoothing_during_initial_phase =
            override.suppressor_lf_smoothing_during_initial_phase;
        config.suppressor.last_permanent_lf_smoothing_band =
            override.suppressor_last_permanent_lf_smoothing_band;
        config.suppressor.last_lf_smoothing_band = override.suppressor_last_lf_smoothing_band;
        config.suppressor.last_lf_band = override.suppressor_last_lf_band;
        config.suppressor.first_hf_band = override.suppressor_first_hf_band;

        // Suppressor DominantNearendDetection
        config.suppressor.dominant_nearend_detection.enr_threshold =
            override.suppressor_dominant_nearend_detection_enr_threshold;
        config.suppressor.dominant_nearend_detection.enr_exit_threshold =
            override.suppressor_dominant_nearend_detection_enr_exit_threshold;
        config.suppressor.dominant_nearend_detection.snr_threshold =
            override.suppressor_dominant_nearend_detection_snr_threshold;
        config.suppressor.dominant_nearend_detection.hold_duration =
            override.suppressor_dominant_nearend_detection_hold_duration;
        config.suppressor.dominant_nearend_detection.trigger_threshold =
            override.suppressor_dominant_nearend_detection_trigger_threshold;
        config.suppressor.dominant_nearend_detection.use_during_initial_phase =
            override.suppressor_dominant_nearend_detection_use_during_initial_phase;
        config.suppressor.dominant_nearend_detection.use_unbounded_echo_spectrum =
            override.suppressor_dominant_nearend_detection_use_unbounded_echo_spectrum;

        // Suppressor SubbandNearendDetection
        config.suppressor.subband_nearend_detection.nearend_average_blocks =
            override.suppressor_subband_nearend_detection_nearend_average_blocks;
        config.suppressor.subband_nearend_detection.subband1.low =
            override.suppressor_subband_nearend_detection_subband1_low;
        config.suppressor.subband_nearend_detection.subband1.high =
            override.suppressor_subband_nearend_detection_subband1_high;
        config.suppressor.subband_nearend_detection.subband2.low =
            override.suppressor_subband_nearend_detection_subband2_low;
        config.suppressor.subband_nearend_detection.subband2.high =
            override.suppressor_subband_nearend_detection_subband2_high;
        config.suppressor.subband_nearend_detection.nearend_threshold =
            override.suppressor_subband_nearend_detection_nearend_threshold;
        config.suppressor.subband_nearend_detection.snr_threshold =
            override.suppressor_subband_nearend_detection_snr_threshold;

        config.suppressor.use_subband_nearend_detection =
            override.suppressor_use_subband_nearend_detection;

        // Suppressor HighBandsSuppression
        config.suppressor.high_bands_suppression.enr_threshold =
            override.suppressor_high_bands_suppression_enr_threshold;
        config.suppressor.high_bands_suppression.max_gain_during_echo =
            override.suppressor_high_bands_suppression_max_gain_during_echo;
        config.suppressor.high_bands_suppression.anti_howling_activation_threshold =
            override.suppressor_high_bands_suppression_anti_howling_activation_threshold;
        config.suppressor.high_bands_suppression.anti_howling_gain =
            override.suppressor_high_bands_suppression_anti_howling_gain;

        config.suppressor.conservative_hf_suppression = override.suppressor_conservative_hf_suppression;

        // MultiChannel
        config.multi_channel.detect_stereo_content = override.multi_channel_detect_stereo_content;
        config.multi_channel.stereo_detection_threshold =
            override.multi_channel_stereo_detection_threshold;
        config.multi_channel.stereo_detection_timeout_threshold_seconds =
            override.multi_channel_stereo_detection_timeout_threshold_seconds;
        config.multi_channel.stereo_detection_hysteresis_seconds =
            override.multi_channel_stereo_detection_hysteresis_seconds;

        // Validate the configuration
        if (!webrtc::EchoCanceller3Config::Validate(&config)) {
            throw std::runtime_error("Config validation failed");
        }
        return config;
    } catch (const std::exception&) {
        // Return WebRTC's default config on any error
        return webrtc::EchoCanceller3Config();
    }
}

class EchoCanceller3Factory : public webrtc::EchoControlFactory {
 public:
  explicit EchoCanceller3Factory(const webrtc::EchoCanceller3Config& config)
      : config_(config) {}

  std::unique_ptr<webrtc::EchoControl> Create(
      int sample_rate_hz,
      int num_render_channels,
      int num_capture_channels) override {
    std::optional<webrtc::EchoCanceller3Config> multichannel_config;
    if (num_render_channels > 1 || num_capture_channels > 1) {
      // Use optimized multichannel config when processing multiple channels
      multichannel_config = webrtc::EchoCanceller3::CreateDefaultMultichannelConfig();
    }
    return std::unique_ptr<webrtc::EchoControl>(new webrtc::EchoCanceller3(
        config_,
        multichannel_config,
        sample_rate_hz,
        num_render_channels,
        num_capture_channels));
  }

 private:
  webrtc::EchoCanceller3Config config_;
};

}  // namespace

struct AudioProcessing {
  std::unique_ptr<webrtc::AudioProcessing> processor;
  webrtc::AudioProcessing::Config config;
  webrtc::StreamConfig capture_stream_config;
  webrtc::StreamConfig render_stream_config;
  std::optional<int> stream_delay_ms;
};

AudioProcessing* audio_processing_create(
    int num_capture_channels,
    int num_render_channels,
    int sample_rate_hz,
    const EchoCanceller3ConfigOverride* aec3_config_override,
    int* error) {
    AudioProcessing* ap = new AudioProcessing;

    webrtc::AudioProcessingBuilder builder;
    if (aec3_config_override != nullptr) {
        auto* factory = new EchoCanceller3Factory(build_aec3_config(*aec3_config_override));
        builder.SetEchoControlFactory(std::unique_ptr<webrtc::EchoControlFactory>(factory));
    }
    ap->processor.reset(builder.Create().release());

    ap->capture_stream_config = webrtc::StreamConfig(
        sample_rate_hz, num_capture_channels);
    ap->render_stream_config = webrtc::StreamConfig(
        sample_rate_hz, num_render_channels);

    // The input and output streams must have the same number of channels.
    webrtc::ProcessingConfig pconfig = {
    ap->capture_stream_config, // capture input
    ap->capture_stream_config, // capture output
    ap->render_stream_config,  // render input
    ap->render_stream_config,  // render output
    };
        const int code = ap->processor->Initialize(pconfig);
        if (code != webrtc::AudioProcessing::kNoError) {
            *error = code;
        delete ap;
        return nullptr;
    }

    return ap;
}

void initialize(AudioProcessing* ap) {
  ap->processor->Initialize();
}

int process_capture_frame(AudioProcessing* ap, float** channels) {
  if (ap->config.echo_canceller.enabled) {
    ap->processor->set_stream_delay_ms(
        ap->stream_delay_ms.value_or(0));
  }

  return ap->processor->ProcessStream(
      channels, ap->capture_stream_config, ap->capture_stream_config, channels);
}

int process_render_frame(AudioProcessing* ap, float** channels) {
  return ap->processor->ProcessReverseStream(
      channels, ap->render_stream_config, ap->render_stream_config, channels);
}

Stats get_stats(AudioProcessing* ap) {
  const webrtc::AudioProcessingStats& stats = ap->processor->GetStatistics();

  return Stats {
      from_absl_optional(stats.voice_detected),
      from_absl_optional(stats.echo_return_loss),
      from_absl_optional(stats.echo_return_loss_enhancement),
      from_absl_optional(stats.divergent_filter_fraction),
      from_absl_optional(stats.delay_median_ms),
      from_absl_optional(stats.delay_standard_deviation_ms),
      from_absl_optional(stats.residual_echo_likelihood),
      from_absl_optional(stats.residual_echo_likelihood_recent_max),
      from_absl_optional(stats.delay_ms),
  };
}

int get_num_samples_per_frame(AudioProcessing* ap) {
    return ap->capture_stream_config.sample_rate_hz() * webrtc::AudioProcessing::kChunkSizeMs / 1000;
}

void set_config(AudioProcessing* ap, const webrtc::AudioProcessing::Config& config) {
  ap->config = config;
  ap->processor->ApplyConfig(config);
}

void set_runtime_setting(AudioProcessing* ap, webrtc::AudioProcessing::RuntimeSetting setting) {
  ap->processor->SetRuntimeSetting(setting);
}

void set_stream_delay_ms(AudioProcessing* ap, int delay) {
  // TODO: Need to mutex lock.
  ap->stream_delay_ms = delay;
}

void set_output_will_be_muted(AudioProcessing* ap, bool muted) {
  ap->processor->set_output_will_be_muted(muted);
}

void set_stream_key_pressed(AudioProcessing* ap, bool pressed) {
  ap->processor->set_stream_key_pressed(pressed);
}

void audio_processing_delete(AudioProcessing* ap) {
  delete ap;
}

bool is_success(const int code) {
  return code == webrtc::AudioProcessing::kNoError;
}

}  // namespace webrtc_audio_processing_wrapper
