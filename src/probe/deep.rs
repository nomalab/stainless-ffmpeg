use crate::order::OutputResult;
use crate::probe::black_and_silence::detect_black_and_silence;
use crate::probe::black_detect::detect_black_frames;
use crate::probe::blackfade_detect::detect_blackfade;
use crate::probe::crop_detect::detect_black_borders;
use crate::probe::dualmono_detect::detect_dualmono;
use crate::probe::loudness_detect::detect_loudness;
use crate::probe::ocr_detect::detect_ocr;
use crate::probe::scene_detect::detect_scene;
use crate::probe::silence_detect::detect_silence;
use crate::probe::sine_detect::detect_sine;
use crate::stream::Stream;
use crate::tools::rational::Rational;
use crate::{format_context::FormatContext, order::Order};
use ffmpeg_sys_next::*;
use log::LevelFilter;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::HashMap, fmt};
use uuid::Uuid;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct DeepProbe {
  #[serde(skip_serializing)]
  filename: String,
  id: Uuid,
  pub result: Option<DeepProbeResult>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct DeepProbeResult {
  #[serde(default)]
  streams: Vec<StreamProbeResult>,
  format: FormatProbeResult,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct SilenceResult {
  pub start: i64,
  pub end: i64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct BlackResult {
  pub start: i64,
  pub end: i64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct BlackFadeResult {
  pub start: i64,
  pub end: i64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct BlackAndSilenceResult {
  pub start: i64,
  pub end: i64,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct CropResult {
  pub pts: i64,
  pub width: i32,
  pub height: i32,
  pub aspect_ratio: f32,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SceneResult {
  pub frame_index: i64,
  pub score: i32,
  pub scene_number: u32,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct FalseSceneResult {
  pub frame: i64,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct OcrResult {
  pub frame_start: u64,
  pub frame_end: u64,
  pub text: String,
  pub word_confidence: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct LoudnessResult {
  pub integrated: f64,
  pub range: f64,
  pub true_peaks: Vec<f64>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct DualMonoResult {
  pub start: i64,
  pub end: i64,
}

#[derive(Copy, Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SineResult {
  pub channel: u8,
  pub start: i64,
  pub end: i64,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct StreamProbeResult {
  pub stream_index: usize,
  pub count_packets: usize,
  pub min_packet_size: i32,
  pub max_packet_size: i32,
  pub color_space: Option<String>,
  pub color_range: Option<String>,
  pub color_primaries: Option<String>,
  pub color_trc: Option<String>,
  pub color_matrix: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub detected_silence: Option<Vec<SilenceResult>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub silent_stream: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub detected_black: Option<Vec<BlackResult>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub detected_blackfade: Option<Vec<BlackFadeResult>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub detected_crop: Option<Vec<CropResult>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub detected_scene: Option<Vec<SceneResult>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub detected_false_scene: Option<Vec<FalseSceneResult>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub detected_ocr: Option<Vec<OcrResult>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub detected_loudness: Option<Vec<LoudnessResult>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub detected_dualmono: Option<Vec<DualMonoResult>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub detected_bitrate: Option<i64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub detected_black_and_silence: Option<Vec<BlackAndSilenceResult>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub detected_sine: Option<Vec<SineResult>>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct FormatProbeResult {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub detected_bitrate_format: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Eq, Hash)]
pub struct Track {
  pub index: u8,
  pub channel: u8,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CheckParameterValue {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub min: Option<u64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub max: Option<u64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub num: Option<u64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub den: Option<u64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub th: Option<f64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub pairs: Option<Vec<Vec<Track>>>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct DeepProbeCheck {
  pub silence_detect: Option<HashMap<String, CheckParameterValue>>,
  pub black_detect: Option<HashMap<String, CheckParameterValue>>,
  pub blackfade_detect: Option<HashMap<String, CheckParameterValue>>,
  pub black_and_silence_detect: Option<HashMap<String, CheckParameterValue>>,
  pub crop_detect: Option<HashMap<String, CheckParameterValue>>,
  pub scene_detect: Option<HashMap<String, CheckParameterValue>>,
  pub ocr_detect: Option<HashMap<String, CheckParameterValue>>,
  pub loudness_detect: Option<HashMap<String, CheckParameterValue>>,
  pub dualmono_detect: Option<HashMap<String, CheckParameterValue>>,
  pub sine_detect: Option<HashMap<String, CheckParameterValue>>,
}

#[derive(Clone, Debug, Default)]
pub struct VideoDetails {
  pub frame_rate: f32,
  pub time_base: f32,
  pub frame_duration: f32,
  pub stream_duration: Option<f32>,
  pub stream_frames: Option<i64>,
  pub bits_raw_sample: Option<i32>,
  pub metadata_width: i32,
  pub metadata_height: i32,
  pub aspect_ratio: Rational,
}

impl fmt::Display for DeepProbeResult {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    for (index, stream) in self.streams.iter().enumerate() {
      writeln!(f, "\n{:30} : {:?}", "Stream Index", index)?;
      writeln!(f, "{:30} : {:?}", "Number of packets", stream.count_packets)?;
      writeln!(
        f,
        "{:30} : {:?}",
        "Minimum packet size", stream.min_packet_size
      )?;
      writeln!(
        f,
        "{:30} : {:?}",
        "Maximum packet size", stream.max_packet_size
      )?;
      writeln!(f, "{:30} : {:?}", "Color space", stream.color_space)?;
      writeln!(f, "{:30} : {:?}", "Color range", stream.color_range)?;
      writeln!(f, "{:30} : {:?}", "Color Primaries", stream.color_primaries)?;
      writeln!(
        f,
        "{:30} : {:?}",
        "Transfer characteristics", stream.color_trc
      )?;
      writeln!(
        f,
        "{:30} : {:?}",
        "Matrix coefficients", stream.color_matrix
      )?;
      writeln!(
        f,
        "{:30} : {:?}",
        "Silence detection", stream.detected_silence
      )?;
      writeln!(f, "{:30} : {:?}", "Black detection", stream.detected_black)?;
      writeln!(
        f,
        "{:30} : {:?}",
        "Blackfade detection", stream.detected_blackfade
      )?;
      writeln!(
        f,
        "{:30} : {:?}",
        "Black and silence detection", stream.detected_black_and_silence
      )?;
      writeln!(f, "{:30} : {:?}", "Crop detection", stream.detected_crop)?;
      writeln!(f, "{:30} : {:?}", "Scene detection", stream.detected_scene)?;
      writeln!(
        f,
        "{:30} : {:?}",
        "False scene detection", stream.detected_false_scene
      )?;
      writeln!(
        f,
        "{:30} : {:?}",
        "Media offline detection", stream.detected_ocr
      )?;
      writeln!(
        f,
        "{:30} : {:?}",
        "Loudness detection", stream.detected_loudness
      )?;
      writeln!(
        f,
        "{:30} : {:?}",
        "DualMono detection", stream.detected_dualmono,
      )?;
      writeln!(f, "{:30} : {:?}", "1000Hz detection", stream.detected_sine)?;
      writeln!(
        f,
        "{:30} : {:?}",
        "Bitrate detection", stream.detected_bitrate
      )?;
    }
    Ok(())
  }
}

impl StreamProbeResult {
  pub fn new() -> Self {
    StreamProbeResult {
      stream_index: 0,
      count_packets: 0,
      color_space: None,
      color_range: None,
      color_primaries: None,
      color_trc: None,
      color_matrix: None,
      min_packet_size: std::i32::MAX,
      max_packet_size: std::i32::MIN,
      detected_silence: None,
      silent_stream: None,
      detected_black: None,
      detected_blackfade: None,
      detected_black_and_silence: None,
      detected_crop: None,
      detected_scene: None,
      detected_false_scene: None,
      detected_ocr: None,
      detected_loudness: None,
      detected_dualmono: None,
      detected_sine: None,
      detected_bitrate: None,
    }
  }
}

impl FormatProbeResult {
  pub fn new() -> Self {
    FormatProbeResult {
      detected_bitrate_format: None,
    }
  }
}

impl Track {
  pub fn new(ind: u8, ch: u8) -> Self {
    Track {
      index: ind,
      channel: ch,
    }
  }
  pub fn get_channels_number(pairing_list: Vec<Vec<Track>>, ind: u8) -> u8 {
    for tracks in pairing_list.iter() {
      for track in tracks {
        if track.index == ind {
          return track.channel;
        }
      }
    }
    0
  }
}

impl VideoDetails {
  pub fn new() -> Self {
    VideoDetails {
      frame_rate: 1.0,
      time_base: 1.0,
      frame_duration: 0.0,
      stream_duration: None,
      stream_frames: None,
      bits_raw_sample: None,
      metadata_width: 0,
      metadata_height: 0,
      aspect_ratio: Rational::new(1, 1),
    }
  }
}

impl DeepProbe {
  pub fn new(filename: &str, id: Uuid) -> Self {
    DeepProbe {
      filename: filename.to_owned(),
      id,
      result: None,
    }
  }

  pub fn process(&mut self, log_level: LevelFilter, check: DeepProbeCheck) -> Result<(), String> {
    let dp_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

    let av_log_level = match log_level {
      LevelFilter::Error => AV_LOG_ERROR,
      LevelFilter::Warn => AV_LOG_WARNING,
      LevelFilter::Info => AV_LOG_INFO,
      LevelFilter::Debug => AV_LOG_DEBUG,
      LevelFilter::Trace => AV_LOG_TRACE,
      LevelFilter::Off => AV_LOG_QUIET,
    };

    unsafe {
      av_log_set_level(av_log_level);
    }
    let mut context = FormatContext::new(&self.filename).unwrap();
    if context.open_input().is_err() {
      self.result = None;
      context.close_input();
      return Ok(());
    }

    let mut audio_indexes = vec![];
    let mut video_indexes = vec![];
    for stream_index in 0..context.get_nb_streams() {
      if context.get_stream_type(stream_index as isize) == AVMediaType::AVMEDIA_TYPE_VIDEO {
        video_indexes.push(stream_index);
      }
      if context.get_stream_type(stream_index as isize) == AVMediaType::AVMEDIA_TYPE_AUDIO {
        audio_indexes.push(stream_index);
      }
    }

    let mut decode_time = 0.0;
    let mut silence_time = 0.0;
    let mut blackframes_time = 0.0;
    let mut blackfades_time = 0.0;
    let mut crop_time = 0.0;
    let mut scene_time = 0.0;
    let mut ocr_time = 0.0;
    let mut loudness_time = 0.0;
    let mut dualmono_time = 0.0;
    let mut sine_time = 0.0;
    let mut start_time;
    let mut end_time;
    let mut output_results: HashMap<String, Vec<OutputResult>> = HashMap::new();
    let mut decode_end = false;
    let mut video_frames = vec![];
    let mut audio_frames = vec![];
    let mut subtitle_packets = vec![];
    let mut order = Order::new(vec![], vec![], vec![]).unwrap();
    let (mut streams, video_details, mut packets) = order.process_input(&mut context);
    let mut orders = HashMap::new();
    orders.insert("src".to_string(), order);

    while !decode_end {
      let decode_start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
      (decode_end, audio_frames, video_frames, subtitle_packets) = orders
        .get_mut("src")
        .unwrap()
        .decode_input(&mut context, &mut packets);
      let decode_end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
      decode_time += (decode_end_time.as_millis() - decode_start_time.as_millis()) as f64;

      if let Some(silence_parameters) = check.silence_detect.clone() {
        start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        detect_silence(
          &mut orders,
          &audio_frames,
          &mut output_results,
          &self.filename,
          &mut streams,
          audio_indexes.clone(),
          silence_parameters,
          video_details.clone(),
          decode_end,
        );
        end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        silence_time += (end_time.as_millis() - start_time.as_millis()) as f64;
      }

      if let Some(black_parameters) = check.black_detect.clone() {
        start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        detect_black_frames(
          &mut orders,
          &video_frames,
          &mut output_results,
          &self.filename,
          &mut streams,
          video_indexes.clone(),
          black_parameters,
          video_details.clone(),
          decode_end,
        );
        end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        blackframes_time += (end_time.as_millis() - start_time.as_millis()) as f64;
      }

      if let Some(ref blackfade_parameters) = check.blackfade_detect {
        start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        detect_blackfade(
          &mut orders,
          &video_frames,
          &mut output_results,
          &self.filename,
          &mut streams,
          video_indexes.clone(),
          blackfade_parameters.clone(),
          video_details.clone(),
          decode_end,
        );
        end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        blackfades_time += (end_time.as_millis() - start_time.as_millis()) as f64;
      }

      if let Some(ref crop_parameters) = check.crop_detect {
        start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        detect_black_borders(
          &mut orders,
          &video_frames,
          &mut output_results,
          &self.filename,
          &mut streams,
          video_indexes.clone(),
          crop_parameters.clone(),
          video_details.clone(),
          decode_end,
        );
        end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        crop_time += (end_time.as_millis() - start_time.as_millis()) as f64;
      }

      if let Some(ref scene_parameters) = check.scene_detect {
        start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        detect_scene(
          &mut orders,
          &video_frames,
          &mut output_results,
          &self.filename,
          &mut streams,
          video_indexes.clone(),
          scene_parameters.clone(),
          video_details.frame_rate,
          decode_end,
        );
        end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        scene_time += (end_time.as_millis() - start_time.as_millis()) as f64;
      }

      if let Some(ref ocr_parameters) = check.ocr_detect {
        start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        detect_ocr(
          &mut orders,
          &video_frames,
          &mut output_results,
          &self.filename,
          &mut streams,
          video_indexes.clone(),
          ocr_parameters.clone(),
          video_details.clone(),
          decode_end,
        );
        end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        ocr_time += (end_time.as_millis() - start_time.as_millis()) as f64;
      }

      if let Some(ref loudness_parameters) = check.loudness_detect {
        start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        detect_loudness(
          &mut orders,
          &audio_frames,
          &mut output_results,
          &self.filename,
          &mut streams,
          audio_indexes.clone(),
          loudness_parameters.clone(),
          decode_end,
        );
        end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        loudness_time += (end_time.as_millis() - start_time.as_millis()) as f64;
      }

      if let Some(ref dualmono_parameters) = check.dualmono_detect {
        start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        detect_dualmono(
          &mut orders,
          &audio_frames,
          &mut output_results,
          &self.filename,
          &mut streams,
          audio_indexes.clone(),
          dualmono_parameters.clone(),
          video_details.clone(),
          decode_end,
        );
        end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        dualmono_time += (end_time.as_millis() - start_time.as_millis()) as f64;
      }

      if let Some(ref sine_parameters) = check.sine_detect {
        start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        detect_sine(
          &mut orders,
          &audio_frames,
          &mut output_results,
          &self.filename,
          &mut streams,
          audio_indexes.clone(),
          sine_parameters.clone(),
          video_details.frame_rate,
          decode_end,
        );
        end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        sine_time += (end_time.as_millis() - start_time.as_millis()) as f64;
      }
    }

    if let Some(ref black_and_silence_parameters) = check.black_and_silence_detect {
      if check.black_detect.is_some() && check.silence_detect.is_some() {
        detect_black_and_silence(
          &mut streams,
          video_indexes.clone(),
          audio_indexes.clone(),
          black_and_silence_parameters.clone(),
          video_details.frame_duration,
        );
      }
    }

    for index in 0..context.get_nb_streams() {
      if let Ok(stream) = Stream::new(context.get_stream(index as isize)) {
        streams[(index) as usize].detected_bitrate = stream.get_bit_rate();
      }
    }

    let mut format = FormatProbeResult::new();
    format.detected_bitrate_format = context.get_bit_rate();

    self.result = Some(DeepProbeResult { streams, format });

    context.close_input();

    let dp_end: std::time::Duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    println!("\nDURATION DETAILS :");
    println!("{:12} : {:?}s", "Decode", (decode_time / 1000.0));
    println!("{:12} : {:?}s", "Silence", (silence_time / 1000.0));
    println!("{:12} : {:?}s", "Blackframes", (blackframes_time / 1000.0));
    println!("{:12} : {:?}s", "Blackfades", (blackfades_time / 1000.0));
    println!("{:12} : {:?}s", "Crop", (crop_time / 1000.0));
    println!("{:12} : {:?}s", "Scene", (scene_time / 1000.0));
    println!("{:12} : {:?}s", "Ocr", (ocr_time / 1000.0));
    println!("{:12} : {:?}s", "Loudness", (loudness_time / 1000.0));
    println!("{:12} : {:?}s", "Dualmono", (dualmono_time / 1000.0));
    println!("{:12} : {:?}s", "Sine", (sine_time / 1000.0));
    println!("{:12} : {:?}\n", "Total", (dp_end - dp_start));

    Ok(())
  }
}

#[test]
fn deep_probe() {
  // use serde_json;
  use std::collections::HashMap;
  use uuid::Uuid;

  let duration_params = CheckParameterValue {
    min: Some(40),
    max: Some(20000),
    num: None,
    den: None,
    th: None,
    pairs: None,
  };
  let black_duration_params = CheckParameterValue {
    min: Some(40),
    max: Some(20000),
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
  let blackfade_duration_params = CheckParameterValue {
    min: Some(40),
    max: None,
    num: None,
    den: None,
    th: None,
    pairs: None,
  };
  let blackfade_pixel_params = CheckParameterValue {
    min: None,
    max: None,
    num: None,
    den: None,
    th: Some(0.5),
    pairs: None,
  };
  let blackfade_picture_params = CheckParameterValue {
    min: None,
    max: None,
    num: None,
    den: None,
    th: Some(1.0),
    pairs: None,
  };
  let spot_check = CheckParameterValue {
    min: None,
    max: Some(5),
    num: None,
    den: None,
    th: None,
    pairs: None,
  };
  let black_and_silence_check = CheckParameterValue {
    min: Some(40),
    max: None,
    num: None,
    den: None,
    th: None,
    pairs: None,
  };
  let mut audio_qualif = vec![];
  // definition : [Track::new(stream_index, channels_number)]
  audio_qualif.push([Track::new(1, 1)].to_vec());
  audio_qualif.push([Track::new(2, 1)].to_vec());
  audio_qualif.push([Track::new(3, 8)].to_vec());
  audio_qualif.push([Track::new(4, 2)].to_vec());
  audio_qualif.push([Track::new(5, 2)].to_vec());
  audio_qualif.push([Track::new(6, 1), Track::new(7, 1)].to_vec()); //dualmono
  let loudness_check = CheckParameterValue {
    min: None,
    max: None,
    num: None,
    den: None,
    th: None,
    pairs: Some(audio_qualif.clone()),
  };
  let dualmono_duration_check = CheckParameterValue {
    min: Some(100),
    max: None,
    num: None,
    den: None,
    th: None,
    pairs: None,
  };
  let dualmono_qualif_check = CheckParameterValue {
    min: None,
    max: None,
    num: None,
    den: None,
    th: None,
    pairs: Some(audio_qualif.clone()),
  };
  let scene_check = CheckParameterValue {
    min: None,
    max: None,
    num: None,
    den: None,
    th: Some(10.0),
    pairs: None,
  };
  let ocr_check = CheckParameterValue {
    min: None,
    max: None,
    num: None,
    den: None,
    th: Some(14.0),
    pairs: None,
  };
  let sine_check = CheckParameterValue {
    min: Some(100),
    max: None,
    num: None,
    den: None,
    th: None,
    pairs: None,
  };
  let sine_qualif_check = CheckParameterValue {
    min: None,
    max: None,
    num: None,
    den: None,
    th: None,
    pairs: Some(audio_qualif),
  };

  let mut silence_params = HashMap::new();
  let mut black_params = HashMap::new();
  let mut blackfade_params = HashMap::new();
  let mut select_params = HashMap::new();
  let mut black_and_silence_params = HashMap::new();
  let mut scene_params = HashMap::new();
  let mut ocr_params = HashMap::new();
  let mut loudness_params = HashMap::new();
  let mut dualmono_params = HashMap::new();
  let mut sine_params = HashMap::new();
  silence_params.insert("duration".to_string(), duration_params);
  black_params.insert("duration".to_string(), black_duration_params);
  black_params.insert("picture".to_string(), black_picture_params);
  black_params.insert("pixel".to_string(), black_pixel_params);
  blackfade_params.insert("duration".to_string(), blackfade_duration_params);
  blackfade_params.insert("picture".to_string(), blackfade_picture_params);
  blackfade_params.insert("pixel".to_string(), blackfade_pixel_params);
  select_params.insert("spot_check".to_string(), spot_check);
  loudness_params.insert("pairing_list".to_string(), loudness_check);
  dualmono_params.insert("duration".to_string(), dualmono_duration_check.clone());
  dualmono_params.insert("pairing_list".to_string(), dualmono_qualif_check.clone());
  black_and_silence_params.insert("duration".to_string(), black_and_silence_check);
  scene_params.insert("threshold".to_string(), scene_check);
  ocr_params.insert("threshold".to_string(), ocr_check);
  sine_params.insert("duration".to_string(), sine_check);
  sine_params.insert("pairing_list".to_string(), sine_qualif_check);
  let check = DeepProbeCheck {
    silence_detect: Some(silence_params),
    black_detect: Some(black_params),
    blackfade_detect: Some(blackfade_params),
    crop_detect: Some(select_params),
    black_and_silence_detect: Some(black_and_silence_params),
    scene_detect: Some(scene_params),
    ocr_detect: None,
    loudness_detect: Some(loudness_params),
    dualmono_detect: Some(dualmono_params),
    sine_detect: Some(sine_params),
  };
  let id = Uuid::parse_str("ef7e3ad9-a08f-4cd0-9fec-3ac465bbdd85").unwrap();
  let mut probe = DeepProbe::new("tests/test_file.mxf", id);
  probe.process(LevelFilter::Error, check).unwrap();
  println!("{}", serde_json::to_string(&probe).unwrap());
  let content = std::fs::read_to_string("tests/deep_probe.json").unwrap();
  let reference: DeepProbe = serde_json::from_str(&content).unwrap();
  assert_eq!(probe, reference);
}
