#[derive(Debug, Deserialize, PartialEq, Eq)]
pub enum InputKind {
  #[serde(rename = "stream")]
  Stream,
  #[serde(rename = "filter")]
  Filter,
}
