use crate::order::{
  filter_input::FilterInput, filter_output::FilterOutput, input::Input, input_kind::InputKind,
  output::Output, output_kind::OutputKind, stream::Stream, Filter, Order, OutputResult::Entry,
  ParameterValue,
};
use crate::probe::deep::{CheckParameterValue, OcrResult, StreamProbeResult};
use std::collections::HashMap;

pub fn create_graph<S: ::std::hash::BuildHasher>(
  filename: &str,
  video_indexes: Vec<u32>,
  params: &HashMap<String, CheckParameterValue, S>,
) -> Result<Order, String> {
  let mut filters = vec![];
  let mut inputs = vec![];
  let mut outputs = vec![];

  for i in video_indexes {
    let input_identifier = format!("video_input_{i}");
    let output_identifier = format!("video_output_{i}");

    let ocrdetect_params: HashMap<String, ParameterValue> = HashMap::new();
    let mut scdet_params: HashMap<String, ParameterValue> = HashMap::new();
    if let Some(th) = params.get("threshold").and_then(|threshold| threshold.th) {
      scdet_params.insert("threshold".to_string(), ParameterValue::Float(th));
    }
    scdet_params.insert("sc_pass".to_string(), ParameterValue::Int64(1));

    let input_streams = vec![Stream {
      index: i,
      label: Some(input_identifier.clone()),
    }];

    filters.push(Filter {
      name: "scdet".to_string(),
      label: Some(format!("scdet_filter{i}")),
      parameters: scdet_params,
      inputs: Some(vec![FilterInput {
        kind: InputKind::Stream,
        stream_label: input_identifier,
      }]),
      outputs: None,
    });
    filters.push(Filter {
      name: "ocr".to_string(),
      label: Some(format!("ocrdetect_filter{i}")),
      parameters: ocrdetect_params.clone(),
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
      kind: Some(OutputKind::VideoMetadata),
      keys: vec![
        "lavfi.ocr.text".to_string(),
        "lavfi.ocr.confidence".to_string(),
        "lavfi.scd.time".to_string(),
      ],
      stream: Some(output_identifier),
      path: None,
      streams: vec![],
      parameters: HashMap::new(),
    });
  }
  Order::new(inputs, filters, outputs)
}

pub fn detect_ocr<S: ::std::hash::BuildHasher>(
  filename: &str,
  streams: &mut [StreamProbeResult],
  video_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue, S>,
  frame_rate: f32,
  stream_frames: Option<i64>,
) {
  let mut order = create_graph(filename, video_indexes.clone(), &params).unwrap();
  if let Err(msg) = order.setup() {
    error!("{:?}", msg);
    return;
  }
  for index in video_indexes {
    streams[index as usize].detected_ocr = Some(vec![]);
  }

  match order.process() {
    Ok(results) => {
      info!("END OF PROCESS");
      info!("-> {:?} frames processed", results.len());
      let mut media_offline_detected = false;
      let last_frame = stream_frames.unwrap_or(results.len() as i64) - 1;

      for result in results {
        if let Entry(entry_map) = result {
          if let Some(stream_id) = entry_map.get("stream_id") {
            let index: i32 = stream_id.parse().unwrap();
            if streams[(index) as usize].detected_ocr.is_none() {
              error!("Error : unexpected detection on stream ${index}");
              break;
            }
            let detected_ocr = streams[(index) as usize].detected_ocr.as_mut().unwrap();
            let mut ocr = OcrResult {
              frame_start: 0,
              frame_end: last_frame as u64,
              text: "".to_string(),
              word_confidence: "".to_string(),
            };

            if media_offline_detected {
              if let Some(last_detect) = detected_ocr.last_mut() {
                if let Some(value) = entry_map.get("lavfi.scd.time") {
                  last_detect.frame_end = (value.parse::<f32>().unwrap() * frame_rate - 1.0) as u64;
                  media_offline_detected = false;
                }
              }
            }
            if let Some(value) = entry_map.get("lavfi.ocr.text") {
              if value.starts_with("MEDIA OFFLINE") || value.starts_with("OFFLINE") {
                media_offline_detected = true;
                ocr.text = value.to_string();
                if let Some(value) = entry_map.get("lavfi.scd.time") {
                  ocr.frame_start = (value.parse::<f32>().unwrap() * frame_rate) as u64;
                }
                if let Some(value) = entry_map.get("lavfi.ocr.confidence") {
                  let mut word_conf = value.to_string().replace(char::is_whitespace, "%,");
                  word_conf.pop();
                  ocr.word_confidence = word_conf;
                  detected_ocr.push(ocr);
                }
              }
            }
          }
        }
      }
    }
    Err(msg) => {
      error!("ERROR: {}", msg);
    }
  }
}
