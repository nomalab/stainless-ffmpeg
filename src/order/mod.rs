use filter_graph::FilterGraph;
use serde_json;
use std::collections::HashMap;

mod decoder_format;
mod encoder_format;
pub mod filter;
pub mod filter_input;
pub mod filter_output;
pub mod frame;
pub mod input;
pub mod input_kind;
pub mod output;
pub mod output_kind;
mod output_result;
pub mod parameters;
mod stream;

use order::decoder_format::DecoderFormat;
use order::encoder_format::EncoderFormat;
pub use order::filter::Filter;
use order::filter_input::FilterInput;
use order::input::Input;
use order::input_kind::InputKind;
use order::output::Output;
use order::output_kind::OutputKind;
pub use order::output_result::OutputResult;
pub use order::parameters::*;

use packet::Packet;
use std::ptr::null_mut;

#[derive(Debug, Deserialize)]
pub struct Order {
  pub inputs: Vec<Input>,
  pub outputs: Vec<Output>,
  pub graph: Vec<Filter>,
  #[serde(skip)]
  total_streams: u32,
  #[serde(skip)]
  input_formats: Vec<DecoderFormat>,
  #[serde(skip)]
  output_formats: Vec<EncoderFormat>,
  #[serde(skip)]
  filter_graph: FilterGraph,
}

impl Order {
  pub fn new(inputs: Vec<Input>, graph: Vec<Filter>, outputs: Vec<Output>) -> Result<Self, String> {
    Ok(Order {
      inputs,
      outputs,
      graph,
      total_streams: 0,
      input_formats: vec![],
      output_formats: vec![],
      filter_graph: FilterGraph::new()?
    })
  }

  pub fn new_parse(message: &str) -> Result<Self, String> {
    serde_json::from_str(message).map_err(|e| e.to_string())
  }

  pub fn setup(&mut self) -> Result<(), String> {
    warn!("Build inputs");
    self.build_input_format()?;
    warn!("Build outputs");
    self.build_output_format()?;
    warn!("Build graph");
    self.build_graph()?;
    warn!("{}", self.filter_graph);

    if let Err(msg) = self.filter_graph.validate() {
      return Err(msg);
    }
    Ok(())
  }

  pub fn process(&mut self) -> Result<Vec<OutputResult>, String> {
    let mut results = vec![];
    let mut decoded_audio_frames = 0;
    let mut decoded_video_frames = 0;
    let mut encoded_audio_frames = 0;
    let mut encoded_video_frames = 0;

    let mut end_position = false;
    loop {
      let mut audio_frames = vec![];
      let mut subtitle_packets = vec![];
      let mut video_frames = vec![];
      let mut end = 0;

      for format in &mut self.input_formats {
        for _ in 0..format.context.get_nb_streams() {
          match format.context.next_packet() {
            Ok(mut packet) => {
              for decoder in &format.audio_decoders {
                if decoder.stream_index == packet.get_stream_index() {
                  if let Ok(frame) = decoder.decode(&packet) {
                    decoded_audio_frames += 1;
                    info!("decode {} frame", decoded_audio_frames);
                    audio_frames.push(frame);
                  }
                }
              }
              for decoder in &format.video_decoders {
                if decoder.stream_index == packet.get_stream_index() {
                  if let Ok(frame) = decoder.decode(&packet) {
                    decoded_video_frames += 1;
                    info!("decode {} frame", decoded_video_frames);
                    video_frames.push(frame);
                  }
                }
              }
              for decoder in &format.subtitle_decoders {
                if decoder.stream_index == packet.get_stream_index() {
                  packet.name = Some(decoder.identifier.clone());
                  subtitle_packets.push(packet);
                  break;
                }
              }
            }
            Err(msg) => {
              if msg == "End of data stream" || msg == "Unable to read next packet" {
                for decoder in &format.video_decoders {
                  let packet = null_mut();

                  let p = Packet {
                    name: None,
                    packet
                  };

                  if let Ok(frame) = decoder.decode(&p) {
                    decoded_video_frames += 1;
                    info!("decode {} frame", decoded_video_frames);
                    video_frames.push(frame);
                  } else {
                    end += 1;
                  }
                }
                for decoder in &format.audio_decoders {
                  let packet = null_mut();

                  let p = Packet {
                    name: None,
                    packet
                  };

                  if let Ok(frame) = decoder.decode(&p) {
                    decoded_audio_frames += 1;
                    info!("decode {} frame", decoded_audio_frames);
                    audio_frames.push(frame);
                  } else {
                    end += 1;
                  }
                }
              } else {
                end += 1;
              }
            }
          }
        }
      }

      if end == self.total_streams {
        break;
      }

      if audio_frames.len() == self.filter_graph.audio_inputs.len()
        && video_frames.len() == self.filter_graph.video_inputs.len()
      {
        let (output_audio_frames, output_video_frames) =
          if audio_frames.len() == 0 && video_frames.len() == 0 {
            (audio_frames, video_frames)
          } else {
            self.filter_graph.process(&audio_frames, &video_frames)?
          };

        for output_frame in output_audio_frames {
          for output in &self.outputs {
            if let Some(OutputKind::AudioMetadata) = output.kind {
              let mut entry = HashMap::new();
              entry.insert("pts".to_owned(), format!("{}", output_frame.get_pts()));

              for key in &output.keys {
                if let Some(value) = output_frame.get_metadata(&key) {
                  entry.insert(key.clone(), value);
                }
              }

              results.push(OutputResult::Entry(entry));
            }
          }

          for output in &mut self.output_formats {
            let (packet_option, position) = output.encode(&output_frame, output_frame.get_pts() as usize)?;
            end_position = position;
            if !end_position {
              encoded_audio_frames += output_frame.get_channels() as usize;
              info!("encode {} frame", encoded_audio_frames);
              if let Some(packet) = packet_option {
                results.push(OutputResult::Packet(packet));
              };
            }
          }
        }

        for output_packet in subtitle_packets {
          for output in &mut self.output_formats {
            output.wrap(&output_packet)?;
          }
        }

        for output_frame in output_video_frames {
          for output in &self.outputs {
            if let Some(OutputKind::VideoMetadata) = output.kind {
              let mut entry = HashMap::new();
              entry.insert("pts".to_owned(), format!("{}", output_frame.get_pts()));

              for key in &output.keys {
                if let Some(value) = output_frame.get_metadata(&key) {
                  entry.insert(key.clone(), value);
                }
              }

              results.push(OutputResult::Entry(entry));
            }
          }

          for output_format in &mut self.output_formats {
            let (packet_option, position) = output_format.encode(&output_frame, encoded_video_frames)?;
            end_position = position;
            if !end_position {
              encoded_video_frames += 1;
              info!("encode {} frame", encoded_video_frames);
              if let Some(packet) = packet_option {
                results.push(OutputResult::Packet(packet));
              };
            }
          }
        }
      }
      if end_position {
        break;
      }
    }

    loop {
      let mut end = true;
      for output in &mut self.output_formats {
        let packets = output.finish()?;
        if packets.len() != 0 {
          end = false;
          continue;
        }
      }
      if end {
        break;
      }
    }

    results.push(OutputResult::ProcessStatistics{
      decoded_audio_frames,
      decoded_video_frames,
      encoded_audio_frames,
      encoded_video_frames,
    });

    Ok(results)
  }

  fn build_input_format(&mut self) -> Result<(), String> {
    for input in &self.inputs {
      let decoder = DecoderFormat::new(&mut self.filter_graph, &input)?;
      self.total_streams += decoder.context.get_nb_streams();
      self.input_formats.push(decoder);
    }
    Ok(())
  }

  fn build_output_format(&mut self) -> Result<(), String> {
    for output in &self.outputs {
      match output.kind {
        Some(OutputKind::File) |
        Some(OutputKind::Packet) => {
          let encoder = EncoderFormat::new(&mut self.filter_graph, &output)?;
          self.output_formats.push(encoder);
        },
        Some(OutputKind::AudioMetadata) => {
          if let Some(ref identifier) = output.stream {
            self.filter_graph.add_audio_output(identifier)?;
          }
        }
        Some(OutputKind::VideoMetadata) => {
          if let Some(ref identifier) = output.stream {
            self.filter_graph.add_video_output(identifier)?;
          }
        }
        None => {}
      }
    }
    Ok(())
  }

  fn build_graph(&mut self) -> Result<Vec<::filter::Filter>, String> {
    let mut filters = vec![];

    for filter_description in &self.graph {
      let filter = self.filter_graph.add_filter(&filter_description)?;
      if let Some(ref inputs) = filter_description.inputs {
        for (index, input) in inputs.iter().enumerate() {
          match *input {
            FilterInput {
              kind: InputKind::Stream,
              stream_label: ref label,
            } => {
              let decoder_stream_index = 0;
              debug!("connect input {} ({})", label, decoder_stream_index);
              if let Err(msg) =
                self
                  .filter_graph
                  .connect_input(&label, decoder_stream_index, &filter, index as u32)
              {
                return Err(format!(
                  "unable to connect input stream {} ({}): {}",
                  label, decoder_stream_index, msg
                ));
              }
            }
            FilterInput {
              kind: InputKind::Filter,
              stream_label: ref label,
            } => {
              let filter_stream_index = 0;
              for description in &self.graph {
                if let Some(ref filter_outputs) = description.filter_outputs {
                  for filter_output in filter_outputs {
                    if filter_output.stream_label == *label {
                      if let Some(ref description_label) = description.label {
                        self.filter_graph.connect_filter(&filters, &description_label, filter_stream_index, &filter, index as u32)?;
                      }
                    }
                  }
                }
              }
            }
          }
        }
      } else if let Some(last_filter) = filters.last() {
        if let Err(msg) = self.filter_graph.connect(last_filter, 0, &filter, 0) {
          return Err(format!("unable to auto-connect : {}", msg));
        }
      } else if let Err(msg) = self.filter_graph.connect_input("", 0, &filter, 0) {
        return Err(format!("unable to auto-connect with input: {}", msg));
      }

      if let Some(ref outputs) = filter_description.outputs {
        for (index, output) in outputs.iter().enumerate() {
          let encoder_stream_index = 0;
          debug!(
            "connect output {} ({})",
            output.stream_label, encoder_stream_index
          );
          self.filter_graph.connect_output(
            &filter,
            index as u32,
            &output.stream_label,
            encoder_stream_index,
          )?;
        }
      }

      filters.push(filter);
    }

    Ok(filters)
  }
}

#[test]
fn parse_sample_audio_encoding_graph() {
  use serde_json;
  use std::io::Read;
  use std::fs::File;
  use order::ParameterValue;
  use order::filter_output::FilterOutput;
  use order::stream::Stream;
  use order::input_kind::InputKind;
  use order::output::OutputStream;
  use order::output_kind::OutputKind;
  use tools::rational::Rational;

  let mut file = File::open("tests/audio_encoding.json").unwrap();

  let mut contents = "".to_string();
  file.read_to_string(&mut contents).unwrap();

  let order : Order = serde_json::from_str(&contents).unwrap();

  let input_channels = ParameterValue::Int64(2);
  let mut amerge_params : HashMap<String, ParameterValue> = HashMap::new();
  amerge_params.insert("inputs".to_string(), input_channels);

  let sample_fmts = ParameterValue::String("s32".to_string());
  let sample_rates = ParameterValue::String("48000".to_string());
  let channel_layouts = ParameterValue::String("stereo".to_string());
  let mut aformat_params : HashMap<String, ParameterValue> = HashMap::new();
  aformat_params.insert("sample_fmts".to_string(), sample_fmts.clone());
  aformat_params.insert("sample_rates".to_string(), sample_rates);
  aformat_params.insert("channel_layouts".to_string(), channel_layouts);

  let sample_rate = ParameterValue::Rational(Rational { num: 48000, den: 1 });
  let mut output_params : HashMap<String, ParameterValue> = HashMap::new();
  output_params.insert("sample_fmt".to_string(), sample_fmts);
  output_params.insert("sample_rate".to_string(), sample_rate);

  assert_eq!(vec![
    Input::Streams {
      id: 1,
      path: "tests/PAL_1080i_MPEG_XDCAM-HD_colorbar.mxf".to_string(),
      streams: vec![
        Stream { index: 1, label: Some("my_audio1".to_string()) },
        Stream { index: 2, label: Some("my_audio2".to_string()) }
      ]
    }
  ], order.inputs);

  assert_eq!(vec![
    Output {
      kind: Some(OutputKind::File),
      keys: vec![],
      path: Some("out.wav".to_string()),
      stream: None,
      parameters: HashMap::new(),
      streams: vec![
        OutputStream {
          label: Some("output1".to_string()),
          codec: "pcm_s24le".to_string(),
          parameters: output_params
        }
      ]
    }
  ], order.outputs);

  assert_eq!(vec![
    Filter {
      name: "amerge".to_string(),
      label: Some("amerge_filter".to_string()),
      parameters: amerge_params,
      inputs: Some(vec![
        FilterInput { kind: InputKind::Stream, stream_label: "my_audio1".to_string() },
        FilterInput { kind: InputKind::Stream, stream_label: "my_audio2".to_string() }
      ]),
      outputs: None,
      filter_outputs: None
    },
    Filter {
      name: "aformat".to_string(),
      label: Some("aformat_filter".to_string()),
      parameters: aformat_params,
      inputs: None,
      outputs: Some(vec![
        FilterOutput { stream_label: "output1".to_string() }
      ]),
      filter_outputs: None
    }
  ], order.graph);
}

#[test]
fn parse_sample_video_encoding_graph() {
  use serde_json;
  use std::io::Read;
  use std::fs::File;
  use order::ParameterValue;
  use order::filter_output::FilterOutput;
  use order::stream::Stream;
  use order::input_kind::InputKind;
  use order::output::OutputStream;
  use order::output_kind::OutputKind;
  use tools::rational::Rational;

  let mut file = File::open("tests/video_encoding.json").unwrap();

  let mut contents = "".to_string();
  file.read_to_string(&mut contents).unwrap();

  let order : Order = serde_json::from_str(&contents).unwrap();

  let idet_params : HashMap<String, ParameterValue> = HashMap::new();

  let pix_fmts = ParameterValue::String("yuv420p".to_string());
  let mut format_params : HashMap<String, ParameterValue> = HashMap::new();
  format_params.insert("pix_fmts".to_string(), pix_fmts.clone());

  let sample_fmts = ParameterValue::String("s32".to_string());
  let sample_rates = ParameterValue::String("48000".to_string());
  let channel_layouts = ParameterValue::String("mono".to_string());
  let mut aformat_1_params : HashMap<String, ParameterValue> = HashMap::new();
  aformat_1_params.insert("sample_fmts".to_string(), sample_fmts.clone());
  aformat_1_params.insert("sample_rates".to_string(), sample_rates.clone());
  aformat_1_params.insert("channel_layouts".to_string(), channel_layouts.clone());

  let mut aformat_2_params : HashMap<String, ParameterValue> = HashMap::new();
  aformat_2_params.insert("sample_fmts".to_string(), sample_fmts.clone());
  aformat_2_params.insert("sample_rates".to_string(), sample_rates);
  aformat_2_params.insert("channel_layouts".to_string(), channel_layouts.clone());

  let frame_rate = ParameterValue::Rational(Rational { num: 25, den: 1 });
  let width = ParameterValue::Int64(1920);
  let height = ParameterValue::Int64(1080);
  let bitrate = ParameterValue::Int64(50000000);
  let gop_size = ParameterValue::Int64(12);
  let max_b_frames = ParameterValue::Int64(2);
  let color_range = ParameterValue::String("head".to_string());
  let sample_aspect_ratio = ParameterValue::Rational(Rational { num: 1, den: 1 });
  let mut output_video_params : HashMap<String, ParameterValue> = HashMap::new();
  output_video_params.insert("frame_rate".to_string(), frame_rate);
  output_video_params.insert("pixel_format".to_string(), pix_fmts);
  output_video_params.insert("width".to_string(), width);
  output_video_params.insert("height".to_string(), height);
  output_video_params.insert("bitrate".to_string(), bitrate);
  output_video_params.insert("gop_size".to_string(), gop_size);
  output_video_params.insert("max_b_frames".to_string(), max_b_frames);
  output_video_params.insert("color_range".to_string(), color_range);
  output_video_params.insert("sample_aspect_ratio".to_string(), sample_aspect_ratio);

  let sample_rate = ParameterValue::Rational(Rational { num: 48000, den: 1 });
  let mut output_audio1_params : HashMap<String, ParameterValue> = HashMap::new();
  output_audio1_params.insert("sample_rate".to_string(), sample_rate.clone());
  output_audio1_params.insert("sample_fmt".to_string(), sample_fmts.clone());
  output_audio1_params.insert("channel_layout".to_string(), channel_layouts.clone());

  let mut output_audio2_params : HashMap<String, ParameterValue> = HashMap::new();
  output_audio2_params.insert("sample_rate".to_string(), sample_rate);
  output_audio2_params.insert("sample_fmt".to_string(), sample_fmts);
  output_audio2_params.insert("channel_layout".to_string(), channel_layouts);

  assert_eq!(vec![
    Input::Streams {
      id: 1,
      path: "tests/PAL_1080i_MPEG_XDCAM-HD_colorbar.mxf".to_string(),
      streams: vec![
        Stream { index: 0, label: Some("input1".to_string()) },
        Stream { index: 1, label: Some("audio1".to_string()) },
        Stream { index: 2, label: Some("audio2".to_string()) }
      ]
    }
  ], order.inputs);

  assert_eq!(vec![
    Output {
      kind: Some(OutputKind::File),
      keys: vec![],
      path: Some("video_encoding.mxf".to_string()),
      stream: None,
      parameters: HashMap::new(),
      streams: vec![
        OutputStream {
          label: Some("output1".to_string()),
          codec: "mpeg2video".to_string(),
          parameters: output_video_params
        },
        OutputStream {
          label: Some("audio_output1".to_string()),
          codec: "pcm_s24le".to_string(),
          parameters: output_audio1_params
        },
        OutputStream {
          label: Some("audio_output2".to_string()),
          codec: "pcm_s24le".to_string(),
          parameters: output_audio2_params
        }
      ]
    }
  ], order.outputs);

  assert_eq!(vec![
    Filter {
      name: "idet".to_string(),
      label: Some("idet_filter".to_string()),
      parameters: idet_params,
      inputs: Some(vec![
        FilterInput {kind: InputKind::Stream, stream_label: "input1".to_string() }
      ]),
      outputs: None,
      filter_outputs: None
    },
    Filter {
      name: "format".to_string(),
      label: Some("format_filter".to_string()),
      parameters: format_params,
      inputs: None,
      outputs: Some(vec![
        FilterOutput { stream_label: "output1".to_string() }
      ]),
      filter_outputs: None
    },
    Filter {
      name: "aformat".to_string(),
      label: Some("aformat_filter".to_string()),
      parameters: aformat_1_params,
      inputs: Some(vec![
        FilterInput { kind: InputKind::Stream, stream_label: "audio1".to_string() }
      ]),
      outputs: Some(vec![
        FilterOutput { stream_label: "audio_output1".to_string() }
      ]),
      filter_outputs: None
    },
    Filter {
      name: "aformat".to_string(),
      label: Some("aformat_filter".to_string()),
      parameters: aformat_2_params,
      inputs: Some(vec![
        FilterInput { kind: InputKind::Stream, stream_label: "audio2".to_string() }
      ]),
      outputs: Some(vec![
        FilterOutput { stream_label: "audio_output2".to_string() }
      ]),
      filter_outputs: None
    }
  ], order.graph);
}
