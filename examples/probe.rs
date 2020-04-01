use env_logger::{Builder, Env};
use log::LevelFilter;
use stainless_ffmpeg::probe::*;
use std::env;

fn main() {
  Builder::from_env(Env::default().default_filter_or("debug")).init();

  if let Some(path) = env::args().last() {
    let mut probe = Probe::new(&path);
    probe.process(LevelFilter::Off).unwrap();
    let result = serde_json::to_string(&probe).unwrap();
    println!("{}", result);

    if let Some(format) = probe.format {
      println!("Format : \n{}", format);
    }
  }
}
