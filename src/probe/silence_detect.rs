use crate::format_context::FormatContext;
use crate::order::{
  filter_input::FilterInput, filter_output::FilterOutput, input::Input, input_kind::InputKind,
  output::Output, output_kind::OutputKind, stream::Stream,
};
use crate::order::{Filter, Order, OutputResult::Entry, ParameterValue};
use crate::probe::deep::{CheckParameterValue, SilenceResult, StreamProbeResult};
use crate::stream::Stream as ContextStream;
use stainless_ffmpeg_sys::AVMediaType;
use std::collections::HashMap;

pub fn create_graph(
  filename: &String,
  audio_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue>,
) -> Result<Order, String> {
  let mut filters = vec![];
  let mut inputs = vec![];
  let mut outputs = vec![];
  for i in audio_indexes {
    let input_identifier = format!("audio_input_{}", i);
    let output_identifier = format!("audio_output_{}", i);

    let input_streams = vec![Stream {
      index: i,
      label: Some(input_identifier.clone()),
    }];

    let mut silencedetect_params: HashMap<String, ParameterValue> = HashMap::new();
    if let Some(duration) = params.get("duration") {
      if let Some(min_duration) = duration.min {
        let min = (min_duration - 0.001) * 1000000.0;
        silencedetect_params.insert("duration".to_string(), ParameterValue::Float(min));
      }
    }

    let channel_layouts = ParameterValue::String("mono".to_string());
    let mut aformat_params: HashMap<String, ParameterValue> = HashMap::new();
    aformat_params.insert("channel_layouts".to_string(), channel_layouts);

    filters.push(Filter {
      name: "silencedetect".to_string(),
      label: Some(format!("silencedetect_filter{}", i)),
      parameters: silencedetect_params.clone(),
      inputs: Some(vec![FilterInput {
        kind: InputKind::Stream,
        stream_label: input_identifier,
      }]),
      outputs: None,
    });
    filters.push(Filter {
      name: "aformat".to_string(),
      label: Some(format!("aformat_filter{}", i)),
      parameters: aformat_params.clone(),
      inputs: None,
      outputs: Some(vec![FilterOutput {
        stream_label: output_identifier.clone(),
      }]),
    });

    inputs.push(Input::Streams {
      id: i,
      path: filename.to_string(),
      streams: input_streams,
    });
    outputs.push(Output {
      kind: Some(OutputKind::AudioMetadata),
      keys: vec![
        "lavfi.silence_start".to_string(),
        "lavfi.silence_end".to_string(),
        "lavfi.silence_duration".to_string(),
      ],
      stream: Some(output_identifier),
      path: None,
      streams: vec![],
      parameters: HashMap::new(),
    });
  }

  Order::new(inputs, filters, outputs)
}

pub fn detect_silence(
  filename: &String,
  streams: &mut Vec<StreamProbeResult>,
  audio_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue>,
) -> () {
  let mut order = create_graph(filename, audio_indexes.clone(), params.clone()).unwrap();
  if let Err(msg) = order.setup() {
    error!("{:?}", msg);
    return;
  }

  match order.process() {
    Ok(results) => {
      info!("END OF PROCESS");
      info!("-> {:?} frames processed", results.len());
      let mut duration = 0.0;
      let mut context = FormatContext::new(&filename).unwrap();
      if let Err(msg) = context.open_input() {
        context.close_input();
        error!("{:?}", msg);
        return;
      }
      for index in 0..context.get_nb_streams() {
        if let Ok(stream) = ContextStream::new(context.get_stream(index as isize)) {
          match context.get_stream_type(index as isize) {
            AVMediaType::AVMEDIA_TYPE_VIDEO => {
              let rational_frame_rate = stream.get_frame_rate();
              let frame_rate = rational_frame_rate.num as f64 / rational_frame_rate.den as f64;
              duration = results.len() as f64 / audio_indexes.len() as f64 / frame_rate;
            }
            _ => {}
          }
        }
      }
      for result in results {
        match result {
          Entry(entry_map) => {
            if let Some(stream_id) = entry_map.get("stream_id") {
              let index: i32 = stream_id.parse().unwrap();
              let mut silence = SilenceResult {
                start: 0.0,
                end: duration,
              };
              let mut max_duration = None;
              if let Some(duration) = params.get("duration") {
                max_duration = duration.max;
              }

              if let Some(value) = entry_map.get("lavfi.silence_start") {
                silence.start = value.parse::<f64>().unwrap();
                streams[(index) as usize].detected_silence.push(silence);
              }
              if let Some(value) = entry_map.get("lavfi.silence_end") {
                if let Some(last_detect) = streams[(index) as usize].detected_silence.last_mut() {
                  last_detect.end = value.parse::<f64>().unwrap();
                }
              }
              if let Some(value) = entry_map.get("lavfi.silence_duration") {
                if let Some(max) = max_duration {
                  if value.parse::<f64>().unwrap() > max {
                    streams[(index) as usize].detected_silence.pop();
                  }
                }
              }
            }
          }
          _ => {}
        }
      }
      for index in 0..context.get_nb_streams() {
        if streams[(index) as usize].detected_silence.len() == 1 {
          if streams[(index) as usize].detected_silence[0].start == 0.0
            && streams[(index) as usize].detected_silence[0].end == duration
          {
            streams[(index) as usize].silent_stream = true;
          }
        }
      }
    }
    Err(msg) => {
      error!("ERROR: {}", msg);
    }
  }
}
