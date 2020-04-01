use stainless_ffmpeg_sys::*;
use std::ffi::CString;
use std::ptr::null_mut;

use crate::format_context::FormatContext;
use crate::frame::Frame;
use crate::packet::Packet;
use crate::tools;

#[derive(Debug)]
pub struct VideoDecoder {
  pub identifier: String,
  pub stream_index: isize,
  pub codec_context: *mut AVCodecContext,
}

impl VideoDecoder {
  pub fn new(
    identifier: String,
    format: &FormatContext,
    stream_index: isize,
  ) -> Result<Self, String> {
    unsafe {
      let codec = avcodec_find_decoder(format.get_codec_id(stream_index));
      let mut codec_context = avcodec_alloc_context3(codec);

      check_result!(
        avcodec_parameters_to_context(
          codec_context,
          (**(*format.format_context).streams.offset(stream_index)).codecpar
        ),
        {
          avcodec_free_context(&mut codec_context);
        }
      );
      check_result!(avcodec_open2(codec_context, codec, null_mut()), {
        avcodec_free_context(&mut codec_context);
      });

      Ok(VideoDecoder {
        identifier,
        stream_index,
        codec_context,
      })
    }
  }

  pub fn new_with_codec(
    identifier: String,
    codec_name: &str,
    width: i32,
    height: i32,
    stream_index: isize,
  ) -> Result<Self, String> {
    unsafe {
      let cn = CString::new(codec_name).unwrap();
      let codec = avcodec_find_decoder_by_name(cn.as_ptr());
      let mut codec_context = avcodec_alloc_context3(codec);

      (*codec_context).width = width;
      (*codec_context).height = height;
      check_result!(avcodec_open2(codec_context, codec, null_mut()), {
        avcodec_free_context(&mut codec_context);
      });

      Ok(VideoDecoder {
        identifier,
        stream_index,
        codec_context,
      })
    }
  }

  pub fn get_width(&self) -> i32 {
    unsafe { (*self.codec_context).width }
  }

  pub fn get_height(&self) -> i32 {
    unsafe { (*self.codec_context).height }
  }

  pub fn get_time_base(&self) -> (i32, i32) {
    unsafe {
      (
        (*self.codec_context).time_base.num,
        (*self.codec_context).time_base.den,
      )
    }
  }

  pub fn get_aspect_ratio(&self) -> (i32, i32) {
    unsafe {
      (
        (*self.codec_context).sample_aspect_ratio.num,
        (*self.codec_context).sample_aspect_ratio.den,
      )
    }
  }

  pub fn get_pix_fmt_name(&self) -> String {
    unsafe {
      let input_fmt_str = av_get_pix_fmt_name((*self.codec_context).pix_fmt);
      tools::to_string(input_fmt_str)
    }
  }

  pub fn decode(&self, packet: &Packet) -> Result<Frame, String> {
    if packet.get_stream_index() != self.stream_index {
      return Err("bad stream".to_string());
    }
    unsafe {
      check_result!(avcodec_send_packet(self.codec_context, packet.packet));

      let frame = av_frame_alloc();

      check_result!(avcodec_receive_frame(self.codec_context, frame));

      Ok(Frame {
        frame,
        name: Some(self.identifier.clone()),
      })
    }
  }
}

impl Drop for VideoDecoder {
  fn drop(&mut self) {
    unsafe {
      if !self.codec_context.is_null() {
        avcodec_close(self.codec_context);
        avcodec_free_context(&mut self.codec_context);
      }
    }
  }
}
