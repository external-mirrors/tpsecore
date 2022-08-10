#![allow(warnings, unused)] // todo: fix these at some point

pub mod tpse;
pub mod import;
mod wasm_entrypoint;

// library cleanup todos:
// - Reintroduce lifetimes into the tpse management to reduce memory overhead
// - Add file trace/stack location to errors
// - Add per-task logging

#[cfg(test)]
mod tests {
    use std::time::Instant;
    use log::LevelFilter;
    use simple_logger::SimpleLogger;
    use crate::import::{Asset, AssetProvider, DefaultAssetProvider, import, ImportContext, ImportType};

    // todo: automated tests should eventually be moved to a separate repository
    // the actual testdata dir is gitignored because it'd be messy to keep in this repository
    // also todo: set up automated tests to check for tetrio update breakages
    #[test]
    fn import_tests() {
        let start = Instant::now();
        SimpleLogger::new()
          .with_level(LevelFilter::Warn)
          .with_module_level("usvg", LevelFilter::Error)
          .with_module_level("tpsecore", LevelFilter::Trace)
          .init().unwrap();
        log::info!("Initialized logger ({:?})", start.elapsed());
        let mut provider = DefaultAssetProvider::default();
        provider.preload(Asset::TetrioJS, include_bytes!("../testdata/tetrio.js")[..].into());
        provider.preload(Asset::TetrioOGG, include_bytes!("../testdata/tetrio.ogg")[..].into());
        log::info!("Preloaded assets ({:?})", start.elapsed());
        let opts = ImportContext::new(&provider, 5);
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
        log::info!("--- Test: simple --- ({:?})", start.elapsed());
        log::info!("{:?}", import(vec![(
            ImportType::Automatic,
            "EmeraldPalaceSimple.zip",
            include_bytes!("../testdata/EmeraldPalaceSimple.zip")
        )], opts));
        log::info!("Done! ({:?})", start.elapsed());
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
