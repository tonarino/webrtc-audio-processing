// This is a c++ header file, but we are using minimal c++ constructs and not
// including any complex header files to keep Rust interoperability simple.
// The provided functions are thread-safe.
//
// TODO: Add support for AEC dump. webrtc-audio-processing library does not
// include TaskQueue implementation, which is needed.

#include "api/audio/audio_processing.h"
#include "api/audio/echo_canceller3_config.h"

namespace webrtc_audio_processing_wrapper {

struct AudioProcessing;

struct OptionalDouble {
  bool has_value = false;
  double value = 0.0;
};

struct OptionalBool {
  bool has_value = false;
  bool value = false;
};

struct OptionalInt {
  bool has_value = false;
  int value = 0;
};

// A variant of AudioProcessingStats without absl::optional dependency,
// which can not be bindgen-ed.
struct Stats {
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

// Creates the `StreamConfig` struct by calling the C++ constructor
// (which bindgen fails to generate bindings for, likely because it is inline).
webrtc::StreamConfig create_stream_config(int sample_rate_hz,
                                          size_t num_channels);

// Instantiates an EchoCanceller3Config with the webrtc's default settings.
webrtc::EchoCanceller3Config create_aec3_config();

// Instantiates an EchoCanceller3Config with the webrtc's default multichannel
// settings. Requires wrapper.cpp built with WEBRTC_AEC3_CONFIG.
webrtc::EchoCanceller3Config create_multichannel_aec3_config();

// Checks and updates the config parameters to lie within (mostly) reasonable
// ranges. Returns true if and only of the config did not need to be changed.
bool validate_aec3_config(webrtc::EchoCanceller3Config* config);

// Creates a new instance of AudioProcessing with default baseline and AEC3
// configuration.
AudioProcessing* create_audio_processing();

// Processes and modifies the audio frame from a capture device.
// Each element in |channels| is an array of float representing a single-channel
// frame of 10 ms length (i.e. deinterleaved). Returns an error code or
// |kNoError|.
int process_capture_frame(AudioProcessing* ap,
                          const webrtc::StreamConfig& capture_stream_config,
                          float* const* channels);

// Processes and optionally modifies the audio frame destined to a playback
// device.
// Each element in |channels| is an array of float representing a single-channel
// frame of 10 ms length (i.e. deinterleaved). Returns an error code or
// |kNoError|.
int process_render_frame(AudioProcessing* ap,
                         const webrtc::StreamConfig& render_stream_config,
                         float* const* channels);

// Analyzes the audio frame destined to a playback device without modifying it.
// Each element in |channels| is an array of float representing a single-channel
// frame of 10 ms length (i.e. deinterleaved). Returns an error code or
// |kNoError|.
int analyze_render_frame(AudioProcessing* ap,
                         const webrtc::StreamConfig& render_stream_config,
                         const float* const* channels);

// Returns statistics from the last |process_capture_frame()| call.
Stats get_stats(AudioProcessing* ap);

// Immediately updates the configurations of the signal processor.
// This config is intended to be used during setup, and to enable/disable
// top-level processing effects. Use during processing may cause undesired
// submodule resets, affecting the audio quality. Use the RuntimeSetting
// construct for runtime configuration.
void set_config(AudioProcessing* ap,
                const webrtc::AudioProcessing::Config& config);

// Set custom AEC3 config (the same for both single- and multi-channel
// processing). |aec3_config| should be either null or valid, otherwise this
// returns non-zero error code, and doesn't apply any config. If null is passed,
// AEC3 config is reset to default (slightly different for single- and
// multi-channel processing). Causes reinitialization of the whole
// AudioProcessing if and only if the configuration contents have changed,
// otherwise returns quickly.
int set_aec3_config(AudioProcessing* ap,
                    const webrtc::EchoCanceller3Config* aec3_config);

// Sets the |delay| in ms between process_render_frame() receiving a far-end
// frame and process_capture_frame() receiving a near-end frame containing the
// corresponding echo. It assumes that there is no such delay if this function
// is not called.
void set_stream_delay_ms(AudioProcessing* ap, int delay);

// Set to true when the output of AudioProcessing will be muted or in some other
// way not used. Ideally, the captured audio would still be processed, but some
// components may change behavior based on this information.
void set_output_will_be_muted(AudioProcessing* ap, bool muted);

// Signals the AEC and AGC that the next frame will contain key press sound
void set_stream_key_pressed(AudioProcessing* ap, bool pressed);

// Initializes internal states, while retaining all user settings. This should
// be called before beginning to process a new audio stream. However, it is not
// necessary to call before processing the first stream after creation.
void initialize(AudioProcessing* ap);

// Every AudioProcessing created by |audio_processing_create()| needs to
// destroyed by this function.
void delete_audio_processing(AudioProcessing* ap);

}  // namespace webrtc_audio_processing_wrapper
