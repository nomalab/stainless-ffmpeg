
use stainless_ffmpeg_sys::*;
use libc::c_void;
use std::collections::HashMap;
use std::ffi::CString;
use tools;
use tools::rational::Rational;

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ParameterValue {
  Bool(bool),
  Int64(i64),
  Float(f64),
  Rational(Rational),
  String(String),
  ChannelLayout(u64)
}

pub fn set_parameters(context: *mut c_void, parameters: &HashMap<String, ParameterValue>) -> Result<(), String> {
  for (key, value) in parameters {
    value.set(key, context)?;
  }
  Ok(())
}

impl ParameterValue {
  pub fn set(&self, key: &str, context: *mut c_void) -> Result<(), String> {
    match self {
      ParameterValue::Bool(data) => self.set_int_parameter(context, &key, *data as i64),
      ParameterValue::Int64(data) => self.set_int_parameter(context, &key, *data),
      ParameterValue::Float(data) => self.set_float_parameter(context, &key, *data),
      ParameterValue::Rational(data) => self.set_rational_parameter(context, &key, data.num, data.den),
      ParameterValue::String(data) => self.set_str_parameter(context, &key, &data),
      ParameterValue::ChannelLayout(data) => {
        let mut ch_layout = [0i8; 64];
        unsafe {
          av_get_channel_layout_string(ch_layout.as_mut_ptr(), 64, 0, *data);
          self.set_parameter(context, &key, ch_layout.as_ptr())
        }
      },
    }
  }
  
  unsafe fn set_parameter(&self, context: *mut c_void, key: &str, value: *const i8) -> Result<(), String> {
    let key_str = CString::new(key).unwrap();
    check_result!(av_opt_set(
      context as *mut c_void,
      key_str.as_ptr(),
      value,
      AV_OPT_SEARCH_CHILDREN
    ));
    Ok(())
  }
  
  fn set_str_parameter(&self, context: *mut c_void, key: &str, value: &str) -> Result<(), String> {
    let key_str = CString::new(key).unwrap();
    let value_str = CString::new(value).unwrap();
    unsafe {
      check_result!(av_opt_set(
        context as *mut c_void,
        key_str.as_ptr(),
        value_str.as_ptr(),
        AV_OPT_SEARCH_CHILDREN
      ));
    }
    Ok(())
  }

  fn set_int_parameter(&self, context: *mut c_void, key: &str, value: i64) -> Result<(), String> {
    let key_str = CString::new(key).unwrap();
    unsafe {
      check_result!(av_opt_set_int(
        context as *mut c_void,
        key_str.as_ptr(),
        value,
        AV_OPT_SEARCH_CHILDREN
      ));
    }
    Ok(())
  }

  fn set_float_parameter(&self, context: *mut c_void, key: &str, value: f64) -> Result<(), String> {
    let key_str = CString::new(key).unwrap();
    unsafe {
      check_result!(av_opt_set_double(
        context as *mut c_void,
        key_str.as_ptr(),
        value,
        AV_OPT_SEARCH_CHILDREN
      ));
    }
    Ok(())
  }

  fn set_rational_parameter(&self, context: *mut c_void, key: &str, num: i32, den: i32) -> Result<(), String> {
    let key_str = CString::new(key).unwrap();
    let rational = AVRational { num, den };

    unsafe {
      check_result!(av_opt_set_q(
        context as *mut c_void,
        key_str.as_ptr(),
        rational,
        AV_OPT_SEARCH_CHILDREN
      ));
    }
    Ok(())
  }
}
