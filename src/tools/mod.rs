use rand::prelude::SliceRandom;
use rand::thread_rng;
use stainless_ffmpeg_sys::{avcodec_find_encoder_by_name, AVCodec, AVMediaType};
use std::ffi::CStr;
use std::ffi::CString;
use std::ptr;
use std::str::from_utf8_unchecked;

pub mod rational;

pub unsafe fn from_buf_raw<T>(ptr: *const T, elts: usize) -> Vec<T> {
  let mut dst = Vec::with_capacity(elts);
  dst.set_len(elts);
  ptr::copy(ptr, dst.as_mut_ptr(), elts);
  dst
}

static ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

pub unsafe fn to_string(data: *const i8) -> String {
  if data.is_null() {
    return "".to_string();
  }
  from_utf8_unchecked(CStr::from_ptr(data).to_bytes()).to_string()
}

pub fn random_string(length: usize) -> String {
  let mut result = vec![];
  let mut rng = thread_rng();

  for _ in 0..length {
    let letter = ALPHABET.choose(&mut rng).unwrap();
    result.push(letter.clone());
  }
  String::from_utf8(result).unwrap()
}

pub fn get_codec(codec_name: &str) -> *mut AVCodec {
  unsafe {
    let cs_codec_name = CString::new(codec_name).unwrap();
    avcodec_find_encoder_by_name(cs_codec_name.as_ptr())
  }
}

pub fn get_codec_type(codec_name: &str) -> Option<AVMediaType> {
  unsafe {
    let cs_codec_name = CString::new(codec_name).unwrap();
    let codec = avcodec_find_encoder_by_name(cs_codec_name.as_ptr());
    if codec.is_null() {
      return None;
    }
    Some((*codec).type_)
  }
}
