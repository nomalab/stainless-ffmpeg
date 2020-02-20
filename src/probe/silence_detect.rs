use crate::order::{
  filter_input::FilterInput, filter_output::FilterOutput, input::Input, input_kind::InputKind,
  output::Output, output_kind::OutputKind, stream::Stream,
};
use crate::order::{Filter, Order, OutputResult::Entry, ParameterValue};
use crate::probe::deep::StreamProbeResult;
use std::collections::HashMap;

pub fn create_graph(filename: &String, audio_indexes: Vec<u32>) -> Result<Order, String> {
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

    let duration = ParameterValue::Int64(2);
    let mut silencedetect_params: HashMap<String, ParameterValue> = HashMap::new();
    silencedetect_params.insert("duration".to_string(), duration);

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
) -> () {
  let mut order = create_graph(filename, audio_indexes).unwrap();
  if let Err(msg) = order.setup() {
    error!("{:?}", msg);
    return;
  }

  match order.process() {
    Ok(results) => {
      info!("END OF PROCESS");
      info!("-> {:?} frames processed", results.len());
      for result in results {
        match result {
          Entry(entry_map) => {
            if let Some(stream_id) = entry_map.get("stream_id") {
              let index: i32 = stream_id.parse().unwrap();
              if let Some(value) = entry_map.get("lavfi.silence_start") {
                streams[(index) as usize]
                  .silence_start
                  .push(value.to_string());
              }
              if let Some(value) = entry_map.get("lavfi.silence_end") {
                streams[(index) as usize]
                  .silence_end
                  .push(value.to_string());
              }
            }
          }
          _ => {}
        }
      }
    }
    Err(msg) => {
      error!("ERROR: {}", msg);
    }
  }
}
