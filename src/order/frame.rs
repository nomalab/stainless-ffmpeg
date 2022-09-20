#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct FrameAddress {
  pub index: u32,
  pub offset: u64,
  pub size: u64,
}
