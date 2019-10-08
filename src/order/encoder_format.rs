use audio_encoder::AudioEncoder;
use stainless_ffmpeg_sys::*;
use filter_graph::FilterGraph;
use format_context::FormatContext;
use frame::Frame;
use order::output::Output;
use order::output_kind::OutputKind;
use packet::Packet;
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
  wrap: bool
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
      let file_path = CString::new(path).unwrap();
      av_dump_format(format.format_context, 0, file_path.as_ptr(), 1);

      check_result!(avio_open(
        &mut (*format.format_context).pb as *mut _,
        file_path.as_ptr(),
        AVIO_FLAG_WRITE
      ));
      av_dump_format(format.format_context, 0, file_path.as_ptr(), 1);
      check_result!(avformat_write_header(format.format_context, null_mut()));
      av_dump_format(format.format_context, 0, file_path.as_ptr(), 1);

    }

    Ok(EncoderFormat {
      context: format,
      audio_encoders,
      subtitle_encoders,
      video_encoders,
      wrap: output.kind == Some(OutputKind::File)
    })
  }

  pub fn wrap(&mut self, packet: &Packet) -> Result<(), String> {
    for subtitle_encoder in &self.subtitle_encoders {
      if let Some(ref name) = packet.name {
        if subtitle_encoder.identifier == *name {
          unsafe {
            (*packet.packet).stream_index = subtitle_encoder.stream_index as i32;
            check_result!(av_interleaved_write_frame(self.context.format_context, packet.packet));
          }
        }
      }
    }

    Ok(())
  }

  pub fn check_recording_time(codec_context: *mut AVCodecContext, duration: f64, encoded_frames: usize) -> bool {
    unsafe {
      let time_base = AVRational { num: 1, den: 1000 };
      if av_compare_ts(encoded_frames as i64, (*codec_context).time_base,
                      (duration * 1000 as f64) as i64, time_base) >= 0 {
        return false;
      }
      return true;
    }
  }

  pub fn encode(&mut self, frame: &Frame, encoded_frames: usize) -> Result<(Option<Packet>, bool), String> {
    let mut r_packet = None;
    for audio_encoder in &mut self.audio_encoders {
      if let Some(ref name) = frame.name {
        if audio_encoder.identifier == *name {
          if audio_encoder.duration.is_some() && !Self::check_recording_time(audio_encoder.codec_context, audio_encoder.duration.unwrap(), encoded_frames) {
            return Ok((r_packet, true));
          }

          unsafe {
            let packet = Packet::new();

            if audio_encoder.encode(frame, &packet)? {
              if self.wrap {
                (*packet.packet).stream_index = audio_encoder.stream_index as i32;
                let stream_timebase = (*self.context.get_stream(audio_encoder.stream_index)).time_base;
                let framerate = (*self.context.get_stream(audio_encoder.stream_index)).r_frame_rate;

                av_packet_rescale_ts(&mut (*packet.packet) as *mut _, av_inv_q(framerate), stream_timebase);

                (*packet.packet).duration = av_rescale_q(1, av_inv_q(framerate), stream_timebase);
                check_result!(av_interleaved_write_frame(self.context.format_context, packet.packet));
              } else {
                r_packet = Some(packet);
              }
            }
          }
        }
      }
    }
    for video_encoder in &mut self.video_encoders {
      if let Some(ref name) = frame.name {
        if video_encoder.identifier == *name {
          if video_encoder.duration.is_some() && !Self::check_recording_time(video_encoder.codec_context, video_encoder.duration.unwrap(), encoded_frames) {
            return Ok((r_packet, true));
          }
          unsafe {
            let packet = Packet::new();
            if video_encoder.encode(frame, &packet)? {
              if self.wrap {
                (*packet.packet).stream_index = video_encoder.stream_index as i32;
                let stream_timebase = (*self.context.get_stream(video_encoder.stream_index)).time_base;
                let framerate = (*self.context.get_stream(video_encoder.stream_index)).r_frame_rate;

                av_packet_rescale_ts(&mut (*packet.packet) as *mut _, av_inv_q(framerate), stream_timebase);

                (*packet.packet).duration = av_rescale_q(1, av_inv_q(framerate), stream_timebase);
                check_result!(av_interleaved_write_frame(self.context.format_context, packet.packet));
              } else {
                r_packet = Some(packet);
              }
            }
          }
        }
      }
    }

    Ok((r_packet, false))
  }

  pub fn finish(&mut self) -> Result<Vec<Packet>, String> {
    for video_encoder in &mut self.video_encoders {
      let mut first = true;

      loop {
        unsafe {
          let packet = Packet::new();
          let frame = Frame::new();

          let result =
            if first {
              video_encoder.encode(&frame, &packet)
            } else {
              video_encoder.flush(&packet)
            };

          if let Ok(true) = result {
            if self.wrap {
              (*packet.packet).stream_index = video_encoder.stream_index as i32;
              let stream_timebase = (*self.context.get_stream(video_encoder.stream_index)).time_base;
              let framerate = (*self.context.get_stream(video_encoder.stream_index)).r_frame_rate;

              av_packet_rescale_ts(&mut (*packet.packet) as *mut _, av_inv_q(framerate), stream_timebase);

              (*packet.packet).duration = av_rescale_q(1, av_inv_q(framerate), stream_timebase);
              av_interleaved_write_frame(self.context.format_context, packet.packet);
              first = false;
            }
          } else {
            break;
          }
        }
      }
    }

    for audio_encoder in &mut self.audio_encoders {
      loop {
        unsafe {
          let packet = Packet::new();
          let frame = Frame::new();
          if let Ok(true) = audio_encoder.encode(&frame, &packet) {
            if self.wrap {
              (*packet.packet).stream_index = audio_encoder.stream_index as i32;
              let stream_timebase = (*self.context.get_stream(audio_encoder.stream_index)).time_base;
              let framerate = (*self.context.get_stream(audio_encoder.stream_index)).r_frame_rate;

              av_packet_rescale_ts(&mut (*packet.packet) as *mut _, av_inv_q(framerate), stream_timebase);

              (*packet.packet).duration = av_rescale_q(1, av_inv_q(framerate), stream_timebase);
              av_interleaved_write_frame(self.context.format_context, packet.packet);
            }
          } else {
            break;
          }
        }
      }
    }

    unsafe {
      av_write_trailer(self.context.format_context);
    }
    Ok(vec![])
  }
}
