/// An example binary to help evaluate webrtc audio processing pipeline, in particular its echo
/// canceller. You can use it to record a sample with your audio setup, and you can run the
/// pipeline repeatedly using the sampled audio, to test different configurations of the pipeline.
///
/// # Record a sample
///
/// Play back a pre-recorded audio stream from your speakers, while recording the microphone
/// input as a WAV file.
///
/// ```
/// $ cargo run --example recording --features bundled --features derive_serde -- --config-file \
///     examples/recording-configs/record-sample.json5
/// ```
///
/// # Run the pipeline with the sample
///
/// Run the audio processing pipeline with the recorded capture and render frames. You can then
/// analyze the capture-processed.wav to understand the effect produced by the pipeline.
///
/// ```
/// $ cargo run --example recording --features bundled --features derive_serde -- --config-file \
///     examples/recording-configs/record-pipeline.json5
/// ```
use anyhow::{anyhow, Error};
use hound::{WavIntoSamples, WavReader, WavWriter};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use structopt::StructOpt;
use webrtc_audio_processing::*;

const AUDIO_SAMPLE_RATE: u32 = 48_000;
const AUDIO_INTERLEAVED: bool = true;

#[derive(Debug, StructOpt)]
struct Args {
    /// Configuration file that stores JSON serialization of [`Option`] struct.
    #[structopt(short, long)]
    pub config_file: PathBuf,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
struct CaptureOptions {
    /// Name of the audio capture device.
    device_name: String,
    /// The number of audio capture channels.
    num_channels: u16,
    /// If specified, it reads the capture stream from the WAV file instead of the device.
    source_path: Option<PathBuf>,
    /// If specified, it writes the capture stream to the WAV file before applying the processing.
    preprocess_sink_path: Option<PathBuf>,
    /// If specified, it writes the capture stream to the WAV file after applying the processing.
    postprocess_sink_path: Option<PathBuf>,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
struct RenderOptions {
    /// Name of the audio playback device.
    device_name: String,
    /// The number of audio playback channels.
    num_channels: u16,
    /// If specified, it plays back the audio stream from the WAV file. Otherwise, a stream of
    /// zeros are sent to the audio device.
    source_path: Option<PathBuf>,
    /// If true, the output is muted.
    #[serde(default)]
    mute: bool,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
struct Options {
    /// Options for audio capture / recording.
    capture: CaptureOptions,
    /// Options for audio render / playback.
    render: RenderOptions,
    /// Configurations of the audio processing pipeline.
    config: Config,
}

fn match_device(
    pa: &portaudio::PortAudio,
    device_name: Regex,
) -> Result<portaudio::DeviceIndex, Error> {
    for device in (pa.devices()?).flatten() {
        if device_name.is_match(device.1.name) {
            return Ok(device.0);
        }
    }
    Err(anyhow!("Audio device matching \"{}\" not found.", device_name))
}

fn create_stream_settings(
    pa: &portaudio::PortAudio,
    opt: &Options,
) -> Result<portaudio::DuplexStreamSettings<f32, f32>, Error> {
    let input_device = match_device(pa, Regex::new(&opt.capture.device_name)?)?;
    let input_device_info = &pa.device_info(input_device)?;
    let input_params = portaudio::StreamParameters::<f32>::new(
        input_device,
        opt.capture.num_channels as i32,
        AUDIO_INTERLEAVED,
        input_device_info.default_low_input_latency,
    );

    let output_device = match_device(pa, Regex::new(&opt.render.device_name)?)?;
    let output_device_info = &pa.device_info(output_device)?;
    let output_params = portaudio::StreamParameters::<f32>::new(
        output_device,
        opt.render.num_channels as i32,
        AUDIO_INTERLEAVED,
        output_device_info.default_low_output_latency,
    );

    pa.is_duplex_format_supported(input_params, output_params, f64::from(AUDIO_SAMPLE_RATE))?;

    Ok(portaudio::DuplexStreamSettings::new(
        input_params,
        output_params,
        f64::from(AUDIO_SAMPLE_RATE),
        NUM_SAMPLES_PER_FRAME as u32,
    ))
}

fn open_wav_writer(path: &Path, channels: u16) -> Result<WavWriter<BufWriter<File>>, Error> {
    let sink = hound::WavWriter::<BufWriter<File>>::create(
        path,
        hound::WavSpec {
            channels,
            sample_rate: AUDIO_SAMPLE_RATE,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )?;

    Ok(sink)
}

fn open_wav_reader(path: &Path) -> Result<WavIntoSamples<BufReader<File>, f32>, Error> {
    let reader = WavReader::<BufReader<File>>::open(path)?;
    Ok(reader.into_samples())
}

// The destination array is an interleaved audio stream.
// Returns false if there are no more entries to read from the source.
fn copy_stream(source: &mut WavIntoSamples<BufReader<File>, f32>, dest: &mut [f32]) -> bool {
    let mut dest_iter = dest.iter_mut();
    for sample in source.flatten() {
        *dest_iter.next().unwrap() = sample;
        if dest_iter.len() == 0 {
            break;
        }
    }

    let source_eof = dest_iter.len() > 0;

    // Zero-fill the remainder of the destination array if we finish consuming
    // the source.
    for sample in dest_iter {
        *sample = 0.0;
    }

    !source_eof
}

fn main() -> Result<(), Error> {
    let args = Args::from_args();
    let opt: Options = json5::from_str(&fs::read_to_string(&args.config_file)?)?;

    let pa = portaudio::PortAudio::new()?;

    let mut processor = Processor::new(&InitializationConfig {
        num_capture_channels: opt.capture.num_channels as i32,
        num_render_channels: opt.render.num_channels as i32,
        ..Default::default()
    })?;

    processor.set_config(opt.config.clone());

    let running = Arc::new(AtomicBool::new(true));

    let mut capture_source =
        if let Some(path) = &opt.capture.source_path { Some(open_wav_reader(path)?) } else { None };
    let mut capture_preprocess_sink = if let Some(path) = &opt.capture.preprocess_sink_path {
        Some(open_wav_writer(path, opt.capture.num_channels)?)
    } else {
        None
    };
    let mut capture_postprocess_sink = if let Some(path) = &opt.capture.postprocess_sink_path {
        Some(open_wav_writer(path, opt.capture.num_channels)?)
    } else {
        None
    };
    let mut render_source =
        if let Some(path) = &opt.render.source_path { Some(open_wav_reader(path)?) } else { None };

    let audio_callback = {
        // Allocate buffers outside the performance-sensitive audio loop.
        let mut input_mut =
            vec![0f32; NUM_SAMPLES_PER_FRAME as usize * opt.capture.num_channels as usize];

        let running = running.clone();
        let mute = opt.render.mute;
        let mut processor = processor.clone();
        move |portaudio::DuplexStreamCallbackArgs { in_buffer, out_buffer, frames, .. }| {
            assert_eq!(frames, NUM_SAMPLES_PER_FRAME as usize);

            let mut should_continue = true;

            if let Some(source) = &mut capture_source {
                if !copy_stream(source, &mut input_mut) {
                    should_continue = false;
                }
            } else {
                input_mut.copy_from_slice(in_buffer);
            }

            if let Some(sink) = &mut capture_preprocess_sink {
                for sample in &input_mut {
                    sink.write_sample(*sample).unwrap();
                }
            }

            processor.process_capture_frame(&mut input_mut).unwrap();

            if let Some(sink) = &mut capture_postprocess_sink {
                for sample in &input_mut {
                    sink.write_sample(*sample).unwrap();
                }
            }

            if let Some(source) = &mut render_source {
                if !copy_stream(source, out_buffer) {
                    should_continue = false;
                }
            } else {
                out_buffer.iter_mut().for_each(|m| *m = 0.0)
            }

            processor.process_render_frame(out_buffer).unwrap();

            if mute {
                out_buffer.iter_mut().for_each(|m| *m = 0.0)
            }

            if should_continue {
                portaudio::Continue
            } else {
                running.store(false, Ordering::SeqCst);
                portaudio::Complete
            }
        }
    };

    let stream_settings = create_stream_settings(&pa, &opt)?;
    let mut stream = pa.open_non_blocking_stream(stream_settings, audio_callback)?;
    stream.start()?;

    ctrlc::set_handler({
        let running = running.clone();
        move || {
            running.store(false, Ordering::SeqCst);
        }
    })?;

    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(10));
    }

    println!("{:#?}", processor.get_stats());

    Ok(())
}
