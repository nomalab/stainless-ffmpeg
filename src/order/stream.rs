#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Stream {
  pub index: u32,
  pub label: Option<String>,
}
