#include "wrapper.hpp"

// These definitions shouldn't affect the implementation of AEC3.
// We are defining them to work around some build-time assertions
// when including the internal header file of echo_canceller3.h
#define WEBRTC_APM_DEBUG_DUMP 0
#define WEBRTC_POSIX
#ifdef WEBRTC_AEC3_CONFIG
#include "modules/audio_processing/aec3/echo_canceller3.h"
#endif

#include <algorithm>
#include <memory>
#include <mutex>
#include <optional>

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

#ifdef WEBRTC_AEC3_CONFIG
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
      multichannel_config =
          webrtc::EchoCanceller3::CreateDefaultMultichannelConfig();
    }
    return std::unique_ptr<webrtc::EchoControl>(
        new webrtc::EchoCanceller3(config_, multichannel_config, sample_rate_hz,
                                   num_render_channels, num_capture_channels));
  }

 private:
  webrtc::EchoCanceller3Config config_;
};
#endif

}  // namespace

webrtc::StreamConfig create_stream_config(int sample_rate_hz,
                                          size_t num_channels) {
  return webrtc::StreamConfig(sample_rate_hz, num_channels);
}

struct AudioProcessing {
  std::unique_ptr<webrtc::AudioProcessing> processor;
};

AudioProcessing* create_audio_processing(
    webrtc::EchoCanceller3Config* aec3_config,
    int* error) {
  auto ap = std::make_unique<AudioProcessing>();

  webrtc::AudioProcessingBuilder builder;
  if (aec3_config != nullptr) {
    // Validate the configuration
    if (!validate_aec3_config(aec3_config)) {
      *error = webrtc::AudioProcessing::kBadParameterError;
      return nullptr;
    }

#ifdef WEBRTC_AEC3_CONFIG
    auto* factory = new EchoCanceller3Factory(*aec3_config);
    builder.SetEchoControlFactory(
        std::unique_ptr<webrtc::EchoControlFactory>(factory));
#else
    // Fallback: advanced AEC3 configuration is not available in
    // non-experimental builds.
    *error = webrtc::AudioProcessing::kUnsupportedComponentError;
    return nullptr;
#endif
  }
  ap->processor.reset(builder.Create().release());

  return ap.release();
}

webrtc::EchoCanceller3Config create_aec3_config() {
  // This needs to happen in the C/C++ world, as the initial values defined in
  // the header file are only visible here.
  webrtc::EchoCanceller3Config config;
  return config;
}

bool validate_aec3_config(webrtc::EchoCanceller3Config* config) {
  if (config == nullptr) {
    return false;
  }
  return webrtc::EchoCanceller3Config::Validate(config);
}

void initialize(AudioProcessing* ap) {
  ap->processor->Initialize();
}

int process_capture_frame(AudioProcessing* ap,
                          const webrtc::StreamConfig& capture_stream_config,
                          float* const* channels) {
  // We don't transform the stream format, hence the same in & out stream
  // configs.
  return ap->processor->ProcessStream(channels, capture_stream_config,
                                      capture_stream_config, channels);
}

int process_render_frame(AudioProcessing* ap,
                         const webrtc::StreamConfig& render_stream_config,
                         float* const* channels) {
  // We don't transform the stream format, hence the same in & out stream
  // configs.
  return ap->processor->ProcessReverseStream(channels, render_stream_config,
                                             render_stream_config, channels);
}

int analyze_render_frame(AudioProcessing* ap,
                         const webrtc::StreamConfig& render_stream_config,
                         const float* const* channels) {
  return ap->processor->AnalyzeReverseStream(channels, render_stream_config);
}

Stats get_stats(AudioProcessing* ap) {
  const webrtc::AudioProcessingStats& stats = ap->processor->GetStatistics();

  return Stats{
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

void set_config(AudioProcessing* ap,
                const webrtc::AudioProcessing::Config& config) {
  ap->processor->ApplyConfig(config);
}

void set_stream_delay_ms(AudioProcessing* ap, int delay) {
  ap->processor->set_stream_delay_ms(delay);
}

void set_output_will_be_muted(AudioProcessing* ap, bool muted) {
  ap->processor->set_output_will_be_muted(muted);
}

void set_stream_key_pressed(AudioProcessing* ap, bool pressed) {
  ap->processor->set_stream_key_pressed(pressed);
}

void delete_audio_processing(AudioProcessing* ap) {
  delete ap;
}

}  // namespace webrtc_audio_processing_wrapper
