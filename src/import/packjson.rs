use std::collections::HashMap;
use std::fmt::Display;
use std::sync::OnceLock;

use globset::{Glob, GlobMatcher};

use crate::import::TypeStage3;

#[derive(Debug, serde::Deserialize)]
pub struct PackJSON {
  /// The version of the pack.json standard this document is written in.
  /// Currently, the only version is 0.
  pub data_version: u32,
  /// General metadata about the pack. This is generally only considered
  /// for the absolute root `/pack.json` in a content pack.
  #[serde(default)]
  pub metadata: PackMetadata,
  /// A map of import group id to import group.
  /// Import groups are a description of a subset of the content in a pack.
  pub import_groups: HashMap<String, Vec<ImportGroupPattern>>,
  /// The import sets for the file. If not specified,
  /// every single import group is exposed as a distinct set
  pub import_sets: Option<Vec<ImportSet>>
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PackMetadata {
  pub title: Option<String>,
  pub author: Option<String>,
  pub version: Option<String>,
  pub description: Option<String>,
}
impl PackMetadata {
  pub fn has_data(&self) -> bool {
    self.title.is_some() || self.author.is_some() || self.version.is_some() || self.description.is_some()
  }
}
impl Display for PackMetadata {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match &self.title {
      Some(title) => write!(f, "'{title}'")?,
      None => write!(f, "Untitled content pack")?
    };
    if let Some(version) = &self.version {
      write!(f, " (version {version})")?;
    }
    if let Some(author) = &self.author {
      write!(f, " by {}", author)?;
    };
    match &self.description {
      Some(desc) => write!(f, ": {desc}")?,
      None => write!(f, " (no description)")?
    };
    Ok(())
  }
}

#[derive(Debug, serde::Deserialize)]
pub struct ImportGroupPattern {
  /// A pattern specifying files relative to the location of the pack.json file that this import group includes.
  /// Supports globbing. When this names a directory, all files in the directory are included.
  pub pattern: Glob,
  
  /// Overrides the import type of all files in this group. This override is absolute and disables both
  /// automatic import type detection and filekey parsing.
  pub override_type: Option<TypeStage3>,
  
  // temporary storage for caching pattern compilations
  #[serde(skip)]
  cached_compilation: OnceLock<GlobMatcher>
}
impl ImportGroupPattern {
  pub fn get_compiled_pattern(&self) -> &GlobMatcher {
    self.cached_compilation.get_or_init(|| {
      self.pattern.compile_matcher()
    })
  }
}


/// An import set describes a list of import groups to pick between.
/// They allow for precisely controlling which parts of a content pack can be imported together.
/// A content pack with, say 20 skins, can have 1 import set with 20 options while a pack with
/// 2 skins and some board/queue graphics and some sound effects might have three: one for
/// selecting the skin, and two more for enabling the optional board/queue and sound effects.
#[derive(Debug, serde::Deserialize)]
pub struct ImportSet {
  /// A short title naming this import set
  pub title: String,
  /// if true, an option must be picked and cannot be skipped.
  #[serde(default)]
  pub required: bool,
  /// The possible options for this group. At most one option is selected by the user when importing.
  pub options: Vec<ImportSetOption>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ImportSetOption {
  /// A short description of this option, to distinguish it from other options in this set
  pub description: String,
  /// The import groups this set option enables
  pub enables_groups: Vec<String>,
}