# TETR.IO PLUS TPSE CORE
This is a rust library for working with TETR.IO PLUS's `tpse` files: importing content, validating, merging, and previewing.

It can be compiled to wasm for use in browser and is used in TETR.IO PLUS itself.

**Currently in early development**

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

## Goals
- Replace TETR.IO PLUS's current javascript-based importers. These are a
  maintainability nightmare, especially the polyfills required to make them work
  in environments without browser APIs.
- Replace the rendering and slicing logic of Omniskin, the YHF renderer, and the
  discord bot.
- Be a plain drop-in javascript library. No bundling, just drop in 1 `wasm` + 1
  `js` file and include via es6 imports.

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
