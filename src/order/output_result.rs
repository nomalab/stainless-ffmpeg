use packet::Packet;
use std::collections::HashMap;

pub enum OutputResult {
  Entry(HashMap<String, String>),
  Packet(Packet),
}
