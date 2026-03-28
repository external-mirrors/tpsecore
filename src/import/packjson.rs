use std::collections::HashMap;
use std::sync::OnceLock;

use globset::{Glob, GlobMatcher};

#[derive(Debug, serde::Deserialize)]
pub struct PackJSON {
  /// The overall pack description
  pub description: String,
  /// A map of import group id to import group.
  /// Import groups are a description of a subset of the content in a pack.
  pub import_groups: HashMap<String, Vec<ImportGroupPattern>>,
  /// The import sets for the file. If not specified,
  /// every single import group is exposed as a distinct set
  pub import_sets: Option<Vec<ImportSet>>
}

#[derive(Debug, serde::Deserialize)]
pub struct ImportGroupPattern {
  /// A pattern specifying files relative to the location of the pack.json file that this import group includes.
  /// Supports globbing. When this names a directory, all files in the directory are included.
  pub pattern: Glob,
  #[serde(skip)]
  cached_compilation: OnceLock<GlobMatcher>
  // future features: import option overrides
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