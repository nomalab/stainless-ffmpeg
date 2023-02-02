use crate::format_context::FormatContext;
use crate::stream::Stream;
use crate::tools::rational::Rational;
use ffmpeg_sys_next::*;
use log::LevelFilter;
use std::collections::{BTreeMap, HashMap};
use std::fmt;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Probe {
  #[serde(skip_serializing)]
  filename: String,
  pub format: Option<Format>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Format {
  format_name: String,
  format_long_name: String,
  program_count: u32,
  start_time: Option<f32>,
  duration: Option<f64>,
  bit_rate: Option<i64>,
  packet_size: u32,
  nb_streams: u32,
  metadata: BTreeMap<String, String>,
  streams: Vec<StreamDescriptor>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct StreamDescriptor {
  index: u32,
  stream_type: String,
  codec_name: Option<String>,
  codec_long_name: Option<String>,
  codec_tag: Option<String>,
  start_time: Option<f32>,
  duration: Option<f32>,
  bit_rate: Option<i64>,
  stream_metadata: HashMap<String, String>,

  #[serde(flatten)]
  video_properties: Option<VideoProperties>,
  #[serde(flatten)]
  audio_properties: Option<AudioProperties>,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct VideoProperties {
  width: i32,
  height: i32,
  display_aspect_ratio: Rational,
  frame_rate: Rational,
  level: Option<i32>,
  profile: Option<String>,
  scanning_type: Option<String>,
  chroma_subsampling: Option<String>,
  timecode: Option<String>,
  pix_fmt: Option<String>,
  nb_frames: Option<i64>,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct AudioProperties {
  channels: i32,
  sample_rate: i32,
  sample_fmt: String,
  bits_per_sample: i32,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct SubtitleProperties {}

impl fmt::Display for Format {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    writeln!(f, "{:30} : {}", "Format name", self.format_name)?;
    writeln!(f, "{:30} : {}", "Format long name", self.format_long_name)?;
    writeln!(f, "{:30} : {:?}", "Start time", self.start_time)?;
    writeln!(f, "{:30} : {:?}", "Duration", self.duration)?;
    writeln!(f, "{:30} : {:?}", "Bit rate", self.bit_rate)?;
    writeln!(f, "{:30} : {}", "Packet size", self.packet_size)?;
    writeln!(f, "{:30} : {}", "Number of streams", self.nb_streams)?;
    writeln!(f, "{:30} : {}", "Number of Programs", self.program_count)?;

    for (key, value) in &self.metadata {
      writeln!(f, "{key:30} : {value}")?;
    }

    for stream in &self.streams {
      writeln!(f, "\n{:30} : {}", "Stream type", stream.stream_type)?;
      writeln!(f, "{:30} : {}", "Index", stream.index)?;
      writeln!(f, "{:30} : {:?}", "Codec name", stream.codec_name)?;
      writeln!(f, "{:30} : {:?}", "Codec long name", stream.codec_long_name)?;
      writeln!(f, "{:30} : {:?}", "Codec tag", stream.codec_tag)?;

      if let Some(ref vp) = stream.video_properties {
        writeln!(f, "{:30} : {}", "Width", vp.width)?;
        writeln!(f, "{:30} : {}", "Height", vp.height)?;
        writeln!(
          f,
          "{:30} : {:?}",
          "Display aspect ratio", vp.display_aspect_ratio
        )?;
        writeln!(f, "{:30} : {:?}", "Frame rate", vp.frame_rate)?;
        writeln!(f, "{:30} : {:?}", "Level", vp.level)?;
        writeln!(f, "{:30} : {:?}", "Profile", vp.profile)?;
        writeln!(f, "{:30} : {:?}", "Start time", stream.start_time)?;
        writeln!(f, "{:30} : {:?}", "Duration", stream.duration)?;
        writeln!(f, "{:30} : {:?}", "Bit rate", stream.bit_rate)?;
        writeln!(f, "{:30} : {:?}", "Scanning type", vp.scanning_type)?;
        writeln!(
          f,
          "{:30} : {:?}",
          "Chroma subsampling", vp.chroma_subsampling
        )?;
        writeln!(f, "{:30} : {:?}", "Timecode", vp.timecode)?;
        writeln!(f, "{:30} : {:?}", "Pixel format", vp.pix_fmt)?;
        writeln!(f, "{:30} : {:?}", "Number of frames", vp.nb_frames)?;
      }
      if let Some(ref ap) = stream.audio_properties {
        writeln!(f, "{:30} : {}", "Channels", ap.channels)?;
        writeln!(f, "{:30} : {}", "Sample rate", ap.sample_rate)?;
        writeln!(f, "{:30} : {}", "Sample format", ap.sample_fmt)?;
        writeln!(f, "{:30} : {}", "Bits per sample", ap.bits_per_sample)?;
        writeln!(f, "{:30} : {:?}", "Start time", stream.start_time)?;
        writeln!(f, "{:30} : {:?}", "Duration", stream.duration)?;
        writeln!(f, "{:30} : {:?}", "Bit rate", stream.bit_rate)?;
      }

      for (key, value) in &stream.stream_metadata {
        writeln!(f, "{key:30} : {value}")?;
      }
    }
    Ok(())
  }
}

impl Probe {
  pub fn new(filename: &str) -> Self {
    Probe {
      filename: filename.to_owned(),
      format: None,
    }
  }

  pub fn process(&mut self, log_level: LevelFilter) -> Result<(), String> {
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
      self.format = None;
      context.close_input();
      return Ok(());
    }
    let format_name = context.get_format_name();
    let format_long_name = context.get_format_long_name();

    let program_count = context.get_program_count();
    let start_time = context.get_start_time();
    let duration = context.get_duration();

    let bit_rate = context.get_bit_rate();
    let packet_size = context.get_packet_size();
    let nb_streams = context.get_nb_streams();

    let metadata = context.get_metadata();
    let mut streams = vec![];

    for index in 0..context.get_nb_streams() {
      if let Ok(stream) = Stream::new(context.get_stream(index as isize)) {
        let stream_type = context.get_stream_type_name(index as isize);
        let codec_name = stream.get_codec_name();
        let codec_long_name = stream.get_codec_long_name();
        let codec_tag = stream.get_codec_tag();
        let duration = stream.get_duration();
        let start_time = stream.get_start_time();
        let bit_rate = stream.get_bit_rate();
        let mut vp = None;
        let mut ap = None;
        let stream_metadata = stream.get_stream_metadata();

        match context.get_stream_type(index as isize) {
          AVMediaType::AVMEDIA_TYPE_VIDEO => {
            let width = stream.get_width();
            let height = stream.get_height();
            let display_aspect_ratio = stream.get_display_aspect_ratio();
            let frame_rate = stream.get_frame_rate();
            let scanning_type = stream.get_scanning_type();
            let chroma_subsampling = stream.get_chroma_sub_sample();
            let level = stream.get_level();
            let profile = stream.get_profile();
            let timecode = stream.get_timecode();
            let pix_fmt = stream.get_pix_fmt_name();
            let nb_frames = stream.get_nb_frames();

            vp = Some(VideoProperties {
              width,
              height,
              display_aspect_ratio,
              frame_rate,
              level,
              profile,
              scanning_type,
              chroma_subsampling,
              timecode,
              pix_fmt,
              nb_frames,
            });
          }
          AVMediaType::AVMEDIA_TYPE_AUDIO => {
            let channels = stream.get_channels();

            let bits_per_sample = stream.get_bits_per_sample();
            let sample_fmt = stream.get_sample_fmt();
            let sample_rate = stream.get_sample_rate();

            ap = Some(AudioProperties {
              channels,
              sample_rate,
              sample_fmt,
              bits_per_sample,
            });
          }
          _ => {}
        }

        streams.push(StreamDescriptor {
          index,
          stream_type,
          codec_name,
          codec_long_name,
          codec_tag,
          start_time,
          duration,
          bit_rate,
          stream_metadata,
          video_properties: vp,
          audio_properties: ap,
        })
      }
    }

    self.format = Some(Format {
      format_name,
      format_long_name,
      program_count,
      start_time,
      duration,
      bit_rate,
      packet_size,
      nb_streams,
      metadata,
      streams,
    });

    context.close_input();
    Ok(())
  }
}

#[test]
fn probe_mxf_sample() {
  use serde_json;
  use std::fs::File;
  use std::io::prelude::*;

  let mut probe = Probe::new("tests/PAL_1080i_MPEG_XDCAM-HD_colorbar.mxf");
  probe.process(LevelFilter::Error).unwrap();

  let mut file = File::open("tests/probe.json").unwrap();
  let mut contents = String::new();
  file.read_to_string(&mut contents).unwrap();

  let reference: Probe = serde_json::from_str(&contents).unwrap();
  assert_eq!(probe, reference);
}
