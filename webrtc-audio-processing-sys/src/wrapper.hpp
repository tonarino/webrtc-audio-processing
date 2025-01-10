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
    int num_filters;
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
