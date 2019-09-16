use order::input_kind::InputKind;

#[derive(Debug, Deserialize, PartialEq)]
pub struct FilterInput {
  pub kind: InputKind,
  pub stream_label: String,
}
