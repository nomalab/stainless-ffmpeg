use crate::order::{
  filter_input::FilterInput, filter_output::FilterOutput, input::Input, input_kind::InputKind,
  output::Output, output_kind::OutputKind, stream::Stream, Filter, Order, OutputResult,
  OutputResult::Entry, ParameterValue,
};
use crate::probe::deep::{
  CheckName, CheckParameterValue, FalseSceneResult, SceneResult, StreamProbeResult, VideoDetails,
};
use std::collections::{BTreeMap, HashMap};

pub fn scene_init(
  filename: &str,
  video_indexes: Vec<u32>,
  params: &HashMap<String, CheckParameterValue>,
) -> Result<Order, String> {
  let mut order = create_graph(filename, video_indexes, &params).unwrap();
  if let Err(msg) = order.setup() {
    error!("{:?}", msg);
  }
  Ok(order)
}

pub fn create_graph<S: ::std::hash::BuildHasher>(
  filename: &str,
  video_indexes: Vec<u32>,
  params: &HashMap<String, CheckParameterValue, S>,
) -> Result<Order, String> {
  let mut filters = vec![];
  let mut inputs = vec![];
  let mut outputs = vec![];

  for i in video_indexes {
    let input_identifier = format!("video_input_{i}");
    let output_identifier = format!("video_output_{i}");

    let mut scdet_params: HashMap<String, ParameterValue> = HashMap::new();
    if let Some(th) = params.get("threshold").and_then(|threshold| threshold.th) {
      scdet_params.insert("threshold".to_string(), ParameterValue::Float(th));
    }

    let input_streams = vec![Stream {
      index: i,
      label: Some(input_identifier.clone()),
    }];

    filters.push(Filter {
      name: "scdet".to_string(),
      label: Some(format!("scdet_filter{i}")),
      parameters: scdet_params,
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
      keys: vec!["lavfi.scd.time".to_string(), "lavfi.scd.score".to_string()],
      stream: Some(output_identifier),
      path: None,
      streams: vec![],
      parameters: HashMap::new(),
    });
  }

  Order::new(inputs, filters, outputs)
}

pub fn detect_scene(
  output_results: &BTreeMap<CheckName, Vec<OutputResult>>,
  streams: &mut [StreamProbeResult],
  video_indexes: Vec<u32>,
  video_details: VideoDetails,
) {
  for index in video_indexes {
    streams[index as usize].detected_scene = Some(vec![SceneResult {
      frame_start: 0,
      frame_end: 0,
      frames_length: 0,
      score: 100,
      index: 0,
    }]);
    streams[index as usize].detected_false_scene = Some(vec![]);
  }
  let results = output_results.get(&CheckName::Scene).unwrap();
  info!("END OF SCENE PROCESS");
  info!("-> {:?} frames processed", results.len());
  let stream_frames = match video_details.stream_frames {
    Some(frames) => frames,
    None => ((results.len() as f32 - 1.0) / video_details.frame_rate * 1000.0).round() as i64,
  };

  for result in results {
    if let Entry(entry_map) = result {
      if let Some(stream_id) = entry_map.get("stream_id") {
        let index: i32 = stream_id.parse().unwrap();
        if streams[(index) as usize].detected_scene.is_none() {
          error!("Error : unexpected detection on stream ${index}");
          break;
        }

        if let Some(value) = entry_map.get("lavfi.scd.time") {
          let detected_scene = streams[(index) as usize].detected_scene.as_mut().unwrap();
          let detected_false_scene = streams[(index) as usize]
            .detected_false_scene
            .as_mut()
            .unwrap();
          let frame_start = (value.parse::<f32>().unwrap() * video_details.frame_rate) as i64;
          let mut scene = SceneResult {
            frame_start,
            frame_end: stream_frames,
            frames_length: stream_frames - frame_start + 1,
            score: 0,
            index: 0,
          };
          let mut false_scene = FalseSceneResult { frame_index: 0 };

          if let Some(value) = entry_map.get("lavfi.scd.score") {
            scene.score = (value.parse::<f32>().unwrap()) as i32;
          }
          if let Some(last_detect) = detected_scene.last_mut() {
            last_detect.frame_end = scene.frame_start - 1;
            last_detect.frames_length = last_detect.frame_end - last_detect.frame_start + 1;
            scene.index = last_detect.index + 1;
            if scene.frame_start - last_detect.frame_start <= 1 {
              false_scene.frame_index = scene.frame_start;
              detected_false_scene.push(false_scene);
            }
          }

          detected_scene.push(scene);
        }
      }
    }
  }
}
