use crate::format_context::FormatContext;
use crate::order::output::OutputStream;
use crate::order::{
  filter_input::FilterInput, filter_output::FilterOutput, input::Input, input_kind::InputKind,
  output::Output, output_kind::OutputKind, stream::Stream, Filter, Order, OutputResult::Entry,
  ParameterValue,
};
use crate::probe::deep::{CheckParameterValue, LoudnessResult, StreamProbeResult};
use std::collections::HashMap;
use std::option::Option::*;
use ffmpeg_sys::log10;



pub fn create_graph_amerge<S: ::std::hash::BuildHasher>(
  filename: &str,
  _audio_indexes: Vec<u32>,
  _params: &HashMap<String, CheckParameterValue, S>,
) -> Result<Order, String> {

  let mut inputs = vec![];
  let mut input_streams_vec = vec![];
  let mut filters = vec![];
  let mut filters_input_vec = vec![];
  let mut filters_output_vec = vec![];
  let mut outputs = vec![];
  let mut output_streams_vec = vec![];
  let mut input_label;
  let mut output_label = String::from("output_label_init");
  let metadata_param = ParameterValue::String("true".to_string());
  let peak_param = ParameterValue::String("true".to_string());
  let mut loudnessdetect_params: HashMap<String, ParameterValue> = HashMap::new();
  loudnessdetect_params.insert("metadata".to_string(), metadata_param);
  loudnessdetect_params.insert("peak".to_string(), peak_param);
  let mut amerge_params: HashMap<String, ParameterValue> = HashMap::new();
  let mut input_channels = ParameterValue::Int64(2);
  let mut aformat_params: HashMap<String, ParameterValue> = HashMap::new();
  let mut channel_layouts = ParameterValue::String("mono".to_string()); //mono/stereo/5.1/7.1
  let mut output_params: HashMap<String, ParameterValue> = HashMap::new();
  let sample_fmts = ParameterValue::String("s16".to_string());
  let sample_rates = ParameterValue::String("48000".to_string());
  output_params.insert("sample_fmt".to_string(), sample_fmts);
  output_params.insert("sample_rate".to_string(), sample_rates);

  let mut pairing = vec![vec![]];
  pairing.drain(..);
  pairing.push([1,3,5,9,11,12,4,8].to_vec());
  pairing.push([2,16].to_vec());
  pairing.push([15].to_vec());
  
  for i in 0..(pairing.len()) {

    if pairing[i].len() > 1 {

      filters_input_vec.drain(..);
      input_streams_vec.drain(..);
      filters_output_vec.drain(..);

      if pairing[i].len() == 1 {
        input_channels = ParameterValue::Int64(1); //amerge parameter
        channel_layouts = ParameterValue::String("mono".to_string()); //aformat parameter
        for  k in pairing[i].iter() { //filter inputs and outputs definition from a list of vector
          input_label = format!("input_label_{}", k);
          filters_input_vec.push(
            FilterInput {
              kind: InputKind::Stream,
              stream_label: input_label.to_string()
            }
          );
          output_label = format!("output_label_{}", k);
        }
      }
      
      else if pairing[i].len() == 2 {
        input_channels = ParameterValue::Int64(2);
        channel_layouts = ParameterValue::String("stereo".to_string());
        for  k in pairing[i].iter() {
          input_label = format!("input_label_{}", k);
          filters_input_vec.push(
            FilterInput {
              kind: InputKind::Stream,
              stream_label: input_label.to_string()
            }
          );
          output_label = format!("output_label_{}", k);
        }
      }
      
      else if pairing[i].len() == 6 {
        input_channels = ParameterValue::Int64(6);
        channel_layouts = ParameterValue::String("5.1".to_string());
        for  k in pairing[i].iter() {
          input_label = format!("input_label_{}", k);
          filters_input_vec.push(
            FilterInput {
              kind: InputKind::Stream,
              stream_label: input_label.to_string()
            }
          );
          output_label = format!("output_label_{}", k);
        }
      }
      
      else if pairing[i].len() == 8 {
        input_channels = ParameterValue::Int64(8);
        channel_layouts = ParameterValue::String("7.1".to_string());
        for  k in pairing[i].iter() {
          input_label = format!("input_label_{}", k);
          filters_input_vec.push(
            FilterInput {
              kind: InputKind::Stream,
              stream_label: input_label.to_string()
            }
          );
          output_label = format!("output_label_{}", k);
        }
      }

      for j in pairing[i].iter() {
        input_label = format!("input_label_{}", j);
        input_streams_vec.push(
          Stream {
            index: *j as u32,
            label: Some(input_label.to_string())
        });
        output_label = format!("output_label_{}", j);
        output_streams_vec.drain(..);
        output_streams_vec.push(
          OutputStream {
            label: Some(output_label.to_string()),
            codec: "".to_string(),
            parameters: output_params.clone(),
        });
      }
      
      filters_output_vec.push(
        FilterOutput {
          stream_label: output_label.clone().to_string()
        }
      );
      
      inputs.push(Input::Streams {
        id: (i+1) as u32,
        path: filename.to_string(),
        streams: input_streams_vec.clone()
      });

      outputs.push(Output {
        kind: Some(OutputKind::AudioMetadata),
        keys: vec![
          "lavfi.r128.I".to_string(),
          "lavfi.r128.LRA".to_string(),
          "lavfi.r128.true_peaks_ch0".to_string(),
          "lavfi.r128.true_peaks_ch1".to_string(),
          "lavfi.r128.true_peaks_ch2".to_string(),
          "lavfi.r128.true_peaks_ch3".to_string(),
          "lavfi.r128.true_peaks_ch4".to_string(),
          "lavfi.r128.true_peaks_ch5".to_string(),
          "lavfi.r128.true_peaks_ch6".to_string(),
          "lavfi.r128.true_peaks_ch7".to_string(),
          "lavfi.r128.true_peaks_ch8".to_string(),
        ],
        stream: Some(output_label.to_string()),
        path: None,
        streams: vec![],
        parameters: HashMap::new(),
      });

      amerge_params.insert("inputs".to_string(), input_channels.clone());
      filters.push(Filter {
        name: "amerge".to_string(),
        label: Some(format!("amerge_filter")),
        parameters: amerge_params.clone(),
        inputs: Some(filters_input_vec.clone()),
        outputs: None,
      });
      filters.push(Filter {
        name: "ebur128".to_string(),
        label: Some(format!("loudness_filter")),
        parameters: loudnessdetect_params.clone(),
        inputs: None,
        outputs: None,
      });
      aformat_params.insert("channel_layouts".to_string(), channel_layouts.clone());
      filters.push(Filter {
        name: "aformat".to_string(),
        label: Some(format!("aformat_filter")),
        parameters: aformat_params.clone(),
        inputs: None,
        outputs: Some(filters_output_vec.clone()),
      });

    }
  
    else {

      filters_input_vec.drain(..);
      input_streams_vec.drain(..);

      for k in pairing[i].iter() {
        let ind = k;
        input_label = format!("input_label_{}", k);
        println!("input_label : {}", input_label);
        output_label = format!("output_label_{}", k);
        println!("output_label : {}", output_label);
        filters_input_vec.push(
          FilterInput {
            kind: InputKind::Stream,
            stream_label: input_label.to_string()
          }
        );
        input_streams_vec.push(
          Stream {
            index: *ind,
            label: Some(input_label.to_string())
        });

        filters.push(Filter {
          name: "ebur128".to_string(),
          label: Some(format!("loudness_filter{}", i)),
          parameters: loudnessdetect_params.clone(),
          inputs: Some(vec![FilterInput {
            kind: InputKind::Stream,
            stream_label: input_label.clone(),
          }]),
          outputs: None,
        });
        channel_layouts = ParameterValue::String("mono".to_string());
        aformat_params.insert("channel_layouts".to_string(), channel_layouts.clone());
        filters.push(Filter {
          name: "aformat".to_string(),
          label: Some(format!("aformat_filter{}", i)),
          parameters: aformat_params.clone(),
          inputs: None,
          outputs: Some(vec![FilterOutput {
            stream_label: output_label.clone(),
          }]),
        });
    
        inputs.push(Input::Streams {
          id: *ind,
          path: filename.to_string(),
          streams: input_streams_vec.clone(),
        });
        outputs.push(Output {
          kind: Some(OutputKind::AudioMetadata),
          keys: vec![
            "lavfi.r128.I".to_string(),
            "lavfi.r128.LRA".to_string(),
            "lavfi.r128.true_peaks_ch0".to_string(),
            "lavfi.r128.true_peaks_ch1".to_string(),
            "lavfi.r128.true_peaks_ch2".to_string(),
            "lavfi.r128.true_peaks_ch3".to_string(),
            "lavfi.r128.true_peaks_ch4".to_string(),
            "lavfi.r128.true_peaks_ch5".to_string(),
            "lavfi.r128.true_peaks_ch6".to_string(),
            "lavfi.r128.true_peaks_ch7".to_string(),
            "lavfi.r128.true_peaks_ch8".to_string(),
          ],
          stream: Some(output_label.clone()),
          path: None,
          streams: vec![],
          parameters: HashMap::new(),
        });
      }

    }

  }

  Order::new(inputs, filters, outputs)
}


pub fn detect_loudness<S: ::std::hash::BuildHasher>(
  filename: &str,
  streams: &mut Vec<StreamProbeResult>,
  audio_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue, S>,
) {

  let mut order = create_graph_amerge(filename, audio_indexes.clone(), &params).unwrap();
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
                  loudness.true_peaks.push((tpk * 100.0).round() / 100.0);
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
