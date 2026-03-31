use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::read;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Instant;
use hex_literal::hex;
use log::LevelFilter;
use serde_json::json;
use sha2::{Digest, Sha256};
use simple_logger::SimpleLogger;
use crate::accel::cached_asset_provider::CachedAssetProvider;
use crate::accel::default_decision_maker::{DefaultDecisionMaker, DefaultDecisionMakerError};
use crate::accel::ffmpeg_audio_handle::FFmpegAudioHandle;
use crate::accel::software_texture_handle::SoftwareTextureHandle;
use crate::accel::traits::{TPSEAccelerator, TextureHandle};
use crate::import::inter_stage_data::ImportFile;
use crate::import::stages::{explore_files, partition_import_groups};
use crate::import::*;
use crate::import::skin_splicer::Piece;
use crate::log::{ImportLogger, LogLevel};
use crate::render::{BoardElement, BoardMap, FrameInfo, RenderContext, RenderOptions, example_maps};
use crate::tpse::TPSE;

pub struct TestState {
  pub files: HashMap<String, TestAsset>
}
impl TestState {
  pub fn get(&self, asset: &str) -> &TestAsset {
    self.files.get(asset).unwrap_or_else(|| panic!("unknown test asset: {asset}"))
  }
}

pub struct TestAsset {
  pub name: String,
  pub content: Arc<[u8]>,
  pub expected_hash: [u8; 32]
}

fn setup() -> &'static TestState {
  static STATE: OnceLock<TestState> = OnceLock::new();
  STATE.get_or_init(|| {
    SimpleLogger::new()
      .with_level(LevelFilter::Warn)
      .with_module_level("usvg", LevelFilter::Error)
      .with_module_level("tpsecore", LevelFilter::Debug)
      .init().unwrap();
    log::info!("Initialized logger");

    let mut assets = HashMap::new();
    let files = [
      // Base TETR.IO content
      (hex!("d820dc63a8b63f0611075e23579608d3ddf07b54bb7d9c0c94a99714bc7a818e"), "tetrio.js"),
      (hex!("d320ec4f6267e087f6886955e11e2310dd27672b0740e63983084320ece526f7"), "tetrio.opus.rsd"),
      (hex!("5721abb52bad64f4e448cd958b9558a6b14274fa47c02d9a6d5a96e1e46fa0bf"), "vanilla/board.png"),
      (hex!("7753c6562c5f53a09164dfff8ad58313453a8af8863958040d43c5473f74f115"), "vanilla/queue.png"),
      (hex!("ec4cc09305e0336806997094138e8f5fe20699d3a0eed37a3ae154a842f529dd"), "vanilla/grid.png"),
      (hex!("be01c800db480c036acbbe0bb9cfe265fe93ecacb9a227393b4f1d200aa7c9ea"), "vanilla/connected.png"),
      (hex!("0bc3ea3e1a358969d41d8deeb8d17b5acdcfd0834a1ee1a1f2aafe7727828bd6"), "vanilla/connected_ghost.png"),
      (hex!("a619506ea08107f09b282c02b292be3c00ac8d1d6ec5909f537618cbada0f96b"), "vanilla/unconnected.png"),
      (hex!("82ae7c26880c35b3a8558e4444b13d510dccb2971a44340c6974394210c706a2"), "vanilla/unconnected_ghost.png"),

      // penguin_colonel's Shimmering Cyclone
      // Popular connected skin with both minos and ghost variants
      (hex!("4a0034bfc40ef4db0cb574b5e5c9459af89f9e50e85550e186cac8d668b9f40d"), "yhf/SHIMMERING_CYCLONE.zip"),
      (hex!("0b6f41e54b2a49035258eefdc6bc1dc493c52a9935e8ac4528feb589d23b0d02"), "yhf/shimmering_cyclone/shimmering_cyclone_connected_minos.png"),
      (hex!("906f5797decadd19c74464349ce7c8e53bdcdc0474ab6ea3a436699d59868d10"), "yhf/shimmering_cyclone/shimmering_cyclone_connected_ghost.png"),

      // Itsmega's Bejeweled Soundpack
      // Large soundpack with subfolder and a non-audio `1st_read_changelog.txt` file
      (hex!("f788cbd0d8c1b72843599d24677c072b9554d4a3bfd80c35ac22e27dc077f5a8"), "yhf/BejeweledSR.zip"),

      // Sobsz's RGB Gamer Minos
      // Animated skin that expands to a _very_ large canvas
      (hex!("2247e22ce8da892b0a37451405c111546e472d7c6d88c18358325d930f48a0c7"), "yhf/rgb_gamer_minos.gif"),

      // Starcat_JP's Starcat's Cute Skin Pack!
      // Varied TPSE exported from slightly older version (v0.23.8) with a large amount of content covering many features
      (hex!("5ac375f6f35441ace825bfdc69c88c9ecb868dfcbfc1da57f1dd01720d22671b"), "yhf/Starcats_Cute_Skin_Pack.tpse"),

      // UniQMG's Concrete
      // Very old original format skin
      (hex!("bd5f082d3314abe661853613345c373acf90cfc4be24e6858fe3412f1acfbf55"), "yhf/Concrete.png"),
    ];
    for (hash, name) in files {
      let asset = match read(format!("testdata/{name}")) {
        Err(err) => panic!("Failed to read test asset {name}: {err}.\nYou might need to run `fetch_test_data.sh`."),
        Ok(bytes) => TestAsset { name: name.to_string(), content: bytes.into(), expected_hash: hash }
      };
      assets.insert(name.to_string(), asset);
    }
    TestState { files: assets }
  })
}

#[test]
fn ensure_test_assets_ok() {
  let files = setup();
  let mut errors = vec![];
  let mut soft_errors_only = true;
  for asset in files.files.values() {
    let mut sha256 = Sha256::new();
    sha256.update(&asset.content);
    let hash = sha256.finalize();
    let ok = hash[..] == asset.expected_hash;
    if !ok {
      // tetrio.js and tetrio.opus.rsd change frequently outside our control,
      // so it's not a huge deal if their hash doesn't match
      let soft = ["tetrio.js", "tetrio.opus.rsd"].contains(&&asset.name[..]);
      if !soft { soft_errors_only = false; }
      errors.push(format!(
        "{} hash mismatch\n    expected {} \n    but got  {}{}",
        asset.name,
        hex::encode(asset.expected_hash),
        hex::encode(&hash[..]),
        if soft {
          "\n    (note: this asset has been marked as a soft error, and will not fail the test by itself)"
        } else {
          ""
        }
      ))
    }
  }
  if !soft_errors_only {
    panic!("{} test assets failed verification: \n- {}", errors.len(), errors.join("\n- "))
  }
}

#[derive(Debug)]
struct TestAccelerator;
impl TPSEAccelerator for TestAccelerator {
  type Decider = DefaultDecisionMaker;
  type Asset = CachedAssetProvider;
  type Texture = SoftwareTextureHandle;
  type Audio = FFmpegAudioHandle;
}

struct LogLogger;
impl ImportLogger for LogLogger {
  fn log(&self, level: LogLevel, _context: &[ImportContextEntry], msg: &dyn Display) {
    log::info!("Import level={level:?} - {}", msg);
  }
}

#[tokio::test]
async fn metadata_json_test() {
  let state = setup();
  let mut provider = CachedAssetProvider::default();
  provider.cache.insert(Asset::TetrioJS, state.get("tetrio.js").content.clone());
  provider.cache.insert(Asset::TetrioRSD, state.get("tetrio.opus.rsd").content.clone());
  let mut ctx = ImportContext::<TestAccelerator>::new(&provider, &DefaultDecisionMaker).with_logger(&LogLogger);
  let mut tpse = TPSE::default();
  let files = vec![
    ImportFile {
      import_type: ImportType::Automatic,
      path: PathBuf::from("skins/SHIMMERING_CYCLONE.zip"),
      binary: state.get("yhf/SHIMMERING_CYCLONE.zip").content.clone()
    },
    ImportFile {
      import_type: ImportType::Automatic,
      path: PathBuf::from("skins/Concrete.png"),
      binary: state.get("yhf/Concrete.png").content.clone()
    },
    ImportFile {
      import_type: ImportType::Automatic,
      path: PathBuf::from("sfx/BejeweledSR.zip"),
      binary: state.get("yhf/BejeweledSR.zip").content.clone()
    },
    ImportFile {
      import_type: ImportType::Automatic,
      path: PathBuf::from("skins/pack.json"),
      binary: Arc::from(r#"
{
  "description": "a nested subpack offering two skin choices",
  "import_groups": {
    "alpha": [{ "pattern": "Concrete.png" }],
    "beta": [{ "pattern": "SHIMMERING_CYCLONE.zip/*" }]
  },
  "import_sets": [
    {
      "title": "select skin",
      "required": true,
      "options": [
        { "enables_groups": ["alpha"], "description": "a colorful composite worn with garbage blocks worn down to the rebar" },
        { "enables_groups": ["beta"], "description": "a connected remake of 'In the blizzard'" }
      ]
    }
  ]
}
      "#.as_bytes())
    },
    ImportFile {
      import_type: ImportType::Automatic,
      path: PathBuf::from("pack.json"),
      binary: Arc::from(r#"
{
  "description": "a content pack for testing",
  "import_groups": {
    "skins": [{ "pattern": "skins/pack.json" }],
    "sfx": [{ "pattern": "sfx/BejeweledSR.zip/*.ogg" }]
  },
  "import_sets": [
    {
      "title": "skins",
      "required": true,
      "options": [
        { "enables_groups": ["skins"], "description": "UI for required groups with one option will generally not be shown at all" }
      ]
    },
    {
      "title": "sound effects",
      "required": false,
      "options": [
        { "enables_groups": ["sfx"], "description": "bejeweled sound effects pack" }
      ]
    }
  ]
}
      "#.as_bytes())
    }
  ];
  
  let results = explore_files(files.clone(), &mut ctx).await.unwrap();
  let results = partition_import_groups(&results, &mut ctx).unwrap();
  
  println!("{results:#?}");
  
  let [skins, sfx, loose] = &results[..] else { panic!("expected 2, got {} root decision trees", results.len()) };
  
  let [skins_subtree_option] = &skins.options[..] else { panic!() };
  let [skin_decision] = &skins_subtree_option.subtrees[..] else { panic!() };
  let [a, b] = &skin_decision.options[..] else { panic!() };
  let [a0] = &a.files[..] else { panic!() };
  let [b0, b1] = &b.files[..] else { panic!() };
  assert_eq!(a0.path, PathBuf::from("skins/Concrete.png"));
  assert_eq!(b1.path, PathBuf::from("skins/SHIMMERING_CYCLONE.zip/shimmering_cyclone_connected_minos.png"));
  assert_eq!(b0.path, PathBuf::from("skins/SHIMMERING_CYCLONE.zip/shimmering_cyclone_connected_ghost.png"));
  
  assert_eq!(sfx.options[0].files.len(), 178);
  
  let [loose] = &loose.options[..] else { panic!() };
  let [txt] = &loose.files[..] else { panic!() };
  assert!(txt.path.ends_with("1st_read_changelog.txt"));
  assert_eq!(txt.import_type, TypeStage3::Unknown);
  
  let Err(res) = import(&mut ctx, files, &mut tpse).await else { panic!() };
  assert!(matches!(res.error, ImportErrorType::DecisionFailure(DefaultDecisionMakerError)));
}

#[tokio::test]
async fn sfx_ignore_heuristic() {
  let state = setup();
  
  let mut provider = CachedAssetProvider::default();
  provider.cache.insert(Asset::TetrioJS, state.get("tetrio.js").content.clone());
  provider.cache.insert(Asset::TetrioRSD, state.get("tetrio.opus.rsd").content.clone());
  let mut ctx = ImportContext::<TestAccelerator>::new(&provider, &DefaultDecisionMaker).with_logger(&LogLogger);
  let files = vec![ImportFile {
    import_type: ImportType::Automatic,
    path: PathBuf::from("sfx/BejeweledSR.zip"),
    binary: state.get("yhf/BejeweledSR.zip").content.clone()
  }];
  
  let mut tpse = TPSE::default();
  import(&mut ctx, files, &mut tpse).await.unwrap();
  
  tpse.custom_sound_atlas.unwrap();
  std::fs::write(
    "./testdata/result/custom_sounds_bejeweled_sr.wav",
    &tpse.custom_sounds.as_ref().unwrap().binary
  ).unwrap();
}

#[tokio::test]
async fn render_tests() {
  let state = setup();

  let start = Instant::now();
  let mut provider = CachedAssetProvider::default();
  provider.cache.insert(Asset::TetrioJS, state.get("tetrio.js").content.clone());
  provider.cache.insert(Asset::TetrioRSD, state.get("tetrio.opus.rsd").content.clone());
  log::info!("Preloaded assets ({:?})", start.elapsed());

  let mut ctx = ImportContext::<TestAccelerator>::new(&provider, &DefaultDecisionMaker).with_logger(&LogLogger);

  log::info!("--- Test: render --- ({:?})", start.elapsed());
  let files = vec![
    ImportFile {
      import_type: ImportType::Automatic,
      path: PathBuf::from("SHIMMERING_CYCLONE.zip"),
      binary: state.get("yhf/SHIMMERING_CYCLONE.zip").content.clone()
    },
    ImportFile {
      import_type: ImportType::Automatic,
      path: PathBuf::from("Concrete.png"),
      binary: state.get("yhf/Concrete.png").content.clone()
    },
    ImportFile {
      import_type: ImportType::Automatic,
      path: PathBuf::from("_board.png"),
      binary: state.get("vanilla/board.png").content.clone()
    },
    ImportFile {
      import_type: ImportType::Automatic,
      path: PathBuf::from("_grid.png"),
      binary: state.get("vanilla/grid.png").content.clone()
    },
    ImportFile {
      import_type: ImportType::Automatic,
      path: PathBuf::from("_queue.png"),
      binary: state.get("vanilla/queue.png").content.clone()
    },
    ImportFile {
      import_type: ImportType::SoundEffects,
      path: PathBuf::from("this_will_be_ignored_but_will_trigger_default_values_to_populate.wav"),
      binary: include_bytes!("../assets/empty_2c.wav").to_vec().into()
    }
  ];
  let mut tpse = TPSE::default();
  import(&mut ctx, files, &mut tpse).await.unwrap();
  std::fs::write("./testdata/result/custom_sounds.wav", &tpse.custom_sounds.as_ref().unwrap().binary).unwrap();
  std::fs::write("./testdata/result/render_result.tpse", &serde_json::to_string(&tpse).unwrap()).unwrap();

  let ctx = RenderContext::<TestAccelerator>::try_from_tpse(&tpse).unwrap();
  
  let board: Vec<Vec<Option<(Piece, u8)>>> = serde_json::from_value(json!(
    // generated using https://you.have.fail/tetrio/connected-map-editor
    [
      [null,null,null,null,null,["t",2],null,null,null,null],
      [null,null,null,null,["t",4],["t",11],null,null,null,null],
      [null,null,null,null,null,["t",8],null,null,null,null],
      [null,null,null,null,null,null,null,null,null,null],
      [null,null,null,null,null,null,null,null,null,null],
      [null,null,null,null,null,null,null,null,null,null],
      [null,null,null,null,null,null,null,null,null,null],
      [null,null,null,null,null,null,null,null,null,null],
      [null,null,null,null,null,null,null,null,null,null],
      [null,null,null,null,null,null,null,null,null,null],
      [null,null,null,null,null,null,null,null,null,null],
      [null,null,null,null,null,null,null,null,null,null],
      [null,null,null,null,null,null,null,null,null,null],
      [null,null,null,null,null,null,null,null,null,null],
      [null,null,null,null,null,null,null,null,null,null],
      [null,null,null,null,null,null,null,null,null,null],
      [["s",4],["s",3],null,null,null,null,["l",6],["l",5],["l",1],["i",2]],
      [["t",2],["s",12],["s",1],null,null,["ghost",2],["l",8],["o",38],["o",19],["i",10]],
      [["t",14],["t",1],["z",6],["z",1],["ghost",4],["ghost",11],["j",2],["o",76],["o",137],["i",10]],
      [["t",8],["z",4],["z",9],null,null,["ghost",8],["j",12],["j",5],["j",1],["i",8]]
    ]
  )).unwrap();
  for part in BoardElement::get_draw_order() {
    let boards = [
      ("", BoardMap::from(example_maps::EMPTY_MAP)),
      ("_with_board", BoardMap::from(board.clone()))
    ];
    for (board_name, board) in boards {
      let frame = ctx.render_frame(&FrameInfo {
        real_time: 0.0,
        render_options: &RenderOptions {
          board: board.clone().into(),
          board_elements: &[*part][..],
          debug_grid: true,
          ..Default::default()
        }
      }).await.unwrap().unwrap();

      log::info!("encoding board part {part:?}{board_name}");
      let buffer = frame.image.encode_png().await.unwrap();
      std::fs::write(format!("./testdata/result/individual_part{board_name}_{part:?}.bmp"), &buffer).unwrap();
    }
  }

  let frame = ctx.render_frame(&FrameInfo {
    real_time: 0.0,
    render_options: &RenderOptions {
      board: board.clone().into(),
      board_elements: BoardElement::get_draw_order(),
      debug_grid: true,
      ..Default::default()
    }
  }).await.unwrap().unwrap();
  let buffer = frame.image.encode_png().await.unwrap();
  std::fs::write("./testdata/result/all_board_elements.bmp", &buffer).unwrap();

  // let frames = todo!();
  // let ctx = VideoContext { frame_rate: 30.0 };
  // let mut frames = Vec::with_capacity(150);
  // for i in 0..150 {
  //   frames.push(RenderOptions {
  //     debug_grid: true,
  //     ..RenderOptions::default()
  //   });
  // }
  // let frames = render_frames(&ctx, &tpse, frames.into_iter()).unwrap();
  // for (i, frame) in frames.enumerate() {
  //   // let frame = frame.expect("there should be renderable assets");
  //   let filename = format!("./testdata/result/{:04}_full_render.bmp", i);
  //   let mut bytes = vec![];
  //   frame.image.write_to(&mut Cursor::new(&mut bytes), ImageOutputFormat::Bmp);
  //   std::fs::write(filename, &bytes).unwrap();
  // }

  log::info!("--- Test: sound effects --- ({:?})", start.elapsed());
  #[derive(serde::Deserialize)]
  struct Replay<'a> {
    /// always "tetrio-plus-music-graph-replay"
    __schema: Cow<'a, str>,
    events: Vec<ReplayEvent<'a>>
  }
  #[derive(serde::Deserialize)]
  struct ReplayEvent<'a> {
    /// milliseconds since epoch
    real_time: u64,
    /// seconds since graph start
    audio_time: f64,
    event: Cow<'a, str>,
    // we don't care about the actual value here right now
    // value: HashMap<Cow<'a, str>, f64>
  }
  let raw = include_str!("../testdata/tetrio-plus-music-graph-replay_322-events_2024-11-12T02_06_14.023Z.json");
  let replay: Replay = serde_json::from_str(raw).unwrap();
  let min_time = replay.events.iter().map(|x| x.audio_time).reduce(f64::min).unwrap_or(0.0);
  // let sounds = replay.events.iter()
  //   .filter(|event|event.event.starts_with("sfx-") && event.event.ends_with("-global"))
  //   .map(|event| {
  //     let sfx = event.event.trim_start_matches("sfx-").trim_end_matches("-global");
  //     SoundEffectInfo {
  //       name: sfx.into(),
  //       time: ((event.audio_time - min_time) * ctx.frame_rate) as usize
  //     }
  //   })
  //   .collect::<Vec<_>>();
  // let audio = render_sound_effects(&ctx, &tpse, &sounds).unwrap();
  // std::fs::write("./testdata/result/audio_tetrio_recording.wav", audio.binary);
  // 
  // let audio = render_sound_effects(&ctx, &tpse, &[
  //   SoundEffectInfo { name: "allclear".into(), time: 0 }
  // ]).unwrap();
  // std::fs::write("./testdata/result/audio_sample.wav", audio.binary);

  //
  // log::info!("--- Test: animated skin --- ({:?})", start.elapsed());
  // let tpse = import(vec![(
  //     ImportType::Automatic,
  //     "rgb_gamer_minos.gif",
  //     include_bytes!("../testdata/rgb_gamer_minos.gif")
  // )], opts).unwrap();
  //
  // std::fs::write(
  //     "./rgb_game_minos.gif-output.tpse",
  //     serde_json::to_string(&tpse).unwrap()
  // ).unwrap();
  //
  // log::info!("Done! ({:?})", start.elapsed());

  // log::info!("--- Test: skin --- ({:?})", start.elapsed());
  // import(vec![(
  //     ImportType::Automatic,
  //     "Emerald_Runes.svg",
  //     include_bytes!("../testdata/Emerald_Runes.svg")
  // )], opts).unwrap();
  // log::info!("Done! ({:?})", start.elapsed());
  // todo: image-rs is choking on this background and panics
  // find an alternative decoder or something
  // log::info!("--- Test: background --- ({:?})", start.elapsed());
  // log::info!("{:?}", import(vec![(
  //     ImportType::Automatic,
  //     "Emerald_PalaceWebp_BG.webp",
  //     include_bytes!("../testdata/Emerald_PalaceWebp_BG.webp")
  // )], opts));
  // log::info!("Done! ({:?})", start.elapsed());
  // log::info!("--- Test: simple --- ({:?})", start.elapsed());
  // log::info!("{:?}", import(vec![(
  //     ImportType::Automatic,
  //     "EmeraldPalaceSimple.zip",
  //     include_bytes!("../testdata/EmeraldPalaceSimple.zip")
  // )], opts));
  // log::info!("Done! ({:?})", start.elapsed());
  // log::info!("--- Test: single folder --- ({:?})", start.elapsed());
  // import(vec![(
  //     ImportType::Automatic,
  //     "EmeraldPalaceSingleFolder.zip",
  //     include_bytes!("../testdata/EmeraldPalaceSingleFolder.zip")
  // )], opts).unwrap();
  // log::info!("--- Test: advanced --- ({:?})", start.elapsed());
  // import(vec![(
  //     ImportType::Automatic,
  //     "EmeraldPalaceAdvanced.zip",
  //     include_bytes!("../testdata/EmeraldPalaceAdvanced.zip")
  // )], opts).unwrap();
  // log::info!("--- Test: _recursive_ --- ({:?})", start.elapsed());
  // import(vec![(
  //     ImportType::Automatic,
  //     "r.zip",
  //     include_bytes!("../testdata/r.zip")
  // )], opts).unwrap();
}
