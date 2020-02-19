use stainless_ffmpeg_sys::*;
use std::mem::swap;

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct Rational {
  pub num: i32,
  pub den: i32,
}

impl Rational {
  pub fn new(num: i32, den: i32) -> Self {
    Rational { num, den }
  }

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

#[test]
fn rational() {
  let r = Rational::new(2, 4);
  let r = r.invert();
  assert!(r.num == 4);
  assert!(r.den == 2);

  let r = r.reduce();
  assert!(r.num == 2);
  assert!(r.den == 1);

  let av_r: AVRational = r.into();
  assert!(av_r.num == 2);
  assert!(av_r.den == 1);
}
