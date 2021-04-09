//! The stainless ffmpeg prelude.
//!
//! The prelude re-exports most commonly used traits and macros from this crate.
//!
//! # Examples
//!
//! Import the prelude with:
//!
//! ```
//! #[allow(unused_imports)]
//! use stainless_ffmpeg::prelude::*;
//! ```

#[doc(no_inline)]
pub use crate::{
  audio_decoder::AudioDecoder,
  audio_encoder::AudioEncoder,
  check_result,
  filter_graph::FilterGraph,
  format_context::FormatContext,
  frame::Frame,
  order::{output::SampleFormat, Filter, ParameterValue},
  packet::Packet,
  tools,
  video_decoder::VideoDecoder,
  video_encoder::VideoEncoder,
};

#[doc(no_inline)]
pub use ffmpeg_sys::*;
