fn main() {
  let avcodec_version_major = ffmpeg_sys_next::LIBAVCODEC_VERSION_MAJOR;
  let avcodec_version_minor = ffmpeg_sys_next::LIBAVCODEC_VERSION_MINOR;
  let avcodec_version_micro = ffmpeg_sys_next::LIBAVCODEC_VERSION_MICRO;

  let ffmpeg_version = match avcodec_version_major {
    57 => {
      if avcodec_version_minor >= 107 {
        Some("ffmpeg_3_4")
      } else if avcodec_version_minor >= 89 {
        Some("ffmpeg_3_3")
      } else if avcodec_version_minor >= 64 {
        Some("ffmpeg_3_2")
      } else if avcodec_version_minor >= 48 {
        Some("ffmpeg_3_1")
      } else if avcodec_version_minor >= 24 {
        Some("ffmpeg_3_0")
      } else {
        None
      }
    }
    58 => {
      if avcodec_version_minor >= 100 {
        Some("ffmpeg_4_4")
      } else if avcodec_version_minor >= 91 {
        Some("ffmpeg_4_3")
      } else if avcodec_version_minor >= 54 {
        Some("ffmpeg_4_2")
      } else if avcodec_version_minor >= 35 {
        Some("ffmpeg_4_1")
      } else if avcodec_version_minor >= 18 {
        Some("ffmpeg_4_0")
      } else {
        None
      }
    }
    59 => {
      if avcodec_version_minor >= 37 {
        Some("ffmpeg_5_1")
      } else if avcodec_version_minor >= 18 {
        Some("ffmpeg_5_0")
      } else {
        None
      }
    }
    _ => None,
  };

  println!("cargo:rustc-check-cfg=cfg(ffmpeg_4_0, ffmpeg_4_1, ffmpeg_4_2, ffmpeg_4_3, ffmpeg_4_4, ffmpeg_5_0, ffmpeg_5_1)");
  if let Some(ffmpeg_version) = ffmpeg_version {
    // println!("cargo:warning={}", ffmpeg_version);
    println!("cargo:rustc-cfg={ffmpeg_version}");
  } else {
    panic!(
      "Cannot define ffmpeg version from libavcodec version: {avcodec_version_major}.{avcodec_version_minor}.{avcodec_version_micro}"
    )
  }
}
