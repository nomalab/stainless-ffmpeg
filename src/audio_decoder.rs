use crate::{format_context::FormatContext, frame::Frame, packet::Packet, tools};
use ffmpeg_sys_next::*;
use std::ptr::null_mut;

#[derive(Debug)]
pub struct AudioDecoder {
  pub identifier: String,
  pub stream_index: isize,
  pub codec_context: *mut AVCodecContext,
}

impl AudioDecoder {
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

      Ok(AudioDecoder {
        identifier,
        stream_index,
        codec_context,
      })
    }
  }

  pub fn get_sample_rate(&self) -> i32 {
    unsafe { (*self.codec_context).sample_rate }
  }

  pub fn get_nb_channels(&self) -> i32 {
    unsafe { (*self.codec_context).channels }
  }

  pub fn get_channel_layout(&self) -> u64 {
    unsafe { (*self.codec_context).channel_layout }
  }

  pub fn get_sample_fmt_name(&self) -> String {
    unsafe {
      let input_fmt_str = av_get_sample_fmt_name((*self.codec_context).sample_fmt);
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
        index: self.stream_index as usize,
      })
    }
  }
}

impl Drop for AudioDecoder {
  fn drop(&mut self) {
    unsafe {
      if !self.codec_context.is_null() {
        avcodec_close(self.codec_context);
        avcodec_free_context(&mut self.codec_context);
      }
    }
  }
}
