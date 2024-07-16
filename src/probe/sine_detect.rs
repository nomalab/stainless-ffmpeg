use crate::format_context::FormatContext;
use crate::order::{
  filter_input::FilterInput, filter_output::FilterOutput, input::Input, input_kind::InputKind,
  output::Output, output_kind::OutputKind, stream::Stream, Filter, Order, OutputResult,
  OutputResult::Entry, ParameterValue,
};
use crate::probe::deep::{
  AudioDetails, CheckName, CheckParameterValue, SineResult, StreamProbeResult, Track,
};
use crate::stream::Stream as ContextStream;
use ffmpeg_sys_next::AVMediaType;
use std::collections::{BTreeMap, HashMap};

pub fn sine_init(
  filename: &str,
  audio_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue>,
) -> Result<Order, String> {
  let mut order = create_graph(filename, audio_indexes, params).unwrap();
  if let Err(msg) = order.setup() {
    error!("{:?}", msg);
  }
  Ok(order)
}

pub fn create_graph(
  filename: &str,
  audio_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue>,
) -> Result<Order, String> {
  let mut filters = vec![];
  let mut inputs = vec![];
  let mut outputs = vec![];

  match params.get("pairing_list") {
    Some(pairing_list) => {
      if let Some(pairs) = pairing_list.pairs.clone() {
        for audio_index in audio_indexes {
          let input_identifier = format!("audio_input_{audio_index}");
          let output_identifier = format!("audio_output_{audio_index}");
          let input_streams = vec![Stream {
            index: audio_index,
            label: Some(input_identifier.clone()),
          }];
          let mut lavfi_keys = vec![];

          let channels = Track::get_channels_number(pairs.clone(), audio_index as u8);
          for channel in 1..(channels + 1) {
            let crest_factor = format!("lavfi.astats.{channel}.Crest_factor");
            lavfi_keys.push(crest_factor);
            let zero_crossing = format!("lavfi.astats.{channel}.Zero_crossings");
            lavfi_keys.push(zero_crossing);
          }

          let mut astats_params: HashMap<String, ParameterValue> = HashMap::new();
          astats_params.insert("metadata".to_string(), ParameterValue::Bool(true));
          astats_params.insert("reset".to_string(), ParameterValue::Int64(1));
          let mut aformat_params: HashMap<String, ParameterValue> = HashMap::new();
          let channel_layouts = ParameterValue::String("mono".to_string());
          aformat_params.insert("channel_layouts".to_string(), channel_layouts);

          filters.push(Filter {
            name: "astats".to_string(),
            label: Some(format!("astats_filter{audio_index}")),
            parameters: astats_params.clone(),
            inputs: Some(vec![FilterInput {
              kind: InputKind::Stream,
              stream_label: input_identifier,
            }]),
            outputs: None,
          });

          filters.push(Filter {
            name: "aformat".to_string(),
            label: Some(format!("aformat_filter{audio_index}")),
            parameters: aformat_params.clone(),
            inputs: None,
            outputs: Some(vec![FilterOutput {
              stream_label: output_identifier.clone(),
            }]),
          });

          inputs.push(Input::Streams {
            id: audio_index,
            path: filename.to_string(),
            streams: input_streams,
          });
          outputs.push(Output {
            kind: Some(OutputKind::AudioMetadata),
            keys: lavfi_keys,
            stream: Some(output_identifier),
            path: None,
            streams: vec![],
            parameters: HashMap::new(),
          });
        }
      }
    }
    None => {
      return Err("No input message for the 1000Hz analysis (audio qualification)".to_string())
    }
  }

  Order::new(inputs, filters, outputs)
}

pub fn detect_sine(
  output_results: &BTreeMap<CheckName, Vec<OutputResult>>,
  filename: &str,
  streams: &mut [StreamProbeResult],
  audio_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue>,
  frame_duration: f32,
  audio_details: Vec<AudioDetails>,
) {
  for index in audio_indexes.clone() {
    streams[index as usize].detected_sine = Some(vec![]);
  }
  let results = output_results.get(&CheckName::Tone).unwrap();
  info!("END OF SINE PROCESS");
  info!("-> {:?} frames processed", results.len());

  let mut tracks: Vec<Vec<Track>> = Vec::new();
  let mut sine: SineResult = Default::default();
  let mut range_value: f64 = 1.0; //contains the range values to code a sample
  let mut last_starts: HashMap<Track, Option<i64>> = HashMap::new(); //contains the previous declared start
  let mut last_crests: HashMap<Track, f64> = HashMap::new(); //contains the crest factor from the previous frame
  let mut frames: HashMap<Track, f64> = HashMap::new(); //contains the current frame number
  let mut zero_cross: HashMap<Track, f64> = HashMap::new(); //contains the number of zero crossings
  let mut max_duration = None;
  let mut min_duration = None;
  if let Some(duration) = params.get("duration") {
    min_duration = duration.min;
    max_duration = duration.max;
  }
  match params.get("pairing_list") {
    Some(pairing_list) => {
      if let Some(pairs) = pairing_list.pairs.clone() {
        tracks = pairs;
      }
    }
    None => return warn!("No input message for the 1000Hz analysis (audio qualification)"),
  }

  let mut context = FormatContext::new(filename).unwrap();
  if let Err(msg) = context.open_input() {
    context.close_input();
    error!("{:?}", msg);
    return;
  }
  for result in results {
    if let Entry(entry_map) = result {
      if let Some(stream_id) = entry_map.get("stream_id") {
        let index: u8 = stream_id.parse().unwrap();

        let audio_stream_details = audio_details
          .iter()
          .find(|d| d.stream_index == index as i32);
        let sample_rate = audio_stream_details.map(|d| d.sample_rate).unwrap_or(1) as f64;
        let nb_samples = audio_stream_details.map(|d| d.nb_samples).unwrap_or(1) as f64;
        let time_base = sample_rate / nb_samples;

        let end_from_duration = match audio_stream_details.and_then(|d| d.stream_duration) {
          Some(duration) => ((duration - frame_duration) * 1000.0).round() as i64,
          None => {
            (((results.len() as f64 / audio_indexes.len() as f64) - 1.0) / time_base * 1000.0)
              .round() as i64
          }
        };
        if streams[(index) as usize].detected_sine.is_none() {
          error!("Error : unexpected detection on stream ${index}");
          break;
        }
        let detected_sine = streams[index as usize].detected_sine.as_mut().unwrap();

        if let Ok(stream) = ContextStream::new(context.get_stream(index as isize)) {
          if let AVMediaType::AVMEDIA_TYPE_AUDIO = context.get_stream_type(index as isize) {
            let sample_fmt = stream.get_sample_fmt();
            range_value = match sample_fmt.as_str() {
              "s16" => i16::MAX as f64,
              "s32" => i32::MAX as f64,
              "s64" => i64::MAX as f64,
              _ => 1.0,
            };
          }
        }

        /*
         * If a crest factor of a signal is sqrt(2), that means that the signal is a sine.
         * If no previous start sine have been declared, we define one.
         * We can look for the end of the sine :
         *      -in the middle of the signal : as soon as we get a crest factor != sqrt(2)
         *       that means that the previous frame was the end of the sine.
         *      -at the end of the signal : if we get a crest factor == sqrt(2) until the
         *       end of the signal, that means that there is a sine until the end.
         * 1000Hz is a sine with 1000 periods per second. There is 2 zero crossings per
         * period :
         * if zero_cross_nb / sine_duration(second) = 2000, then this is a 1000Hz.
         */
        let mut crest_factor_key;
        let mut zero_crossing_key;

        let channels = Track::get_channels_number(tracks.clone(), index);
        for channel in 1..(channels + 1) {
          crest_factor_key = format!("lavfi.astats.{channel}.Crest_factor");
          zero_crossing_key = format!("lavfi.astats.{channel}.Zero_crossings");
          let audio_stream_key = Track::new(index, channel);

          //update frame count
          let frame = *frames.get(&audio_stream_key).unwrap_or(&0.0);
          frames.insert(audio_stream_key.clone(), frame + 1.0);
          let last_start_opt = last_starts.get(&audio_stream_key).unwrap_or(&None);

          //update signal zero crossing count
          if let Some(value) = entry_map.get(&zero_crossing_key) {
            let prev_value = zero_cross.get(&audio_stream_key).unwrap_or(&0.0);
            let new_value = prev_value + value.parse::<f64>().unwrap();
            zero_cross.insert(audio_stream_key.clone(), new_value);
          }

          if let Some(value) = entry_map.get(&crest_factor_key) {
            let crest_factor = value.parse::<f64>().unwrap() / range_value;

            //sqrt(2) +/- 1e-3
            if (2_f64.sqrt() - 1e-3_f64..2_f64.sqrt() + 1e-3_f64).contains(&crest_factor) {
              if last_start_opt.is_some() {
                if let Some(last_start) = last_start_opt {
                  //check if audio ends => 1000Hz until the end
                  if (frame / time_base * 1000.0).round() as i64 == end_from_duration {
                    sine.channel = channel;
                    sine.start = *last_start;
                    sine.end = end_from_duration;
                    //check if sine is a 1000Hz => push and reset
                    if let Some(zero_crossing) = zero_cross.get(&audio_stream_key.clone()) {
                      let sine_duration =
                        ((frame + 1.0) / time_base * 1000.0).round() as i64 - sine.start;
                      if (zero_crossing / sine_duration as f64) == 2.0 {
                        detected_sine.push(sine);
                        last_starts.insert(audio_stream_key.clone(), None);
                        zero_cross.insert(audio_stream_key.clone(), 0.0);
                        if let Some(max) = max_duration {
                          if sine_duration > max as i64 {
                            detected_sine.pop();
                          }
                        }
                        if let Some(min) = min_duration {
                          if sine_duration < min as i64 {
                            detected_sine.pop();
                          }
                        }
                      }
                    }
                  }
                }
              } else {
                sine.start = (frame / time_base * 1000.0).round() as i64;
                last_starts.insert(audio_stream_key.clone(), Some(sine.start));
              }
            } else if (2_f64.sqrt() - 1e-3_f64..2_f64.sqrt() + 1e-3_f64)
              .contains(last_crests.get(&audio_stream_key).unwrap_or(&0.0))
              && last_start_opt.is_some()
            {
              if let Some(last_start) = last_start_opt {
                sine.channel = channel;
                sine.start = *last_start;
                sine.end = ((frame - 1.0) / time_base * 1000.0).round() as i64;
                //check if sine is a 1000Hz => push and reset
                let sine_duration = (frame / time_base * 1000.0).round() as i64 - sine.start;
                if let Some(zero_crossing) = zero_cross.get(&audio_stream_key) {
                  if (zero_crossing / sine_duration as f64) == 2.0 {
                    detected_sine.push(sine);
                    last_starts.insert(audio_stream_key.clone(), None);
                    zero_cross.insert(audio_stream_key.clone(), 0.0);
                    if let Some(max) = max_duration {
                      if sine_duration > max as i64 {
                        detected_sine.pop();
                      }
                    }
                    if let Some(min) = min_duration {
                      if sine_duration < min as i64 {
                        detected_sine.pop();
                      }
                    }
                  }
                }
              }
            }
            //update last crest factor
            last_crests.insert(audio_stream_key, crest_factor);
          }
        }
      }
    }
  }
}
