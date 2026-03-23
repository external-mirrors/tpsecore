#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Node {
  pub id: u64,
  #[serde(rename = "type")]
  pub node_type: NodeType,
  pub name: String,

  /// The ID of the custom song this graph node plays. Exists in the tpse as `song-$audio`
  pub audio: Option<String>,
  /// The ID of the background this graph node shows. Exists in the tpse as `song-$background`
  pub background: Option<String>,
  #[serde(rename = "backgroundLayer")]
  pub background_layer: f64,
  #[serde(rename = "backgroundArea")]
  pub background_area: BackgroundArea,

  #[serde(rename = "audioStart")]
  pub audio_start: f64,
  #[serde(rename = "audioEnd")]
  pub audio_end: f64,
  pub triggers: Vec<Trigger>,

  pub hidden: bool,
  #[serde(rename = "singleInstance")]
  pub single_instance: bool,
  pub effects: Effects,
  pub x: f64,
  pub y: f64
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Effects {
  pub volume: f64,
  pub speed: f64
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType { Normal, Root }

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundArea {
  /// The element appears behind the main game canvas
  Background,
  /// The element appears in front of the main game canvas
  Foreground
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerMode { Fork, Goto, Kill, Random, Dispatch, Set }

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Trigger {
  pub event: String,
  #[serde(rename = "timePassedDuration")]
  pub time_passed_duration: f64,
  
  #[serde(rename = "predicateExpression")]
  pub predicate_expression: String,

  pub mode: TriggerMode,
  pub target: u64,
  #[serde(rename = "dispatchEvent")]
  pub dispatch_event: String, // these aren't allowed to be null, but they can be empty strings
  #[serde(rename = "dispatchExpression")]
  pub dispatch_expression: String,
  #[serde(rename = "setVariable")]
  pub set_variable: String,
  #[serde(rename = "setExpression")]
  pub set_expression: String,
  
  pub crossfade: bool,
  #[serde(rename = "preserveLocation")]
  pub preserve_location: bool,
  #[serde(rename = "crossfadeDuration")]
  pub crossfade_duration: f64,
  #[serde(rename = "locationMultiplier")]
  pub location_multiplier: f64,

  pub anchor: AnchorSet
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AnchorSet {
  pub origin: Anchor,
  pub target: Anchor
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Anchor {
  pub x: f64,
  pub y: f64
}