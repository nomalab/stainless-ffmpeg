use cpal::{SampleRate, Stream};
use env_logger::Builder;
use ringbuf::{Consumer, RingBuffer};
use stainless_ffmpeg::prelude::*;
use std::{collections::HashMap, convert::TryInto, env};

const SAMPLE_RATE: SampleRate = SampleRate(48_000);

fn main() {
  let mut builder = Builder::from_default_env();
  builder.init();

  let (mut producer, consumer) = RingBuffer::<f32>::new(50 * 1024 * 1024).split();
  let _stream = audio_player(consumer);

  if let Some(path) = env::args().last() {
    println!("{}", path);

    let mut format_context = FormatContext::new(&path).unwrap();
    format_context.open_input().unwrap();

    let mut first_audio_stream = None;
    for i in 0..format_context.get_nb_streams() {
      let stream_type = unsafe { format_context.get_stream_type(i as isize) };
      log::info!("Stream {}: {:?}", i, stream_type);

      if stream_type == AVMediaType::AVMEDIA_TYPE_AUDIO {
        first_audio_stream = Some(i as isize);
      }
    }

    let first_audio_stream = first_audio_stream.unwrap();

    let audio_decoder = AudioDecoder::new(
      "audio_decoder".to_string(),
      &format_context,
      first_audio_stream,
    )
    .unwrap();

    log::info!("{}", audio_decoder.get_sample_fmt_name());

    let mut graph = FilterGraph::new().unwrap();

    graph
      .add_input_from_audio_decoder("source_audio", &audio_decoder)
      .unwrap();

    let mut parameters = HashMap::new();
    parameters.insert(
      "sample_rates".to_string(),
      ParameterValue::String("48000".to_string()),
    );
    parameters.insert(
      "channel_layouts".to_string(),
      ParameterValue::String("stereo".to_string()),
    );
    parameters.insert(
      "sample_fmts".to_string(),
      ParameterValue::String("s32".to_string()),
    );

    let filter = Filter {
      name: "aformat".to_string(),
      label: Some("Format audio samples".to_string()),
      parameters,
      inputs: None,
      outputs: None,
    };

    let filter = graph.add_filter(&filter).unwrap();
    graph.add_audio_output("main_audio").unwrap();

    graph.connect_input("source_audio", 0, &filter, 0).unwrap();
    graph.connect_output(&filter, 0, "main_audio", 0).unwrap();
    graph.validate().unwrap();

    while let Ok(packet) = format_context.next_packet() {
      if packet.get_stream_index() != first_audio_stream {
        continue;
      }

      let frame = audio_decoder.decode(&packet).unwrap();

      let (frames, _) = graph.process(&[frame], &[]).unwrap();

      let frame = frames.first().unwrap();

      unsafe {
        let size = ((*frame.frame).channels * (*frame.frame).nb_samples) as usize;
        let sample_format: SampleFormat = (*frame.frame).format.try_into().unwrap();

        log::info!(
          "Frame {} samples, {} channels, {:?}, {} bytes // {} bytes",
          (*frame.frame).nb_samples,
          (*frame.frame).channels,
          sample_format,
          (*frame.frame).linesize[0],
          size,
        );

        let samples: Vec<i32> = Vec::from_raw_parts((*frame.frame).data[0] as _, size, size);

        let float_samples: Vec<f32> = samples
          .iter()
          .map(|value| (*value as f32) / i32::MAX as f32)
          .collect();

        producer.push_slice(&float_samples);
        std::mem::forget(samples);
      }
    }
  }
}

fn audio_player(mut consumer: Consumer<f32>) -> Stream {
  use cpal::traits::{DeviceTrait, HostTrait};

  let host = cpal::default_host();
  let device = host
    .default_output_device()
    .expect("no output device available");

  let mut supported_configs_range = device
    .supported_output_configs()
    .expect("error while querying configs");

  let supported_config = supported_configs_range
    .find(|config| {
      config.channels() == 2
        && SAMPLE_RATE >= config.min_sample_rate()
        && SAMPLE_RATE <= config.max_sample_rate()
    })
    .expect("no supported config?!")
    .with_sample_rate(SAMPLE_RATE);

  let config = supported_config.into();
  let mut started = false;

  device
    .build_output_stream(
      &config,
      move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        for data_index in data.iter_mut() {
          *data_index = 0.0;
        }
        if consumer.len() > 2 * 1024 * 1024 {
          started = true;
        }
        if started {
          consumer.pop_slice(data);
        }
      },
      move |err| log::error!("CPAL error: {:?}", err),
    )
    .unwrap()
}
