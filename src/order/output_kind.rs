#[derive(Debug, Deserialize, PartialEq, Eq)]
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
