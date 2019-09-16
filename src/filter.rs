use ffmpeg_sys::AVOptionType::*;
use ffmpeg_sys::*;
use std::ffi::CString;
use std::fmt;
use std::ptr::null_mut;
use tools;

#[derive(Debug, PartialEq)]
pub struct Filter {
  pub context: *mut AVFilterContext,
}

impl Filter {
  pub unsafe fn new(filter_graph: *mut AVFilterGraph, plugin_name: &str) -> Result<Self, String> {
    Filter::new_with_label(filter_graph, plugin_name, "")
  }

  pub unsafe fn new_with_label(
    filter_graph: *mut AVFilterGraph,
    plugin_name: &str,
    instance_name: &str,
  ) -> Result<Self, String> {
    let label = CString::new(plugin_name).unwrap();
    let filter = avfilter_get_by_name(label.as_ptr());
    if filter.is_null() {
      return Err(format!(
        "Could not find the {} filter",
        label.into_string().unwrap()
      ));
    }

    let context = if instance_name == "" {
      avfilter_graph_alloc_filter(filter_graph, filter, null_mut())
    } else {
      let i_name = CString::new(instance_name).unwrap();
      avfilter_graph_alloc_filter(filter_graph, filter, i_name.as_ptr())
    };

    if context.is_null() {
      return Err(format!(
        "Could not allocate the {} instance",
        label.into_string().unwrap()
      ));
    }

    Ok(Filter { context })
  }

  pub fn get_label(&self) -> String {
    unsafe {
      if (*self.context).name.is_null() {
        "".to_string()
      } else {
        tools::to_string((*self.context).name)
      }
    }
  }

  pub fn init(&self) -> Result<(), String> {
    unsafe {
      check_result!(avfilter_init_str(self.context, null_mut()));
    }
    Ok(())
  }
}

fn dump_option(
  filter: *mut AVFilterContext,
  class: *const AVClass,
  prev_opt: Option<*const AVOption>,
  f: &mut fmt::Formatter,
) -> Option<*const AVOption> {
  unsafe {
    let option = if let Some(po) = prev_opt {
      av_opt_next(class as *mut _, po)
    } else {
      (*class).option
    };

    if option.is_null() {
      return None;
    }

    let option_name = tools::to_string((*option).name);

    let option_help = "".to_owned();
    /*  if (*option).help.is_null() {
      "".to_owned()
    }
    else {
      "\n\t".to_owned() + tools::to_string((*option).help)
    };*/

    let option_unit = if (*option).unit.is_null()
      || (*option).type_ == AV_OPT_TYPE_CONST
      || (*option).type_ == AV_OPT_TYPE_FLAGS
    {
      "".to_owned()
    } else {
      " ".to_owned() + &tools::to_string((*option).unit)
    };

    let option_value = match (*option).type_ {
      AV_OPT_TYPE_BOOL => {
        let value = 0i64;
        let value_ptr: *const i64 = &value;
        av_opt_get_int(
          filter as *mut _,
          (*option).name,
          AV_OPT_SEARCH_CHILDREN,
          value_ptr as *mut _,
        );
        if value == 0 {
          "false".to_owned()
        } else {
          "true".to_owned()
        }
      }
      AV_OPT_TYPE_INT => {
        let value = 0i64;
        let value_ptr: *const i64 = &value;
        av_opt_get_int(
          filter as *mut _,
          (*option).name,
          AV_OPT_SEARCH_CHILDREN,
          value_ptr as *mut _,
        );
        format!("{}", value)
      }
      AV_OPT_TYPE_INT64 => {
        let value = 0i64;
        let value_ptr: *const i64 = &value;
        av_opt_get_int(
          filter as *mut _,
          (*option).name,
          AV_OPT_SEARCH_CHILDREN,
          value_ptr as *mut _,
        );
        format!("{}", value)
      }
      AV_OPT_TYPE_DOUBLE => {
        let value = 0f64;
        let value_ptr: *const f64 = &value;
        av_opt_get_double(
          filter as *mut _,
          (*option).name,
          AV_OPT_SEARCH_CHILDREN,
          value_ptr as *mut _,
        );
        format!("{}", value)
      }
      AV_OPT_TYPE_RATIONAL => {
        let rational = AVRational { num: 0, den: 0 };
        let value_ptr: *const _ = &rational;
        av_opt_get_q(
          filter as *mut _,
          (*option).name,
          AV_OPT_SEARCH_CHILDREN,
          value_ptr as *mut _,
        );
        format!("{}/{}", rational.num, rational.den)
      }
      // AV_OPT_TYPE_BINARY |
      AV_OPT_TYPE_STRING => {
        let data = av_malloc(512);
        let value_ptr: *const _ = &data;
        av_opt_get(
          filter as *mut _,
          (*option).name,
          AV_OPT_SEARCH_CHILDREN,
          value_ptr as *mut *mut _,
        );

        let msg = tools::to_string(data as *const i8);
        msg.to_string()
      }
      AV_OPT_TYPE_SAMPLE_FMT => {
        let format = AVSampleFormat::AV_SAMPLE_FMT_NONE;
        let value_ptr: *const _ = &format;
        av_opt_get_sample_fmt(
          filter as *mut _,
          (*option).name,
          AV_OPT_SEARCH_CHILDREN,
          value_ptr as *mut _,
        );

        let sample_fmt = av_get_sample_fmt_name(format);
        tools::to_string(sample_fmt).to_string()
      }
      AV_OPT_TYPE_IMAGE_SIZE => {
        let width = 0i32;
        let height = 0i32;
        let width_ptr: *const _ = &width;
        let height_ptr: *const _ = &height;
        av_opt_get_image_size(
          filter as *mut _,
          (*option).name,
          AV_OPT_SEARCH_CHILDREN,
          width_ptr as *mut _,
          height_ptr as *mut _,
        );

        format!("{}x{}", width, height)
      }
      _ => {
        // println!("{:?} {:?}", option_name, (*option).type_ );
        "".to_owned()
      }
    };

    let min_max = if (*option).type_ == AV_OPT_TYPE_INT || (*option).type_ == AV_OPT_TYPE_INT64 {
      format!("(min: {}, max: {}) ", (*option).min, (*option).max)
    } else {
      "".to_owned()
    };

    let ret = writeln!(
      f,
      " | {} = {}{} {} {}",
      option_name, option_value, option_unit, min_max, option_help
    );

    if ret.is_err() {
      return None;
    }

    Some(option)
  }
}

fn dump_options(filter: *mut AVFilterContext, class: *const AVClass, f: &mut fmt::Formatter) {
  let mut next = dump_option(filter, class, None, f);
  if next.is_none() {
    return;
  }

  loop {
    next = dump_option(filter, class, next, f);
    if next == None {
      break;
    }
  }
}

fn dump_link(
  pad_name: *const i8,
  link: *mut AVFilterLink,
  mode: &str,
  is_input: bool,
  f: &mut fmt::Formatter,
) -> fmt::Result {
  unsafe {
    let input_name = tools::to_string(pad_name);
    if link.is_null() {
      writeln!(f, "{} {}: not connected", mode, input_name)?;

      return Ok(());
    }

    let context = if is_input { (*link).src } else { (*link).dst };

    let node_label = if (*context).name.is_null() {
      "".to_string()
    } else {
      tools::to_string((*context).name)
    };

    let node_name = tools::to_string((*(*context).filter).name);

    write!(
      f,
      "{} {}: {} ({}) ",
      mode, input_name, node_name, &node_label
    )?;
    let pad = if is_input {
      avfilter_pad_get_name((*link).srcpad, 0)
    } else {
      avfilter_pad_get_name((*link).dstpad, 0)
    };
    let pad_type = avfilter_pad_get_type((*link).srcpad, 0);

    let str_pad = tools::to_string(pad);
    writeln!(f, "[{} | {:?}]", str_pad, pad_type)?;
  }
  Ok(())
}

impl fmt::Display for Filter {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    unsafe {
      let filter = self.context;
      let filter_label = self.get_label();

      let filter_name = tools::to_string((*(*filter).filter).name);
      writeln!(f, "{} ({})", filter_name, filter_label)?;

      dump_options(filter, (*filter).av_class, f);
      if ! (*(*filter).filter).priv_class.is_null() {
        dump_options(filter, (*(*filter).filter).priv_class, f);
      }

      let input_links = tools::from_buf_raw((*filter).inputs, (*filter).nb_inputs as usize);
      for (index, input_link) in input_links.iter().enumerate() {
        let name = avfilter_pad_get_name((*filter).input_pads, index as i32);
        dump_link(name, *input_link, ">-", true, f)?;
      }

      let output_links = tools::from_buf_raw((*filter).outputs, (*filter).nb_outputs as usize);
      for (index, output_link) in output_links.iter().enumerate() {
        let name = avfilter_pad_get_name((*filter).output_pads, index as i32);
        dump_link(name, *output_link, "->", false, f)?;
      }
    }
    Ok(())
  }
}
