use env_logger::{Builder, Env};
use log::LevelFilter;
use stainless_ffmpeg::probe::deep::CheckParameterValue;
use stainless_ffmpeg::probe::*;
use std::collections::HashMap;
use std::env;

fn main() {
  Builder::from_env(Env::default().default_filter_or("debug")).init();

  if let Some(path) = env::args().last() {
    let mut probe = DeepProbe::new(&path);
    let duration_params = CheckParameterValue {
      min: Some(2000),
      max: Some(10000),
      num: None,
      den: None,
      th: None,
    };
    let black_duration_params = CheckParameterValue {
      min: Some(1000),
      max: Some(10000),
      num: None,
      den: None,
      th: None,
    };
    let black_pixel_params = CheckParameterValue {
      min: None,
      max: None,
      num: None,
      den: None,
      th: Some(0.0)
    };
    let black_picture_params = CheckParameterValue {
      min: None,
      max: None,
      num: None,
      den: None,
      th: Some(1.0)
    };
    let spot_check = CheckParameterValue {
      min: None,
      max: Some(3),
      num: None,
      den: None,
      th: None
    };
    let mut params = HashMap::new();
    let mut black_params = HashMap::new();
    let mut select_params = HashMap::new();
    params.insert("duration".to_string(), duration_params);
    black_params.insert("duration".to_string(), black_duration_params);
    black_params.insert("picture".to_string(), black_picture_params);
    black_params.insert("pixel".to_string(), black_pixel_params);
    select_params.insert("spot_check".to_string(), spot_check);
    let check = DeepProbeCheck {
      silence_detect: Some(params),
      black_detect: Some(black_params),
      crop_detect: Some(select_params),
    };
    probe.process(LevelFilter::Off, check).unwrap();
    let result = serde_json::to_string(&probe).unwrap();
    println!("{}", result);

    if let Some(result) = probe.result {
      println!("Deep probe result : \n{}", result);
    }
  }
}
