use crate::format_context::FormatContext;
use crate::order::{
  filter_input::FilterInput, filter_output::FilterOutput, input::Input, input_kind::InputKind,
  output::Output, output_kind::OutputKind, stream::Stream, Filter, Order, OutputResult::Entry,
  ParameterValue,
};
use crate::probe::deep::{CheckParameterValue, LoudnessResult, StreamProbeResult};
use ffmpeg_sys_next::log10;
use std::collections::HashMap;
use std::option::Option::*;

pub fn create_graph<S: ::std::hash::BuildHasher>(
  filename: &str,
  params: &HashMap<String, CheckParameterValue, S>,
) -> Result<Order, String> {
  let mut inputs = vec![];
  let mut outputs = vec![];
  let mut filters = vec![];

  let metadata_param = ParameterValue::String("true".to_string());
  let peak_param = ParameterValue::String("true".to_string());
  let mut loudnessdetect_params: HashMap<String, ParameterValue> = HashMap::new();
  loudnessdetect_params.insert("metadata".to_string(), metadata_param);
  loudnessdetect_params.insert("peak".to_string(), peak_param);

  match params.get("pairing_list") {
    Some(pairing_list) => {
      if let Some(pairs) = pairing_list.pairs.clone() {
        for (iter, pair) in pairs.iter().enumerate() {
          let mut amerge_params: HashMap<String, ParameterValue> = HashMap::new();
          let aformat_params: HashMap<String, ParameterValue> = HashMap::new();
          let mut amerge_input = vec![];
          let mut aformat_output = vec![];
          let mut input_streams_vec = vec![];
          let mut keys_vec = vec!["lavfi.r128.I".to_string()];
          keys_vec.push("lavfi.r128.LRA".to_string());
          for i in 0..8 {
            let val = format!("lavfi.r128.true_peaks_ch{}", i);
            keys_vec.push(val);
          }
          let output_label = format!("output_label_{:?}", iter);
          aformat_output.push(FilterOutput {
            stream_label: output_label.clone().to_string(),
          });

          for track in pair {
            let input_label = format!("input_label_{}", track.index);
            amerge_input.push(FilterInput {
              kind: InputKind::Stream,
              stream_label: input_label.to_string(),
            });
            input_streams_vec.push(Stream {
              index: track.index as u32,
              label: Some(input_label.to_string()),
            });
          }

          inputs.push(Input::Streams {
            id: iter as u32,
            path: filename.to_string(),
            streams: input_streams_vec.clone(),
          });
          outputs.push(Output {
            kind: Some(OutputKind::AudioMetadata),
            keys: keys_vec,
            stream: Some(output_label.to_string()),
            path: None,
            streams: vec![],
            parameters: HashMap::new(),
          });

          amerge_params.insert(
            "inputs".to_string(),
            ParameterValue::Int64(pair.len() as i64),
          );
          filters.push(Filter {
            name: "amerge".to_string(),
            label: Some("amerge_filter".to_string()),
            parameters: amerge_params.clone(),
            inputs: Some(amerge_input.clone()),
            outputs: None,
          });
          filters.push(Filter {
            name: "ebur128".to_string(),
            label: Some("loudness_filter".to_string()),
            parameters: loudnessdetect_params.clone(),
            inputs: None,
            outputs: None,
          });
          filters.push(Filter {
            name: "aformat".to_string(),
            label: Some("aformat_filter".to_string()),
            parameters: aformat_params.clone(),
            inputs: None,
            outputs: Some(aformat_output),
          });
        }
      }
    }
    None => warn!("No input message for the loudness analysis (list of index to merge)"),
  }
  Order::new(inputs, filters, outputs)
}

pub fn detect_loudness<S: ::std::hash::BuildHasher>(
  filename: &str,
  streams: &mut Vec<StreamProbeResult>,
  params: HashMap<String, CheckParameterValue, S>,
) {
  let mut order = create_graph(filename, &params).unwrap();
  if let Err(msg) = order.setup() {
    error!("{:?}", msg);
    return;
  }
  match order.process() {
    Ok(results) => {
      info!("END OF PROCESS");
      info!("-> {:?} frames processed", results.len());
      let mut context = FormatContext::new(filename).unwrap();
      if let Err(msg) = context.open_input() {
        context.close_input();
        error!("{:?}", msg);
        return;
      }
      for result in results {
        if let Entry(entry_map) = result {
          if let Some(stream_id) = entry_map.get("stream_id") {
            let index: i32 = stream_id.parse().unwrap();
            let mut loudness = LoudnessResult {
              range: -99.9,
              integrated: -99.9,
              true_peaks: vec![],
            };

            if let Some(value) = entry_map.get("lavfi.r128.I") {
              let x = (value.parse::<f64>().unwrap()) as f64;
              loudness.integrated = (x * 100.0).round() / 100.0;
              if loudness.integrated == -70.0 {
                loudness.integrated = -99.0;
              }
            }
            if let Some(value) = entry_map.get("lavfi.r128.LRA") {
              let y = (value.parse::<f64>().unwrap()) as f64;
              loudness.range = (y * 100.0).round() / 100.0;
            }
            let mut str_tpk_key;
            let mut energy;
            let mut tpk;
            for i in 0..8 {
              str_tpk_key = format!("lavfi.r128.true_peaks_ch{}", i);
              if let Some(value) = entry_map.get(&str_tpk_key) {
                energy = value.parse::<f64>().unwrap() as f64;
                unsafe {
                  tpk = 20.0 * log10(energy);
                  tpk = (tpk * 100.0).round() / 100.0;
                  if tpk == std::f64::NEG_INFINITY {
                    tpk = -99.00;
                  }
                  loudness.true_peaks.push(tpk);
                }
              }
            }
            streams[(index) as usize].detected_loudness.drain(..);
            streams[(index) as usize].detected_loudness.push(loudness);
          }
        }
      }
    }
    Err(msg) => {
      error!("ERROR: {}", msg);
    }
  }
}
