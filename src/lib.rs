use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;

use std::path::Path;
use lazy_static::lazy_static;
use crate::tpse::{TPSE};
use std::sync::Mutex;

use crate::import::{AnimatedOptions, ImportType, LoadError, OtherSkinType, SkinSlice, SkinSplicer, SkinType};
use crate::import::skin_splicer::maps::tetrio_connections_submap;
use crate::import::skin_splicer::Piece;

mod tpse;
mod import;
mod wasm_entrypoint;

#[derive(Default)]
struct State {
    active_tpse_files: HashMap<u32, TPSE>,
    id_incr: u32
}

lazy_static! {
    static ref GLOBAL_STATE: Mutex<State> = {
        #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Debug);
        }
        #[cfg(not(target_arch = "wasm32"))] {
            simple_logger::SimpleLogger::new().env().init().unwrap();
        }
        Default::default()
    };
}


#[derive(Debug, serde_with::SerializeDisplay, thiserror::Error)]
// #[serde(tag = "error")]
pub enum ImportError {
    #[error("invalid TPSE handle")]
    InvalidTPSEHandle,
    #[error("unknown file type")]
    UnknownFileType,
    #[error("invalid TPSE: {0}")]
    InvalidTPSE(String),
    #[error("files were nested too deeply")]
    TooMuchNesting,
    #[error("failed to load image")]
    ImageError(#[from] LoadError),
    #[error("animated {0} skin results were ambiguous: found multiple possible formats: {1:?}")]
    AmbiguousAnimatedSkinResults(Cow<'static, str>, HashSet<SkinType>)
}

enum FileType {
    Zip,
    TPSE,
    Image,
    Video,
    Audio
}
impl FileType {
    pub fn from_mime(string: &str) -> Option<FileType> {
        todo!()
    }
    pub fn from_extension(filename: &str) -> Option<FileType> {
        let ext = Path::new(&filename).extension()
          .map(|ext| ext.to_string_lossy())
          .unwrap_or(Cow::from(filename));
        match ext.as_ref() {
            "zip" => Some(FileType::Zip),
            "tpse" => Some(FileType::TPSE),
            "svg" | "png" | "jpg" | "jpeg" | "gif" | "webp" => Some(FileType::Image),
            "mp4" | "webm" => Some(FileType::Video),
            "ogg" | "mp3" | "flac" => Some(FileType::Audio),
            _ => return None
        }
    }
}


pub fn import_file(tpse: u32, import_type: ImportType, filename: &str, bytes: &[u8]) -> Result<(), ImportError> {
    let mut state = GLOBAL_STATE.lock().unwrap();
    let tpse = match state.active_tpse_files.get_mut(&tpse) {
        None => return Err(ImportError::InvalidTPSEHandle),
        Some(tpse) => tpse
    };
    let ext = match Path::new(&filename).extension() {
        None => return Err(ImportError::UnknownFileType),
        Some(some) => some
    };
    let file_type = match FileType::from_extension(&ext.to_string_lossy()) {
        None => return Err(ImportError::UnknownFileType),
        Some(some) => some
    };

    match file_type {
        FileType::TPSE => {
            let new_tpse: TPSE = match serde_json::from_slice(bytes) {
                Err(err) => return Err(ImportError::InvalidTPSE(err.to_string())),
                Ok(tpse) => tpse
            };
            tpse.merge(new_tpse);
            Ok(())
        }
        FileType::Image => {
            todo!()
        }
        _ => todo!()
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ImportOptions {
    depth_limit: u8
}
impl Default for ImportOptions {
    fn default() -> Self {
        Self { depth_limit: 5 }
    }
}
impl ImportOptions {
    pub fn minus_one_depth(self) -> Self {
        ImportOptions { depth_limit: self.depth_limit - 1 }
    }
}
pub fn import_zip_file(bytes: &[u8], options: ImportOptions) -> Result<TPSE, ImportError> {
    todo!()
}
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
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ImportResult<'a, 'b> {
    filename: &'a str,
    bytes: &'b [u8],
    options: ImportOptions,
    specific_import_type: SpecificImportType
}
impl<'a, 'b> ImportResult<'a, 'b> {
    pub fn new(filename: &'a str, bytes: &'b [u8], options: ImportOptions, possibility: SpecificImportType) -> Self {
        Self { filename, bytes, options, specific_import_type: possibility }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub enum SpecificImportType {
    Zip,
    TPSE,
    Skin(SkinType),
    OtherSkin(OtherSkinType),
    SoundEffects,
    Background,
    Music
}
// #[derive(Debug, Hash, Eq, PartialEq, Clone)]
// enum AnimatedSkinType {
//     Tetrio61ConnectedAnimated,
//     Tetrio61ConnectedGhostAnimated,
//     TetrioAnimated,
//     JstrisAnimated
// }
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum ImportTask<'a> {
    AnimatedSkinFrames(SkinType, Vec<&'a [u8]>),
    SoundEffects(Vec<(String, &'a [u8])>),
    Basic(SpecificImportType, &'a [u8])
}

// Executes an import task
pub fn execute_task(task: ImportTask) -> Result<TPSE, ImportError> {
    match task {
        ImportTask::AnimatedSkinFrames(skin_type, frames) => todo!(),
        ImportTask::SoundEffects(sound_effects) => todo!(),
        ImportTask::Basic(specific_type, file) => {
            match specific_type {
                SpecificImportType::Zip => todo!(),
                SpecificImportType::TPSE => todo!(),
                SpecificImportType::Skin(skin_type) => todo!(),
                SpecificImportType::OtherSkin(skin_type) => todo!(),
                SpecificImportType::SoundEffects => todo!(),
                SpecificImportType::Background => todo!(),
                SpecificImportType::Music => todo!(),
            }
        }
    }
}

// Collates multiple import results into a list of import tasks
pub fn reduce_types<'a>(results: &'a [ImportResult]) -> Result<Vec<ImportTask<'a>>, ImportError> {
    let mut map: HashMap<SpecificImportType, Vec<ImportResult>> = HashMap::new();
    for res in results {
        map.entry(res.specific_import_type).or_default().push(*res);
    }

    // The import tasks that must be performed
    let mut import_tasks = vec![];
    // Sound effects are collated so that they can be assembled all at once
    // Duplicate order is undefined
    let mut sound_effects: Vec<(String, &'a [u8])> = vec![];
    // Minos and ghosts are collated to allow for animated-from-frames style textures
    let mut animated_minos: Option<(SkinType, Vec<ImportResult>)> = None;
    let mut animated_ghost: Option<(SkinType, Vec<ImportResult>)> = None;
    // If animated minos/ghosts encounter a _different_ type of skin that qualifies as animated,
    // an error is logged here. This is then shoved into one giant error message.
    let mut ambiguous_mino_skin_errors: HashSet<SkinType> = HashSet::new();
    let mut ambiguous_ghost_skin_errors: HashSet<SkinType> = HashSet::new();

    for (key, mut files) in map {
        use SpecificImportType as SIT;
        match key {
            SpecificImportType::Zip => {
                import_tasks.extend(files.into_iter().map(|file| {
                    ImportTask::Basic(SIT::Zip, file.bytes)
                }));
            },
            SpecificImportType::TPSE => {
                import_tasks.extend(files.into_iter().map(|file| {
                    ImportTask::Basic(SIT::TPSE, file.bytes)
                }));
            },
            SpecificImportType::Skin(skin_type) => {
                let (opts, is_minos, is_ghost) = match &skin_type {
                    SkinType::TetrioAnimated { opts } => (Some(opts), true, true),
                    SkinType::Tetrio61ConnectedAnimated { opts } => (Some(opts), true, false),
                    SkinType::Tetrio61ConnectedGhostAnimated { opts } => (Some(opts), false, true),
                    SkinType::JstrisAnimated { opts } => (Some(opts), true, true),
                    SkinType::TetrioSVG => (None, true, true),
                    SkinType::TetrioRaster => (None, true, true),
                    SkinType::Tetrio61 => (None, true, false),
                    SkinType::Tetrio61Ghost => (None, false, true),
                    SkinType::Tetrio61Connected => (None, true, false),
                    SkinType::Tetrio61ConnectedGhost => (None, false, true),
                    SkinType::JstrisRaster => (None, true, true),
                    SkinType::JstrisConnected => (None, true, true)
                };
                let animated = opts.is_some() || files.len() >= 2;
                if animated {
                    let anim_types = [
                        if is_minos { Some((&mut animated_minos, &mut ambiguous_mino_skin_errors)) } else { None },
                        if is_ghost { Some((&mut animated_ghost, &mut ambiguous_ghost_skin_errors)) } else { None }
                    ];
                    for (anim_type, errors) in anim_types.into_iter().filter_map(|el| el) {
                        match anim_type {
                            None => *anim_type = Some((skin_type, files.clone())),
                            Some((existing_skin_type, _)) if *existing_skin_type != skin_type => {
                                errors.insert(*existing_skin_type);
                                errors.insert(skin_type);
                            }
                            Some((_, results)) => results.append(&mut files)
                        }
                    }
                } else {
                    import_tasks.extend(files.into_iter().map(|file| {
                        ImportTask::Basic(SIT::Skin(skin_type), file.bytes)
                    }));
                }
            }
            SpecificImportType::OtherSkin(skin_type) => {
                import_tasks.extend(files.into_iter().map(|file| {
                    ImportTask::Basic(SIT::OtherSkin(skin_type), file.bytes)
                }));
            }
            SpecificImportType::SoundEffects => {
                sound_effects.extend(files.into_iter().map(|file| {
                    let name = Path::new(file.filename).file_stem().unwrap_or(OsStr::new(file.filename));
                    (name.to_string_lossy().to_string(), file.bytes)
                }));
            }
            SpecificImportType::Background => {
                import_tasks.extend(files.into_iter().map(|file| {
                    ImportTask::Basic(SIT::Background, file.bytes)
                }));
            }
            SpecificImportType::Music => {
                import_tasks.extend(files.into_iter().map(|file| {
                    ImportTask::Basic(SIT::Music, file.bytes)
                }));
            }
        }
    }

    if sound_effects.len() > 0 {
        import_tasks.push(ImportTask::SoundEffects(sound_effects));
    }
    if ambiguous_mino_skin_errors.len() > 0 {
        return Err(ImportError::AmbiguousAnimatedSkinResults(Cow::from("mino"), ambiguous_mino_skin_errors));
    }
    if ambiguous_ghost_skin_errors.len() > 0 {
        return Err(ImportError::AmbiguousAnimatedSkinResults(Cow::from("ghost"), ambiguous_mino_skin_errors));
    }
    if animated_minos.as_ref().map(|(t,_)| *t) != animated_ghost.as_ref().map(|(t,_)| *t) {
        if let Some((ghost_type, results)) = animated_ghost {
            import_tasks.push(ImportTask::AnimatedSkinFrames(ghost_type, results.iter().map(|res| res.bytes).collect()))
        }
    }
    if let Some((mino_type, results)) = animated_minos {
        import_tasks.push(ImportTask::AnimatedSkinFrames(mino_type, results.iter().map(|res| res.bytes).collect()))
    }
    todo!()
}

// Prepares a single file for import.
pub fn decide_specific_type<'a, 'b>
  (import_type: ImportType, filename: &'a str, bytes: &'b [u8], options: ImportOptions)
  -> Result<ImportResult<'a, 'b>, ImportError>
{
    if options.depth_limit == 0 { return Err(ImportError::TooMuchNesting) }
    use SpecificImportType as SIT;
    Ok(ImportResult::new(filename, bytes, options, match import_type {
        ImportType::Automatic => {
            if let Some(guess) = ImportType::from_filekey(filename) {
                return decide_specific_type(guess, filename, bytes, options.minus_one_depth())
            } else if let Some(guess) = FileType::from_extension(filename) {
                match guess {
                    FileType::Zip => SIT::Zip,
                    //import_zip_file(bytes, options.minus_one_depth()),
                    FileType::TPSE => SIT::TPSE,
                    // {
                    //     serde_json::from_slice(bytes).map_err(|err| {
                    //         ImportError::InvalidTPSE(err.to_string())
                    //     })
                    // },
                    FileType::Image => {
                        return if let Some(format) = SkinType::guess_format(filename, 0, 0) {
                            let format = ImportType::Skin { subtype: format };
                            decide_specific_type(format, filename, bytes, options.minus_one_depth())
                        } else {
                            Err(ImportError::UnknownFileType)
                        }
                    },
                    FileType::Video => todo!(),
                    FileType::Audio => todo!()
                }
            } else {
                return Err(ImportError::UnknownFileType);
            }
        },
        ImportType::Skin { subtype } => {
            SIT::Skin(subtype)
            // let target_resolution = 96;
            // let mut minos = SkinSplicer::default();
            // let mut ghost = SkinSplicer::default();
            // let mut source = SkinSplicer::default();
            // minos.create_empty(SkinType::Tetrio61Connected, Some(target_resolution));
            // minos.create_empty(SkinType::Tetrio61ConnectedGhost, Some(target_resolution));
            // source.load(subtype, bytes)?;
            // let mut minos_used = false;
            // let mut ghost_used = false;
            //
            // for piece in Piece::values() {
            //     for conn in tetrio_connections_submap.keys() {
            //         let buf = source.get(*piece, conn, Some(target_resolution)).map(|buf| buf.into());
            //         if let Some(buf) = buf {
            //             if let Some(()) = minos.set(*piece, conn, &buf) { minos_used = true; }
            //             if let Some(()) = ghost.set(*piece, conn, &buf) { ghost_used = true; }
            //         }
            //     }
            // }
        },
        ImportType::OtherSkin { subtype } => SIT::OtherSkin(subtype),
        ImportType::SoundEffects => SIT::SoundEffects,
        ImportType::Background => SIT::Background,
        ImportType::Music => SIT::Music
    }))
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use crate::{import_file, ImportType};
    use crate::import::SkinType;
    use crate::wasm_entrypoint::{create_tpse, drop_tpse, export_tpse};

    #[test]
    fn basic_import_export() {
        let id = create_tpse();
        let filename = "tpse-basic-most-keys.tpse";
        let bytes = include_bytes!("../testdata/tpse-basic-most-keys.tpse");
        import_file(id, ImportType::Automatic, filename, bytes).unwrap();
        let exported: Value = serde_json::from_str(&export_tpse(id).unwrap()).unwrap();
        let expected: Value = serde_json::from_slice(bytes).unwrap();
        assert_eq!(
            exported.as_object().unwrap().keys().collect::<Vec<&String>>(),
            expected.as_object().unwrap().keys().collect::<Vec<&String>>()
        );
        assert!(drop_tpse(id));
    }

    #[test]
    fn skin_slicer() {
        let tpse1 = create_tpse();
        let tpse2 = create_tpse();
        let tpse3 = create_tpse();
        let filename = "seven-segment.zip";
        let bytes = include_bytes!("../testdata/seven-segment.zip");
        import_file(tpse1, ImportType::Automatic, filename, bytes).unwrap();
        import_file(tpse2, ImportType::Skin { subtype: SkinType::Tetrio61Connected }, filename, bytes).unwrap();
        import_file(tpse3, ImportType::Skin { subtype: SkinType::TetrioRaster }, filename, bytes).unwrap();
        assert_eq!(export_tpse(tpse1), export_tpse(tpse2));
        assert_ne!(export_tpse(tpse1), export_tpse(tpse3))
    }
}
