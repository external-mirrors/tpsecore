use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::accel::traits::{TPSEAccelerator, TextureHandle};
use crate::import::inter_stage_data::{AnimatedSkinFrame, ImportFile, ImportTask, SoundEffect};
use crate::import::{ImportContext, ImportContextEntry, ImportError, ImportErrorType, ImportErrorWrapHelper, ImportType, OtherSkinType, SkinType, TextureGuessKind, TypeStage3, TypeStage4, err};
use crate::log::LogLevel;
use crate::tpse::SongMetadata;

/// Collates multiple import results into a list of import tasks
pub async fn reduce_types<T: TPSEAccelerator>
  (results: Vec<ImportFile<TypeStage3>>, ctx: &mut ImportContext<'_, T>)
   -> Result<Vec<ImportTask>, ImportError<T>>
{
  let mut by_dir: HashMap<Option<PathBuf>, Vec<ImportFile<TypeStage3>>> = Default::default();
  for result in results {
    by_dir.entry(result.path.parent().map(|p| PathBuf::from(p))).or_default().push(result);
  }
  
  for (dir, entries) in &mut by_dir {
    // weak audio disambiguation
    let count_sfx = entries.iter().filter(|x| matches!(x.import_type, TypeStage3::SoundEffects)).count();
    let count_weak = entries.iter().filter(|x| matches!(x.import_type, TypeStage3::WeakAudio)).count();
    let count_unknown = entries.iter().filter(|x| matches!(x.import_type, TypeStage3::Unknown)).count();
    let count_other = entries.len() - (count_sfx + count_weak + count_unknown);
    {ctx.log(LogLevel::Trace, format_args!("checking directory {dir:?} ({} entries) -- sfx {count_sfx} {count_weak} {count_unknown} {count_other}", entries.len()))}.await;
    // allow some unknown files for several known pre-tpsecore sound packs which have e.g. changelog.txt files in them
    if count_sfx > 0 && count_weak > 0 && count_unknown < 3 && count_other == 0 {
      let guard = ctx.enter_context(ImportContextEntry::WithFiles {
        files: entries.iter().map(|e| e.path.clone()).collect()
      });
      {guard.log(LogLevel::Info, format_args!("Found directory with {count_sfx} sound effects, {count_weak} unknown audio files, {count_unknown} unknown files, and zero other files. Assuming all unknown audio files are sound effects."))}.await;
      for entry in entries.iter_mut() {
        if let TypeStage3::WeakAudio = entry.import_type {
          entry.import_type = TypeStage3::SoundEffects
        }
      }
    }
    if count_sfx > 0 && count_unknown > 0 && count_unknown < 3 && count_other == 0 {
      let affected = entries.iter_mut().filter(|e| matches!(e.import_type, TypeStage3::Unknown)).collect::<Vec<_>>();
      let guard = ctx.enter_context(ImportContextEntry::WithFiles {
        files: affected.iter().map(|e| e.path.clone()).collect()
      });
      {guard.log(LogLevel::Info, format_args!("Ignoring {count_unknown} unknown type files in directory that otherwise consists entirely of sound effects."))}.await;
      for entry in affected {
        entry.import_type = TypeStage3::Ignored;
      }
    }
    
    // weak board,queue,grid trio disambiguation
    if entries.len() == 3 {
      for indices in [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]] {
        let [board, queue, grid] = entries.get_disjoint_mut(indices).unwrap();
        let board_size = OtherSkinType::Board.canonical_texture_size().unwrap();
        
        let board_weak = match board.import_type {
          // it _could_ be a board?
          TypeStage3::WeakTexture { first_guess: f, .. } if f.dim() == board_size => true, 
          // it's _definitely_ a board
          TypeStage3::OtherSkin { subtype: OtherSkinType::Board } => false,
          _ => continue
        };
        let queue_weak = match queue.import_type {
          TypeStage3::WeakTexture { first_guess: f, .. } if f.dim() == board_size => true,
          TypeStage3::OtherSkin { subtype: OtherSkinType::Queue } => false,
          _ => continue
        };
        let grid_weak = match grid.import_type {
          // grid is twice the size of the board/queue
          TypeStage3::WeakTexture { first_guess: f, .. } if f.dim() == board_size.map(|d| d*2) => true,
          TypeStage3::OtherSkin { subtype: OtherSkinType::Grid } => false,
          _ => continue
        };
        {ctx.log(LogLevel::Trace, format_args!("permutation {indices:?} found valid board/queue/grid combo with weak={board_weak},{queue_weak},{grid_weak}"))}.await;
        
        if !board_weak && !queue_weak && !grid_weak {
          break // we already know what they are for sure, no heuristic needed
        }
        
        let guard = ctx.enter_context(ImportContextEntry::WithFiles {
          files: vec![board.path.clone(), queue.path.clone(), grid.path.clone()]
        });
        
        if board_weak || queue_weak {
          // the board and queue are identical in size, so we need an advanced heuristic to tell them apart.
          // In base TETR.IO, the board is 16% opaque and the queue is 61% opaque (bonus fact: the grid is 7%).
          // We're making the assumption here that custom content will follow the same pattern
          // (why wouldn't it, after all). Thus, if the variable `board` has a higher opacity
          // than the variable `queue` then they're _probably_ backwards.
          
          let board_tex = T::Texture::decode_texture(board.binary.clone()).wrap(err!(guard, tex))?;
          let queue_tex = T::Texture::decode_texture(queue.binary.clone()).wrap(err!(guard, tex))?;
          
          let mut board_opaque = board_tex.fraction_opaque().await.wrap(err!(guard, tex))?;
          let mut queue_opaque = queue_tex.fraction_opaque().await.wrap(err!(guard, tex))?;
          
          if board_opaque > queue_opaque {
            std::mem::swap(board, queue);
            std::mem::swap(&mut board_opaque, &mut queue_opaque);
          }
          
          {guard.log(LogLevel::Info, format_args!(
            "Found a directory containing three textures with at least one weak guess, assuming import type based on heuristics:\n\
            - board ({}% opaque, was {})\n\
            - queue ({}% opaque, was {})\n\
            - grid (was {})",
            (board_opaque*1000.0).round()/10.0, board.import_type,
            (queue_opaque*1000.0).round()/10.0, queue.import_type,
            grid.import_type
          ))}.await;
          board.import_type = TypeStage3::OtherSkin { subtype: OtherSkinType::Board };
          queue.import_type = TypeStage3::OtherSkin { subtype: OtherSkinType::Queue };
          grid.import_type = TypeStage3::OtherSkin { subtype: OtherSkinType::Grid };
        } else {
          guard.log(LogLevel::Info, "Found a directory containing a board texture and a queue texture and one third texture with a weak guess in the same directory, assuming the third texture is a grid").await;
          grid.import_type = TypeStage3::OtherSkin { subtype: OtherSkinType::Grid };
        }
        break;
      }
    }
    
    // weak skin,ghost duo disambiguation
    if entries.len() == 2 {
      for indices in [[0, 1], [1, 0]] {
        let [minos, ghost] = entries.get_disjoint_mut(indices).unwrap();
        
        let (minos_weak, minos_res, minos_is_connected) = match minos.import_type {
          TypeStage3::WeakTexture { first_guess: f, .. } => (true, f.dim(), None),
          TypeStage3::Skin { subtype: s@SkinType::Tetrio61Connected } => (false, s.canonical_tex_size().unwrap(), Some(true)),
          TypeStage3::Skin { subtype: s@SkinType::Tetrio61 } => (false, s.canonical_tex_size().unwrap(), Some(false)),
          _ => continue
        };
        let (ghost_weak, ghost_res, ghost_is_connected) = match ghost.import_type {
          TypeStage3::WeakTexture { first_guess: f, .. } => (true, f.dim(), None),
          TypeStage3::Skin { subtype: s@SkinType::Tetrio61ConnectedGhost } => (false, s.canonical_tex_size().unwrap(), Some(true)),
          TypeStage3::Skin { subtype: s@SkinType::Tetrio61Ghost } => (false, s.canonical_tex_size().unwrap(), Some(false)),
          _ => continue
        };
        
        // if they're not square, they're certainly not block skins
        if minos_res[0] != minos_res[1] { break }
        
        // ghost is half the size of the minos
        if minos_res != ghost_res.map(|x| x*2) { continue }
        
        // some opacity percentages:
        // minos/connected.2x.png 49%
        // ghost/connected.2x.png 13%
        // minos/tetrio.2x.png 35%
        // ghost/tetrio.2x.png 12%
        // I don't like the look of those for disambiguating (especially since custom skins may have a
        // radically lower value with transparent blocks), so we're just going to do size detection.
        
        macro_rules! guard {
          () => {
            ctx.enter_context(ImportContextEntry::WithFiles {
              files: vec![minos.path.clone(), ghost.path.clone()]
            })
          }
        }
        
        match (minos_weak, ghost_weak) {
          (false, false) => {}, // types already strong, no heuristics needed
          (true, true) => {
            // for size=512, definitely tetrio61
            // for size=2048, definitely tetrio61connected
            // for size=1024, assumine tetrio61connected
            let (mino_type, ghost_type) = match minos_res[0] {
              0..1024 => (SkinType::Tetrio61, SkinType::Tetrio61Ghost),
              1024.. => (SkinType::Tetrio61Connected, SkinType::Tetrio61ConnectedGhost)
            };
            
            {guard!().log(LogLevel::Info, format_args!("Found a directory containing two textures with weak guesses and one twice the size of the other, assuming they're block skins. Given the resolution of {}, assuming import type {mino_type} and {ghost_type}", minos_res[0]))}.await;
            
            minos.import_type = TypeStage3::Skin { subtype: mino_type };
            minos.import_type = TypeStage3::Skin { subtype: ghost_type };
          }
          (true, false) => {
            let subtype = if ghost_is_connected.unwrap() { SkinType::Tetrio61Connected } else { SkinType::Tetrio61 };
            {guard!().log(LogLevel::Info, format_args!("Found a directory containing a ghost skin and a weakly guessed texture at twice the size, assuming the texture is a {subtype} skin."))}.await;
            minos.import_type = TypeStage3::Skin { subtype };
          }
          (false, true) => {
            let subtype = if minos_is_connected.unwrap() { SkinType::Tetrio61ConnectedGhost } else { SkinType::Tetrio61Ghost };
            {guard!().log(LogLevel::Info, format_args!("Found a directory containing a minos skin and a weakly guessed texture at half the size, assuming the texture is a matching ghost skin."))}.await;
            ghost.import_type = TypeStage3::Skin { subtype };
          }
        }
        break;
      }
    }
  }
  
  let mut by_type: HashMap<TypeStage3, Vec<ImportFile<TypeStage3>>> = HashMap::new();
  for res in by_dir.into_values().flatten() {
    by_type.entry(res.import_type.clone()).or_default().push(res.clone());
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
  
  // Handle `WeakTexture` entries first as they can generate other types of entries
  let mut additional = vec![];
  for (key, files) in &mut by_type {
    let TypeStage3::WeakTexture { first_guess, .. } = key else { continue };
    let ctx = ctx.enter_context(ImportContextEntry::WithFiles {
      files: files.iter().map(|f| f.path.clone()).collect()
    });
    {ctx.log(LogLevel::Info, format_args!("No other heuristic applies to files, assuming first guess of {}.", first_guess.kind))}.await;
    match first_guess.kind {
      TextureGuessKind::Skin(subtype) => {
        for file in &mut *files {
          file.import_type = TypeStage3::Skin { subtype };
        }
        additional.extend(files.drain(..));
      },
      TextureGuessKind::Other(subtype) => {
        import_tasks.extend(files.drain(..).map(|import_result| {
          ImportTask::Basic {
            import_type: TypeStage4::OtherSkin { subtype },
            path: import_result.path,
            binary: import_result.binary.into()
          }
        }));
      }
    }
  }
  for res in additional.into_iter() {
    by_type.entry(res.import_type.clone()).or_default().push(res.clone());
  }
  
  for (key, mut files) in by_type {
    if files.is_empty() { continue }
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
      }
      TypeStage3::Ignored => {
        {ctx.log(LogLevel::Info, format_args!("Content pack contains {} ignored files", files.len()))}.await;
      }
      TypeStage3::WeakTexture { .. } => {
        unreachable!(); // handled above
      }
      TypeStage3::WeakAudio => {
        {ctx.log(LogLevel::Info, format_args!("{} remaining audio files of indeterminate type, assuming custom music.", files.len()))}.await;
        import_tasks.extend(files.into_iter().map(|import_result| {
          ImportTask::Basic {
            import_type: TypeStage4::Music { metadata: SongMetadata::default(), song_override: None },
            path: import_result.path,
            binary: import_result.binary.into()
          }
        }));
      }
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
            import_type: stage4.clone(),
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

