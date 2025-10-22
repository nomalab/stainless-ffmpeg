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
  probe::deep::{CheckName, CheckParameterValue, FreezeResult, StreamProbeResult, VideoDetails},
};
use std::collections::{BTreeMap, HashMap};

pub fn freeze_init(
  filename: &str,
  video_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue>,
) -> Result<Order, String> {
  let mut order = create_graph(filename, video_indexes, params).unwrap();
  if let Err(msg) = order.setup() {
    error!("{:?}", msg);
  }
  Ok(order)
}

pub fn create_graph(
  filename: &str,
  video_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue>,
) -> Result<Order, String> {
  let mut filters = vec![];
  let mut inputs = vec![];
  let mut outputs = vec![];
  for i in video_indexes {
    let input_identifier = format!("video_input_{i}");
    let output_identifier = format!("video_output_{i}");

    let input_streams = vec![Stream {
      index: i,
      label: Some(input_identifier.clone()),
    }];

    let mut freezedetect_params: HashMap<String, ParameterValue> = HashMap::new();
    if let Some(duration) = params.get("duration") {
      if let Some(min_duration) = duration.min {
        let min = (min_duration as f64 - 1.0) * 1000.0;
        freezedetect_params.insert("duration".to_string(), ParameterValue::Float(min));
      }
    }
    if let Some(noise) = params.get("noise") {
      if let Some(noise_th) = noise.th {
        freezedetect_params.insert("noise".to_string(), ParameterValue::Float(noise_th));
      }
    }

    filters.push(Filter {
      name: "freezedetect".to_string(),
      label: Some(format!("freezedetect_filter{i}")),
      parameters: freezedetect_params.clone(),
      inputs: Some(vec![FilterInput {
        kind: InputKind::Stream,
        stream_label: input_identifier,
      }]),
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
      kind: Some(OutputKind::VideoMetadata),
      keys: vec![
        "lavfi.freezedetect.freeze_start".to_string(),
        "lavfi.freezedetect.freeze_end".to_string(),
        "lavfi.freezedetect.freeze_duration".to_string(),
      ],
      stream: Some(output_identifier),
      path: None,
      streams: vec![],
      parameters: HashMap::new(),
    });
  }

  Order::new(inputs, filters, outputs)
}

pub fn detect_freeze(
  output_results: &BTreeMap<CheckName, Vec<OutputResult>>,
  streams: &mut [StreamProbeResult],
  video_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue>,
  video_details: VideoDetails,
) {
  for index in video_indexes {
    streams[index as usize].detected_freeze = Some(vec![]);
  }
  let results = output_results.get(&CheckName::Freeze).unwrap();
  info!("END OF FREEZE PROCESS");
  info!("-> {:?} frames processed", results.len());

  let end_from_duration = match video_details.stream_duration {
    Some(duration) => ((duration - video_details.frame_duration) * 1000.0).round() as i64,
    None => ((results.len() as f32 - 1.0) / video_details.frame_rate * 1000.0).round() as i64,
  };
  let mut max_duration = None;
  let mut min_duration = None;
  if let Some(duration) = params.get("duration") {
    max_duration = duration.max;
    min_duration = duration.min;
  }

  for result in results {
    if let Entry(entry_map) = result {
      if let Some(stream_id) = entry_map.get("stream_id") {
        let index: i32 = stream_id.parse().unwrap();
        if streams[(index) as usize].detected_freeze.is_none() {
          error!("Error : unexpected detection on stream ${index}");
          break;
        }
        let detected_freeze = streams[(index) as usize].detected_freeze.as_mut().unwrap();
        let mut freeze = FreezeResult {
          start: 0,
          end: end_from_duration,
        };

        if let Some(value) = entry_map.get("lavfi.freezedetect.freeze_start") {
          freeze.start = (value.parse::<f32>().unwrap() * 1000.0).round() as i64;
          detected_freeze.push(freeze);
        }
        if let Some(value) = entry_map.get("lavfi.freezedetect.freeze_end") {
          if let Some(last_detect) = detected_freeze.last_mut() {
            last_detect.end = ((value.parse::<f32>().unwrap() - video_details.frame_duration)
              * 1000.0)
              .round() as i64;
            let freeze_duration = last_detect.end - last_detect.start
              + (video_details.frame_duration * 1000.0).round() as i64;
            if let Some(max) = max_duration {
              if freeze_duration > max as i64 {
                detected_freeze.pop();
              }
            }
            if let Some(min) = min_duration {
              if freeze_duration < min as i64 {
                detected_freeze.pop();
              }
            }
          }
        }
      }
    }
  }
}
