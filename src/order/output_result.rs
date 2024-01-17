use crate::packet::Packet;
use std::collections::HashMap;

#[derive(Debug)]
pub enum OutputResult {
  Entry(HashMap<String, String>),
  Packet(Packet),
}
