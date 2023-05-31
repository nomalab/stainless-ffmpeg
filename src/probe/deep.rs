use crate::format_context::FormatContext;
use crate::probe::black_and_silence::detect_black_and_silence;
use crate::probe::black_detect::detect_black_frames;
use crate::probe::crop_detect::detect_black_borders;
use crate::probe::dualmono_detect::detect_dualmono;
use crate::probe::loudness_detect::detect_loudness;
use crate::probe::ocr_detect::detect_ocr;
use crate::probe::scene_detect::detect_scene;
use crate::probe::silence_detect::detect_silence;
use crate::probe::sine_detect::detect_sine;
use crate::stream::Stream;
use ffmpeg_sys_next::*;
use log::LevelFilter;
use std::{cmp, collections::HashMap, fmt};
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
  stream_index: usize,
  count_packets: usize,
  min_packet_size: i32,
  max_packet_size: i32,
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
  pub black_and_silence_detect: Option<HashMap<String, CheckParameterValue>>,
  pub crop_detect: Option<HashMap<String, CheckParameterValue>>,
  pub scene_detect: Option<HashMap<String, CheckParameterValue>>,
  pub ocr_detect: Option<HashMap<String, CheckParameterValue>>,
  pub loudness_detect: Option<HashMap<String, CheckParameterValue>>,
  pub dualmono_detect: Option<HashMap<String, CheckParameterValue>>,
  pub sine_detect: Option<HashMap<String, CheckParameterValue>>,
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

impl DeepProbe {
  pub fn new(filename: &str, id: Uuid) -> Self {
    DeepProbe {
      filename: filename.to_owned(),
      id,
      result: None,
    }
  }

  pub fn process(&mut self, log_level: LevelFilter, check: DeepProbeCheck) -> Result<(), String> {
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

    let mut streams = vec![];
    streams.resize(context.get_nb_streams() as usize, StreamProbeResult::new());
    while let Ok(packet) = context.next_packet() {
      unsafe {
        let stream_index = (*packet.packet).stream_index as usize;
        let packet_size = (*packet.packet).size;

        streams[stream_index].stream_index = stream_index;
        streams[stream_index].count_packets += 1;
        streams[stream_index].min_packet_size =
          cmp::min(packet_size, streams[stream_index].min_packet_size);
        streams[stream_index].max_packet_size =
          cmp::max(packet_size, streams[stream_index].max_packet_size);

        if context.get_stream_type(stream_index as isize) == AVMediaType::AVMEDIA_TYPE_VIDEO {
          if let Ok(stream) = Stream::new(context.get_stream(stream_index as isize)) {
            streams[stream_index].color_space = stream.get_color_space();
            streams[stream_index].color_range = stream.get_color_range();
            streams[stream_index].color_primaries = stream.get_color_primaries();
            streams[stream_index].color_trc = stream.get_color_trc();
            streams[stream_index].color_matrix = stream.get_color_matrix();
          }
        }
      }
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

    if let Some(silence_parameters) = check.silence_detect.clone() {
      detect_silence(
        &self.filename,
        &mut streams,
        audio_indexes.clone(),
        silence_parameters,
      );
    }

    if let Some(black_parameters) = check.black_detect.clone() {
      detect_black_frames(
        &self.filename,
        &mut streams,
        video_indexes.clone(),
        black_parameters,
      );
    }

    if let Some(black_and_silence_parameters) = check.black_and_silence_detect {
      if check.black_detect.is_some() && check.silence_detect.is_some() {
        detect_black_and_silence(
          &mut streams,
          video_indexes.clone(),
          audio_indexes.clone(),
          black_and_silence_parameters,
        );
      }
    }

    if let Some(crop_parameters) = check.crop_detect {
      detect_black_borders(
        &self.filename,
        &mut streams,
        video_indexes.clone(),
        crop_parameters,
      );
    }

    if let Some(scene_parameters) = check.scene_detect {
      detect_scene(
        &self.filename,
        &mut streams,
        video_indexes.clone(),
        scene_parameters,
      );
    }

    if let Some(ocr_parameters) = check.ocr_detect {
      detect_ocr(
        &self.filename,
        &mut streams,
        video_indexes.clone(),
        ocr_parameters,
      );
    }

    for index in 0..context.get_nb_streams() {
      if let Ok(stream) = Stream::new(context.get_stream(index as isize)) {
        streams[(index) as usize].detected_bitrate = stream.get_bit_rate();
      }
    }

    if let Some(loudness_parameters) = check.loudness_detect {
      detect_loudness(
        &self.filename,
        &mut streams,
        audio_indexes.clone(),
        loudness_parameters,
      );
    }

    if let Some(dualmono_parameters) = check.dualmono_detect {
      detect_dualmono(
        &self.filename,
        &mut streams,
        audio_indexes.clone(),
        dualmono_parameters,
      );
    }

    if let Some(sine_parameters) = check.sine_detect {
      detect_sine(&self.filename, &mut streams, audio_indexes, sine_parameters);
    }

    let mut format = FormatProbeResult::new();
    format.detected_bitrate_format = context.get_bit_rate();

    self.result = Some(DeepProbeResult { streams, format });

    context.close_input();
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
    th: Some(0.1),
    pairs: None,
  };
  let black_picture_params = CheckParameterValue {
    min: None,
    max: None,
    num: None,
    den: None,
    th: Some(0.98),
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
