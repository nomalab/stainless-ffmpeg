use crate::format_context::FormatContext;
use crate::probe::black_detect::detect_black_frames;
use crate::probe::crop_detect::detect_black_borders;
use crate::probe::dualmono_detect::detect_dualmono;
use crate::probe::loudness_detect::detect_loudness;
use crate::probe::silence_detect::detect_silence;
use crate::stream::Stream;
use ffmpeg_sys_next::*;
use log::LevelFilter;
use std::{cmp, collections::HashMap, fmt};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct DeepProbe {
  #[serde(skip_serializing)]
  filename: String,
  pub result: Option<DeepProbeResult>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct DeepProbeResult {
  #[serde(default)]
  streams: Vec<StreamProbeResult>,
  format: FormatProbeResult,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SilenceResult {
  pub start: i64,
  pub end: i64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct BlackResult {
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

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct StreamProbeResult {
  stream_index: usize,
  count_packets: usize,
  min_packet_size: i32,
  max_packet_size: i32,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub detected_silence: Vec<SilenceResult>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub silent_stream: Option<bool>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub detected_black: Vec<BlackResult>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub detected_crop: Vec<CropResult>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub detected_loudness: Vec<LoudnessResult>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub detected_dualmono: Vec<DualMonoResult>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub detected_bitrate: Option<i64>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct FormatProbeResult {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub detected_bitrate_format: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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
  pub crop_detect: Option<HashMap<String, CheckParameterValue>>,
  pub loudness_detect: Option<HashMap<String, CheckParameterValue>>,
  pub dualmono_detect: Option<HashMap<String, CheckParameterValue>>,
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
      writeln!(
        f,
        "{:30} : {:?}",
        "Silence detection", stream.detected_silence
      )?;
      writeln!(f, "{:30} : {:?}", "Black detection", stream.detected_black)?;
      writeln!(f, "{:30} : {:?}", "Crop detection", stream.detected_crop)?;
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
      min_packet_size: std::i32::MAX,
      max_packet_size: std::i32::MIN,
      detected_silence: vec![],
      silent_stream: None,
      detected_black: vec![],
      detected_crop: vec![],
      detected_loudness: vec![],
      detected_dualmono: vec![],
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
}

impl DeepProbe {
  pub fn new(filename: &str) -> Self {
    DeepProbe {
      filename: filename.to_owned(),
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
      }
    }

    if let Some(silence_parameters) = check.silence_detect {
      let mut audio_indexes = vec![];
      for stream_index in 0..context.get_nb_streams() {
        if context.get_stream_type(stream_index as isize) == AVMediaType::AVMEDIA_TYPE_AUDIO {
          audio_indexes.push(stream_index);
        }
      }
      detect_silence(
        &self.filename,
        &mut streams,
        audio_indexes,
        silence_parameters,
      );
    }

    if let Some(black_parameters) = check.black_detect {
      let mut video_indexes = vec![];
      for stream_index in 0..context.get_nb_streams() {
        if context.get_stream_type(stream_index as isize) == AVMediaType::AVMEDIA_TYPE_VIDEO {
          video_indexes.push(stream_index);
        }
      }
      detect_black_frames(
        &self.filename,
        &mut streams,
        video_indexes,
        black_parameters,
      );
    }

    if let Some(crop_parameters) = check.crop_detect {
      let mut video_indexes = vec![];
      for stream_index in 0..context.get_nb_streams() {
        if context.get_stream_type(stream_index as isize) == AVMediaType::AVMEDIA_TYPE_VIDEO {
          video_indexes.push(stream_index);
        }
      }
      detect_black_borders(&self.filename, &mut streams, video_indexes, crop_parameters);
    }

    for index in 0..context.get_nb_streams() {
      if let Ok(stream) = Stream::new(context.get_stream(index as isize)) {
        streams[(index) as usize].detected_bitrate = stream.get_bit_rate();
      }
    }

    if let Some(loudness_parameters) = check.loudness_detect {
      detect_loudness(&self.filename, &mut streams, loudness_parameters);
    }

    if let Some(dualmono_parameters) = check.dualmono_detect {
      detect_dualmono(&self.filename, &mut streams, dualmono_parameters);
    }

    let mut format = FormatProbeResult::new();
    format.detected_bitrate_format = context.get_bit_rate();

    self.result = Some(DeepProbeResult { streams, format });

    context.close_input();
    Ok(())
  }
}

#[test]
fn deep_probe_mxf_sample_dualmono() {
  /*
   * Test avec fichier dualmono.mxf :
   * -S#0.0 : mire de barre
   * -S#0.1 : stereo dual-mono 10sec
   * -S#0.2 : stereo non dual-mono (silence+programme) 10sec
   * -S#0.3 : mono programme 10sec
   * -S#0.4 : mono programme 10sec
   * -S#0.5 : mono silence 10sec
   * -S#0.6 : stereo dual-mono 5sec + stereo non dual-mono 5sec
   * -S#0.7 : stereo non dual-mono 5sec + stereo dual-mono 5sec
   * -S#0.8 : stereo dual-mono 2sec + stereo non dual-mono 6sec + stereo dual-mono 2sec
   * -S#0.9 : stereo non dual-mono 2sec + stereo dual-mono 6sec + stereo non dual-mono 2sec
   * audio_qualif en conséquences
   * test réalisables sur ce fichier :
   *   -loudness
   *   -dualmono
   *   -1000hz
   */
  use std::collections::HashMap;

  let mut probe = DeepProbe::new("tests/dualmono.mxf");
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
  probe.process(LevelFilter::Error, check).unwrap();

  println!("{}", serde_json::to_string(&probe).unwrap());

  let content = std::fs::read_to_string("tests/deep_probe_dualmono.json").unwrap();
  let reference: DeepProbe = serde_json::from_str(&content).unwrap();
  assert_eq!(probe, reference);
}
