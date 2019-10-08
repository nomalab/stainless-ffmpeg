use std::collections::HashMap;
use packet::Packet;

pub enum OutputResult {
  Entry(HashMap<String, String>),
  Packet(Packet),
  ProcessStatistics{
    decoded_audio_frames: usize,
    decoded_video_frames: usize,
    encoded_audio_frames: usize,
    encoded_video_frames: usize,
  }
}
