use crate::probe::deep::{BlackAndSilenceResult, CheckParameterValue, StreamProbeResult};
use std::collections::HashMap;

pub fn detect_black_and_silence(
  streams: &mut [StreamProbeResult],
  video_indexes: Vec<u32>,
  audio_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue>,
  frame_duration: f32,
) {
  let mut black_and_silence = BlackAndSilenceResult { start: 0, end: 0 };
  let mut duration_min = None;
  let mut duration_max = None;
  if let Some(duration) = params.get("duration") {
    duration_max = duration.max;
    duration_min = duration.min;
  }
  for index in audio_indexes.clone() {
    streams[index as usize].detected_black_and_silence = Some(vec![]);
  }

  for video_index in video_indexes {
    for black in streams[video_index as usize]
      .detected_black
      .clone()
      .unwrap()
    {
      for audio_index in audio_indexes.clone() {
        for silence in streams[audio_index as usize]
          .detected_silence
          .clone()
          .unwrap()
        {
          if black.end <= silence.end {
            black_and_silence.end = black.end;
          } else {
            black_and_silence.end = silence.end;
          }
          if black.start <= silence.start {
            black_and_silence.start = silence.start;
          } else {
            black_and_silence.start = black.start;
          }
          if black_and_silence.start <= black_and_silence.end {
            let black_and_silence_duration: i64 = black_and_silence.end - black_and_silence.start
              + (frame_duration * 1000.0).round() as i64;
            let detected_black_and_silence = streams[audio_index as usize]
              .detected_black_and_silence
              .as_mut()
              .unwrap();
            detected_black_and_silence.push(black_and_silence.clone());

            if let Some(min) = duration_min {
              if black_and_silence_duration < min as i64 {
                detected_black_and_silence.pop();
              }
            }
            if let Some(max) = duration_max {
              if black_and_silence_duration > max as i64 {
                detected_black_and_silence.pop();
              }
            }
          }
        }
      }
    }
  }
}
