use std::fmt::{Display, Formatter};
use lazy_static::lazy_static;
use regex::Regex;

#[derive(Default, Debug, Hash, Eq, PartialEq, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnimatedOptions {
  /// A frame rate to override with. See `AnimMeta#delay`
  pub delay: Option<u32>,
  /// A combine frames setting to override with. Overrides any inferred gif combine setting.
  pub combine: Option<bool>
}

impl AnimatedOptions {
  pub fn has_fields(&self) -> bool {
    self.delay.is_some() || self.combine.is_some()
  }
}

impl Display for AnimatedOptions {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "at a {} delay and drawn {}",
      match self.delay.as_ref() {
        Some(delay) => format!("{} frame", delay),
        None => format!("unspecified")
      },
      match self.combine {
        None => "unspecified",
        Some(true) => "optimized (combine)",
        Some(false) => "unoptimized (replace)"
      }
    )
  }
}

impl From<&str> for AnimatedOptions {
  fn from(filename: &str) -> Self {
    lazy_static! {
      static ref DELAY_REGEX: Regex = Regex::new(r"_delay=(\d+)").unwrap();
      static ref COMBINE_REGEX: Regex = Regex::new(r"_combine=(true|false)").unwrap();
    }

    AnimatedOptions {
      delay: DELAY_REGEX.captures(filename).and_then(|matches| {
        matches.get(1).unwrap().as_str().parse().ok()
      }),
      combine: COMBINE_REGEX.captures(filename).map(|matches| {
        matches.get(1).unwrap().as_str().parse().unwrap()
      })
    }
  }
}