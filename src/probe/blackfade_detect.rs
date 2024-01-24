use super::black_detect::create_graph;
use crate::{
  order::{
    Order,
    OutputResult::{self, Entry},
  },
  prelude::Frame,
  probe::deep::{BlackFadeResult, CheckParameterValue, StreamProbeResult, VideoDetails},
};
use std::collections::HashMap;

pub fn detect_blackfade(
  orders: &mut HashMap<String, Order>,
  video_frames: &Vec<Frame>,
  output_results: &mut HashMap<String, Vec<OutputResult>>,
  filename: &str,
  streams: &mut [StreamProbeResult],
  video_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue>,
  video_details: VideoDetails,
  decode_end: bool,
) {
  if orders.get("blackfades").is_none() {
    let mut order = create_graph(filename, video_indexes.clone(), &params).unwrap();
    if let Err(msg) = order.setup() {
      error!("{:?}", msg);
    }
    orders.insert("blackfades".to_string(), order);
    output_results.insert("blackfades".to_string(), vec![]);
  }

  if !decode_end {
    match orders
      .get_mut("blackfades")
      .unwrap()
      .process(&vec![], video_frames, &vec![])
    {
      Ok(results) => {
        output_results
          .entry("blackfades".to_string())
          .and_modify(|own_results| own_results.extend(results));
      }
      Err(msg) => {
        error!("ERROR: {}", msg)
      }
    }
  } else {
    for index in video_indexes.clone() {
      streams[index as usize].detected_blackfade = Some(vec![]);
    }

    match orders
      .get_mut("blackfades")
      .unwrap()
      .process(&vec![], video_frames, &vec![])
    {
      Ok(result) => {
        output_results
          .entry("blackfades".to_string())
          .and_modify(|own_results| own_results.extend(result));
        let results = output_results.get("blackfades").unwrap();
        println!("END OF BLACKFADES PROCESS");
        println!("-> {:?} frames processed", results.len());
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
                end: end_from_duration,
              };

              if let Some(value) = entry_map.get("lavfi.black_start") {
                blackfade.start = (value.parse::<f32>().unwrap() * 1000.0).round() as i64;
                detected_blackfade.push(blackfade.clone());
              }
              if let Some(value) = entry_map.get("lavfi.black_end") {
                if let Some(last_detect) = detected_blackfade.last_mut() {
                  last_detect.end = ((value.parse::<f32>().unwrap() - video_details.frame_duration)
                    * 1000.0)
                    .round() as i64;
                  let blackfade_duration = last_detect.end - last_detect.start
                    + (video_details.frame_duration * 1000.0).round() as i64;
                  let detected_black = streams[index as usize].detected_black.clone();
                  let mut fade = false;
                  if let Some(blackframes) = detected_black {
                    for blackframe in blackframes {
                      if (last_detect.start < blackframe.start
                        && blackframe.start <= last_detect.end)
                        || (last_detect.start <= blackframe.end && blackframe.end < last_detect.end)
                      {
                        fade = true;
                      }
                    }
                  }
                  if !fade {
                    detected_blackfade.pop();
                  } else {
                    if let Some(max) = max_duration {
                      if blackfade_duration > max as i64 {
                        detected_blackfade.pop();
                      }
                    }
                    if let Some(min) = min_duration {
                      if blackfade_duration < min as i64 {
                        detected_blackfade.pop();
                      }
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
}
