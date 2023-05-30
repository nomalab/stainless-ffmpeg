use crate::format_context::FormatContext;
use crate::order::{
  filter_input::FilterInput, filter_output::FilterOutput, input::Input, input_kind::InputKind,
  output::Output, output_kind::OutputKind, stream::Stream,
};
use crate::order::{Filter, Order, OutputResult::Entry, ParameterValue};
use crate::probe::deep::{CheckParameterValue, CropResult, StreamProbeResult};
use crate::stream::Stream as ContextStream;
use crate::tools::rational::Rational;
use ffmpeg_sys_next::AVMediaType;
use std::collections::HashMap;

pub fn create_graph(
  filename: &str,
  video_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue>,
  nb_frames: i64,
  limit: i32,
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

    let mut select_params = HashMap::new();
    if let Some(spot_check) = params.get("spot_check") {
      if let Some(max_checks) = spot_check.max {
        let scale = (nb_frames / max_checks as i64) - 1;
        let expr = format!("not(mod(n,{scale}))");
        select_params.insert("expr".to_string(), ParameterValue::String(expr));
      }
    }

    let mut crop_params = HashMap::new();
    crop_params.insert("limit".to_string(), ParameterValue::Int64(limit as i64));

    filters.push(Filter {
      name: "cropdetect".to_string(),
      label: Some(format!("cropdetect_filter{i}")),
      parameters: crop_params,
      inputs: Some(vec![FilterInput {
        kind: InputKind::Stream,
        stream_label: input_identifier,
      }]),
      outputs: None,
    });
    filters.push(Filter {
      name: "select".to_string(),
      label: Some(format!("select_filter{i}")),
      parameters: select_params,
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
      kind: Some(OutputKind::VideoMetadata),
      keys: vec![
        "lavfi.cropdetect.w".to_string(),
        "lavfi.cropdetect.h".to_string(),
        "lavfi.cropdetect.x1".to_string(),
        "lavfi.cropdetect.x2".to_string(),
        "lavfi.cropdetect.y1".to_string(),
        "lavfi.cropdetect.y2".to_string(),
      ],
      stream: Some(output_identifier),
      path: None,
      streams: vec![],
      parameters: HashMap::new(),
    });
  }

  Order::new(inputs, filters, outputs)
}

pub fn detect_black_borders(
  filename: &str,
  streams: &mut [StreamProbeResult],
  video_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue>,
) {
  let mut context = FormatContext::new(filename).unwrap();
  if let Err(msg) = context.open_input() {
    context.close_input();
    error!("{:?}", msg);
    return;
  }

  let mut nb_frames = 0;
  let mut limit = 0;
  for index in 0..context.get_nb_streams() {
    if let Ok(stream) = ContextStream::new(context.get_stream(index as isize)) {
      if let AVMediaType::AVMEDIA_TYPE_VIDEO = context.get_stream_type(index as isize) {
        if let Some(frames) = stream.get_nb_frames() {
          nb_frames = frames;
        }
        // black threshold : 16 pour 8bits / 64 pour 10bits / 256 pour 12bits
        limit = match stream.get_bits_per_raw_sample() {
          Some(10) => 64,
          Some(12) => 256,
          _ => 16,
        }
      }
    }
  }
  let mut order = create_graph(filename, video_indexes.clone(), params, nb_frames, limit).unwrap();
  if let Err(msg) = order.setup() {
    error!("{:?}", msg);
    return;
  }
  for index in video_indexes {
    streams[index as usize].detected_crop = Some(vec![]);
  }

  match order.process() {
    Ok(results) => {
      info!("END OF PROCESS");
      info!("-> {:?} frames processed", results.len());
      let mut time_base = 1.0;
      let mut metadata_width = 0;
      let mut metadata_height = 0;
      let mut real_width = 0;
      let mut real_height = 0;
      let mut w_changed = false;
      let mut h_changed = false;
      let mut pict_size = Rational::new(1, 1);

      for index in 0..context.get_nb_streams() {
        if let Ok(stream) = ContextStream::new(context.get_stream(index as isize)) {
          if let AVMediaType::AVMEDIA_TYPE_VIDEO = context.get_stream_type(index as isize) {
            time_base = stream.get_time_base();
            metadata_width = stream.get_width();
            metadata_height = stream.get_height();
            pict_size = stream.get_picture_aspect_ratio();
            real_width = metadata_width;
            real_height = metadata_height;
          }
        }
      }
      for result in results {
        if let Entry(entry_map) = result {
          if let Some(stream_id) = entry_map.get("stream_id") {
            let index: i32 = stream_id.parse().unwrap();
            if streams[(index) as usize].detected_crop.is_none() {
              error!("Error : unexpected detection on stream ${index}");
              break;
            }
            let detected_crop = streams[(index) as usize].detected_crop.as_mut().unwrap();
            let mut crop = CropResult {
              width: metadata_width,
              height: metadata_height,
              ..Default::default()
            };
            if let (Some(x1), Some(x2)) = (
              entry_map.get("lavfi.cropdetect.x1"),
              entry_map.get("lavfi.cropdetect.x2"),
            ) {
              let width = x2.parse::<i32>().unwrap() - x1.parse::<i32>().unwrap() + 1;
              if width != metadata_width {
                w_changed = true;
              }
              real_width = width;
            }
            if let (Some(y1), Some(y2)) = (
              entry_map.get("lavfi.cropdetect.y1"),
              entry_map.get("lavfi.cropdetect.y2"),
            ) {
              let height = y2.parse::<i32>().unwrap() - y1.parse::<i32>().unwrap() + 1;
              if height != metadata_height {
                h_changed = true;
              }
              real_height = height;
            }
            if let Some(pts) = entry_map.get("pts") {
              if w_changed || h_changed {
                crop.width = real_width;
                crop.height = real_height;
                crop.pts = (pts.parse::<f32>().unwrap() * time_base * 1000.0) as i64;
                let real_aspect =
                  (real_width * pict_size.num) as f32 / (real_height * pict_size.den) as f32;
                crop.aspect_ratio = real_aspect;
                detected_crop.push(crop);
                w_changed = false;
                h_changed = false;
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
