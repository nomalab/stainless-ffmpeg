use std::collections::HashMap;
use packet::Packet;

pub enum OutputResult {
  Entry(HashMap<String, String>),
  Packet(Packet),
}
