extern crate env_logger;
extern crate stainless_ffmpeg_sys;
#[macro_use]
extern crate log;
extern crate serde_json;
extern crate stainless_ffmpeg;

use env_logger::{Builder, Env};
use stainless_ffmpeg_sys::{
  av_log_set_level,
  AV_LOG_ERROR
};
use std::env;
use std::fs::File;
use std::io::Read;
use stainless_ffmpeg::order::*;
use stainless_ffmpeg::order::OutputResult;

fn main() {
  Builder::from_env(Env::default().default_filter_or("debug")).init();
  unsafe {
    av_log_set_level(AV_LOG_ERROR);
  }

  if let Some(path) = env::args().last() {
    let mut file = File::open(&path).unwrap();
    let mut message = String::new();
    file.read_to_string(&mut message).unwrap();

    let mut order = Order::new_parse(&message).unwrap();
    if let Err(msg) = order.setup() {
      error!("{:?}", msg);
      return;
    }

    match order.process() {
      Ok(results) => {
        info!("Job is finished");
        let processed: Vec<&OutputResult> = results.iter().filter(|r|
          if let OutputResult::ProcessStatistics{..} = r {
            false
          } else {
            true
          }
          ).collect();

        info!("-> {:?} frames analysed", processed.len());

        for result in &results {
          if let OutputResult::ProcessStatistics{
            decoded_audio_frames,
            decoded_video_frames,
            encoded_audio_frames,
            encoded_video_frames
            } = result {
            info!("-> {:?} audio frames decoded", decoded_audio_frames);
            info!("-> {:?} video frames decoded", decoded_video_frames);
            info!("-> {:?} audio frames encoded", encoded_audio_frames);
            info!("-> {:?} video frames encoded", encoded_video_frames);
          }
        }

        for result in &results {
          match result {
            OutputResult::Entry(entry_map) => {
              if let Some(value) = entry_map.get("lavfi.silence_start") {
                info!("silence start: {}", value);
              }
              if let Some(value) = entry_map.get("lavfi.silence_duration") {
                info!("silence duration: {}", value);
              }
              if let Some(value) = entry_map.get("lavfi.r128.I") {
                info!("Program Loudness: {}", value);
              }
            },
            _ => {},
          }
        }
      }
      Err(msg) => {
        error!("ERROR: {}", msg);
      }
    }
  }
}
