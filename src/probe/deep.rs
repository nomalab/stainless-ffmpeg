use crate::order::input::Input;
use crate::order::stream::Stream as StreamOrder;
use crate::order::OutputResult;
use crate::probe::black_and_silence::detect_black_and_silence;
use crate::probe::black_detect::{blackframes_init, detect_black_frames};
use crate::probe::blackfade_detect::detect_blackfade;
use crate::probe::crop_detect::{black_borders_init, detect_black_borders};
use crate::probe::dualmono_detect::{detect_dualmono, dualmono_init};
use crate::probe::loudness_detect::{detect_loudness, loudness_init};
use crate::probe::ocr_detect::{detect_ocr, ocr_init};
use crate::probe::scene_detect::{detect_scene, scene_init};
use crate::probe::silence_detect::{detect_silence, silence_init};
use crate::probe::sine_detect::{detect_sine, sine_init};
use crate::stream::Stream;
use crate::tools::rational::Rational;
use crate::{format_context::FormatContext, order::Order};
use ffmpeg_sys_next::*;
use log::LevelFilter;
use std::collections::BTreeMap;
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
  pub frame_start: i64,
  pub frame_end: i64,
  pub frames_length: i64,
  pub score: i32,
  pub index: u32,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct FalseSceneResult {
  pub frame_index: i64,
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

#[derive(Default)]
pub struct DeepOrder {
  check: DeepProbeCheck,
  orders: BTreeMap<CheckName, Order>,
  output_results: BTreeMap<CheckName, Vec<OutputResult>>,
  streams: Vec<StreamProbeResult>,
  video_details: VideoDetails,
  audio_indexes: Vec<u32>,
  video_indexes: Vec<u32>,
}

#[derive(Eq, Hash, PartialEq, PartialOrd, Ord)]
pub enum CheckName {
  Silence,
  BlackFrame,
  BlackFade,
  BlackBorder,
  BlackAndSilence,
  MediaOffline,
  Scene,
  Loudness,
  DualMono,
  Tone,
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

impl DeepOrder {
  pub fn new(check: DeepProbeCheck) -> Self {
    DeepOrder {
      check,
      orders: BTreeMap::new(),
      output_results: BTreeMap::new(),
      streams: vec![],
      video_details: VideoDetails::new(),
      audio_indexes: vec![],
      video_indexes: vec![],
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

  fn setup(
    &self,
    context: &mut FormatContext,
    deep_orders: &mut DeepOrder,
    src_inputs: &mut Vec<Input>,
  ) -> Result<(), String> {
    deep_orders
      .streams
      .resize(context.get_nb_streams() as usize, StreamProbeResult::new());
    while let Ok(packet) = context.next_packet() {
      unsafe {
        let stream_index = (*packet.packet).stream_index as usize;
        let packet_size = (*packet.packet).size;

        deep_orders.streams[stream_index].stream_index = stream_index;
        deep_orders.streams[stream_index].count_packets += 1;
        deep_orders.streams[stream_index].min_packet_size = cmp::min(
          packet_size,
          deep_orders.streams[stream_index].min_packet_size,
        );
        deep_orders.streams[stream_index].max_packet_size = cmp::max(
          packet_size,
          deep_orders.streams[stream_index].max_packet_size,
        );

        if context.get_stream_type(stream_index as isize) == AVMediaType::AVMEDIA_TYPE_VIDEO {
          if let Ok(stream) = Stream::new(context.get_stream(stream_index as isize)) {
            deep_orders.streams[stream_index].color_space = stream.get_color_space();
            deep_orders.streams[stream_index].color_range = stream.get_color_range();
            deep_orders.streams[stream_index].color_primaries = stream.get_color_primaries();
            deep_orders.streams[stream_index].color_trc = stream.get_color_trc();
            deep_orders.streams[stream_index].color_matrix = stream.get_color_matrix();
            deep_orders.video_details.frame_duration = stream.get_frame_rate().invert().to_float();
            deep_orders.video_details.frame_rate = stream.get_frame_rate().to_float();
            deep_orders.video_details.time_base = stream.get_time_base().to_float();
            deep_orders.video_details.stream_duration = stream.get_duration();
            deep_orders.video_details.stream_frames = stream.get_nb_frames();
            deep_orders.video_details.bits_raw_sample = stream.get_bits_per_raw_sample();
            deep_orders.video_details.metadata_width = stream.get_width();
            deep_orders.video_details.metadata_height = stream.get_height();
            deep_orders.video_details.aspect_ratio = stream.get_picture_aspect_ratio();
          }
        }
      }
    }

    for stream_index in 0..context.get_nb_streams() {
      let mut input_id = format!("unknown_input_{}", stream_index);
      if context.get_stream_type(stream_index as isize) == AVMediaType::AVMEDIA_TYPE_VIDEO {
        deep_orders.video_indexes.push(stream_index);
        input_id = format!("video_input_{}", stream_index);
      }
      if context.get_stream_type(stream_index as isize) == AVMediaType::AVMEDIA_TYPE_AUDIO {
        deep_orders.audio_indexes.push(stream_index);
        input_id = format!("audio_input_{}", stream_index);
      }
      if context.get_stream_type(stream_index as isize) == AVMediaType::AVMEDIA_TYPE_SUBTITLE {
        input_id = format!("subtitle_input_{}", stream_index);
      }
      let input_streams = vec![StreamOrder {
        index: stream_index,
        label: Some(input_id),
      }];
      src_inputs.push(Input::Streams {
        id: stream_index,
        path: context.filename.to_string(),
        streams: input_streams,
      });
    }

    if let Some(params) = deep_orders.check.black_detect.clone() {
      deep_orders.orders.insert(
        CheckName::BlackFrame,
        blackframes_init(&self.filename, deep_orders.video_indexes.clone(), &params).unwrap(),
      );
      deep_orders
        .output_results
        .insert(CheckName::BlackFrame, vec![]);
    }

    if let Some(params) = deep_orders.check.blackfade_detect.clone() {
      deep_orders.orders.insert(
        CheckName::BlackFade,
        blackframes_init(&self.filename, deep_orders.video_indexes.clone(), &params).unwrap(),
      );
      deep_orders
        .output_results
        .insert(CheckName::BlackFade, vec![]);
    }

    if let Some(params) = deep_orders.check.crop_detect.clone() {
      deep_orders.orders.insert(
        CheckName::BlackBorder,
        black_borders_init(
          &self.filename,
          deep_orders.video_indexes.clone(),
          &params,
          deep_orders.video_details.clone(),
        )
        .unwrap(),
      );
      deep_orders
        .output_results
        .insert(CheckName::BlackBorder, vec![]);
    }

    if let Some(params) = deep_orders.check.scene_detect.clone() {
      deep_orders.orders.insert(
        CheckName::Scene,
        scene_init(&self.filename, deep_orders.video_indexes.clone(), &params).unwrap(),
      );
      deep_orders.output_results.insert(CheckName::Scene, vec![]);
    }

    if let Some(params) = deep_orders.check.ocr_detect.clone() {
      deep_orders.orders.insert(
        CheckName::MediaOffline,
        ocr_init(&self.filename, deep_orders.video_indexes.clone(), &params).unwrap(),
      );
      deep_orders
        .output_results
        .insert(CheckName::MediaOffline, vec![]);
    }

    if let Some(params) = deep_orders.check.silence_detect.clone() {
      deep_orders.orders.insert(
        CheckName::Silence,
        silence_init(&self.filename, deep_orders.audio_indexes.clone(), &params).unwrap(),
      );
      deep_orders
        .output_results
        .insert(CheckName::Silence, vec![]);
    }

    if let Some(params) = deep_orders.check.loudness_detect.clone() {
      deep_orders.orders.insert(
        CheckName::Loudness,
        loudness_init(&self.filename, &params).unwrap(),
      );
      deep_orders
        .output_results
        .insert(CheckName::Loudness, vec![]);
    }

    if let Some(params) = deep_orders.check.dualmono_detect.clone() {
      deep_orders.orders.insert(
        CheckName::DualMono,
        dualmono_init(&self.filename, &params).unwrap(),
      );
      deep_orders
        .output_results
        .insert(CheckName::DualMono, vec![]);
    }

    if let Some(params) = deep_orders.check.sine_detect.clone() {
      deep_orders.orders.insert(
        CheckName::Tone,
        sine_init(&self.filename, deep_orders.audio_indexes.clone(), &params).unwrap(),
      );
      deep_orders.output_results.insert(CheckName::Tone, vec![]);
    }

    Ok(())
  }

  fn get_results(
    &self,
    context: &FormatContext,
    deep_orders: &mut DeepOrder,
  ) -> Result<(), String> {
    for order in &deep_orders.orders {
      match order.0 {
        CheckName::Silence => {
          if let Some(params) = deep_orders.check.silence_detect.clone() {
            detect_silence(
              &deep_orders.output_results,
              &mut deep_orders.streams,
              deep_orders.audio_indexes.clone(),
              &params,
              deep_orders.video_details.clone(),
            );
          }
        }
        CheckName::BlackFrame => {
          if let Some(params) = deep_orders.check.black_detect.clone() {
            detect_black_frames(
              &deep_orders.output_results,
              &mut deep_orders.streams,
              deep_orders.video_indexes.clone(),
              &params,
              deep_orders.video_details.clone(),
            )
          }
        }
        CheckName::BlackFade => {
          if let Some(params) = deep_orders.check.blackfade_detect.clone() {
            detect_blackfade(
              &deep_orders.output_results,
              &mut deep_orders.streams,
              deep_orders.video_indexes.clone(),
              &params,
              deep_orders.video_details.clone(),
            )
          }
        }
        CheckName::BlackBorder => {
          detect_black_borders(
            &deep_orders.output_results,
            &mut deep_orders.streams,
            deep_orders.video_indexes.clone(),
            deep_orders.video_details.clone(),
          );
        }
        CheckName::BlackAndSilence => {}
        CheckName::MediaOffline => {
          detect_ocr(
            &deep_orders.output_results,
            &mut deep_orders.streams,
            deep_orders.video_indexes.clone(),
            deep_orders.video_details.clone(),
          );
        }
        CheckName::Scene => {
          detect_scene(
            &deep_orders.output_results,
            &mut deep_orders.streams,
            deep_orders.video_indexes.clone(),
            deep_orders.video_details.clone(),
          );
        }
        CheckName::Loudness => {
          if let Some(params) = deep_orders.check.loudness_detect.clone() {
            detect_loudness(
              &deep_orders.output_results,
              &mut deep_orders.streams,
              deep_orders.audio_indexes.clone(),
              &params,
            )
          }
        }
        CheckName::DualMono => {
          if let Some(params) = deep_orders.check.dualmono_detect.clone() {
            detect_dualmono(
              &deep_orders.output_results,
              &mut deep_orders.streams,
              deep_orders.audio_indexes.clone(),
              &params,
              deep_orders.video_details.clone(),
            )
          }
        }
        CheckName::Tone => {
          if let Some(params) = deep_orders.check.sine_detect.clone() {
            detect_sine(
              &deep_orders.output_results,
              &self.filename,
              &mut deep_orders.streams,
              deep_orders.audio_indexes.clone(),
              &params,
              deep_orders.video_details.frame_rate,
            )
          }
        }
      }
    }

    if let Some(params) = deep_orders.check.black_and_silence_detect.clone() {
      if deep_orders.check.black_detect.is_some() && deep_orders.check.silence_detect.is_some() {
        detect_black_and_silence(
          &mut deep_orders.streams,
          deep_orders.video_indexes.clone(),
          deep_orders.audio_indexes.clone(),
          &params,
          deep_orders.video_details.frame_duration,
        );
      }
    }
    for index in 0..context.get_nb_streams() {
      if let Ok(stream) = Stream::new(context.get_stream(index as isize)) {
        deep_orders.streams[(index) as usize].detected_bitrate = stream.get_bit_rate();
      }
    }

    Ok(())
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

    let mut deep_orders = DeepOrder::new(check);
    let mut src_inputs = vec![];
    if let Err(msg) = self.setup(&mut context, &mut deep_orders, &mut src_inputs) {
      error!("Error while setup deep probe orders : {msg}");
    }

    let mut order_src = Order::new(src_inputs, vec![], vec![]).unwrap();
    order_src.build_input_format()?;
    let mut decode_end = false;

    while !decode_end {
      let (in_audio_frames, in_video_frames, in_subtitle_packets, end) = order_src.process_input();
      decode_end = end;

      for order in &mut deep_orders.orders {
        match order
          .1
          .filtering(&in_audio_frames, &in_video_frames, &in_subtitle_packets)
        {
          Ok(results) => {
            let res = deep_orders.output_results.get_mut(&order.0).unwrap();
            res.extend(results);
          }
          Err(msg) => {
            error!("Error while filtering : {msg}");
          }
        }
      }
    }

    if let Err(msg) = self.get_results(&context, &mut deep_orders) {
      error!("Error while processing results : {msg}");
    }

    let mut format = FormatProbeResult::new();
    format.detected_bitrate_format = context.get_bit_rate();

    self.result = Some(DeepProbeResult {
      streams: deep_orders.streams,
      format,
    });

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
