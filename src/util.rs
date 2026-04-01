use std::fmt::Formatter;
use std::ops::Deref;
use std::sync::Arc;


/// A thin wrapper for an Arc<[u8]> with a concise Debug representation
pub struct Buffer(Arc<[u8]>);
impl std::fmt::Debug for Buffer {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "<{} bytes>", self.0.len())
  }
}
impl From<Arc<[u8]>> for Buffer {
  fn from(value: Arc<[u8]>) -> Self {
    Self(value)
  }
}
impl Into<Arc<[u8]>> for Buffer {
  fn into(self) -> Arc<[u8]> {
    self.0
  }
}
impl Deref for Buffer {
  type Target = Arc<[u8]>;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

#[derive(Default, Ord, PartialOrd, Eq, PartialEq)]
pub struct SoundEffectsSortPriority<'a> {
  /// these sort first so that they're easy to find, as they're hard to ctrl-f for
  not_piece_sound: bool,
  /// then sort alphabetically by prefix. The only keys where the prefix isn't equal to the
  /// full sound effect name are the combo sound effects, where the prefix is always `combo_`.
  prefix: &'a str,
  /// power sounds are grouped seperately from nonpower sounds
  combo_power: bool,
  /// combo levels are kept in numerical order rather than alphabetical order
  combo_level: u8,
}
pub fn sound_effects_sort_key(sfx: &str) -> SoundEffectsSortPriority<'_> {
  let mut priority = SoundEffectsSortPriority::default();
  priority.prefix = sfx;
  if sfx.len() > 1 {
    priority.not_piece_sound = true;
  }
  if sfx.starts_with("combo_") {
    let mut trim = sfx.trim_start_matches("combo_");
    if sfx.ends_with("_power") {
      trim = trim.trim_end_matches("_power");
      priority.combo_power = true;
    }
    priority.prefix = "combo_";
    priority.combo_level = trim.parse().unwrap_or(0);
  }
  priority
}