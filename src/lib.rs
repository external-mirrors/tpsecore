#![allow(warnings, unused)] // todo: fix these at some point

pub mod tpse;
pub mod import;
pub mod render;
mod wasm_entrypoint;

// library cleanup todos:
// - Reintroduce lifetimes into the tpse management to reduce memory overhead

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::time::Instant;
    use image::ImageOutputFormat;
    use log::LevelFilter;
    use simple_logger::SimpleLogger;
    use crate::import::{Asset, AssetProvider, DefaultAssetProvider, import, ImportContext, ImportType};
    use crate::import::skin_splicer::Piece;
    use crate::render::{BoardElement, render, RenderOptions};

    // todo: automated tests should eventually be moved to a separate repository
    // the actual testdata dir is gitignored because it'd be messy to keep in this repository
    // also todo: set up automated tests to check for tetrio update breakages
    #[test]
    fn import_tests() {
        let start = Instant::now();

        SimpleLogger::new()
          .with_level(LevelFilter::Warn)
          .with_module_level("usvg", LevelFilter::Error)
          .with_module_level("tpsecore", LevelFilter::Debug)
          .init().unwrap();
        log::info!("Initialized logger ({:?})", start.elapsed());

        let mut provider = DefaultAssetProvider::default();
        provider.preload(Asset::TetrioJS, include_bytes!("../testdata/tetrio.js")[..].into());
        provider.preload(Asset::TetrioOGG, include_bytes!("../testdata/tetrio.ogg")[..].into());
        log::info!("Preloaded assets ({:?})", start.elapsed());

        let opts = ImportContext::new(&provider, 5).with_logger(&|level, args| {
            log::log!(level, "Import: {}", args);
        });

        log::info!("--- Test: render --- ({:?})", start.elapsed());
        let tpse = import(vec![(
            ImportType::Automatic,
            "render_test.zip",
            include_bytes!("../testdata/render_test.zip")
        )], opts).unwrap();
        std::fs::write("./testdata/render_result.tpse", &serde_json::to_string(&tpse).unwrap()).unwrap();

        // for part in BoardElement::get_draw_order() {
        //     let image = render(&tpse, RenderOptions {
        //         board_pieces: &[*part][..],
        //         debug_grid: true
        //     }).unwrap().expect("there should be renderable assets");
        //     let mut bytes = vec![];
        //     image.write_to(&mut Cursor::new(&mut bytes), ImageOutputFormat::Bmp);
        //     std::fs::write(format!("./testdata/render_parts/part_{:?}.bmp", part), &bytes).unwrap();
        // }

        let frames = render(&tpse, RenderOptions {
            debug_grid: true,
            ..RenderOptions::default()
        }).unwrap();
        for (i, frame) in frames.enumerate() {
            let frame = frame.expect("there should be renderable assets");
            let filename = format!("./testdata/render_parts/{:04}_full_render.bmp", i);
            let mut bytes = vec![];
            frame.write_to(&mut Cursor::new(&mut bytes), ImageOutputFormat::Bmp);
            std::fs::write(filename, &bytes).unwrap();
        }

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
