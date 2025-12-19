#include "wrapper.hpp"

// These definitions shouldn't affect the implementation of AEC3.
// We are defining them to work around some build-time assertions
// when including the internal header file of echo_canceller3.h
#define WEBRTC_APM_DEBUG_DUMP 0
#define WEBRTC_POSIX
#include "webrtc/modules/audio_processing/aec3/echo_canceller3.h"

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

}  // namespace

struct AudioProcessing {
  std::mutex mutex;
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
    webrtc::EchoCanceller3Config* aec3_config,
    int* error) {
  auto ap = std::make_unique<AudioProcessing>();

  webrtc::AudioProcessingBuilder builder;
  if (aec3_config != nullptr) {
    // Validate the configuration
    if (!validate_aec3_config(aec3_config)) {
        return nullptr;
    }

    auto* factory = new EchoCanceller3Factory(*aec3_config);
    builder.SetEchoControlFactory(
        std::unique_ptr<webrtc::EchoControlFactory>(factory));
  }
  ap->processor.reset(builder.Create().release());

  ap->capture_stream_config =
      webrtc::StreamConfig(sample_rate_hz, num_capture_channels);
  ap->render_stream_config =
      webrtc::StreamConfig(sample_rate_hz, num_render_channels);

  // The input and output streams must have the same number of channels.
  webrtc::ProcessingConfig pconfig = {
      ap->capture_stream_config,  // capture input
      ap->capture_stream_config,  // capture output
      ap->render_stream_config,   // render input
      ap->render_stream_config,   // render output
  };
  const int code = ap->processor->Initialize(pconfig);
  if (code != webrtc::AudioProcessing::kNoError) {
    *error = code;
    return nullptr;
  }

  return ap.release();
}

webrtc::EchoCanceller3Config create_aec3_config() {
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

int process_capture_frame(AudioProcessing* ap, float** channels) {
  {
    std::lock_guard<std::mutex> lock(ap->mutex);
    if (ap->config.echo_canceller.enabled) {
      ap->processor->set_stream_delay_ms(ap->stream_delay_ms.value_or(0));
    }
  }

  return ap->processor->ProcessStream(channels, ap->capture_stream_config,
                                      ap->capture_stream_config, channels);
}

int process_render_frame(AudioProcessing* ap, float** channels) {
  return ap->processor->ProcessReverseStream(
      channels, ap->render_stream_config, ap->render_stream_config, channels);
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

int get_num_samples_per_frame(AudioProcessing* ap) {
  return ap->capture_stream_config.sample_rate_hz() *
         webrtc::AudioProcessing::kChunkSizeMs / 1000;
}

void set_config(AudioProcessing* ap,
                const webrtc::AudioProcessing::Config& config) {
  std::lock_guard<std::mutex> lock(ap->mutex);
  ap->config = config;

  // Exporting linear AEC output is only supported at 16kHz and in full AEC
  // mode.
  if (ap->capture_stream_config.sample_rate_hz() != 16000 ||
      ap->config.echo_canceller.mobile_mode) {
    ap->config.echo_canceller.export_linear_aec_output = false;
  }

  // If linear AEC output is not exported, we cannot analyze it.
  if (!ap->config.echo_canceller.export_linear_aec_output) {
    ap->config.noise_suppression.analyze_linear_aec_output_when_available =
        false;
  }

  ap->processor->ApplyConfig(ap->config);
}

void set_runtime_setting(AudioProcessing* ap,
                         webrtc::AudioProcessing::RuntimeSetting setting) {
  ap->processor->SetRuntimeSetting(setting);
}

void set_stream_delay_ms(AudioProcessing* ap, int delay) {
  std::lock_guard<std::mutex> lock(ap->mutex);
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
