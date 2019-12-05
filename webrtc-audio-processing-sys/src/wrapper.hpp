// This is a c++ header file, but we are using minimal c++ constructs and not
// including any complex header files to keep Rust interoperability simple.

#ifndef WEBRTC_AUDIO_PROCESSING_WRAPPER_HPP_
#define WEBRTC_AUDIO_PROCESSING_WRAPPER_HPP_

namespace webrtc_audio_processing {

// AudioProcessing accepts only one of 48000, 32000, 16000, and 8000 hz.
// Tonari only cares about 48000.
const int SAMPLE_RATE_HZ = 48000;

// AudioProcessing expects each frame to be of fixed 10 ms.
const int FRAME_MS = 10;

const int NUM_SAMPLES_PER_FRAME = SAMPLE_RATE_HZ * FRAME_MS / 1000;

struct AudioProcessing;

struct InitializationConfig {
  int num_capture_channels;
  int num_render_channels;

  // TODO(ryo): Investigate how it's different from the default gain control and
  // the effect of using the two at the same time.
  bool enable_experimental_agc;

  bool enable_intelligibility_enhancer;
};

struct EchoCancellation {
  bool enable;

  enum SuppressionLevel {
      LOW,
      MODERATE,
      HIGH,
  };

  // Determines the aggressiveness of the suppressor. A higher level trades off
  // double-talk performance for increased echo suppression.
  SuppressionLevel suppression_level;
};

struct GainControl {
  bool enable;

  // Sets the target peak level (or envelope) of the AGC in dBFs (decibels from
  // digital full-scale). The convention is to use positive values.
  // For instance, passing in a value of 3 corresponds to -3 dBFs, or a target
  // level 3 dB below full-scale. Limited to [0, 31].
  int target_level_dbfs;

  // Sets the maximum gain the digital compression stage may apply, in dB. A
  // higher number corresponds to greater compression, while a value of 0 will
  // leave the signal uncompressed. Limited to [0, 90].
  int compression_gain_db;

  // When enabled, the compression stage will hard limit the signal to the
  // target level. Otherwise, the signal will be compressed but not limited
  // above the target level.
  bool enable_limiter;
};

struct NoiseSuppression {
  bool enable;

  enum SuppressionLevel {
      LOW,
      MODERATE,
      HIGH,
      VERY_HIGH,
  };

  // Determines the aggressiveness of the suppression. Increasing the level will
  // reduce the noise level at the expense of a higher speech distortion.
  SuppressionLevel suppression_level;
};

struct VoiceDetection {
  bool enable;

  enum DetectionLikelihood {
      VERY_LOW,
      LOW,
      MODERATE,
      HIGH,
  };

  // Specifies the likelihood that a frame will be declared to contain voice. A
  // higher value makes it more likely that speech will not be clipped, at the
  // expense of more noise being detected as voice.
  DetectionLikelihood detection_likelihood;
};

struct Config {
  EchoCancellation echo_cancellation;
  GainControl gain_control;
  NoiseSuppression noise_suppression;
  VoiceDetection voice_detection;

  // Use to enable the extended filter mode in the AEC, along with robustness
  // measures around the reported system delays. It comes with a significant
  // increase in AEC complexity, but is much more robust to unreliable reported
  // delays.
  bool enable_extended_filter;

  // Enables delay-agnostic echo cancellation. This feature relies on internally
  // estimated delays between the process and reverse streams, thus not relying
  // on reported system delays.
  bool enable_delay_agnostic;

  // Use to enable experimental transient noise suppression.
  bool enable_transient_suppressor;

  // Use to enable a filtering component which removes DC offset and
  // low-frequency noise.
  bool enable_high_pass_filter;
};

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

struct Stats {
  // True if voice is detected in the current frame.
  OptionalBool has_voice;

  // False if the current frame almost certainly contains no echo and true if it
  // _might_ contain echo.
  OptionalBool has_echo;

  // Root mean square (RMS) level in dBFs (decibels from digital full-scale), or
  // alternately dBov. It is computed over all primary stream frames since the
  // last call to |get_stats()|. The returned value is constrained to [-127, 0],
  // where -127 indicates muted.
  OptionalInt rms_dbfs;

  // Prior speech probability of the current frame averaged over output
  // channels, internally computed by noise suppressor.
  OptionalDouble speech_probability;

  // RERL = ERL + ERLE
  OptionalDouble residual_echo_return_loss;

  // ERL = 10log_10(P_far / P_echo)
  OptionalDouble echo_return_loss;

  // ERLE = 10log_10(P_echo / P_out)
  OptionalDouble echo_return_loss_enhancement;

  // (Pre non-linear processing suppression) A_NLP = 10log_10(P_echo / P_a)
  OptionalDouble a_nlp;

  // Median of the measured delay in ms. The values are aggregated until the
  // first call to |get_stats()| and afterwards aggregated and updated every
  // second.
  OptionalInt delay_median_ms;

  // Standard deviation of the measured delay in ms. The values are aggregated
  // until the first call to |get_stats()| and afterwards aggregated and updated
  // every second.
  OptionalInt delay_standard_deviation_ms;

  // The fraction of delay estimates that can make the echo cancellation perform
  // poorly.
  OptionalDouble delay_fraction_poor_delays;
};

// Creates a new instance of the signal processor.
AudioProcessing* audio_processing_create(const InitializationConfig& init_config, int* error);

// Processes and modifies the audio frame from a capture device. Each element in
// |channels| is an array of float representing a single-channel frame of 10 ms
// length. Returns an error code or |kNoError|.
int process_capture_frame(AudioProcessing* ap, float** channels);

// Processes and optionally modifies the audio frame from a playback device.
// Each element in |channels| is an array of float representing a single-channel
// frame of 10 ms length. Returns an error code or |kNoError|.
int process_render_frame(AudioProcessing* ap, float** channel3);

// Returns statistics from the last |process_capture_frame()| call.
Stats get_stats(AudioProcessing* ap);

// Immediately updates the configurations of the signal processor.
// May be called multiple times after the initialization and during processing.
void set_config(AudioProcessing* ap, const Config& config);

// Every processor created by |audio_processing_create()| needs to destroyed by
// this function.
void audio_processing_delete(AudioProcessing* ap);

// Returns true iff the code indicates a successful operation.
bool is_success(int code);

}  // namespace webrtc_audio_processing

#endif  // WEBRTC_AUDIO_PROCESSING_WRAPPER_HPP_
