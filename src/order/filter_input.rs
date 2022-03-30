use crate::order::input_kind::InputKind;

#[derive(Debug, PartialEq, Deserialize, Clone)]
pub struct FilterInput {
  pub kind: InputKind,
  pub stream_label: String,
}
