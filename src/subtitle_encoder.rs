use crate::{
  order::{output::OutputStream, parameters::ParameterValue},
  tools,
};
use ffmpeg_sys_next::*;

#[derive(Debug)]
pub struct SubtitleEncoder {
  pub identifier: String,
  pub stream_index: isize,
  pub codec_context: *mut AVCodecContext,
}

impl SubtitleEncoder {
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
      let codec_context = avcodec_alloc_context3(codec);

      if let Some(ParameterValue::Rational(data)) = output_stream.parameters.get("frame_rate") {
        (*codec_context).time_base = data.clone().invert().into();
      }

      Ok(SubtitleEncoder {
        identifier,
        stream_index,
        codec_context,
      })
    }
  }
}

impl Drop for SubtitleEncoder {
  fn drop(&mut self) {
    unsafe {
      if !self.codec_context.is_null() {
        avcodec_close(self.codec_context);
        avcodec_free_context(&mut self.codec_context);
      }
    }
  }
}
