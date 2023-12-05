/// An example binary to help evaluate webrtc audio processing pipeline in a crosstalk scenario.
///
/// It plays one track from tonari built-in speakers, another track from an external speaker
/// that is to be placed in front of tonari and then it records the mixed result and individual
/// processing steps done on it.
///
/// ```
/// $ cargo run --example crosstalk-benchmark --features derive_serde -- \
///     --config-file examples/crosstalk-benchmark.json5
/// ```
use failure::{format_err, Error};
use hound::{WavIntoSamples, WavReader, WavWriter};
use portaudio::StreamCallbackResult;
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
    source_path: PathBuf,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
struct PlaybackOptions {
    /// Played from the tonari speakers as if coming from the far end.
    far_end: RenderOptions,
    /// Played from a testing speaker placed *in front of* tonari to simulate a local sound source like a person.
    near_end: RenderOptions,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
struct Options {
    /// Options for audio capture / recording.
    capture: CaptureOptions,
    /// Options for audio render / playback.
    playback: PlaybackOptions,
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
    Err(format_err!("Audio device matching \"{}\" not found.", device_name))
}

fn create_input_stream_settings(
    pa: &portaudio::PortAudio,
    opt: &CaptureOptions,
) -> Result<portaudio::InputStreamSettings<f32>, Error> {
    let input_device = match_device(pa, Regex::new(&opt.device_name)?)?;
    let input_device_info = &pa.device_info(input_device)?;
    let input_params = portaudio::StreamParameters::<f32>::new(
        input_device,
        opt.num_channels as i32,
        AUDIO_INTERLEAVED,
        input_device_info.default_low_input_latency,
    );

    Ok(portaudio::InputStreamSettings::new(
        input_params,
        f64::from(AUDIO_SAMPLE_RATE),
        NUM_SAMPLES_PER_FRAME as u32,
    ))
}

fn create_output_stream_settings(
    pa: &portaudio::PortAudio,
    opt: &RenderOptions,
) -> Result<portaudio::OutputStreamSettings<f32>, Error> {
    let output_device_far_end = match_device(pa, Regex::new(&opt.device_name)?)?;
    let output_device_info = &pa.device_info(output_device_far_end)?;
    let output_params = portaudio::StreamParameters::<f32>::new(
        output_device_far_end,
        opt.num_channels as i32,
        AUDIO_INTERLEAVED,
        output_device_info.default_low_output_latency,
    );

    Ok(portaudio::OutputStreamSettings::new(
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
    'outer: for sample in source {
        for channel in &sample {
            *dest_iter.next().unwrap() = *channel;
            if dest_iter.len() == 0 {
                break 'outer;
            }
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

fn create_output_callback(
    mut source: WavIntoSamples<BufReader<File>, f32>,
    mut processor: Processor,
    running: Arc<AtomicBool>,
) -> impl FnMut(portaudio::OutputStreamCallbackArgs<f32>) -> StreamCallbackResult + 'static {
    move |portaudio::OutputStreamCallbackArgs { buffer, frames, .. }| {
        assert_eq!(frames, NUM_SAMPLES_PER_FRAME as usize);

        let should_continue = copy_stream(&mut source, buffer);

        processor.process_render_frame(buffer).unwrap();

        if should_continue {
            portaudio::Continue
        } else {
            running.store(false, Ordering::SeqCst);
            portaudio::Complete
        }
    }
}

fn main() -> Result<(), Error> {
    let args = Args::from_args();
    let opt: Options = json5::from_str(&fs::read_to_string(&args.config_file)?)?;

    let pa = portaudio::PortAudio::new()?;

    let mut processor = Processor::new(&InitializationConfig {
        num_capture_channels: opt.capture.num_channels as i32,
        num_render_channels: opt.playback.far_end.num_channels as i32,
        ..Default::default()
    })?;

    processor.set_config(opt.config.clone());

    let running = Arc::new(AtomicBool::new(true));

    let mut capture_preprocess_sink = opt
        .capture
        .preprocess_sink_path
        .as_ref()
        .map(|path| open_wav_writer(path, opt.capture.num_channels))
        .transpose()?;
    let mut capture_postprocess_sink = opt
        .capture
        .postprocess_sink_path
        .as_ref()
        .map(|path| open_wav_writer(path, opt.capture.num_channels))
        .transpose()?;
    let far_end_source = open_wav_reader(&opt.playback.far_end.source_path)?;
    let near_end_source = open_wav_reader(&opt.playback.near_end.source_path)?;

    let input_stream_settings = create_input_stream_settings(&pa, &opt.capture)?;
    let mut input_stream = pa.open_non_blocking_stream(input_stream_settings, {
        let mut processor = processor.clone();
        let mut input_mut =
            vec![0f32; NUM_SAMPLES_PER_FRAME as usize * opt.capture.num_channels as usize];
        move |portaudio::InputStreamCallbackArgs { buffer, frames, .. }| {
            assert_eq!(frames, NUM_SAMPLES_PER_FRAME as usize);

            input_mut.copy_from_slice(buffer);

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

            portaudio::Continue
        }
    })?;

    let far_end_stream_settings = create_output_stream_settings(&pa, &opt.playback.far_end)?;
    let mut far_end_stream = pa.open_non_blocking_stream(
        far_end_stream_settings,
        create_output_callback(far_end_source, processor.clone(), running.clone()),
    )?;

    let near_end_stream_settings = create_output_stream_settings(&pa, &opt.playback.near_end)?;
    let mut near_end_stream = pa.open_non_blocking_stream(
        near_end_stream_settings,
        create_output_callback(near_end_source, processor.clone(), running.clone()),
    )?;

    input_stream.start()?;
    far_end_stream.start()?;
    near_end_stream.start()?;

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
