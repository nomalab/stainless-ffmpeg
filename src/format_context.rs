use crate::audio_encoder::AudioEncoder;
use crate::order::frame::FrameAddress;
use crate::order::*;
use crate::packet::Packet;
use crate::subtitle_encoder::SubtitleEncoder;
use crate::tools;
use crate::video_encoder::VideoEncoder;
use stainless_ffmpeg_sys::*;
use std::collections::{BTreeMap, HashMap};
use std::ffi::CString;
use std::ptr::null_mut;

use std::ffi::c_void;

#[derive(Debug)]
pub struct FormatContext {
  pub filename: String,
  pub format_context: *mut AVFormatContext,
  streams: Vec<*mut AVStream>,
  frames: Vec<FrameAddress>,
  frame_index: usize,
}

impl FormatContext {
  pub fn new(filename: &str) -> Result<FormatContext, String> {
    Ok(FormatContext {
      filename: filename.to_string(),
      format_context: null_mut(),
      streams: vec![],
      frames: vec![],
      frame_index: 0,
    })
  }

  pub fn set_frames_addresses(&mut self, frames: &[FrameAddress]) {
    self.frames = frames.to_vec();
  }

  pub fn open_input(&mut self) -> Result<(), String> {
    unsafe {
      self.format_context = avformat_alloc_context();
      let filename = CString::new(self.filename.to_owned()).unwrap();
      if avformat_open_input(
        &mut self.format_context,
        filename.as_ptr(),
        null_mut(),
        null_mut(),
      ) < 0
      {
        return Err(format!("Unable to open input file {:?}", self.filename));
      }
      avformat_find_stream_info(self.format_context, null_mut());
    }
    Ok(())
  }

  pub fn close_input(&mut self) {
    unsafe {
      avformat_close_input(&mut self.format_context);
    }
  }

  pub fn open_output(
    &mut self,
    parameters: &HashMap<String, ParameterValue>,
  ) -> Result<(), String> {
    unsafe {
      let filename = CString::new(self.filename.to_owned()).unwrap();

      if avformat_alloc_output_context2(
        &mut self.format_context,
        null_mut(),
        null_mut(),
        filename.as_ptr(),
      ) < 0
      {
        return Err(format!("Unable to open output file {:?}", self.filename));
      }

      set_parameters(self.format_context as *mut c_void, parameters)?;
    }
    Ok(())
  }

  pub fn add_video_stream(&mut self, encoder: &VideoEncoder) -> Result<(), String> {
    unsafe {
      let av_stream = avformat_new_stream(self.format_context, null_mut());
      if av_stream.is_null() {
        return Err("Unable to create new stream".to_owned());
      }

      (*av_stream).id = ((*self.format_context).nb_streams - 1) as i32;
      (*av_stream).time_base = (*encoder.codec_context).time_base;
      avcodec_parameters_from_context((*av_stream).codecpar, encoder.codec_context);
      self.streams.push(av_stream);
    }
    Ok(())
  }

  pub fn add_audio_stream(&mut self, encoder: &AudioEncoder) -> Result<(), String> {
    unsafe {
      let av_stream = avformat_new_stream(self.format_context, null_mut());
      if av_stream.is_null() {
        return Err("Unable to create new stream".to_owned());
      }

      (*av_stream).id = ((*self.format_context).nb_streams - 1) as i32;
      (*av_stream).time_base = (*encoder.codec_context).time_base;
      avcodec_parameters_from_context((*av_stream).codecpar, encoder.codec_context);
      self.streams.push(av_stream);
    }
    Ok(())
  }

  pub fn add_subtitle_stream(&mut self, encoder: &SubtitleEncoder) -> Result<(), String> {
    unsafe {
      let av_stream = avformat_new_stream(self.format_context, null_mut());
      if av_stream.is_null() {
        return Err("Unable to create new stream".to_owned());
      }

      (*av_stream).id = ((*self.format_context).nb_streams - 1) as i32;
      (*av_stream).time_base = (*encoder.codec_context).time_base;
      avcodec_parameters_from_context((*av_stream).codecpar, encoder.codec_context);
      self.streams.push(av_stream);
    }
    Ok(())
  }

  pub fn get_stream(&self, stream_index: isize) -> *mut AVStream {
    unsafe { *(*self.format_context).streams.offset(stream_index) }
  }

  pub fn get_nb_streams(&self) -> u32 {
    if !self.frames.is_empty() {
      return 1;
    }
    unsafe { (*self.format_context).nb_streams }
  }

  pub fn get_format_name(&self) -> String {
    unsafe { tools::to_string((*(*self.format_context).iformat).name) }
  }

  pub fn get_format_long_name(&self) -> String {
    unsafe { tools::to_string((*(*self.format_context).iformat).long_name) }
  }

  pub fn get_program_count(&self) -> u32 {
    unsafe { (*self.format_context).nb_programs }
  }

  pub fn get_start_time(&self) -> Option<f32> {
    unsafe {
      if (*self.format_context).start_time == AV_NOPTS_VALUE {
        None
      } else {
        Some((*self.format_context).start_time as f32 / AV_TIME_BASE as f32)
      }
    }
  }

  pub fn get_duration(&self) -> Option<f64> {
    unsafe {
      if (*self.format_context).duration == AV_NOPTS_VALUE {
        None
      } else {
        Some((*self.format_context).duration as f64 / f64::from(AV_TIME_BASE))
      }
    }
  }

  pub fn get_bit_rate(&self) -> Option<i64> {
    unsafe {
      if (*self.format_context).bit_rate == AV_NOPTS_VALUE {
        None
      } else {
        Some((*self.format_context).bit_rate)
      }
    }
  }

  pub fn get_packet_size(&self) -> u32 {
    unsafe { (*self.format_context).packet_size }
  }

  pub fn get_stream_type(&self, stream_index: isize) -> AVMediaType {
    unsafe { (*(**(*self.format_context).streams.offset(stream_index)).codecpar).codec_type }
  }

  pub fn get_stream_type_name(&self, stream_index: isize) -> String {
    unsafe { tools::to_string(av_get_media_type_string(self.get_stream_type(stream_index))) }
  }

  pub fn get_codec_id(&self, stream_index: isize) -> AVCodecID {
    unsafe { (*(**(*self.format_context).streams.offset(stream_index)).codecpar).codec_id }
  }

  pub fn get_metadata(&self) -> BTreeMap<String, String> {
    unsafe {
      let mut tag = null_mut();
      let key = CString::new("").unwrap();
      let mut metadata = BTreeMap::new();

      loop {
        tag = av_dict_get(
          (*self.format_context).metadata,
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

  pub fn next_packet(&mut self) -> Result<Packet, String> {
    if !self.frames.is_empty() {
      if self.frame_index >= self.frames.len() as usize {
        return Err("End of data stream".to_string());
      }
      let frame = &self.frames[self.frame_index];
      unsafe {
        let filename = CString::new(self.filename.to_owned()).unwrap();
        let mut avio_context: *mut AVIOContext = null_mut();
        check_result!(avio_open(
          &mut avio_context,
          filename.as_ptr(),
          AVIO_FLAG_READ
        ));
        if avio_seek(avio_context, frame.offset as i64, 0) < 0 {
          println!("ERROR !");
        };

        let packet = av_packet_alloc();
        check_result!(av_new_packet(packet, frame.size as i32));
        check_result!(avio_read(avio_context, (*packet).data, (*packet).size));
        check_result!(avio_close(avio_context));

        self.frame_index += 1;

        return Ok(Packet { name: None, packet });
      }
    }

    unsafe {
      let packet = av_packet_alloc();
      av_init_packet(packet);

      if av_read_frame(self.format_context, packet) < 0 {
        return Err("Unable to read next packet".to_string());
      }

      Ok(Packet { name: None, packet })
    }
  }
}

impl Drop for FormatContext {
  fn drop(&mut self) {
    unsafe {
      if !self.format_context.is_null() {
        avformat_free_context(self.format_context);
      }
    }
  }
}
