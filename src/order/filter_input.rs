use crate::order::input_kind::InputKind;

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct FilterInput {
  pub kind: InputKind,
  pub stream_label: String,
}
