use stainless_ffmpeg_sys::{
  AVPacket,
  av_init_packet,
  av_packet_alloc,
  av_packet_free
};
use std::ptr::null_mut;

pub struct Packet {
  pub name: Option<String>,
  pub packet: *mut AVPacket,
}

impl Packet {
  pub fn new() -> Self {
    unsafe {
      let packet = av_packet_alloc();
      av_init_packet(packet);
      (*packet).data = null_mut();
      (*packet).size = 0;
      Packet { name: None, packet }
    }
  }

  pub fn get_stream_index(&self) -> isize {
    if self.packet.is_null() {
      return 0;
    }
    unsafe { (*self.packet).stream_index as isize }
  }
}

impl Drop for Packet {
  fn drop(&mut self) {
    unsafe {
      if !self.packet.is_null() {
        av_packet_free(&mut self.packet);
      }
    }
  }
}
