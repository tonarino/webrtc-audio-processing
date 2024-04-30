// This is a c++ header file, but we are using minimal c++ constructs and not
// including any complex header files to keep Rust interoperability simple.

#ifndef WEBRTC_AUDIO_PROCESSING_WRAPPER_HPP_
#define WEBRTC_AUDIO_PROCESSING_WRAPPER_HPP_

namespace webrtc_audio_processing {

// AudioProcessing accepts only one of 48000, 32000, 16000, and 8000 hz.
// TODO: support multiple sample rates.
const int SAMPLE_RATE_HZ = 48000;

// AudioProcessing expects each frame to be of fixed 10 ms.
const int FRAME_MS = 10;

/// <div rustbindgen>The number of expected samples per frame.</div>
const int NUM_SAMPLES_PER_FRAME = SAMPLE_RATE_HZ * FRAME_MS / 1000;

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

/// <div rustbindgen>A configuration used only when initializing a Processor.</div>
struct InitializationConfig {
  int num_capture_channels;
  int num_render_channels;

  // TODO: Investigate how it's different from the default gain control and the effect of using the two at the same time.
  bool enable_experimental_agc;

  bool enable_intelligibility_enhancer;
};

/// <div rustbindgen>Echo cancellation configuration.</div>
struct EchoCancellation {
  /// <div rustbindgen>Whether to use echo cancellation.</div>
  bool enable;

  /// <div rustbindgen>A level of echo suppression.</div>
  enum SuppressionLevel {
      LOWEST,
      LOWER,
      LOW,
      MODERATE,
      HIGH,
  };

  /// <div rustbindgen>
  /// Determines the aggressiveness of the suppressor. A higher level trades off
  /// double-talk performance for increased echo suppression.
  /// </div>
  SuppressionLevel suppression_level;

  /// <div rustbindgen>
  /// Use to enable the extended filter mode in the AEC, along with robustness
  /// measures around the reported system delays. It comes with a significant
  /// increase in AEC complexity, but is much more robust to unreliable reported
  /// delays.
  /// </div>
  bool enable_extended_filter;

  /// <div rustbindgen>
  /// Enables delay-agnostic echo cancellation. This feature relies on internally
  /// estimated delays between the process and reverse streams, thus not relying
  /// on reported system delays.
  /// </div>
  bool enable_delay_agnostic;

  /// <div rustbindgen>
  /// Sets the delay in ms between process_render_frame() receiving a far-end
  /// frame and process_capture_frame() receiving a near-end frame containing
  /// the corresponding echo. You should set this only if you are certain that
  /// the delay will be stable and constant. enable_delay_agnostic will be
  /// ignored when this option is set.
  /// </div>
  OptionalInt stream_delay_ms;
};

/// <div rustbindgen>Gain control configuration.</div>
struct GainControl {
  /// <div rustbindgen>Whether to use gain control.</div>
  bool enable;

  /// <div rustbindgen>Mode of gain control.</div>
  enum Mode {
      /// <div rustbindgen>Not supported yet.</div>
      /// TODO(skywhale): Expose set_stream_analog_level() and
      /// stream_analog_level().
      ADAPTIVE_ANALOG,

      /// <div rustbindgen>
      /// Bring the signal to an appropriate range by applying an adaptive gain
      /// control. The volume is dynamically amplified with a microphone with
      /// small pickup and vice versa.
      /// </div>
      ADAPTIVE_DIGITAL,

      /// <div rustbindgen>
      /// Unlike ADAPTIVE_DIGITAL, it only compresses (i.e. gradually reduces
      /// gain with increasing level) the input signal when at higher levels.
      /// Use this where the capture signal level is predictable, so that a
      /// known gain can be applied.
      /// </div>
      FIXED_DIGITAL,
  };

  /// <div rustbindgen>Determines what type of gain control is applied.</div>
  Mode mode;

  /// <div rustbindgen>
  /// Sets the target peak level (or envelope) of the AGC in dBFs (decibels from
  /// digital full-scale). The convention is to use positive values.
  /// For instance, passing in a value of 3 corresponds to -3 dBFs, or a target
  /// level 3 dB below full-scale. Limited to [0, 31].
  /// </div>
  int target_level_dbfs;

  /// <div rustbindgen>
  /// Sets the maximum gain the digital compression stage may apply, in dB. A
  /// higher number corresponds to greater compression, while a value of 0 will
  /// leave the signal uncompressed. Limited to [0, 90].
  /// </div>
  int compression_gain_db;

  /// <div rustbindgen>
  /// When enabled, the compression stage will hard limit the signal to the
  /// target level. Otherwise, the signal will be compressed but not limited
  /// above the target level.
  /// </div>
  bool enable_limiter;
};

/// <div rustbindgen>Noise suppression configuration.</div>
struct NoiseSuppression {
  /// <div rustbindgen>Whether to use noise supression.</div>
  bool enable;

  /// <div rustbindgen>A level of noise suppression.</div>
  enum SuppressionLevel {
      LOW,
      MODERATE,
      HIGH,
      VERY_HIGH,
  };

  /// <div rustbindgen>
  /// Determines the aggressiveness of the suppression. Increasing the level will
  /// reduce the noise level at the expense of a higher speech distortion.
  /// </div>
  SuppressionLevel suppression_level;
};

/// <div rustbindgen>Voice detection configuration.</div>
struct VoiceDetection {
  /// <div rustbindgen>Whether to use voice detection.</div>
  bool enable;

  /// <div rustbindgen>The sensitivity of the noise detector.</div>
  enum DetectionLikelihood {
      VERY_LOW,
      LOW,
      MODERATE,
      HIGH,
  };

  /// <div rustbindgen>
  /// Specifies the likelihood that a frame will be declared to contain voice. A
  /// higher value makes it more likely that speech will not be clipped, at the
  /// expense of more noise being detected as voice.
  /// </div>
  DetectionLikelihood detection_likelihood;
};

/// <div rustbindgen>Config that can be used mid-processing.</div>
struct Config {
  EchoCancellation echo_cancellation;
  GainControl gain_control;
  NoiseSuppression noise_suppression;
  VoiceDetection voice_detection;

  /// <div rustbindgen>
  /// Use to enable experimental transient noise suppression.
  /// </div>
  bool enable_transient_suppressor;

  /// <div rustbindgen>
  /// Use to enable a filtering component which removes DC offset and
  /// low-frequency noise.
  /// </div>
  bool enable_high_pass_filter;
};

/// <div rustbindgen>Statistics about the processor state.</div>
struct Stats {
  /// <div rustbindgen>
  /// True if voice is detected in the current frame.
  /// </div>
  OptionalBool has_voice;

  /// <div rustbindgen>
  /// False if the current frame almost certainly contains no echo and true if it
  /// _might_ contain echo.
  /// </div>
  OptionalBool has_echo;

  /// <div rustbindgen>
  /// Root mean square (RMS) level in dBFs (decibels from digital full-scale), or
  /// alternately dBov. It is computed over all primary stream frames since the
  /// last call to |get_stats()|. The returned value is constrained to [-127, 0],
  /// where -127 indicates muted.
  /// </div>
  OptionalInt rms_dbfs;

  /// <div rustbindgen>
  /// Prior speech probability of the current frame averaged over output
  /// channels, internally computed by noise suppressor.
  /// </div>
  OptionalDouble speech_probability;

  /// <div rustbindgen>
  /// RERL = ERL + ERLE
  /// </div>
  OptionalDouble residual_echo_return_loss;

  /// <div rustbindgen>
  /// ERL = 10log_10(P_far / P_echo)
  /// </div>
  OptionalDouble echo_return_loss;

  /// <div rustbindgen>
  /// ERLE = 10log_10(P_echo / P_out)
  /// </div>
  OptionalDouble echo_return_loss_enhancement;

  /// <div rustbindgen>
  /// (Pre non-linear processing suppression) A_NLP = 10log_10(P_echo / P_a)
  /// </div>
  OptionalDouble a_nlp;

  /// <div rustbindgen>
  /// Median of the measured delay in ms. The values are aggregated until the
  /// first call to |get_stats()| and afterwards aggregated and updated every
  /// second.
  /// </div>
  OptionalInt delay_median_ms;

  /// <div rustbindgen>
  /// Standard deviation of the measured delay in ms. The values are aggregated
  /// until the first call to |get_stats()| and afterwards aggregated and updated
  /// every second.
  /// </div>
  OptionalInt delay_standard_deviation_ms;

  /// <div rustbindgen>
  /// The fraction of delay estimates that can make the echo cancellation perform
  /// poorly.
  /// </div>
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

// Signals the AEC and AGC that the audio output will be / is muted.
// They may use the hint to improve their parameter adaptation.
void set_output_will_be_muted(AudioProcessing* ap, bool muted);

/// Signals the AEC and AGC that the next frame will contain key press sound
void set_stream_key_pressed(AudioProcessing* ap, bool pressed);

// Every processor created by |audio_processing_create()| needs to destroyed by
// this function.
void audio_processing_delete(AudioProcessing* ap);

// Returns true iff the code indicates a successful operation.
bool is_success(int code);

}  // namespace webrtc_audio_processing

#endif  // WEBRTC_AUDIO_PROCESSING_WRAPPER_HPP_
