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
      min: None, //None, //Some(1000),
      max: None, //Some(10000),
      num: None,
      den: None,
      th: None,
      pair: None,
    };
    let black_duration_params = CheckParameterValue {
      min: None, //Some(1000),
      max: None, //Some(10000),
      num: None,
      den: None,
      th: None,
      pair: None,
    };
    let black_pixel_params = CheckParameterValue {
      min: None,
      max: None,
      num: None,
      den: None,
      th: None, //Some(0.0),
      pair: None,
    };
    let black_picture_params = CheckParameterValue {
      min: None,
      max: None,
      num: None,
      den: None,
      th: None, //Some(1.0),
      pair: None,
    };
    let spot_check = CheckParameterValue {
      min: None,
      max: None, //Some(3),
      num: None,
      den: None,
      th: None,
      pair: None,
    };
    let mut pairing = vec![vec![]];
    pairing.push([1].to_vec());
    pairing.push([2].to_vec());
    let loudness_check = CheckParameterValue {
      min: None,
      max: None,
      num: None,
      den: None,
      th: None,
      pair: Some(pairing),
    };

    let mut silence_params = HashMap::new();
    let mut black_params = HashMap::new();
    let mut select_params = HashMap::new();
    let mut loudness_params = HashMap::new();
    silence_params.insert("duration".to_string(), duration_params);
    black_params.insert("duration".to_string(), black_duration_params);
    black_params.insert("picture".to_string(), black_picture_params);
    black_params.insert("pixel".to_string(), black_pixel_params);
    select_params.insert("spot_check".to_string(), spot_check);
    loudness_params.insert("pairing_list".to_string(), loudness_check);
    let check = DeepProbeCheck {
      silence_detect: None,  //Some(silence_params),
      black_detect: None,    //Some(black_params),
      crop_detect: None,     //Some(select_params),
      loudness_detect: None, //Some(loudness_params),
    };
    probe.process(LevelFilter::Off, check).unwrap();
    let result = serde_json::to_string(&probe).unwrap();
    println!("RESULT : \n{}\n", result);

    if let Some(result) = probe.result {
      println!("DEEP PROBE : \n{}", result);
    }
  }
}
