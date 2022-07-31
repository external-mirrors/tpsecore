use crate::tpse::File;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum MiscTPSEValue {
  File(File),
  Other(serde_json::Value)
}