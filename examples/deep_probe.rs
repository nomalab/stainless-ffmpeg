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
      min: Some(1000),
      max: Some(20000),
      num: None,
      den: None,
      th: None,
      pairs: None,
    };
    let black_duration_params = CheckParameterValue {
      min: Some(40),
      max: Some(10000),
      num: None,
      den: None,
      th: None,
      pairs: None,
    };
    let black_pixel_params = CheckParameterValue {
      min: None,
      max: None,
      num: None,
      den: None,
      th: Some(0.0),
      pairs: None,
    };
    let black_picture_params = CheckParameterValue {
      min: None,
      max: None,
      num: None,
      den: None,
      th: Some(1.0),
      pairs: None,
    };
    let spot_check = CheckParameterValue {
      min: None,
      max: Some(3),
      num: None,
      den: None,
      th: None,
      pairs: None,
    };

    let mut audio_qualif = vec![];
    // definition : [Track::new(stream_index, channels_number)]
    audio_qualif.push([Track::new(1, 2)].to_vec());
    audio_qualif.push([Track::new(2, 2)].to_vec());
    // audio_qualif.push([Track::new(3, 1)].to_vec());
    audio_qualif.push([Track::new(4, 1), Track::new(3, 1)].to_vec());
    audio_qualif.push([Track::new(6, 2)].to_vec());
    audio_qualif.push([Track::new(7, 2)].to_vec());
    audio_qualif.push([Track::new(8, 2)].to_vec());
    audio_qualif.push([Track::new(9, 2)].to_vec());
    // audio_qualif.push(
    //   [
    //     Track::new(4, 1),
    //     Track::new(5, 1),
    //     Track::new(6, 1),
    //     Track::new(7, 1),
    //     Track::new(8, 1),
    //     Track::new(9, 1),
    //   ]
    //   .to_vec(),
    // ); //to merge to get 5.1
    // This audio_qualif needs the stream to have at least 9 audio streams
    let loudness_check = CheckParameterValue {
      min: None,
      max: None,
      num: None,
      den: None,
      th: None,
      pairs: Some(audio_qualif.clone()),
    };
    let dualmono_duration_params = CheckParameterValue {
      min: Some(1000),
      max: None,
      num: None,
      den: None,
      th: None,
      pairs: Some(audio_qualif),
    };

    let mut params = HashMap::new();
    let mut black_params = HashMap::new();
    let mut select_params = HashMap::new();
    let mut loudness_params = HashMap::new();
    let mut dualmono_params = HashMap::new();
    params.insert("duration".to_string(), duration_params);
    black_params.insert("duration".to_string(), black_duration_params);
    black_params.insert("picture".to_string(), black_picture_params);
    black_params.insert("pixel".to_string(), black_pixel_params);
    select_params.insert("spot_check".to_string(), spot_check);
    loudness_params.insert("pairing_list".to_string(), loudness_check.clone());
    dualmono_params.insert("duration".to_string(), dualmono_duration_params.clone());
    dualmono_params.insert("pairing_list".to_string(), dualmono_duration_params);
    let check = DeepProbeCheck {
      silence_detect: Some(params),
      black_detect: Some(black_params),
      crop_detect: Some(select_params),
      dualmono_detect: Some(dualmono_params),
      loudness_detect: Some(loudness_params),
    };
    probe.process(LevelFilter::Off, check).unwrap();
    let result = serde_json::to_string(&probe).unwrap();
    println!("RESULT : \n{}\n", result);

    if let Some(result) = probe.result {
      println!("DEEP PROBE : \n{}", result);
    }
  }
}
