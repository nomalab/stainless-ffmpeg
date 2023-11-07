use super::black_detect::create_graph;
use crate::{
  format_context::FormatContext,
  order::OutputResult::Entry,
  probe::deep::{BlackFadeResult, CheckParameterValue, StreamProbeResult},
  stream::Stream as ContextStream,
};
use ffmpeg_sys_next::AVMediaType;
use std::collections::HashMap;

pub fn detect_blackfade(
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
  for index in video_indexes.clone() {
    streams[index as usize].detected_blackfade = Some(vec![]);
  }

  match order.process() {
    Ok(results) => {
      info!("END OF PROCESS");
      info!("-> {:?} frames processed", results.len());
      let mut duration = 0;
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
            let frame_rate = stream.get_frame_rate().to_float();
            if let Some(stream_duration) = stream.get_duration() {
              duration = (stream_duration * 1000.0) as i64;
            } else {
              duration = (results.len() as f32 / frame_rate * 1000.0) as i64;
            }
          }
        }
      }
      for result in results {
        if let Entry(entry_map) = result {
          if let Some(stream_id) = entry_map.get("stream_id") {
            let index: i32 = stream_id.parse().unwrap();
            if streams[(index) as usize].detected_blackfade.is_none() {
              error!("Error : unexpected detection on stream ${index}");
              break;
            }
            let detected_blackfade = streams[(index) as usize]
              .detected_blackfade
              .as_mut()
              .unwrap();
            let mut blackfade = BlackFadeResult {
              start: 0,
              end: duration,
            };

            if let Some(value) = entry_map.get("lavfi.black_start") {
              blackfade.start = (value.parse::<f32>().unwrap() * 1000.0).round() as i64;
              detected_blackfade.push(blackfade.clone());
            }
            if let Some(value) = entry_map.get("lavfi.black_end") {
              if let Some(last_detect) = detected_blackfade.last_mut() {
                last_detect.end = (value.parse::<f32>().unwrap() * 1000.0).round() as i64;
                let black_duration = last_detect.end - last_detect.start;
                if let Some(max) = max_duration {
                  if black_duration > max as i64 {
                    detected_blackfade.pop();
                  }
                }
                if let Some(min) = min_duration {
                  if black_duration < min as i64 {
                    detected_blackfade.pop();
                  }
                }
              }
            }
          }
        }
      }

      for index in video_indexes {
        let detected_blackfade = streams[(index) as usize]
          .detected_blackfade
          .as_mut()
          .unwrap();
        let detected_black = streams[index as usize].detected_black.clone();
        let mut removed_blackfade_count = 0;
        for (index, blackfade) in detected_blackfade.clone().iter().enumerate() {
          let mut fade = false;
          if let Some(blackframes) = detected_black.clone() {
            for blackframe in blackframes {
              if (blackfade.start < blackframe.start && blackframe.start < blackfade.end)
                || (blackfade.start < blackframe.end && blackframe.end < blackfade.end)
              {
                fade = true;
                break;
              }
            }
          }
          if !fade {
            detected_blackfade.remove(index - removed_blackfade_count);
            removed_blackfade_count += 1;
          }
        }
      }
    }
    Err(msg) => {
      error!("ERROR: {}", msg);
    }
  }
}
