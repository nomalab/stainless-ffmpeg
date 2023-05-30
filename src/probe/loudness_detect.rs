use crate::format_context::FormatContext;
use crate::order::{
  filter_input::FilterInput, filter_output::FilterOutput, input::Input, input_kind::InputKind,
  output::Output, output_kind::OutputKind, stream::Stream, Filter, Order, OutputResult::Entry,
  ParameterValue,
};
use crate::probe::deep::{CheckParameterValue, LoudnessResult, StreamProbeResult};
use ffmpeg_sys_next::log10;
use std::collections::HashMap;

pub fn create_graph<S: ::std::hash::BuildHasher>(
  filename: &str,
  params: &HashMap<String, CheckParameterValue, S>,
) -> Result<Order, String> {
  let mut inputs = vec![];
  let mut outputs = vec![];
  let mut filters = vec![];

  let metadata_param = ParameterValue::Bool(true);
  let peak_param = ParameterValue::String("true".to_string());
  let mut loudnessdetect_params: HashMap<String, ParameterValue> = HashMap::new();
  loudnessdetect_params.insert("metadata".to_string(), metadata_param);
  loudnessdetect_params.insert("peak".to_string(), peak_param);

  match params.get("pairing_list") {
    Some(pairing_list) => {
      if let Some(pairs) = pairing_list.pairs.clone() {
        for (iter, pair) in pairs.iter().enumerate() {
          let mut amerge_params: HashMap<String, ParameterValue> = HashMap::new();
          let mut amerge_input = vec![];
          let mut input_streams_vec = vec![];
          let mut lavfi_keys = vec!["lavfi.r128.I".to_string(), "lavfi.r128.LRA".to_string()];
          let output_label = format!("output_label_{iter:?}");

          for track in pair {
            let input_label = format!("input_label_{}", track.index);
            amerge_input.push(FilterInput {
              kind: InputKind::Stream,
              stream_label: input_label.clone(),
            });
            input_streams_vec.push(Stream {
              index: track.index as u32,
              label: Some(input_label),
            });
          }

          let channel: u8 = if pair.len() == 1 {
            pair[0].channel
          } else {
            pair.len() as u8
          };
          for ch in 0..channel {
            let key = format!("lavfi.r128.true_peaks_ch{ch}");
            if !lavfi_keys.contains(&key) {
              lavfi_keys.push(key);
            }
          }

          inputs.push(Input::Streams {
            id: iter as u32,
            path: filename.to_string(),
            streams: input_streams_vec,
          });
          outputs.push(Output {
            kind: Some(OutputKind::AudioMetadata),
            keys: lavfi_keys,
            stream: Some(output_label.clone()),
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
            label: Some(format!("amerge_filter_{iter:?}")),
            parameters: amerge_params,
            inputs: Some(amerge_input),
            outputs: None,
          });
          filters.push(Filter {
            name: "ebur128".to_string(),
            label: Some(format!("loudness_filter_{iter:?}")),
            parameters: loudnessdetect_params.clone(),
            inputs: None,
            outputs: None,
          });
          filters.push(Filter {
            name: "aformat".to_string(),
            label: Some(format!("aformat_filter_{iter:?}")),
            parameters: HashMap::new(),
            inputs: None,
            outputs: Some(vec![FilterOutput {
              stream_label: output_label,
            }]),
          });
        }
      }
    }
    None => {
      return Err("No input message for the loudness analysis (audio qualification)".to_string())
    }
  }
  Order::new(inputs, filters, outputs)
}

pub fn detect_loudness<S: ::std::hash::BuildHasher>(
  filename: &str,
  streams: &mut [StreamProbeResult],
  audio_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue, S>,
) {
  match create_graph(filename, &params) {
    Ok(mut order) => {
      if let Err(msg) = order.setup() {
        error!("{:?}", msg);
        return;
      }
      for index in audio_indexes {
        streams[index as usize].detected_loudness = Some(vec![]);
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
                if streams[(index) as usize].detected_loudness.is_none() {
                  error!("Error : unexpected detection on stream ${index}");
                  break;
                }
                let detected_loudness = streams[(index) as usize]
                  .detected_loudness
                  .as_mut()
                  .unwrap();
                let mut loudness = LoudnessResult {
                  range: -99.9,
                  integrated: -99.9,
                  true_peaks: vec![],
                };
                let mut channel_start = 0;
                let mut channel_end = 0;

                if let Some(value) = entry_map.get("lavfi.r128.I") {
                  let x = value.parse::<f64>().unwrap();
                  loudness.integrated = (x * 100.0).round() / 100.0;
                  if loudness.integrated == -70.0 {
                    loudness.integrated = -99.0;
                  }
                }
                if let Some(value) = entry_map.get("lavfi.r128.LRA") {
                  let y = value.parse::<f64>().unwrap();
                  loudness.range = (y * 100.0).round() / 100.0;
                }

                match params.get("pairing_list") {
                  Some(pairing_list) => {
                    if let Some(pairs) = &pairing_list.pairs {
                      for pair in pairs {
                        for (pos, track) in pair.iter().enumerate() {
                          if index == track.index as i32 {
                            if pair.len() == 1 {
                              channel_start = 0;
                              channel_end = track.channel;
                            } else {
                              channel_start = pos as u8;
                              channel_end = (pos + 1) as u8;
                            }
                          }
                        }
                      }
                    }
                  }
                  None => warn!("No input message for the loudness analysis (audio qualification)"),
                }
                for i in channel_start..channel_end {
                  let str_tpk_key = format!("lavfi.r128.true_peaks_ch{i}");
                  if let Some(value) = entry_map.get(&str_tpk_key) {
                    let energy = value.parse::<f64>().unwrap();
                    unsafe {
                      let mut tpk = 20.0 * log10(energy);
                      tpk = (tpk * 100.0).round() / 100.0;
                      if tpk == std::f64::NEG_INFINITY {
                        tpk = -99.00;
                      }
                      loudness.true_peaks.push(tpk);
                    }
                  }
                }
                detected_loudness.drain(..);
                detected_loudness.push(loudness);
              }
            }
          }
        }
        Err(msg) => {
          error!("ERROR: {}", msg);
        }
      }
    }
    Err(error) => error!("{:?}", error),
  }
}
