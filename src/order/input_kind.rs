#[derive(Debug, Deserialize, PartialEq, Clone)]
pub enum InputKind {
  #[serde(rename = "stream")]
  Stream,
  #[serde(rename = "filter")]
  Filter,
}
