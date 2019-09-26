use stainless_ffmpeg_sys::*;
use std::char;
use std::collections::HashMap;
use std::ffi::CString;
use std::ptr::null_mut;
use tools;
use tools::rational::Rational;

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

  fn get_time_base(&self) -> f32 {
    unsafe { (*self.stream).time_base.num as f32 / (*self.stream).time_base.den as f32 }
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
          long_name.push_str(" ");
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
          codec_tag_str.push_str(&c.to_string());
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

  pub fn get_duration(&self) -> Option<f32> {
    unsafe {
      if (*self.stream).duration == AV_NOPTS_VALUE {
        None
      } else {
        Some((*self.stream).duration as f32 * self.get_time_base())
      }
    }
  }

  pub fn get_start_time(&self) -> Option<f32> {
    unsafe {
      if (*self.stream).start_time == AV_NOPTS_VALUE {
        None
      } else {
        Some((*self.stream).start_time as f32 * self.get_time_base())
      }
    }
  }

  pub fn get_width(&self) -> i32 {
    unsafe { (*(*self.stream).codec).width }
  }

  pub fn get_height(&self) -> i32 {
    unsafe { (*(*self.stream).codec).height }
  }

  pub fn get_display_aspect_ratio(&self) -> Rational {
    unsafe {
      if (*self.stream).display_aspect_ratio.den == 0 {
        if (*(*self.stream).codecpar).sample_aspect_ratio.num == 0 {
          Rational {
            num: (*(*self.stream).codecpar).width * (*self.stream).sample_aspect_ratio.num,
            den: (*(*self.stream).codecpar).height * (*self.stream).sample_aspect_ratio.den,
          }
          .reduce()
        } else {
          Rational {
            num: (*(*self.stream).codecpar).width
              * (*(*self.stream).codecpar).sample_aspect_ratio.num,
            den: (*(*self.stream).codecpar).height
              * (*(*self.stream).codecpar).sample_aspect_ratio.den,
          }
          .reduce()
        }
      } else {
        Rational {
          num: (*self.stream).display_aspect_ratio.num,
          den: (*self.stream).display_aspect_ratio.den,
        }
      }
    }
  }

  pub fn get_bit_rate(&self) -> Option<i64> {
    unsafe {
      if (*(*self.stream).codec).bit_rate == AV_NOPTS_VALUE {
        None
      } else {
        Some((*(*self.stream).codec).bit_rate)
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
      let level = (*(*self.stream).codec).level;
      if level == FF_LEVEL_UNKNOWN {
        None
      } else {
        Some(level)
      }
    }
  }

  pub fn get_profile(&self) -> Option<String> {
    unsafe {
      let profile = (*(*self.stream).codec).profile;
      if profile == FF_PROFILE_UNKNOWN {
        None
      } else {
        Some(tools::to_string(avcodec_profile_name(
          (*(*self.stream).codec).codec_id,
          profile,
        )))
      }
    }
  }

  pub fn get_scanning_type(&self) -> Option<String> {
    unsafe {
      match (*(*self.stream).codec).field_order {
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
      let pix_fmt = (*(*self.stream).codec).pix_fmt;
      if pix_fmt == AVPixelFormat::AV_PIX_FMT_NONE {
        return None;
      }
      avcodec_get_chroma_sub_sample(pix_fmt, hshift, vshift);
      match (hshift, vshift) {
        (0, 0) => Some("4:4:4".to_string()),
        (1, 0) => Some("4:2:2".to_string()),
        (1, 1) => Some("4:2:0".to_string()),
        (2, 0) => Some("4:1:1".to_string()),
        (_, _) => Some(tools::to_string(av_get_pix_fmt_name(pix_fmt))),
      }
    }
  }

  pub fn get_pix_fmt_name(&self) -> Option<String> {
    unsafe {
      if (*(*self.stream).codec).pix_fmt == AVPixelFormat::AV_PIX_FMT_NONE {
        return None;
      }
      let input_fmt_str = av_get_pix_fmt_name((*(*self.stream).codec).pix_fmt);
      Some(tools::to_string(input_fmt_str))
    }
  }

  pub fn get_bits_per_sample(&self) -> i32 {
    unsafe { av_get_bits_per_sample((*(*self.stream).codec).codec_id) }
  }

  pub fn get_sample_fmt(&self) -> String {
    unsafe { tools::to_string(av_get_sample_fmt_name((*(*self.stream).codec).sample_fmt)) }
  }

  pub fn get_sample_rate(&self) -> i32 {
    unsafe { (*(*self.stream).codec).sample_rate }
  }

  pub fn get_channels(&self) -> i32 {
    unsafe { (*(*self.stream).codec).channels }
  }

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
}
