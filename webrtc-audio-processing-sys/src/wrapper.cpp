// TODO(ryo): Add TraceCallback.

#include "wrapper.hpp"

#include <algorithm>
#include <memory>

#define WEBRTC_POSIX
#define WEBRTC_AUDIO_PROCESSING_ONLY_BUILD

#include <webrtc/modules/audio_processing/include/audio_processing.h>
#include <webrtc/modules/interface/module_common_types.h>

namespace webrtc_audio_processing {
namespace {

// This is the default that Chromium uses.
const int AGC_STARTUP_MIN_VOLUME = 85;

OptionalDouble make_optional_double(const double value) {
  OptionalDouble rv;
  rv.has_value = true;
  rv.value = value;
  return rv;
}

OptionalInt make_optional_int(const int value) {
  OptionalInt rv;
  rv.has_value = true;
  rv.value = value;
  return rv;
}

OptionalBool make_optional_bool(const bool value) {
  OptionalBool rv;
  rv.has_value = true;
  rv.value = value;
  return rv;
}

}  // namespace

struct AudioProcessing {
  std::unique_ptr<webrtc::AudioProcessing> processor;
  webrtc::StreamConfig capture_stream_config;
  webrtc::StreamConfig render_stream_config;
  OptionalInt stream_delay_ms;
};

AudioProcessing* audio_processing_create(
    const InitializationConfig& init_config,
    int* error) {
  webrtc::Config config;
  if (init_config.enable_experimental_agc) {
    config.Set<webrtc::ExperimentalAgc>(
        new webrtc::ExperimentalAgc(true, AGC_STARTUP_MIN_VOLUME));
  }
  if (init_config.enable_intelligibility_enhancer) {
    config.Set<webrtc::Intelligibility>(new webrtc::Intelligibility(true));
  }
  // TODO(ryo): Experiment with the webrtc's builtin beamformer. There are some
  // preconditions; see |ec_fixate_spec()| in the pulseaudio's example.

  AudioProcessing* ap = new AudioProcessing;
  ap->processor.reset(webrtc::AudioProcessing::Create(config));

  const bool has_keyboard = false;
  ap->capture_stream_config = webrtc::StreamConfig(
      SAMPLE_RATE_HZ, init_config.num_capture_channels, has_keyboard);
  ap->render_stream_config = webrtc::StreamConfig(
      SAMPLE_RATE_HZ, init_config.num_render_channels, has_keyboard);

  webrtc::ProcessingConfig pconfig = {
    ap->capture_stream_config,
    ap->capture_stream_config,
    ap->render_stream_config,
    ap->render_stream_config,
  };
  const int code = ap->processor->Initialize(pconfig);
  if (code != webrtc::AudioProcessing::kNoError) {
    *error = code;
    delete ap;
    return nullptr;
  }

  return ap;
}

int process_capture_frame(AudioProcessing* ap, float** channels) {
  auto* p = ap->processor.get();

  if (p->echo_cancellation()->is_enabled()) {
    p->set_stream_delay_ms(
        ap->stream_delay_ms.has_value ? ap->stream_delay_ms.value : 0);
  }

  return p->ProcessStream(
      channels, ap->capture_stream_config, ap->capture_stream_config, channels);
}

int process_render_frame(AudioProcessing* ap, float** channels) {
  return ap->processor->ProcessReverseStream(
      channels, ap->render_stream_config, ap->render_stream_config, channels);
}

Stats get_stats(AudioProcessing* ap) {
  auto* p = ap->processor.get();

  Stats stats;
  if (p->voice_detection()->is_enabled()) {
    stats.has_voice =
        make_optional_bool(p->voice_detection()->stream_has_voice());
  }
  if (p->echo_cancellation()->is_enabled()) {
    stats.has_echo =
        make_optional_bool(p->echo_cancellation()->stream_has_echo());
  }
  if (p->level_estimator()->is_enabled()) {
    stats.rms_dbfs = make_optional_int(-1 * p->level_estimator()->RMS());
  }
  if (p->noise_suppression()->is_enabled()) {
    if (p->noise_suppression()->speech_probability()
        != webrtc::AudioProcessing::kUnsupportedFunctionError) {
      stats.speech_probability =
          make_optional_double(p->noise_suppression()->speech_probability());
    }
    // TODO(ryo): NoiseSuppression supports NoiseEstimate function in the latest
    // master.
  }

  // TODO(ryo): AudioProcessing supports useful GetStatistics function in the
  // latest master.
  if (p->echo_cancellation()->is_enabled()) {
    webrtc::EchoCancellation::Metrics metrics;
    if (p->echo_cancellation()->GetMetrics(&metrics)
        == webrtc::AudioProcessing::kNoError) {
      stats.residual_echo_return_loss =
          make_optional_double(metrics.residual_echo_return_loss.instant);
      stats.echo_return_loss =
          make_optional_double(metrics.echo_return_loss.instant);
      stats.echo_return_loss_enhancement =
          make_optional_double(metrics.echo_return_loss_enhancement.instant);
      stats.a_nlp = make_optional_double(metrics.a_nlp.instant);
    }

    int delay_median_ms = -1;
    int delay_stddev_ms = -1;
    float fraction_poor_delays = -1;
    if (p->echo_cancellation()->GetDelayMetrics(
            &delay_median_ms, &delay_stddev_ms, &fraction_poor_delays)
        == webrtc::AudioProcessing::kNoError) {
      stats.delay_median_ms = make_optional_int(delay_median_ms);
      stats.delay_standard_deviation_ms = make_optional_int(delay_stddev_ms);
      stats.delay_fraction_poor_delays =
          make_optional_double(fraction_poor_delays);
    }
  }

  return stats;
}

void set_config(AudioProcessing* ap, const Config& config) {
  auto* p = ap->processor.get();

  webrtc::Config extra_config;
  extra_config.Set<webrtc::ExtendedFilter>(
      new webrtc::ExtendedFilter(
        config.echo_cancellation.enable_extended_filter));
  extra_config.Set<webrtc::DelayAgnostic>(
      new webrtc::DelayAgnostic(
        !config.echo_cancellation.stream_delay_ms.has_value &&
        config.echo_cancellation.enable_delay_agnostic));
  extra_config.Set<webrtc::ExperimentalNs>(
      new webrtc::ExperimentalNs(config.enable_transient_suppressor));
  // TODO(ryo): There is a new RefinedAdaptiveFilter in the latest master.
  p->SetExtraOptions(extra_config);

  // TODO(ryo): Look into EchoCanceller3.
  if (config.echo_cancellation.enable) {
    ap->stream_delay_ms = config.echo_cancellation.stream_delay_ms;
    // According to the webrtc documentation, drift compensation should not be
    // necessary as long as we are using the same audio device for input and
    // output.
    p->echo_cancellation()->enable_drift_compensation(false);
    p->echo_cancellation()->enable_metrics(true);
    p->echo_cancellation()->enable_delay_logging(true);
    p->echo_cancellation()->set_suppression_level(
        static_cast<webrtc::EchoCancellation::SuppressionLevel>(
            config.echo_cancellation.suppression_level));
    p->echo_cancellation()->Enable(true);
  } else {
    p->echo_cancellation()->Enable(false);
  }

  if (config.gain_control.enable) {
    p->gain_control()->set_mode(
        static_cast<webrtc::GainControl::Mode>(config.gain_control.mode));
    p->gain_control()->set_target_level_dbfs(
        config.gain_control.target_level_dbfs);
    p->gain_control()->set_compression_gain_db(
        config.gain_control.compression_gain_db);
    p->gain_control()->enable_limiter(config.gain_control.enable_limiter);
    p->gain_control()->Enable(true);
  } else {
    p->gain_control()->Enable(false);
  }

  if (config.noise_suppression.enable) {
    p->noise_suppression()->set_level(
        static_cast<webrtc::NoiseSuppression::Level>(
            config.noise_suppression.suppression_level));
    p->noise_suppression()->Enable(true);
  } else {
    p->noise_suppression()->Enable(false);
  }

  if (config.voice_detection.enable) {
    p->voice_detection()->set_likelihood(
        static_cast<webrtc::VoiceDetection::Likelihood>(
            config.voice_detection.detection_likelihood));
    p->voice_detection()->set_frame_size_ms(FRAME_MS);
    p->voice_detection()->Enable(true);
  } else {
    p->voice_detection()->Enable(false);
  }

  p->high_pass_filter()->Enable(config.enable_high_pass_filter);

  p->level_estimator()->Enable(true);
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

}  // namespace webrtc_audio_processing
