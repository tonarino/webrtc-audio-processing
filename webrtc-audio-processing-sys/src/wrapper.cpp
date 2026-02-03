#include "wrapper.hpp"

// These definitions shouldn't affect the implementation of AEC3.
// We are defining them to work around some build-time assertions
// when including the internal header file of echo_canceller3.h
#define WEBRTC_APM_DEBUG_DUMP 0
#define WEBRTC_POSIX
#ifdef WEBRTC_AEC3_CONFIG
#include "modules/audio_processing/aec3/echo_canceller3.h"
#endif

#include <memory>
#include <mutex>
#include <optional>

// EchoCanceller3Config doesn't provide == operator. We write a poor man's
// version ourselves. We need to inject it to webrtc namespace.
namespace webrtc {

bool operator==(const webrtc::EchoCanceller3Config& a,
                const webrtc::EchoCanceller3Config& b) {
  // Do byte-by-byte comparison. Reportedly this can cause false negatives
  // because padding bytes can be technically garbage. But never false
  // positives.
  if (std::memcmp(&a, &b, sizeof(a)) == 0) {
    return true;
  }

  return false;
}

}  // namespace webrtc

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

// Utility class to share EchoCanceller3Config between `EchoCanceller3Factory`
// and `AudioProcessing` (when wrapped in shared_ptr).
// Thread-safe (protected by a mutex).
class Aec3ConfigHolder {
 public:
  Aec3ConfigHolder() {}

  std::optional<webrtc::EchoCanceller3Config> get_config() const {
    std::lock_guard lock(mutex_);
    return config_;
  }

  // Returns true if the config has changed, false otherwise.
  bool set_config(std::optional<webrtc::EchoCanceller3Config> config) {
    std::lock_guard lock(mutex_);
    if (config == config_) {
      return false;
    } else {
      config_ = std::move(config);
      return true;
    }
  }

 private:
  // Protects access to `config_`.
  mutable std::mutex mutex_;
  // You must lock `mutex_` to access this field.
  std::optional<webrtc::EchoCanceller3Config> config_ = std::nullopt;
};

#ifdef WEBRTC_AEC3_CONFIG
class EchoCanceller3Factory : public webrtc::EchoControlFactory {
 public:
  explicit EchoCanceller3Factory(
      const std::shared_ptr<Aec3ConfigHolder>& config_holder)
      : config_holder_(config_holder) {}

  std::unique_ptr<webrtc::EchoControl> Create(
      int sample_rate_hz,
      int num_render_channels,
      int num_capture_channels) override {
    webrtc::EchoCanceller3Config config;  // (single-channel defaults)
    std::optional<webrtc::EchoCanceller3Config> multichannel_config =
        std::nullopt;

    auto explicit_config = config_holder_->get_config();
    if (explicit_config) {
      // Keep null multichannel_config to use explicit_config at all times.
      config = *explicit_config;
    } else {
      // Keep default (single-channel) config and use different multichannel
      // default. This behavior mimics the logic of
      // AudioProcessingImpl::InitializeEchoController().
      multichannel_config = create_multichannel_aec3_config();
    }

    return std::make_unique<webrtc::EchoCanceller3>(
        config, multichannel_config, sample_rate_hz, num_render_channels,
        num_capture_channels);
  }

 private:
  std::shared_ptr<Aec3ConfigHolder> config_holder_;
};
#endif

}  // namespace

webrtc::StreamConfig create_stream_config(int sample_rate_hz,
                                          size_t num_channels) {
  return webrtc::StreamConfig(sample_rate_hz, num_channels);
}

webrtc::EchoCanceller3Config create_aec3_config() {
  // This needs to happen in the C/C++ world, as the initial values defined in
  // the header file are only visible here.
  webrtc::EchoCanceller3Config config;
  return config;
}

#ifdef WEBRTC_AEC3_CONFIG
webrtc::EchoCanceller3Config create_multichannel_aec3_config() {
  return webrtc::EchoCanceller3::CreateDefaultMultichannelConfig();
}
#endif

bool validate_aec3_config(webrtc::EchoCanceller3Config* config) {
  if (config == nullptr) {
    return false;
  }
  return webrtc::EchoCanceller3Config::Validate(config);
}

struct AudioProcessing {
  std::unique_ptr<webrtc::AudioProcessing> processor;
  std::shared_ptr<Aec3ConfigHolder> aec3_config_holder;
};

AudioProcessing* create_audio_processing() {
  auto aec3_config_holder = std::make_shared<Aec3ConfigHolder>();

  webrtc::AudioProcessingBuilder builder;
#ifdef WEBRTC_AEC3_CONFIG
  auto* factory = new EchoCanceller3Factory(aec3_config_holder);
  builder.SetEchoControlFactory(
      std::unique_ptr<webrtc::EchoControlFactory>(factory));
#endif
  auto processor =
      std::unique_ptr<webrtc::AudioProcessing>(builder.Create().release());

  return new AudioProcessing{std::move(processor),
                             std::move(aec3_config_holder)};
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

#ifdef WEBRTC_AEC3_CONFIG
// Set custom AEC3 config (for both single- and multi-channel processing).
// `aec3_config` should be either null or valid, otherwise this returns non-zero
// error code, and doesn't apply any config. If null is passed, AEC3 config is
// reset to default (slightly different for single- and multi-channel
// processing).
int set_aec3_config(AudioProcessing* ap,
                    const webrtc::EchoCanceller3Config* aec3_config) {
  std::optional<webrtc::EchoCanceller3Config> opt_config = std::nullopt;

  if (aec3_config) {
    opt_config = *aec3_config;
    // Validate the just-made copy so that we don't modify the argument.
    if (!validate_aec3_config(&(*opt_config))) {
      return webrtc::AudioProcessing::kBadParameterError;
    }
  }

  bool has_changed = ap->aec3_config_holder->set_config(std::move(opt_config));

  // Trigger reinit so that the factory is called again and new config is read.
  if (has_changed) {
    initialize(ap);
  }

  return 0;
}
#endif

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
