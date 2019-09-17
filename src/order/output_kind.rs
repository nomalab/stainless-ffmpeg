#[derive(Debug, Deserialize, PartialEq)]
pub enum OutputKind {
  #[serde(rename = "file")]
  File,
  #[serde(rename = "packet")]
  Packet,
  #[serde(rename = "audio_metadata")]
  AudioMetadata,
  #[serde(rename = "video_metadata")]
  VideoMetadata,
}
