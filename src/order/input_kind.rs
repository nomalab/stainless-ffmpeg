#[derive(Debug, Deserialize, PartialEq)]
pub enum InputKind {
  #[serde(rename = "stream")]
  Stream,
  #[serde(rename = "filter")]
  Filter,
}
