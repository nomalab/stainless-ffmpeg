use stainless_ffmpeg_sys::*;
use frame::Frame;
use order::output::{ChannelLayout, OutputStream, SampleFormat};
use order::parameters::ParameterValue;
use packet::Packet;
use std::collections::HashMap;
use std::ptr::null_mut;
use tools;
use std::cmp::min;

#[derive(Debug)]
pub struct AudioEncoder {
  pub identifier: String,
  pub stream_index: isize,
  pub codec_context: *mut AVCodecContext,
  pub codec: *mut AVCodec,
  pub fifo: *mut AVAudioFifo,
  pub pts: i64,
  pub duration: Option<f64>,
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
      let mut duration = None;
      let parameters = &output_stream.parameters;
      if let Some(ParameterValue::Rational(data)) = parameters.get("sample_rate") {
        (*codec_context).time_base = data.clone().invert().into();
        (*codec_context).sample_rate = data.num / data.den;
      }

      if let Some(ParameterValue::String(data)) = parameters.get("sample_fmt") {
        let sample_fmt: SampleFormat = data.parse().unwrap();
        (*codec_context).sample_fmt = sample_fmt.into();
      }

      if let Some(ParameterValue::Float(data)) = parameters.get("duration") {
        duration = Some(*data);
      }
      (*codec_context).channel_layout = AudioEncoder::select_channel_layout(codec, &parameters);
      (*codec_context).channels =
        av_get_channel_layout_nb_channels((*codec_context).channel_layout);

      check_result!(avcodec_open2(codec_context, codec, null_mut()), {
        avcodec_free_context(&mut codec_context);
      });

      let fifo = av_audio_fifo_alloc((*codec_context).sample_fmt, (*codec_context).channels, 1920*4*2);

      Ok(AudioEncoder {
        identifier,
        stream_index,
        codec_context,
        codec,
        fifo,
        pts: 0,
        duration,
      })
    }
  }

  pub fn encode(&mut self, frame: &Frame, packet: &Packet) -> Result<bool, String> {
    unsafe {
      if !frame.frame.is_null() {
        let source_frame_size = (*frame.frame).nb_samples;
        av_audio_fifo_realloc(self.fifo as *mut _, av_audio_fifo_size(self.fifo) + source_frame_size);
        av_audio_fifo_write(self.fifo as *mut _, (*frame.frame).extended_data as *mut *mut _, source_frame_size);
      }

      let frame_size =
        if (*self.codec_context).frame_size > 0 {
          min(av_audio_fifo_size(self.fifo), (*self.codec_context).frame_size)
        } else {
          av_audio_fifo_size(self.fifo)
        };

      let mut adapted_frame = av_frame_alloc();
      (*adapted_frame).nb_samples     = frame_size;
      (*adapted_frame).channel_layout = (*self.codec_context).channel_layout;
      (*adapted_frame).format         = (*self.codec_context).sample_fmt as i32;
      (*adapted_frame).sample_rate    = (*self.codec_context).sample_rate;
      (*adapted_frame).pts            = self.pts;

      check_result!(av_frame_get_buffer(adapted_frame as *mut _, 0));

      let ptr = (*adapted_frame).data.as_mut_ptr();

      av_audio_fifo_read(self.fifo, ptr as *mut *mut _, frame_size);
      check_result!(avcodec_send_frame(self.codec_context, adapted_frame));

      let ret = avcodec_receive_packet(self.codec_context, packet.packet as *mut _);

      av_frame_free(&mut adapted_frame as *mut *mut _);

      if ret == AVERROR(EAGAIN) || ret == AVERROR_EOF {
        let mut data = [0i8; AV_ERROR_MAX_STRING_SIZE as usize];
        av_strerror(ret, data.as_mut_ptr(), AV_ERROR_MAX_STRING_SIZE);
        trace!("{}", tools::to_string(data.as_ptr()));
        return Ok(false);
      }

      check_result!(ret);
      self.pts += frame_size as i64;

      info!(
        "received encoded packet with {} bytes",
        (*packet.packet).size
      );

      Ok(true)
    }
  }

  fn select_channel_layout(codec: *mut AVCodec, parameters: &HashMap<String, ParameterValue>) -> u64 {
    unsafe {
      if codec.is_null() || (*codec).channel_layouts.is_null() {
        if let Some(ParameterValue::String(data)) = parameters.get("channel_layout") {
          let layout: ChannelLayout = data.parse().unwrap();
          layout.clone().into()
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
