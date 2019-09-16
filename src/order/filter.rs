use order::filter_input::FilterInput;
use order::filter_output::FilterOutput;
use order::parameters::ParameterValue;
use std::collections::HashMap;

#[derive(Debug, Deserialize, PartialEq)]
pub struct Filter {
  pub name: String,
  pub label: Option<String>,
  pub parameters: HashMap<String, ParameterValue>,
  pub inputs: Option<Vec<FilterInput>>,
  pub outputs: Option<Vec<FilterOutput>>,
}
