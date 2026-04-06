use std::sync::LazyLock;

use ordered_float::NotNan;
use regex::Regex;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Song {
  pub id: String,
  pub filename: String,
  #[serde(rename = "override")]
  pub song_override: Option<String>,
  pub metadata: SongMetadata
}

#[derive(Default, Debug, Hash, Eq, PartialEq, Clone, Ord, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct SongMetadata {
  pub name: String,
  pub jpname: String,
  pub artist: String,
  pub jpartist: String,
  pub genre: SongGenre,
  pub source: String,
  #[serde(rename = "loop")]
  pub song_loop: bool,
  #[serde(rename = "loopStart")]
  pub loop_start: u32,
  #[serde(rename = "loopLength")]
  pub loop_length: u32,
  pub hidden: bool,
  #[serde(rename = "normalizeDb")]
  pub normalize_db: NotNan<f64>
}

macro_rules! regex {
  ($id:ident, $regex:literal) => {
    static $id: LazyLock<Regex> = LazyLock::new(|| Regex::new($regex).unwrap());
  }
}
impl SongMetadata {
  /// Parses filekey information, returning the SongMetadata and an override string (exists on Song)
  pub fn from_filename(filename: &str) -> (Self, Option<String>) {
    let mut this = Self::default();
    let mut teto = None;
    regex!(TETO, r"_override=([a-z0-9-]+)");
    regex!(GENRE, r"_(pool|genre)=(?i:(CALM|BATTLE|INTERFACE|OVERRIDE|DISABLED))");
    regex!(LOOPS, r"_loop=(\d+),(\d+)");
    regex!(DB, r"_db=\+?(-?\d+)");
    
    if let Some(cap) = TETO.captures(&filename) {
      teto = Some(cap.get(1).unwrap().as_str().to_string());
      this.genre = SongGenre::Override;
    }
    if let Some(cap) = GENRE.captures(&filename) {
      let genre = cap.get(2).unwrap().as_str();
      let genre = match () { // todo: warning on unknown genre
        _ if genre.eq_ignore_ascii_case("calm") => Some(SongGenre::Calm),
        _ if genre.eq_ignore_ascii_case("battle") => Some(SongGenre::Battle),
        _ if genre.eq_ignore_ascii_case("interface") => Some(SongGenre::Interface),
        _ if genre.eq_ignore_ascii_case("override") => Some(SongGenre::Override),
        _ if genre.eq_ignore_ascii_case("disabled") => Some(SongGenre::Disabled),
        _ => None
      };
      if let Some(genre) = genre {
        this.genre = genre;
      }
    }
    if let Some(cap) = LOOPS.captures(&filename) {
      // todo: warning on parse failure (input is guaranteed digits, so only in case of overflow)
      this.song_loop = true;
      this.loop_start = cap.get(1).unwrap().as_str().parse().unwrap_or(0);
      this.loop_length = cap.get(2).unwrap().as_str().parse().unwrap_or(0);
    }
    if let Some(cap) = DB.captures(&filename) {
      this.normalize_db = cap.get(1).unwrap().as_str().parse().expect("well-formed float parsing should never fail");
    }
    
    (this, teto)
  }
}

#[cfg(test)] #[test]
fn song_filekey_parsing() {
  assert!(matches!(
    dbg!(SongMetadata::from_filename("_pool=oVeRrIdE_loop=0,1_db=+999")),
    (SongMetadata { genre: SongGenre::Override, song_loop: true, loop_start: 0, loop_length: 1, normalize_db, .. }, None)
      if (999.0 - *normalize_db).abs() < 0.01
  ));
  assert!(matches!(
    dbg!(SongMetadata::from_filename("_override=aerial-city")),
    (SongMetadata { genre: SongGenre::Override, .. }, Some(song))
      if song == "aerial-city"
  ));
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Ord, PartialOrd, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SongGenre {
  Interface,
  Disabled,
  Override,
  Calm,
  Battle
}
impl Default for SongGenre {
  fn default() -> Self {
    Self::Calm
  }
}