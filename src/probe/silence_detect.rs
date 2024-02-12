use crate::order::{
  filter_input::FilterInput, filter_output::FilterOutput, input::Input, input_kind::InputKind,
  output::Output, output_kind::OutputKind, stream::Stream, Filter, Order, OutputResult,
  OutputResult::Entry, ParameterValue,
};
use crate::probe::deep::{
  CheckName, CheckParameterValue, SilenceResult, StreamProbeResult, VideoDetails,
};
use std::collections::{BTreeMap, HashMap};

pub fn silence_init(
  filename: &str,
  audio_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue>,
) -> Result<Order, String> {
  let mut order = create_graph(filename, audio_indexes.clone(), params).unwrap();
  if let Err(msg) = order.setup() {
    error!("{:?}", msg);
  }
  Ok(order)
}

pub fn create_graph<S: ::std::hash::BuildHasher>(
  filename: &str,
  audio_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue, S>,
) -> Result<Order, String> {
  let mut filters = vec![];
  let mut inputs = vec![];
  let mut outputs = vec![];
  for i in audio_indexes {
    let input_identifier = format!("audio_input_{i}");
    let output_identifier = format!("audio_output_{i}");

    let input_streams = vec![Stream {
      index: i,
      label: Some(input_identifier.clone()),
    }];

    let mut silencedetect_params: HashMap<String, ParameterValue> = HashMap::new();
    if let Some(min_duration) = params.get("duration").and_then(|duration| duration.min) {
      let min = (min_duration as f64 - 1.0) * 1000.0;
      silencedetect_params.insert("duration".to_string(), ParameterValue::Float(min));
    }
    if let Some(noise_th) = params.get("noise").and_then(|noise| noise.th) {
      silencedetect_params.insert("noise".to_string(), ParameterValue::Float(noise_th));
    }

    let channel_layouts = ParameterValue::String("mono".to_string());
    let mut aformat_params: HashMap<String, ParameterValue> = HashMap::new();
    aformat_params.insert("channel_layouts".to_string(), channel_layouts);

    filters.push(Filter {
      name: "silencedetect".to_string(),
      label: Some(format!("silencedetect_filter{i}")),
      parameters: silencedetect_params.clone(),
      inputs: Some(vec![FilterInput {
        kind: InputKind::Stream,
        stream_label: input_identifier,
      }]),
      outputs: None,
    });
    filters.push(Filter {
      name: "aformat".to_string(),
      label: Some(format!("aformat_filter{i}")),
      parameters: aformat_params.clone(),
      inputs: None,
      outputs: Some(vec![FilterOutput {
        stream_label: output_identifier.clone(),
      }]),
    });

    inputs.push(Input::Streams {
      id: i,
      path: filename.to_string(),
      streams: input_streams,
    });
    outputs.push(Output {
      kind: Some(OutputKind::AudioMetadata),
      keys: vec![
        "lavfi.silence_start".to_string(),
        "lavfi.silence_end".to_string(),
        "lavfi.silence_duration".to_string(),
      ],
      stream: Some(output_identifier),
      path: None,
      streams: vec![],
      parameters: HashMap::new(),
    });
  }

  Order::new(inputs, filters, outputs)
}

pub fn detect_silence<S: ::std::hash::BuildHasher>(
  output_results: &BTreeMap<CheckName, Vec<OutputResult>>,
  streams: &mut [StreamProbeResult],
  audio_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue, S>,
  video_details: VideoDetails,
) {
  for index in audio_indexes.clone() {
    streams[index as usize].detected_silence = Some(vec![]);
  }
  let results = output_results.get(&CheckName::Silence).unwrap();
  info!("END OF SILENCE PROCESS");
  info!("-> {:?} frames processed", results.len());

  let end_from_duration = (((results.len() as f64 / audio_indexes.clone().len() as f64) - 1.0)
    / video_details.frame_rate as f64
    * 1000.0)
    .round() as i64;
  let mut max_duration = None;
  if let Some(duration) = params.get("duration") {
    max_duration = duration.max;
  }

  for result in results {
    if let Entry(entry_map) = result {
      if let Some(stream_id) = entry_map.get("stream_id") {
        let index: i32 = stream_id.parse().unwrap();
        if streams[(index) as usize].detected_silence.is_none() {
          error!("Error : unexpected detection on stream ${index}");
          break;
        }
        let detected_silence = streams[(index) as usize].detected_silence.as_mut().unwrap();
        let mut silence = SilenceResult {
          start: 0,
          end: end_from_duration,
        };

        if let Some(value) = entry_map.get("lavfi.silence_start") {
          silence.start = (value.parse::<f64>().unwrap() * 1000.0).round() as i64;
          detected_silence.push(silence);
        }
        if let Some(value) = entry_map.get("lavfi.silence_end") {
          if let Some(last_detect) = detected_silence.last_mut() {
            last_detect.end =
              ((value.parse::<f64>().unwrap() - video_details.frame_duration as f64) * 1000.0)
                .round() as i64;
          }
        }
        if let Some(value) = entry_map.get("lavfi.silence_duration") {
          if let Some(max) = max_duration {
            if (value.parse::<f64>().unwrap() * 1000.0).round() as u64 > max {
              detected_silence.pop();
            }
          }
        }
      }
    }
  }
  for index in audio_indexes {
    let detected_silence = streams[(index) as usize].detected_silence.as_mut().unwrap();
    if detected_silence.len() == 1
      && detected_silence[0].start == 0
      && detected_silence[0].end == end_from_duration
    {
      streams[(index) as usize].silent_stream = Some(true);
    }
    if let Some(max) = max_duration {
      if let Some(last_detect) = detected_silence.last() {
        let silence_duration = last_detect.end - last_detect.start
          + (video_details.frame_duration * 1000.0).round() as i64;
        if silence_duration > max as i64 {
          detected_silence.pop();
        }
      }
    }
  }
}
