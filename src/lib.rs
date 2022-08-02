use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;

use std::path::Path;
use lazy_static::lazy_static;
use std::sync::Mutex;
use image::DynamicImage;
// use crate::import::{FileType, ImportError, ImportType};

pub mod tpse;
pub mod import;
mod wasm_entrypoint;

// pub fn import_file(tpse: u32, import_type: ImportType, filename: &str, bytes: &[u8]) -> Result<(), ImportError> {
//     let mut state = GLOBAL_STATE.lock().unwrap();
//     let tpse = match state.active_tpse_files.get_mut(&tpse) {
//         None => return Err(ImportError::InvalidTPSEHandle),
//         Some(tpse) => tpse
//     };
//     let ext = match Path::new(&filename).extension() {
//         None => return Err(ImportError::UnknownFileType),
//         Some(some) => some
//     };
//     let file_type = match FileType::from_extension(&ext.to_string_lossy()) {
//         None => return Err(ImportError::UnknownFileType),
//         Some(some) => some
//     };
//
//     match file_type {
//         FileType::TPSE => {
//             let new_tpse: TPSE = match serde_json::from_slice(bytes) {
//                 Err(err) => return Err(ImportError::InvalidTPSE(err.to_string())),
//                 Ok(tpse) => tpse
//             };
//             tpse.merge(new_tpse);
//             Ok(())
//         }
//         FileType::Image => {
//             todo!()
//         }
//         _ => todo!()
//     }
// }

// #[derive(Copy, Clone, Eq, PartialEq)]
// pub struct ImportOptions {
//     depth_limit: u8
// }
// impl Default for ImportOptions {
//     fn default() -> Self {
//         Self { depth_limit: 5 }
//     }
// }
// impl ImportOptions {
//     pub fn minus_one_depth(self) -> Self {
//         ImportOptions { depth_limit: self.depth_limit - 1 }
//     }
// }
// pub fn import_zip_file(bytes: &[u8], options: ImportOptions) -> Result<TPSE, ImportError> {
//     todo!()
// }
// Alright here's the issue:
// Some import units require multiple files to load
// We currently only operate on single files
// Moving to a multiple file system would be annoying
//
// Sticky points:
// 256x256 minos + 128x128 ghost:
// - solved by just importing them separately and merging the resulting TPSE
// multiple frames minos:
// - solved by seeing that all the types are the same and then merging them
// - this requires them to _not_ be present in a single canvas, so tpse generation will need to be deferred
// multiple sound effects:
// - solved by merging all the SoundEffects import types everywhere
//
// Here's how it's going to work:
// - Each import_the_file call will run on _one file_, and will come up with multiple possibilities
// - The top level multi-file call will apply rules to collapse the possibilities

//
//
// pub fn splice_to_t61(skin_type: SkinType, bytes: &[u8])
//   -> Result<(Option<DynamicImage>, Option<DynamicImage>), ImportError>
// {
//     let target_resolution = 96;
//     let mut source = SkinSplicer::default();
//     source.load(skin_type, bytes)?;
//     let minos = source.convert(SkinType::Tetrio61Connected, Some(target_resolution));
//     let ghost = source.convert(SkinType::Tetrio61ConnectedGhost, Some(target_resolution));
//     Ok((minos, ghost))
// }
//
// #[cfg(test)]
// mod tests {
//     use serde_json::Value;
//     use crate::{import_file, ImportType};
//     use crate::import::SkinType;
//     use crate::wasm_entrypoint::{create_tpse, drop_tpse, export_tpse};
//
//     #[test]
//     fn basic_import_export() {
//         let id = create_tpse();
//         let filename = "tpse-basic-most-keys.tpse";
//         let bytes = include_bytes!("../testdata/tpse-basic-most-keys.tpse");
//         import_file(id, ImportType::Automatic, filename, bytes).unwrap();
//         let exported: Value = serde_json::from_str(&export_tpse(id).unwrap()).unwrap();
//         let expected: Value = serde_json::from_slice(bytes).unwrap();
//         assert_eq!(
//             exported.as_object().unwrap().keys().collect::<Vec<&String>>(),
//             expected.as_object().unwrap().keys().collect::<Vec<&String>>()
//         );
//         assert!(drop_tpse(id));
//     }
//
//     #[test]
//     fn skin_slicer() {
//         let tpse1 = create_tpse();
//         let tpse2 = create_tpse();
//         let tpse3 = create_tpse();
//         let filename = "seven-segment.zip";
//         let bytes = include_bytes!("../testdata/seven-segment.zip");
//         import_file(tpse1, ImportType::Automatic, filename, bytes).unwrap();
//         import_file(tpse2, ImportType::Skin { subtype: SkinType::Tetrio61Connected }, filename, bytes).unwrap();
//         import_file(tpse3, ImportType::Skin { subtype: SkinType::TetrioRaster }, filename, bytes).unwrap();
//         assert_eq!(export_tpse(tpse1), export_tpse(tpse2));
//         assert_ne!(export_tpse(tpse1), export_tpse(tpse3))
//     }
// }
