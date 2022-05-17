use crate::format_context::FormatContext;
use crate::order::{
  filter_input::FilterInput, filter_output::FilterOutput, input::Input, input_kind::InputKind,
  output::Output, output_kind::OutputKind, stream::Stream, Filter, Order, OutputResult::Entry,
  ParameterValue,
};
use crate::probe::deep::{CheckParameterValue, SineResult, StreamProbeResult};
use crate::stream::Stream as ContextStream;
use ffmpeg_sys_next::AVMediaType;
use std::collections::HashMap;

pub fn create_graph(filename: &str, audio_indexes: Vec<u32>) -> Result<Order, String> {
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
    let mut keys_vec = vec![];
    for i in 1..9 {
      let cf = format!("lavfi.astats.{}.Crest_factor", i);
      keys_vec.push(cf);
      let zc = format!("lavfi.astats.{}.Zero_crossings", i);
      keys_vec.push(zc);
    }

    let mut astats_params: HashMap<String, ParameterValue> = HashMap::new();
    astats_params.insert("metadata".to_string(), ParameterValue::Bool(true));
    astats_params.insert("reset".to_string(), ParameterValue::Int64(1));
    let mut aformat_params: HashMap<String, ParameterValue> = HashMap::new();
    let channel_layouts = ParameterValue::String("mono".to_string());
    aformat_params.insert("channel_layouts".to_string(), channel_layouts);

    filters.push(Filter {
      name: "astats".to_string(),
      label: Some(format!("astats_filter{}", i)),
      parameters: astats_params.clone(),
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
      keys: keys_vec,
      stream: Some(output_identifier),
      path: None,
      streams: vec![],
      parameters: HashMap::new(),
    });
  }

  Order::new(inputs, filters, outputs)
}

pub fn detect_sine(
  filename: &str,
  streams: &mut [StreamProbeResult],
  audio_indexes: Vec<u32>,
  params: HashMap<String, CheckParameterValue>,
) {
  let mut order = create_graph(filename, audio_indexes).unwrap();
  if let Err(msg) = order.setup() {
    error!("{:?}", msg);
    return;
  }

  match order.process() {
    Ok(results) => {
      info!("END OF PROCESS");
      info!("-> {:?} frames processed", results.len());
      let mut duration = 0;
      let mut time_base = 1.0;
      let mut frame_rate;
      let mut bits_nb = 0.0; //contains the bits number to code a sample
      let mut sine: SineResult = Default::default(); //contains the start time and the end time of the current sine for the current frame
      let mut last_starts: [[Option<i64>; 8]; 16] = Default::default(); //contains the previous declared start of the corresponding channel and stream (16 possible streams, for 8 possible channels)
      let mut last_crests: [[f64; 8]; 16] = Default::default(); //contains the last crest (from the previous frame) of the corresponding channel and stream
      let mut frames: [[f32; 8]; 16] = Default::default(); //contains the current frame number of the corresponding channel and stream
      let mut zero_cross: [[u32; 8]; 16] = Default::default(); //contains the number of zero crossings of the corresponding channel and stream while there is a sine on the corresponding channel and stream
      let mut max_duration = None;
      let mut min_duration = None;
      if let Some(duration) = params.get("duration") {
        min_duration = duration.min;
        max_duration = duration.max;
      }

      let mut context = FormatContext::new(filename).unwrap();
      if let Err(msg) = context.open_input() {
        context.close_input();
        error!("{:?}", msg);
        return;
      }
      for index in 0..context.get_nb_streams() {
        if let Ok(stream) = ContextStream::new(context.get_stream(index as isize)) {
          if let AVMediaType::AVMEDIA_TYPE_VIDEO = context.get_stream_type(index as isize) {
            let rational_frame_rate = stream.get_frame_rate();
            frame_rate = rational_frame_rate.num as f32 / rational_frame_rate.den as f32;
            if let Some(stream_duration) = stream.get_duration() {
              duration = (stream_duration * 1000.0) as i64;
            } else {
              duration = (results.len() as f32 / frame_rate * 1000.0) as i64;
            }
            time_base = stream.get_time_base();
          }
        }
      }
      for result in results {
        if let Entry(entry_map) = result {
          if let Some(stream_id) = entry_map.get("stream_id") {
            let index: i32 = stream_id.parse().unwrap();

            if let Ok(stream) = ContextStream::new(context.get_stream(index as isize)) {
              if let AVMediaType::AVMEDIA_TYPE_AUDIO = context.get_stream_type(index as isize) {
                let bit_depth = stream.get_bits_per_sample();
                bits_nb = 2_i32.pow(bit_depth as u32) as f64;
              }
            }

            let mut cf_key;
            let mut zc_key;
            for i in 1..9 {
              cf_key = format!("lavfi.astats.{}.Crest_factor", i);
              zc_key = format!("lavfi.astats.{}.Zero_crossings", i);
              frames[(index - 1) as usize][i - 1] += 1.0;
              if let Some(value) = entry_map.get(&cf_key) {
                let crest_factor = bits_nb / (value.parse::<f64>().unwrap());

                /*
                 * We are always working on the current frame of the current channel from the
                 * current index.
                 * If a crest factor of a signal is sqrt(2), that means that the signal is a sine.
                 * We define a start sine, if there is no previous start sine that have been
                 * declared.
                 * We can look for the end of the sine :
                 *      -in the middle of the signal : as soon as we get a crest factor != sqrt(2)
                 *       that means that the previous frame was the end of the sine.
                 *      -at the end of the signal : if we get a crest factor == sqrt(2) until the
                 *       end of the signal, that means that there is a sine until the end.
                 * 1000Hz is a sine with 1000 periods per second. There is 2 zero crossings per
                 * period :
                 * if zero_cross_nb / sine_dur(second) = 2000, then this is a 1000Hz.
                 */

                if 1.4129 < crest_factor && crest_factor < 1.4151 {
                  if last_starts[(index - 1) as usize][i - 1] != None {
                    if (frames[(index - 1) as usize][i - 1] * (time_base * 1000.0)) as i64
                      == duration
                    {
                      if let Some(start) = last_starts[(index - 1) as usize][i - 1] {
                        sine.channel = i as u8;
                        sine.start = start;
                        sine.end =
                          ((frames[(index - 1) as usize][i - 1]) * (time_base * 1000.0)) as i64;
                        if ((zero_cross[(index - 1) as usize][i - 1] + 80) as i64
                          / (sine.end - sine.start))
                          == 2
                        {
                          streams[index as usize].detected_sine.push(sine);
                          last_starts[(index - 1) as usize][i - 1] = None;
                          zero_cross[(index - 1) as usize][i - 1] = 0;
                          if let Some(max) = max_duration {
                            if (sine.end - sine.start) > max as i64 {
                              streams[index as usize].detected_sine.pop();
                            }
                          }
                          if let Some(min) = min_duration {
                            if (sine.end - sine.start) < min as i64 {
                              streams[index as usize].detected_sine.pop();
                            }
                          }
                        }
                      }
                    }
                  } else {
                    sine.start =
                      ((frames[(index - 1) as usize][i - 1] - 1.0) * (time_base * 1000.0)) as i64;
                    last_starts[(index - 1) as usize][i - 1] = Some(sine.start);
                  }
                } else if last_starts[(index - 1) as usize][i - 1] != None
                  && 1.4129 < last_crests[(index - 1) as usize][i - 1]
                  && last_crests[(index - 1) as usize][i - 1] < 1.4151
                {
                  if let Some(start) = last_starts[(index - 1) as usize][i - 1] {
                    sine.channel = i as u8;
                    sine.start = start;
                    sine.end =
                      ((frames[(index - 1) as usize][i - 1] - 1.0) * (time_base * 1000.0)) as i64;

                    if (zero_cross[(index - 1) as usize][i - 1] as i64 / (sine.end - sine.start))
                      == 2
                    {
                      streams[index as usize].detected_sine.push(sine);
                      last_starts[(index - 1) as usize][i - 1] = None;
                      zero_cross[(index - 1) as usize][i - 1] = 0;
                      if let Some(max) = max_duration {
                        if (sine.end - sine.start) < max as i64 {
                          streams[index as usize].detected_sine.pop();
                        }
                      }
                      if let Some(min) = min_duration {
                        if (sine.end - sine.start) < min as i64 {
                          streams[index as usize].detected_sine.pop();
                        }
                      }
                    }
                  }
                }

                last_crests[(index - 1) as usize][i - 1] = crest_factor;
                if last_starts[(index - 1) as usize][i - 1] != None {
                  if let Some(value) = entry_map.get(&zc_key) {
                    zero_cross[(index - 1) as usize][i - 1] +=
                      (value.parse::<f32>().unwrap()) as u32;
                  }
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
