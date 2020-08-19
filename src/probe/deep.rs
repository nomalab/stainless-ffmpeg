use crate::format_context::FormatContext;
use crate::probe::silence_detect::detect_silence;
use log::LevelFilter;
use stainless_ffmpeg_sys::*;
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
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SilenceResult {
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
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct DeepProbeCheck {
  pub silence_detect: HashMap<String, CheckParameterValue>,
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

    if !check.silence_detect.is_empty() {
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
        check.silence_detect,
      );
    }

    self.result = Some(DeepProbeResult { streams });

    context.close_input();
    Ok(())
  }
}

#[test]
fn deep_probe_mxf_sample() {
  use serde_json;
  use std::collections::HashMap;

  let mut probe = DeepProbe::new("tests/PAL_1080i_MPEG_XDCAM-HD_colorbar.mxf");
  let mut params = HashMap::new();
  let duration = CheckParameterValue {
    min: Some(2000),
    max: None,
    num: None,
    den: None,
  };
  params.insert("duration".to_string(), duration);
  let check_list = DeepProbeCheck {
    silence_detect: params,
  };
  probe.process(LevelFilter::Error, check_list).unwrap();

  // println!("{}", serde_json::to_string(&probe).unwrap());

  // let content = std::fs::read_to_string("tests/deep_probe.json").unwrap();
  // let reference: DeepProbe = serde_json::from_str(&content).unwrap();
  // assert_eq!(probe, reference);
}
