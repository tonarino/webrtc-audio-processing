#include "wrapper.hpp"

#include <algorithm>
#include <memory>

#define WEBRTC_POSIX

namespace webrtc_audio_processing_wrapper {
namespace {

OptionalDouble from_absl_optional(const absl::optional<double>& optional) {
  OptionalDouble rv;
  rv.has_value = optional.has_value();
  rv.value = optional.value_or(0.0);
  return rv;
}

OptionalInt from_absl_optional(const absl::optional<int>& optional) {
  OptionalInt rv;
  rv.has_value = optional.has_value();
  rv.value = optional.value_or(0);
  return rv;
}

OptionalBool from_absl_optional(const absl::optional<bool>& optional) {
  OptionalBool rv;
  rv.has_value = optional.has_value();
  rv.value = optional.value_or(false);
  return rv;
}

}  // namespace

struct AudioProcessing {
  std::unique_ptr<webrtc::AudioProcessing> processor;
  webrtc::AudioProcessing::Config config;
  webrtc::StreamConfig capture_stream_config;
  webrtc::StreamConfig render_stream_config;
  absl::optional<int> stream_delay_ms;
};

AudioProcessing* audio_processing_create(
    int num_capture_channels,
    int num_render_channels,
    int sample_rate_hz,
    int* error) {
  AudioProcessing* ap = new AudioProcessing;
  ap->processor.reset(webrtc::AudioProcessingBuilder().Create());

  const bool has_keyboard = false;
  ap->capture_stream_config = webrtc::StreamConfig(
      sample_rate_hz, num_capture_channels, has_keyboard);
  ap->render_stream_config = webrtc::StreamConfig(
      sample_rate_hz, num_render_channels, has_keyboard);

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
      from_absl_optional(stats.output_rms_dbfs),
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

void audio_processing_delete(AudioProcessing* ap) {
  delete ap;
}

bool is_success(const int code) {
  return code == webrtc::AudioProcessing::kNoError;
}

}  // namespace webrtc_audio_processing_wrapper
