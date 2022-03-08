use crate::order::{output_kind::OutputKind, parameters::ParameterValue};
use ffmpeg_sys_next::*;
use std::{collections::HashMap, convert::TryFrom, str::FromStr};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub enum SampleFormat {
  #[serde(rename = "s8")]
  Unsigned8,
  #[serde(rename = "s8p")]
  Unsigned8Planar,
  #[serde(rename = "s16")]
  Signed16,
  #[serde(rename = "s16p")]
  Signed16Planar,
  #[serde(rename = "s32")]
  Signed32,
  #[serde(rename = "s32p")]
  Signed32Planar,
  #[serde(rename = "float")]
  Float,
  #[serde(rename = "floatp")]
  FloatPlanar,
  #[serde(rename = "double")]
  Double,
  #[serde(rename = "doublep")]
  DoublePlanar,
}

impl TryFrom<i32> for SampleFormat {
  type Error = String;
  fn try_from(value: i32) -> Result<Self, Self::Error> {
    match value {
      0 => Ok(SampleFormat::Unsigned8),
      1 => Ok(SampleFormat::Signed16),
      2 => Ok(SampleFormat::Signed32),
      3 => Ok(SampleFormat::Float),
      4 => Ok(SampleFormat::Double),
      5 => Ok(SampleFormat::Unsigned8Planar),
      6 => Ok(SampleFormat::Signed16Planar),
      7 => Ok(SampleFormat::Signed32Planar),
      8 => Ok(SampleFormat::FloatPlanar),
      9 => Ok(SampleFormat::DoublePlanar),
      _ => Err(format!("'{value}' is not a valid value for SampleFormat")),
    }
  }
}

impl FromStr for SampleFormat {
  type Err = String;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "s8" => Ok(SampleFormat::Unsigned8),
      "s8p" => Ok(SampleFormat::Unsigned8Planar),
      "s16" => Ok(SampleFormat::Signed16),
      "s16p" => Ok(SampleFormat::Signed16Planar),
      "s32" => Ok(SampleFormat::Signed32),
      "s32p" => Ok(SampleFormat::Signed32Planar),
      "float" => Ok(SampleFormat::Float),
      "floatp" => Ok(SampleFormat::FloatPlanar),
      "double" => Ok(SampleFormat::Double),
      "doublep" => Ok(SampleFormat::DoublePlanar),
      _ => Err(format!("'{s}' is not a valid value for SampleFormat")),
    }
  }
}

impl From<SampleFormat> for AVSampleFormat {
  fn from(sample: SampleFormat) -> AVSampleFormat {
    match sample {
      SampleFormat::Unsigned8 => AVSampleFormat::AV_SAMPLE_FMT_U8,
      SampleFormat::Unsigned8Planar => AVSampleFormat::AV_SAMPLE_FMT_U8P,
      SampleFormat::Signed16 => AVSampleFormat::AV_SAMPLE_FMT_S16,
      SampleFormat::Signed16Planar => AVSampleFormat::AV_SAMPLE_FMT_S16P,
      SampleFormat::Signed32 => AVSampleFormat::AV_SAMPLE_FMT_S32,
      SampleFormat::Signed32Planar => AVSampleFormat::AV_SAMPLE_FMT_S32P,
      SampleFormat::Float => AVSampleFormat::AV_SAMPLE_FMT_FLT,
      SampleFormat::FloatPlanar => AVSampleFormat::AV_SAMPLE_FMT_FLTP,
      SampleFormat::Double => AVSampleFormat::AV_SAMPLE_FMT_DBL,
      SampleFormat::DoublePlanar => AVSampleFormat::AV_SAMPLE_FMT_DBLP,
    }
  }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub enum PixelFormat {
  #[serde(rename = "yuv420p")]
  Yuv420p,
  #[serde(rename = "yuv422p")]
  Yuv422p,
  #[serde(rename = "rgb24")]
  Rgb24,
  #[serde(rename = "rgb48be")]
  Rgb48be,
  #[serde(rename = "rgb48le")]
  Rgb48le,
}

impl std::str::FromStr for PixelFormat {
  type Err = String;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "yuv420p" => Ok(PixelFormat::Yuv420p),
      "yuv422p" => Ok(PixelFormat::Yuv422p),
      "rgb24" => Ok(PixelFormat::Rgb24),
      "rgb48be" => Ok(PixelFormat::Rgb48be),
      "rgb48le" => Ok(PixelFormat::Rgb48le),
      _ => Err(format!("'{s}' is not a valid value for PixelFormat")),
    }
  }
}

impl From<PixelFormat> for AVPixelFormat {
  fn from(format: PixelFormat) -> AVPixelFormat {
    match format {
      PixelFormat::Yuv420p => AVPixelFormat::AV_PIX_FMT_YUV420P,
      PixelFormat::Yuv422p => AVPixelFormat::AV_PIX_FMT_YUV422P,
      PixelFormat::Rgb24 => AVPixelFormat::AV_PIX_FMT_RGB24,
      PixelFormat::Rgb48be => AVPixelFormat::AV_PIX_FMT_RGB48BE,
      PixelFormat::Rgb48le => AVPixelFormat::AV_PIX_FMT_RGB48LE,
    }
  }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub enum Colorspace {
  #[serde(rename = "rgb")]
  Rgb,
  #[serde(rename = "bt470bg")]
  Bt470bg,
  #[serde(rename = "smpte170m")]
  Smpte170m,
  #[serde(rename = "smpte240m")]
  Smpte240m,
  #[serde(rename = "smpte2085")]
  Smpte2085,
  #[serde(rename = "bt709")]
  Bt709,
  #[serde(rename = "bt2020_ncl")]
  Bt2020Ncl,
  #[serde(rename = "bt2020_cl")]
  Bt2020Cl,
}

impl std::str::FromStr for Colorspace {
  type Err = String;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "rgb" => Ok(Colorspace::Rgb),
      "bt470bg" => Ok(Colorspace::Bt470bg),
      "smpte170m" => Ok(Colorspace::Smpte170m),
      "smpte240m" => Ok(Colorspace::Smpte240m),
      "smpte2085" => Ok(Colorspace::Smpte2085),
      "bt709" => Ok(Colorspace::Bt709),
      "bt2020_ncl" => Ok(Colorspace::Bt2020Ncl),
      "bt2020_cl" => Ok(Colorspace::Bt2020Cl),
      _ => Err(format!("'{s}' is not a valid value for Colorspace")),
    }
  }
}

impl From<Colorspace> for AVColorSpace {
  fn from(space: Colorspace) -> AVColorSpace {
    match space {
      Colorspace::Rgb => AVColorSpace::AVCOL_SPC_RGB,
      Colorspace::Bt470bg => AVColorSpace::AVCOL_SPC_BT470BG,
      Colorspace::Bt709 => AVColorSpace::AVCOL_SPC_BT709,
      Colorspace::Smpte170m => AVColorSpace::AVCOL_SPC_SMPTE170M,
      Colorspace::Smpte240m => AVColorSpace::AVCOL_SPC_SMPTE240M,
      Colorspace::Bt2020Ncl => AVColorSpace::AVCOL_SPC_BT2020_NCL,
      Colorspace::Bt2020Cl => AVColorSpace::AVCOL_SPC_BT2020_CL,
      Colorspace::Smpte2085 => AVColorSpace::AVCOL_SPC_SMPTE2085,
    }
  }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub enum ColorRange {
  #[serde(rename = "head")]
  Head,
  #[serde(rename = "full")]
  Full,
}

impl std::str::FromStr for ColorRange {
  type Err = String;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "head" => Ok(ColorRange::Head),
      "full" => Ok(ColorRange::Full),
      _ => Err(format!("'{s}' is not a valid value for ColorRange")),
    }
  }
}

impl From<ColorRange> for AVColorRange {
  fn from(range: ColorRange) -> AVColorRange {
    match range {
      ColorRange::Head => AVColorRange::AVCOL_RANGE_MPEG,
      ColorRange::Full => AVColorRange::AVCOL_RANGE_JPEG,
    }
  }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub enum ChannelLayout {
  #[serde(rename = "mono")]
  Mono,
  #[serde(rename = "stereo")]
  Stereo,
  #[serde(rename = "5.1")]
  Multi5_1,
  #[serde(rename = "7.1")]
  Multi7_1,
}

impl std::str::FromStr for ChannelLayout {
  type Err = String;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "mono" => Ok(ChannelLayout::Mono),
      "stereo" => Ok(ChannelLayout::Stereo),
      "5.1" => Ok(ChannelLayout::Multi5_1),
      "7.1" => Ok(ChannelLayout::Multi7_1),
      _ => Err(format!("'{s}' is not a valid value for ChannelLayout")),
    }
  }
}

impl From<ChannelLayout> for u64 {
  fn from(layout: ChannelLayout) -> u64 {
    match layout {
      ChannelLayout::Mono => AV_CH_LAYOUT_MONO,
      ChannelLayout::Stereo => AV_CH_LAYOUT_STEREO,
      ChannelLayout::Multi5_1 => AV_CH_LAYOUT_5POINT1,
      ChannelLayout::Multi7_1 => AV_CH_LAYOUT_7POINT1,
    }
  }
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct OutputStream {
  pub label: Option<String>,
  pub codec: String,
  pub parameters: HashMap<String, ParameterValue>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Output {
  pub kind: Option<OutputKind>,
  #[serde(default)]
  pub keys: Vec<String>,
  #[serde(default)]
  pub parameters: HashMap<String, ParameterValue>,
  pub path: Option<String>,
  pub stream: Option<String>,
  #[serde(default)]
  pub streams: Vec<OutputStream>,
}
