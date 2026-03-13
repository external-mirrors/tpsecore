use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::path::Path;
use crate::accel::traits::TPSEAccelerator;
use crate::import::{ImportContext, ImportError, ImportErrorType, ImportResult, SkinType, SpecificImportType};
use crate::import::import_task::{AnimatedSkinFrame, ImportTask, SoundEffect};

/// Collates multiple import results into a list of import tasks
pub fn reduce_types<T: TPSEAccelerator>
  (results: &[ImportResult<T>], ctx: ImportContext<'_, T>)
   -> Result<Vec<ImportTask>, ImportError<T>>
{
  let mut map: HashMap<SpecificImportType, Vec<ImportResult<T>>> = HashMap::new();
  for res in results {
    map.entry(res.specific_import_type).or_default().push(res.clone());
  }

  // The import tasks to be performed
  let mut import_tasks = vec![];
  // Sound effects are collated so that they can be assembled all at once
  // Duplicate order is undefined
  let mut sound_effects: Vec<SoundEffect> = vec![];
  // Minos and ghosts are collated to allow for animated-from-frames style textures
  let mut animated_minos: Option<(SkinType, Vec<ImportResult<T>>)> = None;
  let mut animated_ghost: Option<(SkinType, Vec<ImportResult<T>>)> = None;
  // If animated minos/ghosts encounter a _different_ type of skin that qualifies as animated,
  // an error is logged here. This is then shoved into one giant error message.
  let mut ambiguous_mino_skin_errors: HashSet<SkinType> = HashSet::new();
  let mut ambiguous_ghost_skin_errors: HashSet<SkinType> = HashSet::new();

  for (key, mut files) in map {
    use SpecificImportType as SIT;
    match key {
      SpecificImportType::Zip => {
        import_tasks.extend(files.into_iter().map(|import_result| {
          ImportTask::Basic {
            import_type: SIT::Zip,
            filename: import_result.filename,
            file: import_result.file
          }
        }));
      },
      SpecificImportType::TPSE => {
        import_tasks.extend(files.into_iter().map(|import_result| {
          ImportTask::Basic {
            import_type: SIT::TPSE,
            filename: import_result.filename,
            file: import_result.file
          }
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
          import_tasks.extend(files.into_iter().map(|import_result| {
            ImportTask::Basic {
              import_type: SIT::Skin(skin_type),
              filename: import_result.filename,
              file: import_result.file
            }
          }));
        }
      }
      SpecificImportType::OtherSkin(skin_type) => {
        import_tasks.extend(files.into_iter().map(|import_result| {
          ImportTask::Basic {
            import_type: SIT::OtherSkin(skin_type),
            filename: import_result.filename,
            file: import_result.file
          }
        }));
      }
      SpecificImportType::SoundEffects => {
        sound_effects.extend(files.into_iter().map(|import_result| {
          let name = Path::new(&import_result.filename)
            .file_stem()
            .unwrap_or(OsStr::new(&import_result.filename))
            .to_string_lossy()
            .to_string();
          SoundEffect {
            name,
            filename: import_result.filename,
            file: import_result.file
          }
        }));
      }
      SpecificImportType::Background(bg_type) => {
        import_tasks.extend(files.into_iter().map(|import_result| {
          ImportTask::Basic {
            import_type: SIT::Background(bg_type),
            filename: import_result.filename,
            file: import_result.file
          }
        }));
      }
      SpecificImportType::Music => {
        import_tasks.extend(files.into_iter().map(|import_result| {
          ImportTask::Basic {
            import_type: SIT::Music,
            filename: import_result.filename,
            file: import_result.file
          }
        }));
      }
    }
  }

  if sound_effects.len() > 0 {
    import_tasks.push(ImportTask::SoundEffects(sound_effects));
  }
  if ambiguous_mino_skin_errors.len() > 0 {
    return Err(ctx.wrap(ImportErrorType::AmbiguousAnimatedSkinResults(
      Cow::from("mino"),
      ambiguous_mino_skin_errors
    )));
  }
  if ambiguous_ghost_skin_errors.len() > 0 {
    return Err(ctx.wrap(ImportErrorType::AmbiguousAnimatedSkinResults(
      Cow::from("ghost"),
      ambiguous_mino_skin_errors
    )));
  }
  if animated_minos.as_ref().map(|(t,_)| *t) != animated_ghost.as_ref().map(|(t,_)| *t) {
    if let Some((ghost_type, results)) = animated_ghost {
      let files = results
        .into_iter()
        .map(|res| AnimatedSkinFrame { filename: res.filename, file: res.file })
        .collect();
      import_tasks.push(ImportTask::AnimatedSkinFrames(ghost_type, files))
    }
  }
  if let Some((mino_type, results)) = animated_minos {
    let files = results
      .into_iter()
      .map(|res| AnimatedSkinFrame { filename: res.filename, file: res.file })
      .collect();
    import_tasks.push(ImportTask::AnimatedSkinFrames(mino_type, files))
  }

  Ok(import_tasks)
}
