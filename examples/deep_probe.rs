use env_logger::Builder;
use log::LevelFilter;
use stainless_ffmpeg::probe::*;
use std::{collections::HashMap, env};

fn main() {
  let mut builder = Builder::from_default_env();
  builder.init();

  if let Some(path) = env::args().last() {
    let mut probe = DeepProbe::new(&path);
    let duration_params = CheckParameterValue {
      min: Some(40),
      max: Some(20000),
      num: None,
      den: None,
      th: None,
    };
    let black_duration_params = CheckParameterValue {
      min: Some(40),
      max: Some(20000),
      num: None,
      den: None,
      th: None,
    };
    let black_pixel_params = CheckParameterValue {
      min: None,
      max: None,
      num: None,
      den: None,
      th: Some(0.0),
    };
    let black_picture_params = CheckParameterValue {
      min: None,
      max: None,
      num: None,
      den: None,
      th: Some(1.0),
    };
    let spot_check = CheckParameterValue {
      min: None,
      max: Some(3),
      num: None,
      den: None,
      th: None,
    };
    let black_and_silence_check = CheckParameterValue {
      min: Some(40),
      max: None,
      num: None,
      den: None,
      th: None,
    };
    let scene_check = CheckParameterValue {
      min: None,
      max: None,
      num: None,
      den: None,
      th: Some(10.0),
    };

    let mut silence_params = HashMap::new();
    let mut black_params = HashMap::new();
    let mut select_params = HashMap::new();
    let mut black_and_silence_params = HashMap::new();
    let mut scene_params = HashMap::new();
    silence_params.insert("duration".to_string(), duration_params);
    black_params.insert("duration".to_string(), black_duration_params);
    black_params.insert("picture".to_string(), black_picture_params);
    black_params.insert("pixel".to_string(), black_pixel_params);
    select_params.insert("spot_check".to_string(), spot_check);
    black_and_silence_params.insert("duration".to_string(), black_and_silence_check);
    scene_params.insert("threshold".to_string(), scene_check);
    let check = DeepProbeCheck {
      silence_detect: Some(silence_params),
      black_detect: Some(black_params),
      crop_detect: Some(select_params),
      black_and_silence_detect: Some(black_and_silence_params),
      scene_detect: Some(scene_params),
    };
    probe.process(LevelFilter::Off, check).unwrap();
    let result = serde_json::to_string(&probe).unwrap();
    println!("RESULT : \n{}\n", result);

    if let Some(result) = probe.result {
      println!("DEEP PROBE : \n{}", result);
    }
  }
}
