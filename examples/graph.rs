#[macro_use]
extern crate log;

use env_logger::Builder;
use stainless_ffmpeg::{order::OutputResult::Entry, order::*, prelude::*};
use std::{env, fs::File, io::Read};

fn main() {
  let mut builder = Builder::from_default_env();
  builder.init();

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
        info!("END OF PROCESS");
        info!("-> {:?} frames processed", results.len());
        for result in results {
          if let Entry(entry_map) = result {
              if let Some(value) = entry_map.get("lavfi.silence_start") {
                info!("silence start: {}", value);
              }
              if let Some(value) = entry_map.get("lavfi.silence_duration") {
                info!("silence duration: {}", value);
              }
              if let Some(value) = entry_map.get("lavfi.r128.I") {
                info!("Program Loudness: {}", value);
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
