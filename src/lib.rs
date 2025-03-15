#![allow(warnings, unused)] // todo: fix these at some point

pub mod tpse;
pub mod import;
pub mod render;
pub mod log;
#[cfg(target_arch = "wasm32")]
mod wasm_entrypoint;

// library cleanup todos:
// - Reintroduce lifetimes into the tpse management to reduce memory overhead

#[cfg(test)]
mod tests {
  use std::borrow::Cow;
  use std::collections::HashMap;
  use std::fmt::Arguments;
  use std::fs::read;
  use std::io::Cursor;
  use std::sync::OnceLock;
  use std::time::Instant;
  use hex_literal::hex;
  use log::{Level, LevelFilter};
  use serde_json::json;
  use sha2::{Digest, Sha256};
  use simple_logger::SimpleLogger;
  use crate::import::{Asset, AssetProvider, DefaultAssetProvider, import, ImportContext, ImportType};
  use crate::import::skin_splicer::Piece;
  use crate::log::ImportLogger;
  use crate::render::{BoardElement, BoardMap, example_maps, render_frames, render_sound_effects, RenderOptions, SoundEffectInfo, VideoContext};

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
    pub content: Vec<u8>,
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
        (hex!("f794282c44f599cf0da1309726af0eccfc3203f6627d14ed006e484f52f8007c"), "tetrio.js"),
        (hex!("c4f55aa0a78417f568ced5872d1ebd1a55c17312a3a454fa318d66aef0744352"), "tetrio.opus.rsd"),
        (hex!("1c144f1066efbd6ebffcfcf299d8b2a397a3a8d9fa19e63e3105d88ebff0536a"), "vanilla/board.png"),
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
        (hex!("0bbaef669dc21b65ccb96b74bd9b418740221f97c6e71cdca99161c48c62e122"), "yhf/BejeweledSR.zip"),

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
          Ok(bytes) => TestAsset { name: name.to_string(), content: bytes, expected_hash: hash }
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

  #[test]
  fn import_tests() {
    let state = setup();

    let start = Instant::now();
    let mut provider = DefaultAssetProvider::default();
    provider.preload(Asset::TetrioJS, state.get("tetrio.js").content.clone());
    provider.preload(Asset::TetrioRSD, state.get("tetrio.opus.rsd").content.clone());
    log::info!("Preloaded assets ({:?})", start.elapsed());

    struct LogLogger;
    impl ImportLogger for LogLogger {
      fn log(&self, level: Level, msg: Arguments) {
        log::log!(level, "Import: {}", msg);
      }
    }
    let opts = ImportContext::new(&provider, 5).with_logger(&LogLogger);

    log::info!("--- Test: render --- ({:?})", start.elapsed());
    let tpse = import(vec![(
      ImportType::Automatic,
      "SHIMMERING_CYCLONE.zip",
      &state.get("yhf/SHIMMERING_CYCLONE.zip").content
    ), (
      ImportType::Automatic,
      "Concrete.png",
      &state.get("yhf/Concrete.png").content
    ), (
      ImportType::Automatic,
      "_board.png",
      &state.get("vanilla/board.png").content
    ), (
      ImportType::Automatic,
      "_grid.png",
      &state.get("vanilla/grid.png").content
    ), (
      ImportType::Automatic,
      "_queue.png",
      &state.get("vanilla/queue.png").content
    ), (
      ImportType::SoundEffects,
      "this_will_be_ignored_but_will_trigger_default_values_to_populate.wav",
      &[]
    )], opts).unwrap();
    std::fs::write("./testdata/result/custom_sounds.wav", &tpse.custom_sounds.as_ref().unwrap().binary).unwrap();
    std::fs::write("./testdata/result/render_result.tpse", &serde_json::to_string(&tpse).unwrap()).unwrap();

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
    let ctx = VideoContext {
      frame_rate: 60.0
    };
    for part in BoardElement::get_draw_order() {
      for (board_name, board) in [("", BoardMap::from(example_maps::EMPTY_MAP)), ("_with_board", BoardMap::from(board.clone()))] {
        let frames = render_frames(&ctx, &tpse, [RenderOptions {
          board: board.clone().into(),
          board_elements: &[*part][..],
          debug_grid: true,
          ..Default::default()
        }]).unwrap().collect::<Vec<_>>();

        assert_eq!(frames.len(), 1);
        let mut bytes = vec![];
        frames[0].image.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Bmp);
        std::fs::write(format!("./testdata/result/individual_part{board_name}_{:?}.bmp", part), &bytes).unwrap();
      }
    }

    let frames = render_frames(&ctx, &tpse, [RenderOptions {
      board: board.clone().into(),
      board_elements: BoardElement::get_draw_order(),
      debug_grid: true,
      ..Default::default()
    }]).unwrap().collect::<Vec<_>>();
    assert_eq!(frames.len(), 1);
    let mut bytes = vec![];
    frames[0].image.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Bmp);
    std::fs::write("./testdata/result/all_board_elements.bmp", &bytes).unwrap();

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
    let sounds = replay.events.iter()
      .filter(|event|event.event.starts_with("sfx-") && event.event.ends_with("-global"))
      .map(|event| {
        let sfx = event.event.trim_start_matches("sfx-").trim_end_matches("-global");
        SoundEffectInfo {
          name: sfx.into(),
          time: ((event.audio_time - min_time) * ctx.frame_rate) as usize
        }
      })
      .collect::<Vec<_>>();
    let audio = render_sound_effects(&ctx, &tpse, &sounds).unwrap();
    std::fs::write("./testdata/result/audio_tetrio_recording.wav", audio.binary);

    let audio = render_sound_effects(&ctx, &tpse, &[
      SoundEffectInfo { name: "allclear".into(), time: 0 }
    ]).unwrap();
    std::fs::write("./testdata/result/audio_sample.wav", audio.binary);

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
}
