#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TouchControlConfig {
  /// What mode touch controls are in
  pub mode: TouchControlMode,
  /// The configured touchpad directional bindings
  pub binding: TouchControlBinding,
  /// A list of configured keys
  pub keys: Vec<TouchControlKey>,
  /// The deadzone of the touchpads, which adjusts how far the user must move their
  /// finger from the initial touch position before activation.
  pub deadzone: f64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TouchControlKey {
  pub behavior: KeyBehavior,
  pub bind: InputType,
  /// The x location of the key, as a percent of screen width
  pub x: f64,
  /// The y location of the key, as a percent of screen height
  pub y: f64,
  /// The width of the key, as a percent of screen width
  pub w: f64,
  /// The height of the key, as a percent of screen height
  pub h: f64
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyBehavior {
  /// An active touch can move over the key to trigger it
  Hover,
  /// The press needs to start on the key to trigger it
  Tap
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TouchControlMode {
  /// Touchpad mode is enabled, enabling two joystick-like touch surfaces.
  Touchpad,
  /// Both touchpad and keys modes are enabled
  Hybrid,
  /// Touchkeys mode is enabled, showing distinct buttons on the screen.
  Keys
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TouchControlBinding {
  #[serde(rename = "L_down")]
  pub left_pad_down: InputType,
  #[serde(rename = "L_left")]
  pub left_pad_left: InputType,
  #[serde(rename = "L_right")]
  pub left_pad_right: InputType,
  #[serde(rename = "L_up")]
  pub left_pad_up: InputType,
  #[serde(rename = "R_down")]
  pub right_pad_down: InputType,
  #[serde(rename = "R_left")]
  pub right_pad_left: InputType,
  #[serde(rename = "R_right")]
  pub right_pad_right: InputType,
  #[serde(rename = "R_up")]
  pub right_pad_up: InputType
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum InputType {
  #[serde(rename = "hardDrop")]
  HardDrop,
  #[serde(rename = "softDrop")]
  SoftDrop,
  #[serde(rename = "moveLeft")]
  MoveLeft,
  #[serde(rename = "moveRight")]
  MoveRight,
  #[serde(rename = "rotateCW")]
  RotateCW,
  #[serde(rename = "rotateCCW")]
  RotateCCW,
  #[serde(rename = "rotate180")]
  Rotate180,
  #[serde(rename = "hold")]
  Hold,
  #[serde(rename = "exit")]
  Exit,
  #[serde(rename = "retry")]
  Retry,
  #[serde(rename = "fullscreen")]
  Fullscreen,
}