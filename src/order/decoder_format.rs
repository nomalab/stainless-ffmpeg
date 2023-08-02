use crate::audio_decoder::AudioDecoder;
use crate::filter_graph::FilterGraph;
use crate::format_context::FormatContext;
use crate::order::input::Input;
use crate::stream::Stream;
use crate::subtitle_decoder::SubtitleDecoder;
use crate::tools;
use crate::video_decoder::VideoDecoder;
use ffmpeg_sys_next::AVMediaType;

#[derive(Debug)]
pub struct DecoderFormat {
  pub context: FormatContext,
  pub audio_decoders: Vec<AudioDecoder>,
  pub subtitle_decoders: Vec<SubtitleDecoder>,
  pub video_decoders: Vec<VideoDecoder>,
}

impl DecoderFormat {
  pub fn new(graph: &mut FilterGraph, input: &Input) -> Result<Self, String> {
    match input {
      Input::VideoFrames {
        path,
        frames,
        label,
        codec,
        width,
        height,
        ..
      } => {
        let audio_decoders = vec![];
        let subtitle_decoders = vec![];
        let mut video_decoders = vec![];
        let mut context = FormatContext::new(path)?;
        context.open_input()?;
        context.set_frames_addresses(frames);

        let identifier = if let Some(ref identifier) = label {
          identifier.clone()
        } else {
          tools::random_string(8)
        };

        let video_decoder =
          VideoDecoder::new_with_codec(identifier.clone(), codec, *width, *height, 0)?;
        let video_stream = Stream::new(context.get_stream(0))?;
        graph.add_input_from_video_decoder(&identifier, &video_decoder, video_stream)?;
        video_decoders.push(video_decoder);

        Ok(DecoderFormat {
          context,
          audio_decoders,
          subtitle_decoders,
          video_decoders,
        })
      }
      Input::Streams { path, streams, .. } => {
        let mut audio_decoders = vec![];
        let mut subtitle_decoders = vec![];
        let mut video_decoders = vec![];
        let mut context = FormatContext::new(path)?;
        context.open_input()?;

        for stream in streams {
          let identifier = if let Some(ref identifier) = stream.label {
            identifier.clone()
          } else {
            tools::random_string(8)
          };

          match context.get_stream_type(stream.index as isize) {
            AVMediaType::AVMEDIA_TYPE_VIDEO => {
              let video_decoder =
                VideoDecoder::new(identifier.clone(), &context, stream.index as isize)?;
              let video_stream = Stream::new(context.get_stream(stream.index as isize))?;
              graph.add_input_from_video_decoder(&identifier, &video_decoder, video_stream)?;
              video_decoders.push(video_decoder);
            }
            AVMediaType::AVMEDIA_TYPE_AUDIO => {
              let audio_decoder =
                AudioDecoder::new(identifier.clone(), &context, stream.index as isize)?;
              graph.add_input_from_audio_decoder(&identifier, &audio_decoder)?;
              audio_decoders.push(audio_decoder);
            }
            AVMediaType::AVMEDIA_TYPE_SUBTITLE => {
              let subtitle_decoder =
                SubtitleDecoder::new(identifier.clone(), &context, stream.index as isize)?;
              subtitle_decoders.push(subtitle_decoder);
            }
            _ => {}
          }
        }

        Ok(DecoderFormat {
          context,
          audio_decoders,
          subtitle_decoders,
          video_decoders,
        })
      }
    }
  }
}
