#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub enum InputKind {
  #[serde(rename = "stream")]
  Stream,
  #[serde(rename = "filter")]
  Filter,
}
