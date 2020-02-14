extern crate env_logger;
extern crate stainless_ffmpeg_sys;
#[macro_use]
extern crate log;
extern crate serde_json;
extern crate stainless_ffmpeg;

use env_logger::{Builder, Env};
use stainless_ffmpeg_sys::{
  av_log_set_level,
  AV_LOG_ERROR
};
use std::env;
use std::fs::File;
use std::io::Read;
use stainless_ffmpeg::order::*;
use stainless_ffmpeg::order::OutputResult::Entry;
use stainless_ffmpeg::format_context::FormatContext;
use stainless_ffmpeg_sys::AVMediaType;
use stainless_ffmpeg::stream::Stream;
use order::input::Input;

fn main() {
  Builder::from_env(Env::default().default_filter_or("debug")).init();
  unsafe {
    av_log_set_level(AV_LOG_ERROR);
  }

  if let Some(path) = env::args().last() {
    let mut file = File::open(&path).unwrap();
    let mut message = String::new();
    file.read_to_string(&mut message).unwrap();

    let mut order = Order::new_parse(&message).unwrap();
    if let Err(msg) = order.setup() {
      error!("{:?}", msg);
      return;
    }
  
    let index = 0;
    let mut context = FormatContext::new(&filename).unwrap();
    if let Err(msg) = context.open_input() {
      context.close_input();
      error!("{:?}", msg);
      return;
    }
    if let Ok(stream) = Stream::new(context.get_stream(index as isize)) {
      match context.get_stream_type(index as isize) {
        AVMediaType::AVMEDIA_TYPE_VIDEO => {
          let frame_rate = stream.get_frame_rate();
          println!("FRAME RATE == {:?}",  );
        },
        _ => {}
      }
    }
    let mut value_start: f64 = 0.0;
    let mut value_end: f64 = 0.0;
    let mut framerate = 50.0;
    let mut silence_duration = 0.0;
    let mut program_loudness = 0.0;
    let mut duration_end: f64 = 0.0;
    match order.process() {
      Ok(results) => {
        info!("END OF PROCESS");
        info!("-> {:?} frames processed", results.len());
        for result in results {
          match result {
            Entry(entry_map) => {
              /* SILENCE DETECT */
              let duration_image: f64  = 1.0 / framerate ; 
              if let Some(value) = entry_map.get("lavfi.silence_duration") {
                silence_duration = value.parse::<f64>().unwrap();
                duration_end = duration_image * 2.0 ;
              }
              if let Some(value) = entry_map.get("lavfi.r128.I") {
                program_loudness = value.parse::<f64>().unwrap();
              }
              if duration_image < silence_duration &&  duration_end > silence_duration {
                if let Some(value) = entry_map.get("lavfi.silence_start") {
                info!("Silence Detect : le silence commence : {} et la durée du silence est de : {} avec un Program Loundness de {}", value, silence_duration, program_loudness);
               }
              } 

             /* BLACK DETECT */
              
              if let Some(value) = entry_map.get("lavfi.black_start") {
                 value_start = value.parse::<f64>().unwrap();
              }
              if let Some(value) = entry_map.get("lavfi.black_end") {
                 value_end = value.parse::<f64>().unwrap();
                 let mut framerate: f64 = framerate / 2.0 ;
                 let image_start = value_start/framerate;
                 let image_end = value_end/framerate;
                 let difference_image =  image_end - image_start  ;
                 let result_start_seconde: f64 = image_start / framerate;
                if difference_image == 1.0
                {
                  info!( "Black Detect : 1 image en noir détectée à : {} secondes de la vidéo et débute à l'image n° {} et finit à l'image n° {} ", result_start_seconde ,image_start, image_end);
                }
              }
              /* BLACKFRAME */
              if let Some(value) = entry_map.get("lavfi.type") {
                info!("Black  :n° {} frames", value);
             }
               /* CROPDETECT */ 
             let mut height: f64 = 0.0;
             let mut width: f64 = 0.0;  
             let mut coordonne_x: f64 = 0.0;   
             let mut coordonne_y: f64 = 0.0;         
             let video_duration: f64 = 40.0;    
             let first_time: f64 = video_duration / 4.0;
             let second_time: f64 = first_time * 2.0;
             let third_time: f64 = first_time * 3.0;
             let ratio: f64 = 0.0;
              
              if let Some(value) = entry_map.get("lavfi.cropdetect.h") {
                height = value.parse::<f64>().unwrap();
              }
              if let Some(value) = entry_map.get("lavfi.cropdetect.w") {
                width = value.parse::<f64>().unwrap();
              }
              if let Some(value) = entry_map.get("lavfi.cropdetect.x") {
                coordonne_x = value.parse::<f64>().unwrap();
              }  
              if let Some(value) = entry_map.get("lavfi.cropdetect.y") {
                coordonne_y = value.parse::<f64>().unwrap();
              }
              let result_ratio = height / width ;

              if result_ratio != ratio && result_ratio != ratio && ratio != 0.0 {
                info!("Cropdetect : Mauvais ratio détecté :   le ratio doit être : ", );
              }
              if coordonne_x != 0.0 && coordonne_y != 0.0 {
                info!("Cropdetect : Bande noir détecté   Pixel en x =  Pixel en y = ",  );
              }             
            },
            _ => {},
          }
        }   
      }
      Err(msg) => {
        error!("ERROR: {}", msg);
      }
    }
  }
}