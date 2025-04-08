// This is a c++ header file, but we are using minimal c++ constructs and not
// including any complex header files to keep Rust interoperability simple.
// The provided functions are thread-safe.
//
// TODO: Add support for AEC dump. webrtc-audio-processing library does not include TaskQueue
// implementation, which is needed.

#include <modules/audio_processing/include/audio_processing.h>

namespace webrtc_audio_processing_wrapper {

struct AudioProcessing;

struct OptionalDouble {
  bool has_value = false;
  double value = 0.0;
};

struct OptionalInt {
  bool has_value = false;
  int value = 0;
};

struct OptionalBool {
  bool has_value = false;
  bool value = false;
};

// A variant of AudioProcessingStats without absl::optional dependency,
// which can not be bindgen-ed.
struct Stats {
  OptionalInt output_rms_dbfs;
  OptionalBool voice_detected;
  OptionalDouble echo_return_loss;
  OptionalDouble echo_return_loss_enhancement;
  OptionalDouble divergent_filter_fraction;
  OptionalInt delay_median_ms;
  OptionalInt delay_standard_deviation_ms;
  OptionalDouble residual_echo_likelihood;
  OptionalDouble residual_echo_likelihood_recent_max;
  OptionalInt delay_ms;
};

// A slimmed-down version of webrtc::EchoCanceller3Config.
// We can not just expose the webrtc variant as the binding loses all the default values.
struct EchoCanceller3ConfigOverride {
    // Buffering
    size_t buffering_excess_render_detection_interval_blocks;
    size_t buffering_max_allowed_excess_render_blocks;

    // Delay
    size_t delay_default_delay;
    size_t delay_down_sampling_factor;
    size_t delay_num_filters;
    size_t delay_delay_headroom_samples;
    size_t delay_hysteresis_limit_blocks;
    size_t delay_fixed_capture_delay_samples;
    float delay_estimate_smoothing;
    float delay_candidate_detection_threshold;
    int32_t delay_selection_thresholds_initial;
    int32_t delay_selection_thresholds_converged;
    bool delay_use_external_delay_estimator;
    bool delay_log_warning_on_delay_changes;

    // Delay AlignmentMixing (Render)
    bool delay_render_alignment_mixing_downmix;
    bool delay_render_alignment_mixing_adaptive_selection;
    float delay_render_alignment_mixing_activity_power_threshold;
    bool delay_render_alignment_mixing_prefer_first_two_channels;

    // Delay AlignmentMixing (Capture)
    bool delay_capture_alignment_mixing_downmix;
    bool delay_capture_alignment_mixing_adaptive_selection;
    float delay_capture_alignment_mixing_activity_power_threshold;
    bool delay_capture_alignment_mixing_prefer_first_two_channels;

    // Filter
    size_t filter_refined_length_blocks;
    float filter_refined_leakage_converged;
    float filter_refined_leakage_diverged;
    float filter_refined_error_floor;
    float filter_refined_error_ceil;
    float filter_refined_noise_gate;

    // Filter (continued)
    size_t filter_coarse_length_blocks;
    float filter_coarse_rate;
    float filter_coarse_noise_gate;

    size_t filter_refined_initial_length_blocks;
    float filter_refined_initial_leakage_converged;
    float filter_refined_initial_leakage_diverged;
    float filter_refined_initial_error_floor;
    float filter_refined_initial_error_ceil;
    float filter_refined_initial_noise_gate;

    size_t filter_coarse_initial_length_blocks;
    float filter_coarse_initial_rate;
    float filter_coarse_initial_noise_gate;

    size_t filter_config_change_duration_blocks;
    float filter_initial_state_seconds;
    bool filter_conservative_initial_phase;
    bool filter_enable_coarse_filter_output_usage;
    bool filter_use_linear_filter;
    bool filter_export_linear_aec_output;

    // Erle
    float erle_min;
    float erle_max_l;
    float erle_max_h;
    bool erle_onset_detection;
    size_t erle_num_sections;
    bool erle_clamp_quality_estimate_to_zero;
    bool erle_clamp_quality_estimate_to_one;

    // EpStrength
    float ep_strength_default_gain;
    float ep_strength_default_len;
    bool ep_strength_echo_can_saturate;
    bool ep_strength_bounded_erl;

    // EchoAudibility
    float echo_audibility_low_render_limit;
    float echo_audibility_normal_render_limit;
    float echo_audibility_floor_power;
    float echo_audibility_audibility_threshold_lf;
    float echo_audibility_audibility_threshold_mf;
    float echo_audibility_audibility_threshold_hf;
    bool echo_audibility_use_stationarity_properties;
    bool echo_audibility_use_stationarity_properties_at_init;

    // RenderLevels
    float render_levels_active_render_limit;
    float render_levels_poor_excitation_render_limit;
    float render_levels_poor_excitation_render_limit_ds8;
    float render_levels_render_power_gain_db;

    // EchoRemovalControl
    bool echo_removal_control_has_clock_drift;
    bool echo_removal_control_linear_and_stable_echo_path;

    // EchoModel
    size_t echo_model_noise_floor_hold;
    float echo_model_min_noise_floor_power;
    float echo_model_stationary_gate_slope;
    float echo_model_noise_gate_power;
    float echo_model_noise_gate_slope;
    size_t echo_model_render_pre_window_size;
    size_t echo_model_render_post_window_size;
    bool echo_model_model_reverb_in_nonlinear_mode;

    // ComfortNoise
    float comfort_noise_noise_floor_dbfs;

    // Suppressor
    size_t suppressor_nearend_average_blocks;

    // Suppressor Normal Tuning
    float suppressor_normal_tuning_mask_lf_enr_transparent;
    float suppressor_normal_tuning_mask_lf_enr_suppress;
    float suppressor_normal_tuning_mask_lf_emr_transparent;
    float suppressor_normal_tuning_mask_hf_enr_transparent;
    float suppressor_normal_tuning_mask_hf_enr_suppress;
    float suppressor_normal_tuning_mask_hf_emr_transparent;
    float suppressor_normal_tuning_max_inc_factor;
    float suppressor_normal_tuning_max_dec_factor_lf;

    // Suppressor Nearend Tuning
    float suppressor_nearend_tuning_mask_lf_enr_transparent;
    float suppressor_nearend_tuning_mask_lf_enr_suppress;
    float suppressor_nearend_tuning_mask_lf_emr_transparent;
    float suppressor_nearend_tuning_mask_hf_enr_transparent;
    float suppressor_nearend_tuning_mask_hf_enr_suppress;
    float suppressor_nearend_tuning_mask_hf_emr_transparent;
    float suppressor_nearend_tuning_max_inc_factor;
    float suppressor_nearend_tuning_max_dec_factor_lf;

    // Suppressor DominantNearendDetection
    float suppressor_dominant_nearend_detection_enr_threshold;
    float suppressor_dominant_nearend_detection_enr_exit_threshold;
    float suppressor_dominant_nearend_detection_snr_threshold;
    int32_t suppressor_dominant_nearend_detection_hold_duration;
    int32_t suppressor_dominant_nearend_detection_trigger_threshold;
    bool suppressor_dominant_nearend_detection_use_during_initial_phase;

    // Suppressor SubbandNearendDetection
    size_t suppressor_subband_nearend_detection_nearend_average_blocks;
    size_t suppressor_subband_nearend_detection_subband1_low;
    size_t suppressor_subband_nearend_detection_subband1_high;
    size_t suppressor_subband_nearend_detection_subband2_low;
    size_t suppressor_subband_nearend_detection_subband2_high;
    float suppressor_subband_nearend_detection_nearend_threshold;
    float suppressor_subband_nearend_detection_snr_threshold;

    bool suppressor_use_subband_nearend_detection;

    // Suppressor HighBandsSuppression
    float suppressor_high_bands_suppression_enr_threshold;
    float suppressor_high_bands_suppression_max_gain_during_echo;
    float suppressor_high_bands_suppression_anti_howling_activation_threshold;
    float suppressor_high_bands_suppression_anti_howling_gain;

    float suppressor_floor_first_increase;
};

// Creates a new instance of AudioProcessing.
AudioProcessing* audio_processing_create(
    int num_capture_channels,
    int num_render_channels,
    int sample_rate_hz,
    const EchoCanceller3ConfigOverride* aec3_config_override,
    int* error);

// Processes and modifies the audio frame from a capture device.
// Each element in |channels| is an array of float representing a single-channel frame of 10 ms
// length (i.e. deinterleaved). Returns an error code or |kNoError|.
int process_capture_frame(AudioProcessing* ap, float** channels);

// Processes and optionally modifies the audio frame from a playback device.
// Each element in |channels| is an array of float representing a single-channel frame of 10 ms
// length (i.e. deinterleaved). Returns an error code or |kNoError|.
int process_render_frame(AudioProcessing* ap, float** channel3);

// Returns statistics from the last |process_capture_frame()| call.
Stats get_stats(AudioProcessing* ap);

// Returns the number of samples per frame based on the current configuration of sample rate and the
// frame chunk size. As of 2021/08/21, the chunk size is fixed to 10ms.
int get_num_samples_per_frame(AudioProcessing* ap);

// Immediately updates the configurations of the signal processor.
// This config is intended to be used during setup, and to enable/disable top-level processing
// effects. Use during processing may cause undesired submodule resets, affecting the audio quality.
// Use the RuntimeSetting construct for runtime configuration.
void set_config(AudioProcessing* ap, const webrtc::AudioProcessing::Config& config);

// Enqueues a runtime setting.
void set_runtime_setting(AudioProcessing* ap, webrtc::AudioProcessing::RuntimeSetting setting);

// Sets the |delay| in ms between process_render_frame() receiving a far-end frame and
// process_capture_frame() receiving a near-end frame containing the corresponding echo.
// It assumes that there is no such delay if this function is not called.
void set_stream_delay_ms(AudioProcessing* ap, int delay);

// Set to true when the output of AudioProcessing will be muted or in some other way not used.
// Ideally, the captured audio would still be processed, but some components may change behavior
// based on this information.
void set_output_will_be_muted(AudioProcessing* ap, bool muted);

/// Signals the AEC and AGC that the next frame will contain key press sound
void set_stream_key_pressed(AudioProcessing* ap, bool pressed);

// Initializes internal states, while retaining all user settings. This should be called before
// beginning to process a new audio stream. However, it is not necessary to call before processing
// the first stream after creation.
void initialize(AudioProcessing* ap);

// Every AudioProcessing created by |audio_processing_create()| needs to destroyed by this function.
void audio_processing_delete(AudioProcessing* ap);

// Returns true iff the code indicates a successful operation.
bool is_success(int code);

} // namespace webrtc_audio_processing_wrapper