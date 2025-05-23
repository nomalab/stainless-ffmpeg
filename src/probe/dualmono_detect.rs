use crate::{
  order::{
    filter_input::FilterInput,
    filter_output::FilterOutput,
    input::Input,
    input_kind::InputKind,
    output::Output,
    output_kind::OutputKind,
    stream::Stream,
    Filter, Order,
    OutputResult::{self, Entry},
    ParameterValue,
  },
  probe::deep::{AudioDetails, CheckName, CheckParameterValue, DualMonoResult, StreamProbeResult},
};
use std::collections::{BTreeMap, HashMap};

pub fn dualmono_init(
  filename: &str,
  params: HashMap<String, CheckParameterValue>,
) -> Result<Order, String> {
  let mut order = create_graph(filename, params).unwrap();
  if let Err(msg) = order.setup() {
    error!("{:?}", msg);
  }
  Ok(order)
}

pub fn create_graph<S: ::std::hash::BuildHasher>(
  filename: &str,
  params: HashMap<String, CheckParameterValue, S>,
) -> Result<Order, String> {
  let mut filters = vec![];
  let mut inputs = vec![];
  let mut outputs = vec![];

  let mut aphasemeter_params: HashMap<String, ParameterValue> = HashMap::new();
  if let Some(min_duration) = params.get("duration").and_then(|duration| duration.min) {
    let min = (min_duration * 1000) as i64;
    aphasemeter_params.insert("duration".to_string(), ParameterValue::Int64(min));
  }
  aphasemeter_params.insert("video".to_string(), ParameterValue::Bool(false));
  aphasemeter_params.insert("phasing".to_string(), ParameterValue::Bool(true));
  aphasemeter_params.insert("tolerance".to_string(), ParameterValue::Float(0.001));

  let channel_layouts = ParameterValue::String("mono".to_string());
  let mut aformat_params: HashMap<String, ParameterValue> = HashMap::new();
  aformat_params.insert("channel_layouts".to_string(), channel_layouts);

  match params.get("pairing_list") {
    Some(pairing_list) => {
      if let Some(pairs) = pairing_list.pairs.clone() {
        for (iter, pair) in pairs.iter().enumerate() {
          if pair.len() == 2 || pair.len() == 1 {
            let mut amerge_params: HashMap<String, ParameterValue> = HashMap::new();
            let mut amerge_input = vec![];
            let mut input_streams_vec = vec![];
            let output_label = format!("audio_output_{iter}");
            let mut is_stereo = true;

            for track in pair {
              is_stereo =
                (pair.len() == 1 && track.channel == 2) || pair.len() == 2 && track.channel == 1;
              let input_label = format!("audio_input_{}", track.index);
              amerge_input.push(FilterInput {
                kind: InputKind::Stream,
                stream_label: input_label.clone(),
              });
              input_streams_vec.push(Stream {
                index: track.index as u32,
                label: Some(input_label),
              });
            }

            if is_stereo {
              amerge_params.insert(
                "inputs".to_string(),
                ParameterValue::Int64(pair.len() as i64),
              );
              filters.push(Filter {
                name: "amerge".to_string(),
                label: Some(format!("amerge_filter{iter}")),
                parameters: amerge_params,
                inputs: Some(amerge_input),
                outputs: None,
              });
              filters.push(Filter {
                name: "aphasemeter".to_string(),
                label: Some(format!("aphasemeter_filter{iter}")),
                parameters: aphasemeter_params.clone(),
                inputs: None,
                outputs: None,
              });
              filters.push(Filter {
                name: "aformat".to_string(),
                label: Some(format!("aformat_filter{iter}")),
                parameters: aformat_params.clone(),
                inputs: None,
                outputs: Some(vec![FilterOutput {
                  stream_label: output_label.clone(),
                }]),
              });

              inputs.push(Input::Streams {
                id: iter as u32,
                path: filename.to_string(),
                streams: input_streams_vec,
              });
              outputs.push(Output {
                kind: Some(OutputKind::AudioMetadata),
                keys: vec![
                  "lavfi.aphasemeter.mono_start".to_string(),
                  "lavfi.aphasemeter.mono_end".to_string(),
                  "lavfi.aphasemeter.mono_duration".to_string(),
                ],
                stream: Some(output_label),
                path: None,
                streams: vec![],
                parameters: HashMap::new(),
              });
            }
          }
        }
      }
    }
    None => warn!("No input message for the dualmono analysis (list of indexes to merge)"),
  }

  Order::new(inputs, filters, outputs)
}

pub fn detect_dualmono<S: ::std::hash::BuildHasher>(
  output_results: &BTreeMap<CheckName, Vec<OutputResult>>,
  streams: &mut [StreamProbeResult],
  audio_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue, S>,
  frame_duration: f32,
  audio_details: Vec<AudioDetails>,
) {
  for index in audio_indexes.clone() {
    streams[index as usize].detected_dualmono = Some(vec![]);
  }
  let results = output_results.get(&CheckName::DualMono).unwrap();
  info!("END OF DUALMONO PROCESS");
  info!("-> {:?} frames processed", results.len());

  let mut max_duration = None;
  if let Some(duration) = params.get("duration") {
    max_duration = duration.max;
  }

  let mut audio_stream_qualif_number = 0;
  match params.get("pairing_list") {
    Some(pairing_list) => {
      if let Some(pairs) = pairing_list.pairs.clone() {
        for pair in pairs {
          for track in pair.clone() {
            if (pair.len() == 1 && track.channel == 2) || pair.len() == 2 && track.channel == 1 {
              audio_stream_qualif_number += 1;
            }
          }
        }
      }
    }
    None => warn!("No input message for the dualmono analysis (list of indexes to merge)"),
  }

  for result in results {
    if let Entry(entry_map) = result {
      if let Some(stream_id) = entry_map.get("stream_id") {
        let index: i32 = stream_id.parse().unwrap();
        let audio_stream_details = audio_details.iter().find(|d| d.stream_index == index);
        let end_from_duration = (((results.len() as f64 / audio_stream_qualif_number as f64) - 1.0)
          / (audio_stream_details.map(|d| d.sample_rate).unwrap_or(1) as f64
            / audio_stream_details
              .map(|d| d.samples_per_frame)
              .unwrap_or(1) as f64)
          * 1000.0)
          .round() as i64;
        if streams[(index) as usize].detected_dualmono.is_none() {
          error!("Error : unexpected detection on stream ${index}");
          break;
        }
        let detected_dualmono = streams[(index) as usize]
          .detected_dualmono
          .as_mut()
          .unwrap();
        let mut dualmono = DualMonoResult {
          start: 0,
          end: end_from_duration,
        };
        if let Some(value) = entry_map.get("lavfi.aphasemeter.mono_start") {
          dualmono.start = (value.parse::<f64>().unwrap() * 1000.0).round() as i64;
          detected_dualmono.push(dualmono);
        }
        if let Some(value) = entry_map.get("lavfi.aphasemeter.mono_end") {
          if let Some(last_detect) = detected_dualmono.last_mut() {
            last_detect.end =
              ((value.parse::<f64>().unwrap() - frame_duration as f64) * 1000.0).round() as i64;
          }
        }
        if let Some(value) = entry_map.get("lavfi.aphasemeter.mono_duration") {
          if let Some(max) = max_duration {
            if value.parse::<f64>().unwrap() * 1000.0 > max as f64 {
              detected_dualmono.pop();
            }
          }
        }
      }
    }
  }

  for index in audio_indexes {
    let detected_dualmono = streams[(index) as usize]
      .detected_dualmono
      .as_mut()
      .unwrap();
    if let Some(last_detect) = detected_dualmono.last() {
      let duration = last_detect.end - last_detect.start + (frame_duration * 1000.0).round() as i64;
      if let Some(max) = max_duration {
        if duration > max as i64 {
          detected_dualmono.pop();
        }
      }
    }
  }
}
