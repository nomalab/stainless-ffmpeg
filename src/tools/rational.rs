
use stainless_ffmpeg_sys::*;
use std::mem::swap;

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct Rational {
  pub num: i32,
  pub den: i32,
}

impl Rational {
  pub fn invert(mut self) -> Self {
    swap(&mut self.den, &mut self.num);
    self
  }

  pub fn reduce(self) -> Self {
    let gcd = gcd(self.num, self.den);
    Rational {
      num: self.num / gcd,
      den: self.den / gcd,
    }
  }
}

impl Into<AVRational> for Rational {
  fn into(self) -> AVRational {
    AVRational {
      num: self.num,
      den: self.den,
    }
  }
}

fn gcd(x: i32, y: i32) -> i32 {
  let mut x = x;
  let mut y = y;
  while y != 0 {
    let t = y;
    y = x % y;
    x = t;
  }
  x
}
