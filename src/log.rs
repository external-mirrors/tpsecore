use log::Level;

pub trait ImportLogger {
  fn log(&self, level: Level, msg: std::fmt::Arguments);
}