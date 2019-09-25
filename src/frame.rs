use stainless_ffmpeg_sys::{av_dict_get, av_frame_free, AVFrame};
use std::ffi::CString;
use std::ptr::null_mut;
use tools;

pub struct Frame {
  pub name: Option<String>,
  pub frame: *mut AVFrame,
}

impl Frame {
  pub fn get_metadata(&self, key: &str) -> Option<String> {
    unsafe {
      let metadata = (*self.frame).metadata;
      let metadata_str = CString::new(key).unwrap();
      let entry = av_dict_get(metadata, metadata_str.as_ptr(), null_mut(), 0);
      if entry.is_null() {
        return None;
      }
      Some(tools::to_string((*entry).value))
    }
  }

  pub fn get_pts(&self) -> i64 {
    unsafe { (*self.frame).pts }
  }
}

impl Drop for Frame {
  fn drop(&mut self) {
    unsafe {
      if !self.frame.is_null() {
        av_frame_free(&mut self.frame);
      }
    }
  }
}
