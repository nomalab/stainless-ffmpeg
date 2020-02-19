use crate::format_context::FormatContext;
use crate::tools;
use stainless_ffmpeg_sys::*;

#[derive(Debug)]
pub struct SubtitleDecoder {
  pub identifier: String,
  pub stream_index: isize,
  pub codec_context: *mut AVCodecContext,
}

impl SubtitleDecoder {
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

      Ok(SubtitleDecoder {
        identifier,
        stream_index,
        codec_context,
      })
    }
  }
}

impl Drop for SubtitleDecoder {
  fn drop(&mut self) {
    unsafe {
      if !self.codec_context.is_null() {
        avcodec_close(self.codec_context);
        avcodec_free_context(&mut self.codec_context);
      }
    }
  }
}
