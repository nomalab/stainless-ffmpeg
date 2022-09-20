use crate::order::{frame::FrameAddress, stream::Stream};

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Input {
  Streams {
    id: u32,
    path: String,
    streams: Vec<Stream>,
  },
  VideoFrames {
    id: u32,
    label: Option<String>,
    path: String,
    codec: String,
    width: i32,
    height: i32,
    frames: Vec<FrameAddress>,
  },
}
