use crate::format_context::FormatContext;
use crate::order::{
  filter_input::FilterInput, filter_output::FilterOutput, input::Input, input_kind::InputKind,
  output::Output, output_kind::OutputKind, stream::Stream, Filter, Order, OutputResult::Entry,
  ParameterValue,
};
use crate::probe::deep::{CheckParameterValue, FalseSceneResult, SceneResult, StreamProbeResult};
use crate::stream::Stream as ContextStream;
use ffmpeg_sys_next::AVMediaType;
use std::collections::HashMap;

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

pub fn detect_scene<S: ::std::hash::BuildHasher>(
  filename: &str,
  streams: &mut [StreamProbeResult],
  video_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue, S>,
) {
  let mut order = create_graph(filename, video_indexes.clone(), &params).unwrap();
  if let Err(msg) = order.setup() {
    error!("{:?}", msg);
    return;
  }
  for index in video_indexes {
    streams[index as usize].detected_scene = Some(vec![]);
    streams[index as usize].detected_false_scene = Some(vec![]);
  }

  match order.process() {
    Ok(results) => {
      info!("END OF PROCESS");
      info!("-> {:?} frames processed", results.len());
      let mut frame_rate = 1.0;
      let mut time_base = 1.0;
      let mut scene_count = 0;
      let mut context = FormatContext::new(filename).unwrap();
      if let Err(msg) = context.open_input() {
        context.close_input();
        error!("{:?}", msg);
        return;
      }
      for index in 0..context.get_nb_streams() {
        if let Ok(stream) = ContextStream::new(context.get_stream(index as isize)) {
          if let AVMediaType::AVMEDIA_TYPE_VIDEO = context.get_stream_type(index as isize) {
            let rational_frame_rate = stream.get_frame_rate();
            frame_rate = rational_frame_rate.num as f32 / rational_frame_rate.den as f32;
            time_base = stream.get_time_base();
          }
        }
      }
      for result in results {
        if let Entry(entry_map) = result {
          if let Some(stream_id) = entry_map.get("stream_id") {
            let index: i32 = stream_id.parse().unwrap();
            if streams[(index) as usize].detected_scene.is_none() {
              error!("Error : unexpected detection on stream ${index}");
              break;
            }
            let detected_scene = streams[(index) as usize].detected_scene.as_mut().unwrap();
            let detected_false_scene = streams[(index) as usize]
              .detected_false_scene
              .as_mut()
              .unwrap();
            let mut scene = SceneResult {
              frame_index: 0,
              score: 0,
              scene_number: 0,
            };
            let mut false_scene = FalseSceneResult { frame: 0 };

            if let Some(value) = entry_map.get("lavfi.scd.time") {
              scene.frame_index =
                (value.parse::<f32>().unwrap() * time_base / 25.0 * frame_rate) as i64;
              if let Some(value) = entry_map.get("lavfi.scd.score") {
                scene.score = (value.parse::<f32>().unwrap()) as i32;
              }

              if let Some(last_detect) = detected_scene.last() {
                if scene.frame_index - last_detect.frame_index <= 1 {
                  false_scene.frame = scene.frame_index;
                  detected_false_scene.push(false_scene);
                }
              }

              scene_count += 1;
              scene.scene_number = scene_count;
              detected_scene.push(scene);
            }
          }
        }
      }
    }
    Err(msg) => {
      error!("ERROR: {}", msg);
    }
  }
}
