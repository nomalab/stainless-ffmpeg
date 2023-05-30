use crate::probe::deep::{BlackAndSilenceResult, CheckParameterValue, StreamProbeResult};
use std::collections::HashMap;

pub fn detect_black_and_silence(
  streams: &mut [StreamProbeResult],
  video_indexes: Vec<u32>,
  audio_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue>,
) {
  let mut bas = BlackAndSilenceResult { start: 0, end: 0 };
  let mut duration_min = None;
  let mut duration_max = None;
  if let Some(duration) = params.get("duration") {
    duration_max = duration.max;
    duration_min = duration.min;
  }
  for index in audio_indexes.clone() {
    streams[index as usize].detected_black_and_silence = Some(vec![]);
  }

  for bl_index in video_indexes {
    for bl_detect in streams[bl_index as usize].detected_black.clone().unwrap() {
      for si_index in audio_indexes.clone() {
        for si_detect in streams[si_index as usize].detected_silence.clone().unwrap() {
          if bl_detect.end <= si_detect.end {
            bas.end = bl_detect.end;
          } else {
            bas.end = si_detect.end;
          }
          if bl_detect.start <= si_detect.start {
            bas.start = si_detect.start;
          } else {
            bas.start = bl_detect.start;
          }
          if bas.start < bas.end {
            let bas_duration: i64 = bas.end - bas.start;
            let detected_black_and_silence = streams[si_index as usize]
              .detected_black_and_silence
              .as_mut()
              .unwrap();
            detected_black_and_silence.push(bas.clone());

            if let Some(min) = duration_min {
              if bas_duration < min as i64 {
                detected_black_and_silence.pop();
              }
            }
            if let Some(max) = duration_max {
              if bas_duration > max as i64 {
                detected_black_and_silence.pop();
              }
            }
          }
        }
      }
    }
  }
}
