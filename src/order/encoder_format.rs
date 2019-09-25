use audio_encoder::AudioEncoder;
use filter_graph::FilterGraph;
use format_context::FormatContext;
use frame::Frame;
use order::output::Output;
use order::output_kind::OutputKind;
use packet::Packet;
use stainless_ffmpeg_sys::*;
use std::ffi::CString;
use std::ptr::null_mut;
use subtitle_encoder::SubtitleEncoder;
use tools;
use video_encoder::VideoEncoder;

#[derive(Debug)]
pub struct EncoderFormat {
  pub context: FormatContext,
  pub audio_encoders: Vec<AudioEncoder>,
  pub subtitle_encoders: Vec<SubtitleEncoder>,
  pub video_encoders: Vec<VideoEncoder>,
  wrap: bool,
}

impl Drop for EncoderFormat {
  fn drop(&mut self) {
    unsafe {
      av_write_trailer(self.context.format_context);
    }
  }
}

impl EncoderFormat {
  pub fn new(graph: &mut FilterGraph, output: &Output) -> Result<Self, String> {
    let mut audio_encoders = vec![];
    let mut subtitle_encoders = vec![];
    let mut video_encoders = vec![];
    if output.path.is_none() {
      return Err("missing output filename".to_owned());
    }

    let path = output.path.clone().unwrap();
    let mut format = FormatContext::new(&path)?;
    format.open_output(&output.parameters)?;

    for (index, stream) in output.streams.iter().enumerate() {
      let identifier = if let Some(ref identifier) = stream.label {
        identifier.clone()
      } else {
        tools::random_string(8)
      };

      match tools::get_codec_type(&stream.codec) {
        Some(AVMediaType::AVMEDIA_TYPE_VIDEO) => {
          let video_encoder = VideoEncoder::new(identifier.clone(), index as isize, &stream)?;
          format.add_video_stream(&video_encoder)?;
          video_encoders.push(video_encoder);
          graph.add_video_output(&identifier)?;
        }
        Some(AVMediaType::AVMEDIA_TYPE_AUDIO) => {
          let audio_encoder = AudioEncoder::new(identifier.clone(), index as isize, &stream)?;
          format.add_audio_stream(&audio_encoder)?;
          audio_encoders.push(audio_encoder);
          graph.add_audio_output(&identifier)?;
        }
        Some(AVMediaType::AVMEDIA_TYPE_SUBTITLE) => {
          let subtitle_encoder = SubtitleEncoder::new(identifier.clone(), index as isize, &stream)?;
          format.add_subtitle_stream(&subtitle_encoder)?;
          subtitle_encoders.push(subtitle_encoder);
        }
        _ => {}
      }
    }

    unsafe {
      let p = CString::new(path).unwrap();
      av_dump_format(format.format_context, 0, p.as_ptr(), 1);

      check_result!(avio_open(
        &mut (*format.format_context).pb as *mut _,
        p.as_ptr(),
        AVIO_FLAG_WRITE
      ));
      check_result!(avformat_write_header(format.format_context, null_mut()));
    }

    Ok(EncoderFormat {
      context: format,
      audio_encoders,
      subtitle_encoders,
      video_encoders,
      wrap: output.kind == Some(OutputKind::File),
    })
  }

  pub fn wrap(&mut self, packet: &Packet) -> Result<(), String> {
    for subtitle_encoder in &self.subtitle_encoders {
      if let Some(ref name) = packet.name {
        if subtitle_encoder.identifier == *name {
          unsafe {
            (*packet.packet).stream_index = subtitle_encoder.stream_index as i32;
            check_result!(av_interleaved_write_frame(
              self.context.format_context,
              packet.packet
            ));
          }
        }
      }
    }

    Ok(())
  }

  pub fn encode(&mut self, frame: &Frame) -> Result<Option<Packet>, String> {
    let mut r_packet = None;
    for audio_encoder in &self.audio_encoders {
      if let Some(ref name) = frame.name {
        if audio_encoder.identifier == *name {
          unsafe {
            let packet = av_packet_alloc();
            av_init_packet(packet);
            (*packet).data = null_mut();
            (*packet).size = 0;
            let p = Packet { name: None, packet };

            let status = audio_encoder.encode(frame, &p)?;

            if status {
              if self.wrap {
                (*packet).stream_index = audio_encoder.stream_index as i32;
                check_result!(av_interleaved_write_frame(
                  self.context.format_context,
                  packet
                ));
              } else {
                r_packet = Some(p);
              }
            }
          }
        }
      }
    }
    for video_encoder in &mut self.video_encoders {
      if let Some(ref name) = frame.name {
        if video_encoder.identifier == *name {
          unsafe {
            let packet = av_packet_alloc();
            av_init_packet(packet);
            (*packet).data = null_mut();
            (*packet).size = 0;
            let p = Packet { name: None, packet };

            let status = video_encoder.encode(frame, &p)?;
            if status {
              if self.wrap {
                (*packet).stream_index = video_encoder.stream_index as i32;
                check_result!(av_interleaved_write_frame(
                  self.context.format_context,
                  packet
                ));
              } else {
                r_packet = Some(p);
              }
            }
          }
        }
      }
    }

    Ok(r_packet)
  }
}
