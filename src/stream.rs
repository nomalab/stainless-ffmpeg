use crate::{tools, tools::rational::Rational};
use ffmpeg_sys_next::*;
use regex::Regex;
use std::{char, collections::HashMap, ffi::CString, ptr::null_mut};

#[derive(Clone)]
pub struct Stream {
  pub stream: *mut AVStream,
}

impl Stream {
  pub fn new(stream: *mut AVStream) -> Result<Stream, String> {
    if stream.is_null() {
      return Err("Null stream pointer".to_string());
    }
    Ok(Stream { stream })
  }

  pub fn get_time_base(&self) -> Rational {
    unsafe {
      Rational {
        num: (*self.stream).time_base.num,
        den: (*self.stream).time_base.den,
      }
    }
  }

  pub fn get_codec_name(&self) -> Option<String> {
    unsafe {
      let av_codec_id = avcodec_descriptor_get((*(*self.stream).codecpar).codec_id);
      if av_codec_id.is_null() {
        None
      } else {
        Some(tools::to_string((*av_codec_id).name))
      }
    }
  }

  pub fn get_codec_long_name(&self) -> Option<String> {
    unsafe {
      let av_codec_id = avcodec_descriptor_get((*(*self.stream).codecpar).codec_id);
      if av_codec_id.is_null() {
        None
      } else {
        let mut long_name = tools::to_string((*av_codec_id).long_name);
        if let Some(suffix) = self.get_codec_tag() {
          long_name.push(' ');
          long_name.push_str(&suffix);
        }
        Some(long_name)
      }
    }
  }

  pub fn get_codec_tag(&self) -> Option<String> {
    unsafe {
      let mut codec_tag = (*(*self.stream).codecpar).codec_tag;
      let mut codec_tag_str = "".to_string();
      for _i in 0..4 {
        let character = codec_tag & 0xFF;
        if let Some(c) = char::from_u32(character) {
          codec_tag_str.push(c);
        }
        codec_tag >>= 8;
      }
      match codec_tag_str.as_str() {
        "ap4x" => Some("4444 XQ".to_string()),
        "ap4h" => Some("4444".to_string()),
        "apch" => Some("422 HQ".to_string()),
        "apcn" => Some("422".to_string()),
        "apcs" => Some("422 LT".to_string()),
        "apco" => Some("422 Proxy".to_string()),
        _ => None,
      }
    }
  }

  fn metadata_duration(duration: &str) -> Option<f32> {
    let regex_tc_ms = Regex::new("[0-9]{2}:[0-9]{2}:[0-9]{2}.[0-9]+").unwrap();
    if let Some(tc_ms) = regex_tc_ms.find(duration) {
      let splitted_tc: Vec<&str> = tc_ms.as_str().split(':').collect();
      let hours = splitted_tc[0].parse::<f32>().unwrap();
      let minutes = splitted_tc[1].parse::<f32>().unwrap();
      let seconds = splitted_tc[2].parse::<f32>().unwrap();
      return Some(hours * 3600.0 + minutes * 60.0 + seconds);
    }
    None
  }

  pub fn get_duration(&self) -> Option<f32> {
    unsafe {
      if (*self.stream).duration == AV_NOPTS_VALUE {
        // try to find duration from metadata
        self
          .get_stream_metadata()
          .get("DURATION")
          .and_then(|duration| Self::metadata_duration(duration))
      } else {
        Some((*self.stream).duration as f32 * self.get_time_base().to_float())
      }
    }
  }

  pub fn get_duration_pts(&self) -> Option<i64> {
    unsafe {
      if (*self.stream).duration == AV_NOPTS_VALUE {
        None
      } else {
        Some((*self.stream).duration)
      }
    }
  }

  pub fn get_nb_frames(&self) -> Option<i64> {
    unsafe {
      if (*self.stream).nb_frames == 0 {
        self.get_duration_pts()
      } else {
        Some((*self.stream).nb_frames)
      }
    }
  }

  pub fn get_picture_aspect_ratio(&self) -> Rational {
    unsafe {
      if (*self.stream).sample_aspect_ratio.num == 0 {
        if (*(*self.stream).codecpar).sample_aspect_ratio.num == 0 {
          Rational { num: 1, den: 1 }
        } else {
          Rational {
            num: (*(*self.stream).codecpar).sample_aspect_ratio.num,
            den: (*(*self.stream).codecpar).sample_aspect_ratio.den,
          }
        }
      } else {
        Rational {
          num: (*self.stream).sample_aspect_ratio.num,
          den: (*self.stream).sample_aspect_ratio.den,
        }
      }
    }
  }

  pub fn get_start_time(&self) -> Option<f32> {
    unsafe {
      if (*self.stream).start_time == AV_NOPTS_VALUE {
        None
      } else {
        Some((*self.stream).start_time as f32 * self.get_time_base().to_float())
      }
    }
  }

  pub fn get_width(&self) -> i32 {
    unsafe { (*(*self.stream).codecpar).width }
  }

  pub fn get_height(&self) -> i32 {
    unsafe { (*(*self.stream).codecpar).height }
  }

  pub fn get_display_aspect_ratio(&self) -> Rational {
    unsafe {
      if (*(*self.stream).codecpar).sample_aspect_ratio.num == 0 {
        if (*self.stream).sample_aspect_ratio.num == 0 {
          Rational {
            num: (*(*self.stream).codecpar).width,
            den: (*(*self.stream).codecpar).height,
          }
          .reduce()
        } else {
          Rational {
            num: (*(*self.stream).codecpar).width * (*self.stream).sample_aspect_ratio.num,
            den: (*(*self.stream).codecpar).height * (*self.stream).sample_aspect_ratio.den,
          }
          .reduce()
        }
      } else {
        Rational {
          num: (*(*self.stream).codecpar).width
            * (*(*self.stream).codecpar).sample_aspect_ratio.num,
          den: (*(*self.stream).codecpar).height
            * (*(*self.stream).codecpar).sample_aspect_ratio.den,
        }
        .reduce()
      }
    }
  }

  pub fn get_bit_rate(&self) -> Option<i64> {
    unsafe {
      if (*(*self.stream).codecpar).bit_rate == AV_NOPTS_VALUE
        || (*(*self.stream).codecpar).bit_rate == 0
      {
        None
      } else {
        Some((*(*self.stream).codecpar).bit_rate)
      }
    }
  }

  pub fn get_frame_rate(&self) -> Rational {
    unsafe {
      Rational {
        num: (*self.stream).r_frame_rate.num,
        den: (*self.stream).r_frame_rate.den,
      }
    }
  }

  pub fn get_level(&self) -> Option<i32> {
    unsafe {
      let level = (*(*self.stream).codecpar).level;
      if level == FF_LEVEL_UNKNOWN {
        None
      } else {
        Some(level)
      }
    }
  }

  pub fn get_profile(&self) -> Option<String> {
    unsafe {
      let profile = (*(*self.stream).codecpar).profile;
      if profile == FF_PROFILE_UNKNOWN {
        None
      } else {
        Some(tools::to_string(avcodec_profile_name(
          (*(*self.stream).codecpar).codec_id,
          profile,
        )))
      }
    }
  }

  pub fn get_scanning_type(&self) -> Option<String> {
    unsafe {
      match (*(*self.stream).codecpar).field_order {
        AVFieldOrder::AV_FIELD_PROGRESSIVE => Some("progressive".to_string()),
        AVFieldOrder::AV_FIELD_TT
        | AVFieldOrder::AV_FIELD_BB
        | AVFieldOrder::AV_FIELD_TB
        | AVFieldOrder::AV_FIELD_BT => Some("interlaced".to_string()),
        _ => None,
      }
    }
  }

  pub fn get_chroma_sub_sample(&self) -> Option<String> {
    unsafe {
      let hshift = &mut 0;
      let vshift = &mut 0;
      let pixel_format: AVPixelFormat = std::mem::transmute((*(*self.stream).codecpar).format);
      if pixel_format == AVPixelFormat::AV_PIX_FMT_NONE {
        return None;
      }
      av_pix_fmt_get_chroma_sub_sample(pixel_format, hshift, vshift);
      match (hshift, vshift) {
        (0, 0) => Some("4:4:4".to_string()),
        (1, 0) => Some("4:2:2".to_string()),
        (1, 1) => Some("4:2:0".to_string()),
        (2, 0) => Some("4:1:1".to_string()),
        (_, _) => Some(tools::to_string(av_get_pix_fmt_name(pixel_format))),
      }
    }
  }

  pub fn get_pix_fmt_name(&self) -> Option<String> {
    unsafe {
      let pixel_format: AVPixelFormat = std::mem::transmute((*(*self.stream).codecpar).format);
      if pixel_format == AVPixelFormat::AV_PIX_FMT_NONE {
        return None;
      }
      let input_fmt_str = av_get_pix_fmt_name(pixel_format);
      Some(tools::to_string(input_fmt_str))
    }
  }

  pub fn get_bits_per_sample(&self) -> i32 {
    unsafe { av_get_bits_per_sample((*(*self.stream).codecpar).codec_id) }
  }

  pub fn get_sample_fmt(&self) -> String {
    unsafe {
      let pixel_format: AVSampleFormat = std::mem::transmute((*(*self.stream).codecpar).format);
      tools::to_string(av_get_sample_fmt_name(pixel_format))
    }
  }

  pub fn get_sample_rate(&self) -> i32 {
    unsafe { (*(*self.stream).codecpar).sample_rate }
  }

  pub fn get_channels(&self) -> i32 {
    unsafe { (*(*self.stream).codecpar).channels }
  }

  #[cfg(any(ffmpeg_4_4, ffmpeg_5_0, ffmpeg_5_1))]
  pub fn get_timecode(&self) -> Option<String> {
    unsafe {
      let timecode_side_data = av_stream_get_side_data(
        self.stream,
        AVPacketSideDataType::AV_PKT_DATA_S12M_TIMECODE,
        null_mut(),
      );

      let timecode = &mut 0;
      if timecode_side_data.is_null() {
        return None;
      }
      av_timecode_make_mpeg_tc_string(timecode, timecode_side_data as u32);
      Some(timecode.to_string())
    }
  }

  #[cfg(any(ffmpeg_4_0, ffmpeg_4_1, ffmpeg_4_2, ffmpeg_4_3))]
  pub fn get_timecode(&self) -> Option<String> {
    unsafe {
      let tc = &mut 0;
      let timecode = (*(*self.stream).codec).timecode_frame_start;
      if timecode < 0 {
        return None;
      }

      av_timecode_make_mpeg_tc_string(tc, timecode as u32);
      Some(tc.to_string())
    }
  }

  pub fn get_bits_per_raw_sample(&self) -> Option<i32> {
    unsafe {
      if (*(*self.stream).codecpar).bits_per_raw_sample == 0 {
        None
      } else {
        Some((*(*self.stream).codecpar).bits_per_raw_sample)
      }
    }
  }

  pub fn get_stream_metadata(&self) -> HashMap<String, String> {
    unsafe {
      let mut tag = null_mut();
      let key = CString::new("").unwrap();
      let mut metadata = HashMap::new();

      loop {
        tag = av_dict_get(
          (*self.stream).metadata,
          key.as_ptr(),
          tag,
          AV_DICT_IGNORE_SUFFIX,
        );
        if tag.is_null() {
          break;
        }
        let k = tools::to_string((*tag).key);
        let v = tools::to_string((*tag).value);
        metadata.insert(k.to_string(), v.to_string());
      }

      metadata
    }
  }

  pub fn get_color_range(&self) -> Option<String> {
    unsafe {
      if (*(*self.stream).codecpar).color_range == AVColorRange::AVCOL_RANGE_UNSPECIFIED {
        None
      } else {
        let range = av_color_range_name((*(*self.stream).codecpar).color_range);
        if tools::to_string(range) == "tv" {
          Some("tv (limited)".to_string())
        } else if tools::to_string(range) == "pc" {
          Some("pc (full)".to_string())
        } else {
          Some(tools::to_string(range))
        }
      }
    }
  }

  pub fn get_color_matrix(&self) -> Option<String> {
    unsafe {
      if (*(*self.stream).codecpar).color_space == AVColorSpace::AVCOL_SPC_UNSPECIFIED {
        None
      } else {
        let matrix = av_color_space_name((*(*self.stream).codecpar).color_space);
        Some(tools::to_string(matrix))
      }
    }
  }

  pub fn get_color_trc(&self) -> Option<String> {
    unsafe {
      if (*(*self.stream).codecpar).color_trc
        == AVColorTransferCharacteristic::AVCOL_TRC_UNSPECIFIED
      {
        None
      } else {
        let trc = av_color_transfer_name((*(*self.stream).codecpar).color_trc);
        Some(tools::to_string(trc))
      }
    }
  }

  pub fn get_color_primaries(&self) -> Option<String> {
    unsafe {
      if (*(*self.stream).codecpar).color_primaries == AVColorPrimaries::AVCOL_PRI_UNSPECIFIED {
        None
      } else {
        let primaries = av_color_primaries_name((*(*self.stream).codecpar).color_primaries);
        Some(tools::to_string(primaries))
      }
    }
  }

  pub fn get_color_space(&self) -> Option<String> {
    unsafe {
      let pixel_format: AVPixelFormat = std::mem::transmute((*(*self.stream).codecpar).format);
      let av_pix_fmt_desc = av_pix_fmt_desc_get(pixel_format);
      if av_pix_fmt_desc.is_null() {
        None
      } else {
        let cxyz = CString::new("xyz").unwrap();
        let xyzptr = cxyz.as_ptr();

        if (*av_pix_fmt_desc).flags as i32 & AV_PIX_FMT_FLAG_PAL > 0
          || (*av_pix_fmt_desc).flags as i32 & AV_PIX_FMT_FLAG_RGB > 0
        {
          Some("RGB".to_string())
        } else if (*av_pix_fmt_desc).nb_components == 1 || (*av_pix_fmt_desc).nb_components == 2 {
          Some("GRAY".to_string())
        } else if !(*av_pix_fmt_desc).name.is_null()
          && strncmp((*av_pix_fmt_desc).name, xyzptr, 3) == 0
        {
          Some("XYZ".to_string())
        } else if (*av_pix_fmt_desc).nb_components == 0 {
          Some("N/A".to_string())
        } else {
          Some("YUV".to_string())
        }
      }
    }
  }
}
