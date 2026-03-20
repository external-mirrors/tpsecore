use std::fmt::Display;

use crate::import::ImportContextEntry;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
  /// Messages describing a direct cause of an import failure
  Error,
  /// Messages describing a significant possible problem with the import that's not severe enough to cause it to fail outright
  Warn,
  /// Informational messages about the import process that the content developer may be interested in
  Info,
  /// Progress-related messages about the import process that don't convey useful information beyond
  /// how far along the import process is
  Status,
  /// Messages mainly of use for tpsecore developers or advanced users troubleshooting problems
  Debug,
  /// Messages mainly of use for tpsecore developers at a very high verbosity
  Trace
}

pub trait ImportLogger {
  fn log(&self, level: LogLevel, context: &[ImportContextEntry], msg: &dyn Display);
}