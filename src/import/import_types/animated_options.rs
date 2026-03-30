use std::fmt::{Display, Formatter};
use std::path::Path;
use std::sync::LazyLock;
use regex::Regex;

#[derive(Default, Debug, Hash, Eq, PartialEq, Copy, Clone, Ord, PartialOrd, serde::Serialize, serde::Deserialize)]
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

impl From<&Path> for AnimatedOptions {
  fn from(value: &Path) -> Self {
    // attempt to extract valid filekeys as hard as possible
    value.to_string_lossy().as_ref().into()
  }
}

impl From<&str> for AnimatedOptions {
  fn from(filename: &str) -> Self {
    const DELAY_REGEX_STR: &str = r"_delay=(\d+)";
    const COMBINE_REGEX_STR: &str = r"_combine=(true|false)";
    static DELAY_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(DELAY_REGEX_STR).unwrap());
    static COMBINE_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(COMBINE_REGEX_STR).unwrap());

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