use crate::{
  format_context::FormatContext,
  order::{
    filter_input::FilterInput, filter_output::FilterOutput, input::Input, input_kind::InputKind,
    output::Output, output_kind::OutputKind, stream::Stream, Filter, Order, OutputResult::Entry,
    ParameterValue,
  },
  probe::deep::{BlackResult, CheckParameterValue, StreamProbeResult},
  stream::Stream as ContextStream,
};
use ffmpeg_sys_next::AVMediaType;
use std::collections::HashMap;

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

    let mut blackdetect_params: HashMap<String, ParameterValue> = HashMap::new();
    if let Some(picture) = params.get("picture") {
      if let Some(pic_th) = picture.th {
        blackdetect_params.insert(
          "picture_black_ratio_th".to_string(),
          ParameterValue::Float(pic_th),
        );
      }
    }
    if let Some(pixel) = params.get("pixel") {
      if let Some(pix_th) = pixel.th {
        blackdetect_params.insert("pixel_black_th".to_string(), ParameterValue::Float(pix_th));
      }
    }

    filters.push(Filter {
      name: "blackdetect".to_string(),
      label: Some(format!("blackdetect_filter{i}")),
      parameters: blackdetect_params.clone(),
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
        "lavfi.black_start".to_string(),
        "lavfi.black_end".to_string(),
      ],
      stream: Some(output_identifier),
      path: None,
      streams: vec![],
      parameters: HashMap::new(),
    });
  }

  Order::new(inputs, filters, outputs)
}

pub fn detect_black_frames(
  filename: &str,
  streams: &mut [StreamProbeResult],
  video_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue>,
) {
  let mut order = create_graph(filename, video_indexes.clone(), params.clone()).unwrap();
  if let Err(msg) = order.setup() {
    error!("{:?}", msg);
    return;
  }
  for index in video_indexes {
    streams[index as usize].detected_black = Some(vec![]);
  }

  match order.process() {
    Ok(results) => {
      info!("END OF PROCESS");
      info!("-> {:?} frames processed", results.len());
      let mut duration = 0;
      let mut time_base = 1.0;
      let mut frame_rate = 1.0;
      let mut black_duration = 0;
      let mut max_duration = None;
      let mut min_duration = None;
      if let Some(duration) = params.get("duration") {
        max_duration = duration.max;
        min_duration = duration.min;
      }
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
            if let Some(stream_duration) = stream.get_duration() {
              duration = (stream_duration * 1000.0) as i64;
            } else {
              duration = (results.len() as f32 / frame_rate * 1000.0) as i64;
            }
            time_base = stream.get_time_base();
          }
        }
      }
      for result in results {
        if let Entry(entry_map) = result {
          if let Some(stream_id) = entry_map.get("stream_id") {
            let index: i32 = stream_id.parse().unwrap();
            if streams[(index) as usize].detected_black.is_none() {
              error!("Error : unexpected detection on stream ${index}");
              break;
            }
            let detected_black = streams[(index) as usize].detected_black.as_mut().unwrap();
            let mut black = BlackResult {
              start: 0,
              end: duration,
            };

            if let Some(value) = entry_map.get("lavfi.black_start") {
              black.start =
                (value.parse::<f32>().unwrap() * time_base / frame_rate * 1000.0) as i64;
              black_duration = black.start;
              detected_black.push(black);
            }
            if let Some(value) = entry_map.get("lavfi.black_end") {
              if let Some(last_detect) = detected_black.last_mut() {
                last_detect.end =
                  (value.parse::<f32>().unwrap() * time_base / frame_rate * 1000.0) as i64;
                black_duration = last_detect.end - black_duration;
                if let Some(max) = max_duration {
                  if black_duration > max as i64 {
                    detected_black.pop();
                  }
                }
                if let Some(min) = min_duration {
                  if black_duration < min as i64 {
                    detected_black.pop();
                  }
                }
              }
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
