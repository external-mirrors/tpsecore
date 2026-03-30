use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use crate::accel::traits::TPSEAccelerator;
use crate::import::inter_stage_data::{AnimatedSkinFrame, ImportFile, ImportTask, SoundEffect};
use crate::import::{ImportContext, ImportContextEntry, ImportError, ImportErrorType, ImportType, SkinType, TypeStage3, TypeStage4};

/// Collates multiple import results into a list of import tasks
pub fn reduce_types<T: TPSEAccelerator>
  (results: &[ImportFile<TypeStage3>], ctx: &mut ImportContext<'_, T>)
   -> Result<Vec<ImportTask>, ImportError<T>>
{
  let mut map: HashMap<TypeStage3, Vec<ImportFile<TypeStage3>>> = HashMap::new();
  for res in results {
    map.entry(res.import_type).or_default().push(res.clone());
  }

  // The import tasks to be performed
  let mut import_tasks = vec![];
  // Sound effects are collated so that they can be assembled all at once
  // Duplicate order is undefined
  let mut sound_effects: Vec<SoundEffect> = vec![];
  // Minos and ghosts are collated to allow for animated-from-frames style textures
  let mut animated_minos: Option<(SkinType, Vec<ImportFile<TypeStage3>>)> = None;
  let mut animated_ghost: Option<(SkinType, Vec<ImportFile<TypeStage3>>)> = None;
  // If animated minos/ghosts encounter a _different_ type of skin that qualifies as animated,
  // an error is logged here. This is then shoved into one giant error message.
  let mut ambiguous_mino_skin_errors: HashSet<SkinType> = HashSet::new();
  let mut ambiguous_ghost_skin_errors: HashSet<SkinType> = HashSet::new();

  for (key, mut files) in map {
    let ctx = ctx.enter_context(ImportContextEntry::WithFiles {
      files: files.iter().map(|f| f.path.clone()).collect()
    });
    match key {
      // This is where we finally bail if we can't figure out what type something is.
      // At this point, we've considered:
      // - the explicit type passed to import()
      // - the filekey
      // - the guessed type
      // - the pack.json override
      // and if none of those produced a type, fail!
      TypeStage3::Unknown => {
        return Err(ctx.wrap_error(ImportErrorType::UnknownFileType));
      },
      // animated skins, determined by animation options or more than 2 files, are handled seperately.
      // normal non-animated skins are wrapped up in the wildcard branch below.
      TypeStage3::Skin { subtype } if subtype.get_anim_options().is_some() || files.len() >= 2 => {
        let (has_minos, has_ghost) = subtype.has_minos_and_ghost();
        let anim_types = [
          if has_minos { Some((&mut animated_minos, &mut ambiguous_mino_skin_errors)) } else { None },
          if has_ghost { Some((&mut animated_ghost, &mut ambiguous_ghost_skin_errors)) } else { None }
        ];
        for (anim_type, errors) in anim_types.into_iter().filter_map(|el| el) {
          match anim_type {
            None => *anim_type = Some((subtype, files.clone())),
            Some((existing_skin_type, _)) if *existing_skin_type != subtype => {
              errors.insert(*existing_skin_type);
              errors.insert(subtype);
            }
            Some((_, results)) => results.append(&mut files)
          }
        }
      }
      TypeStage3::SoundEffects => {
        sound_effects.extend(files.into_iter().map(|import_result| {
          let name = import_result.path
            .file_stem()
            .unwrap_or(import_result.path.as_os_str())
            .to_string_lossy()
            .to_string();
          SoundEffect {
            name,
            path: import_result.path,
            binary: import_result.binary.into()
          }
        }));
      }
      stage4 => {
        let stage4 = TypeStage4::try_from(ImportType::from(stage4)).expect("all stage 3 types should be handled");
        import_tasks.extend(files.into_iter().map(|import_result| {
          ImportTask::Basic {
            import_type: stage4,
            path: import_result.path,
            binary: import_result.binary.into()
          }
        }));
      }
    }
  }

  if sound_effects.len() > 0 {
    import_tasks.push(ImportTask::SoundEffects(sound_effects));
  }
  if ambiguous_mino_skin_errors.len() > 0 {
    return Err(ctx.wrap_error(ImportErrorType::AmbiguousAnimatedSkinResults(
      Cow::from("mino"),
      ambiguous_mino_skin_errors
    )));
  }
  if ambiguous_ghost_skin_errors.len() > 0 {
    return Err(ctx.wrap_error(ImportErrorType::AmbiguousAnimatedSkinResults(
      Cow::from("ghost"),
      ambiguous_mino_skin_errors
    )));
  }
  if animated_minos.as_ref().map(|(t,_)| *t) != animated_ghost.as_ref().map(|(t,_)| *t) {
    if let Some((ghost_type, results)) = animated_ghost {
      let files = results
        .into_iter()
        .map(|res| AnimatedSkinFrame { path: res.path, binary: res.binary.into() })
        .collect();
      import_tasks.push(ImportTask::AnimatedSkinFrames(ghost_type, files))
    }
  }
  if let Some((mino_type, results)) = animated_minos {
    let files = results
      .into_iter()
      .map(|res| AnimatedSkinFrame { path: res.path, binary: res.binary.into() })
      .collect();
    import_tasks.push(ImportTask::AnimatedSkinFrames(mino_type, files))
  }

  Ok(import_tasks)
}

