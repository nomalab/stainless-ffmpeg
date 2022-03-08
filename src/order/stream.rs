#[derive(Debug, PartialEq, Deserialize, Clone)]
pub struct Stream {
  pub index: u32,
  pub label: Option<String>,
}
