use crate::{
  audio_decoder::AudioDecoder, filter::Filter, frame::Frame, order::*, tools,
  tools::rational::Rational, video_decoder::VideoDecoder,
};
use ffmpeg_sys_next::*;
use libc::c_void;
use std::{fmt, ptr::null_mut};

#[derive(Debug, PartialEq, Eq)]
pub enum GraphKind {
  Video,
  Audio,
}

#[derive(Debug, PartialEq, Eq)]
pub struct FilterGraph {
  pub kind: GraphKind,
  pub graph: *mut AVFilterGraph,
  pub audio_inputs: Vec<Filter>,
  pub audio_outputs: Vec<Filter>,
  pub video_inputs: Vec<Filter>,
  pub video_outputs: Vec<Filter>,
}

impl Default for FilterGraph {
  fn default() -> Self {
    FilterGraph::new().unwrap()
  }
}

impl FilterGraph {
  pub fn new() -> Result<Self, String> {
    unsafe {
      let filter_graph = avfilter_graph_alloc();
      if filter_graph.is_null() {
        return Err("Unable to create filter graph".to_string());
      }

      Ok(FilterGraph {
        kind: GraphKind::Audio,
        graph: filter_graph,
        audio_inputs: vec![],
        audio_outputs: vec![],
        video_inputs: vec![],
        video_outputs: vec![],
      })
    }
  }

  pub fn add_input_from_video_decoder(
    &mut self,
    label: &str,
    video_decoder: &VideoDecoder,
  ) -> Result<(), String> {
    let buffer = unsafe { Filter::new_with_label(self.graph, "buffer", label)? };

    let width = ParameterValue::Int64(i64::from(video_decoder.get_width()));
    width.set("width", buffer.context as *mut c_void)?;

    let height = ParameterValue::Int64(i64::from(video_decoder.get_height()));
    height.set("height", buffer.context as *mut c_void)?;

    let (mut num, den) = video_decoder.get_frame_rate();
    if num == 0 {
      num = 25;
    }
    let time_base = ParameterValue::Rational(Rational { num, den });
    time_base.set("time_base", buffer.context as *mut c_void)?;

    let (num, den) = video_decoder.get_aspect_ratio();
    let pixel_aspect = ParameterValue::Rational(Rational { num, den });
    pixel_aspect.set("pixel_aspect", buffer.context as *mut c_void)?;

    let pix_fmt = ParameterValue::String(video_decoder.get_pix_fmt_name());
    pix_fmt.set("pix_fmt", buffer.context as *mut c_void)?;
    buffer.init()?;

    self.video_inputs.push(buffer);
    Ok(())
  }

  pub fn add_input_from_audio_decoder(
    &mut self,
    label: &str,
    audio_decoder: &AudioDecoder,
  ) -> Result<(), String> {
    let abuffer = unsafe { Filter::new_with_label(self.graph, "abuffer", label)? };

    let layout = audio_decoder.get_channel_layout();
    if layout > 0 {
      let channel_layout = ParameterValue::ChannelLayout(layout);
      channel_layout.set("channel_layout", abuffer.context as *mut c_void)?;
    }

    let sample_rate = ParameterValue::Int64(i64::from(audio_decoder.get_sample_rate()));
    sample_rate.set("sample_rate", abuffer.context as *mut c_void)?;

    let channels = ParameterValue::Int64(i64::from(audio_decoder.get_nb_channels()));
    channels.set("channels", abuffer.context as *mut c_void)?;

    let sample_fmt = ParameterValue::String(audio_decoder.get_sample_fmt_name());
    sample_fmt.set("sample_fmt", abuffer.context as *mut c_void)?;

    abuffer.init()?;

    self.audio_inputs.push(abuffer);
    Ok(())
  }

  pub fn add_video_output(&mut self, label: &str) -> Result<(), String> {
    let buffersink = unsafe { Filter::new_with_label(self.graph, "buffersink", label)? };
    buffersink.init()?;

    self.video_outputs.push(buffersink);
    Ok(())
  }

  pub fn add_audio_output(&mut self, label: &str) -> Result<(), String> {
    let abuffersink = unsafe { Filter::new_with_label(self.graph, "abuffersink", label)? };
    abuffersink.init()?;

    self.audio_outputs.push(abuffersink);
    Ok(())
  }

  pub fn add_filter(&self, args: &filter::Filter) -> Result<Filter, String> {
    let filter = if let Some(ref label) = args.label {
      unsafe { Filter::new_with_label(self.graph, &args.name, label)? }
    } else {
      unsafe { Filter::new(self.graph, &args.name)? }
    };

    set_parameters(filter.context as *mut c_void, &args.parameters)?;
    filter.init()?;

    Ok(filter)
  }

  pub fn connect(
    &mut self,
    src: &Filter,
    src_index: u32,
    dst: &Filter,
    dst_index: u32,
  ) -> Result<(), String> {
    unsafe {
      check_result!(avfilter_link(
        src.context,
        src_index,
        dst.context,
        dst_index
      ));
    }
    Ok(())
  }

  pub fn connect_input(
    &mut self,
    label: &str,
    src_index: u32,
    dst: &Filter,
    dst_index: u32,
  ) -> Result<(), String> {
    for audio_input in &self.audio_inputs {
      if audio_input.get_label() == label {
        unsafe {
          check_result!(avfilter_link(
            audio_input.context,
            src_index,
            dst.context,
            dst_index
          ));
        }
        return Ok(());
      }
    }

    for video_input in &self.video_inputs {
      if video_input.get_label() == label {
        unsafe {
          check_result!(avfilter_link(
            video_input.context,
            src_index,
            dst.context,
            dst_index
          ));
        }
        return Ok(());
      }
    }

    Err("Unable to connect".to_string())
  }

  pub fn connect_output(
    &mut self,
    src: &Filter,
    src_index: u32,
    label: &str,
    dst_index: u32,
  ) -> Result<(), String> {
    for audio_output in &self.audio_outputs {
      if audio_output.get_label() == label {
        unsafe {
          check_result!(avfilter_link(
            src.context,
            src_index,
            audio_output.context,
            dst_index
          ));
        }
        return Ok(());
      }
    }

    for video_output in &self.video_outputs {
      if video_output.get_label() == label {
        unsafe {
          check_result!(avfilter_link(
            src.context,
            src_index,
            video_output.context,
            dst_index
          ));
        }
        return Ok(());
      }
    }

    Err("Unable to connect".to_string())
  }

  pub fn validate(&mut self) -> Result<(), String> {
    unsafe {
      check_result!(avfilter_graph_config(self.graph, null_mut()));
      Ok(())
    }
  }

  pub fn process(
    &self,
    in_audio_frames: &[Frame],
    in_video_frames: &[Frame],
  ) -> Result<(Vec<Frame>, Vec<Frame>), String> {
    if in_audio_frames.len() != self.audio_inputs.len() {
      return Err(format!(
        "unable to process graph, mistmatch input frames ({}) with graph inputs ({})",
        in_audio_frames.len(),
        self.audio_inputs.len()
      ));
    }
    if in_video_frames.len() != self.video_inputs.len() {
      return Err(format!(
        "unable to process graph, mistmatch input frames ({}) with graph inputs ({})",
        in_video_frames.len(),
        self.video_inputs.len()
      ));
    }

    let mut output_audio_frames = vec![];
    let mut output_video_frames = vec![];

    unsafe {
      for (index, frame) in in_audio_frames.iter().enumerate() {
        check_result!(av_buffersrc_add_frame(
          self.audio_inputs[index].context,
          frame.frame
        ));
      }
      for (index, frame) in in_video_frames.iter().enumerate() {
        check_result!(av_buffersrc_add_frame(
          self.video_inputs[index].context,
          frame.frame
        ));
      }

      for (index, output_filter) in self.audio_outputs.iter().enumerate() {
        let output_frame = av_frame_alloc();
        let result = av_buffersink_get_frame(output_filter.context, output_frame);
        if result == AVERROR(EAGAIN) || result == AVERROR_EOF {
          break;
        } else {
          check_result!(result);
        }
        output_audio_frames.push(Frame {
          name: Some(output_filter.get_label()),
          frame: output_frame,
          index,
        });
      }

      for (index, output_filter) in self.video_outputs.iter().enumerate() {
        let output_frame = av_frame_alloc();
        let result = av_buffersink_get_frame(output_filter.context, output_frame);
        if result == AVERROR(EAGAIN) || result == AVERROR_EOF {
          break;
        } else {
          check_result!(result);
        }
        output_video_frames.push(Frame {
          name: Some(output_filter.get_label()),
          frame: output_frame,
          index,
        });
      }
    }

    Ok((output_audio_frames, output_video_frames))
  }
}

impl Drop for FilterGraph {
  fn drop(&mut self) {
    unsafe {
      if !self.graph.is_null() {
        avfilter_graph_free(&mut self.graph);
      }
    }
  }
}

impl fmt::Display for FilterGraph {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    writeln!(f, "Filter Graph:")?;
    unsafe {
      let filters = tools::from_buf_raw((*self.graph).filters, (*self.graph).nb_filters as usize);
      for context_filter in filters {
        writeln!(f, "---------------")?;
        write!(
          f,
          "{}",
          Filter {
            context: context_filter
          }
        )?;
      }
    }
    Ok(())
  }
}
