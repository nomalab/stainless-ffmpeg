use env_logger::Builder;
use log::LevelFilter;
use stainless_ffmpeg::probe::*;
use std::env;

fn main() {
  let mut builder = Builder::from_default_env();
  builder.init();

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
