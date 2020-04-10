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
    };
    let mut params = HashMap::new();
    params.insert("duration".to_string(), duration_params);
    let check = DeepProbeCheck {
      silence_detect: params,
    };
    probe.process(LevelFilter::Off, check).unwrap();
    let result = serde_json::to_string(&probe).unwrap();
    println!("{}", result);

    if let Some(result) = probe.result {
      println!("Deep probe result : \n{}", result);
    }
  }
}
