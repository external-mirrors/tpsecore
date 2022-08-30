# TETR.IO PLUS TPSE CORE
This is a library for working with TETR.IO PLUS's `tpse` files.
It's written in Rust and compiled to WebAssembly.

**Currently in very early development**

## Feature Roadmap
- Import content
  - Custom skins
  - Sound effects (Partially functional - encodes to wav instead of ogg)
  - Music
  - Backgrounds
- Work with TPSE files
  - Parse
  - Validate (Partially implemented)
  - Merge (Partially implemented, untested)
  - Migrate (Not implemented)
- Generate previews from TPSE files
  - Slice skin textures into blocks with specific connections
  - Slice and play specific sound effects
  - Generate game-like previews showing most of a `tpse`'s content. This
    includes at least skins, animated skins, sound effects, and board,
    queue, and background skins. These take the form of a still image
    or a short video. (Not implemented)

## Why would you want this as a user?
- It's more portable, so all tetrio plus related tools will have consistent behavior. No more `❌ Error: Unsupported
  image type`, better previews on the site, previews in tetrio plus itself, fewer arbitrary restrictions on asset size
  in importers, etc.
- Asset IDs are now based on file hashes. If two people import the same song, it's guaranteed to have the same ID. If
  you reimport a song after deleting it, you won't need to update it in the music graph.
- Slightly reworked automatic import process with more file keys and the ability to specify animated skin framerate as a
  parameterized file key.
- Better error messages.
- (Thereotically) better performance.

## Potential blockades
- Limited availability of AV libraries for Rust
  - Currently in dire need of a wasm-compatible OGG encoder and any wasm-compatible video encoder
  - Wasm-compatible ffmpeg bindings would be ideal
    (`ffmpeg-dev` isn't wasm-compatible and `ffmpeg-wasi` isn't the same thing as wasm. Changing to a wasi toolchain
    would be annoying due to lack of wasm-bindgen support, and `ffmpeg-wasi` also requires an
    [older version of rust](https://github.com/jedisct1/rust-ffmpeg-wasi/issues/3#issuecomment-1184741515) to build
    properly, which some of this project's dependencies don't support.)
- wasm32 memory limitations: TETR.IO PLUS's current importers generally run
  under 2GB for sound effects, but animated  skins can blow up in size with high
  framerates because every frame is an extra 1024x1024 pixels to be added to a
  singular huge canvas.

## Goals
- Replace TETR.IO PLUS's current javascript-based importers. These are a
  maintainability nightmare, especially the polyfills required to make them work
  in environments without browser APIs.
- Replace the rendering and slicing logic of Omniskin, the YHF renderer, and the
  discord bot.
- Be a plain drop-in javascript library. No bundling, just drop in 1 `wasm` + 1
  `js` file and include via es6 imports.
- Performance: Ideally this library should be able to generate footage that
  resembles TETR.IO gameplay in realtime. The viability of this is unknown. At
  the very least, generating a 10 second 1080p60 render of assets should take no
  longer than a few minutes.

## Differences to existing tetrio plus conventions
- The automatic import format has been made more flexible. (todo: elaborate)
- The connection bitfield format has been changed to have 4 corner connection bits (identifying distinct corners)
  instead of 1. To fix this, just fill in all upper four bits when one is set, e.g.
  `field = (field & 0b00010000) ? (field | 0b11110000) : field`.

## Preferred naming
Just to get this out of the way:
- The base game is stylized as `TETR.IO` / `tetrio`
- The mod is stylized to match: `TETR.IO PLUS` / `tetrio plus` / `tetrioplus` for short
- This library follows: `TETR.IO PLUS TPSE CORE` / `tetrio plus tpse core` / `tpsecore` for short
