use crate::frame::Frame;
use crate::order::output::{ColorRange, Colorspace, OutputStream, PixelFormat};
use crate::order::parameters::ParameterValue;
use crate::packet::Packet;
use crate::tools;
use stainless_ffmpeg_sys::*;
use std::ptr::null_mut;

#[derive(Debug)]
pub struct VideoEncoder {
  pub identifier: String,
  pub stream_index: isize,
  pub codec_context: *mut AVCodecContext,
  pub codec: *mut AVCodec,
  pub pts: i64,
}

impl VideoEncoder {
  pub fn new(
    identifier: String,
    stream_index: isize,
    output_stream: &OutputStream,
  ) -> Result<Self, String> {
    unsafe {
      let codec = tools::get_codec(&output_stream.codec);
      if codec.is_null() {
        return Err(format!("Unable to found codec {}", output_stream.codec));
      }
      let mut codec_context = avcodec_alloc_context3(codec);

      let parameters = &output_stream.parameters;
      if let Some(ParameterValue::Rational(data)) = parameters.get("frame_rate") {
        (*codec_context).time_base = data.clone().invert().into();
      }

      if let Some(ParameterValue::Rational(data)) = parameters.get("sample_aspect_ratio") {
        (*codec_context).sample_aspect_ratio = data.clone().into();
      }

      if let Some(ParameterValue::String(data)) = parameters.get("pixel_format") {
        let format: PixelFormat = data.parse().unwrap();
        (*codec_context).pix_fmt = format.into();
      }

      if let Some(ParameterValue::Int64(data)) = parameters.get("width") {
        (*codec_context).width = *data as i32;
      }

      if let Some(ParameterValue::Int64(data)) = parameters.get("height") {
        (*codec_context).height = *data as i32;
      }

      if let Some(ParameterValue::Int64(data)) = parameters.get("bitrate") {
        (*codec_context).bit_rate = *data;
      }

      if let Some(ParameterValue::Int64(data)) = parameters.get("gop_size") {
        (*codec_context).gop_size = *data as i32;
      }

      if let Some(ParameterValue::Int64(data)) = parameters.get("max_b_frames") {
        (*codec_context).max_b_frames = *data as i32;
      }

      if let Some(ParameterValue::Int64(data)) = parameters.get("refs") {
        (*codec_context).refs = *data as i32;
      }

      if let Some(ParameterValue::Int64(data)) = parameters.get("keyint_min") {
        (*codec_context).keyint_min = *data as i32;
      }

      if let Some(ParameterValue::String(data)) = parameters.get("colorspace") {
        let colorspace: Colorspace = data.parse().unwrap();
        (*codec_context).colorspace = colorspace.into();
      }

      if let Some(ParameterValue::String(data)) = parameters.get("color_range") {
        let color_range: ColorRange = data.parse().unwrap();
        (*codec_context).color_range = color_range.into();
      }

      check_result!(avcodec_open2(codec_context, codec, null_mut()), {
        avcodec_free_context(&mut codec_context);
      });

      Ok(VideoEncoder {
        identifier,
        stream_index,
        codec_context,
        codec,
        pts: 0,
      })
    }
  }

  pub fn set_width(&self, width: i32) {
    unsafe {
      (*self.codec_context).width = width;
    }
  }

  pub fn set_height(&self, height: i32) {
    unsafe {
      (*self.codec_context).height = height;
    }
  }

  pub fn set_time_base(&self, num: i32, den: i32) {
    unsafe {
      (*self.codec_context).time_base.num = num;
      (*self.codec_context).time_base.den = den;
    }
  }

  pub fn get_aspect_ratio(&self, num: i32, den: i32) {
    unsafe {
      (*self.codec_context).sample_aspect_ratio.num = num;
      (*self.codec_context).sample_aspect_ratio.den = den;
    }
  }

  pub fn encode(&mut self, frame: &Frame, packet: &Packet) -> Result<bool, String> {
    unsafe {
      (*frame.frame).pts = self.pts;
      self.pts += 1;

      check_result!(avcodec_send_frame(self.codec_context, frame.frame));
      let ret = avcodec_receive_packet(self.codec_context, packet.packet as *mut _);

      if ret == AVERROR(EAGAIN) || ret == AVERROR_EOF {
        let mut data = [0i8; AV_ERROR_MAX_STRING_SIZE];
        av_strerror(
          ret,
          data.as_mut_ptr() as *mut libc::c_char,
          AV_ERROR_MAX_STRING_SIZE as u64,
        );
        trace!("{}", tools::to_string(data.as_ptr() as *const libc::c_char));
        return Ok(false);
      }

      check_result!(ret);

      trace!(
        "received encoded packet with {} bytes",
        (*packet.packet).size
      );
      Ok(true)
    }
  }
}

impl Drop for VideoEncoder {
  fn drop(&mut self) {
    unsafe {
      if !self.codec_context.is_null() {
        avcodec_close(self.codec_context);
        avcodec_free_context(&mut self.codec_context);
      }
    }
  }
}
