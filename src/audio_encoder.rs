use crate::frame::Frame;
use crate::order::output::{ChannelLayout, OutputStream, SampleFormat};
use crate::order::parameters::ParameterValue;
use crate::packet::Packet;
use crate::tools;
use stainless_ffmpeg_sys::*;
use std::collections::HashMap;
use std::ptr::null_mut;

#[derive(Debug)]
pub struct AudioEncoder {
  pub identifier: String,
  pub stream_index: isize,
  pub codec_context: *mut AVCodecContext,
  pub codec: *mut AVCodec,
}

impl AudioEncoder {
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
      if let Some(ParameterValue::Rational(data)) = parameters.get("sample_rate") {
        (*codec_context).time_base = data.clone().invert().into();
        (*codec_context).sample_rate = data.num / data.den;
      }

      if let Some(ParameterValue::String(data)) = parameters.get("sample_fmt") {
        let sample_fmt: SampleFormat = data.parse().unwrap();
        (*codec_context).sample_fmt = sample_fmt.into();
      }

      (*codec_context).channel_layout = AudioEncoder::select_channel_layout(codec, &parameters);
      (*codec_context).channels =
        av_get_channel_layout_nb_channels((*codec_context).channel_layout);

      check_result!(avcodec_open2(codec_context, codec, null_mut()), {
        avcodec_free_context(&mut codec_context);
      });

      Ok(AudioEncoder {
        identifier,
        stream_index,
        codec_context,
        codec,
      })
    }
  }

  pub fn encode(&self, frame: &Frame, packet: &Packet) -> Result<bool, String> {
    unsafe {
      check_result!(avcodec_send_frame(self.codec_context, frame.frame));
      let ret = avcodec_receive_packet(self.codec_context, packet.packet as *mut _);

      if ret == AVERROR(EAGAIN) || ret == AVERROR_EOF {
        let mut data = [0i8; AV_ERROR_MAX_STRING_SIZE];
        av_strerror(ret, data.as_mut_ptr(), AV_ERROR_MAX_STRING_SIZE as u64);
        trace!("{}", tools::to_string(data.as_ptr()));
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

  fn select_channel_layout(
    codec: *mut AVCodec,
    parameters: &HashMap<String, ParameterValue>,
  ) -> u64 {
    unsafe {
      if codec.is_null() || (*codec).channel_layouts.is_null() {
        if let Some(ParameterValue::String(data)) = parameters.get("channel_layout") {
          let layout: ChannelLayout = data.parse().unwrap();
          layout.into()
        } else {
          AV_CH_LAYOUT_STEREO
        }
      } else {
        AV_CH_LAYOUT_STEREO
      }
    }
  }
}

impl Drop for AudioEncoder {
  fn drop(&mut self) {
    unsafe {
      if !self.codec_context.is_null() {
        avcodec_close(self.codec_context);
        avcodec_free_context(&mut self.codec_context);
      }
    }
  }
}
