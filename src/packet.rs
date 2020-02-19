use stainless_ffmpeg_sys::{av_packet_free, AVPacket};

pub struct Packet {
  pub name: Option<String>,
  pub packet: *mut AVPacket,
}

impl Packet {
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
