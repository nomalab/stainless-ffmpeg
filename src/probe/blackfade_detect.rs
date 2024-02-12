use crate::{
  order::OutputResult::{self, Entry},
  probe::deep::{BlackFadeResult, CheckName, CheckParameterValue, StreamProbeResult, VideoDetails},
};
use std::collections::{BTreeMap, HashMap};

pub fn detect_blackfade(
  output_results: &BTreeMap<CheckName, Vec<OutputResult>>,
  streams: &mut [StreamProbeResult],
  video_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue>,
  video_details: VideoDetails,
) {
  for index in video_indexes.clone() {
    streams[index as usize].detected_blackfade = Some(vec![]);
  }
  let results = output_results.get(&CheckName::BlackFade).unwrap();
  info!("END OF BLACKFADES PROCESS");
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
                if (last_detect.start < blackframe.start && blackframe.start <= last_detect.end)
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
