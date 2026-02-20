// todo: finish rewrites of the below

// #[wasm_bindgen]
// pub fn get_atlas(tpse: u32) -> Result<JsValue, JsError> {
//   log::debug!("[TPSE {}] Get atlas", tpse);
//   with_tpse(tpse, |tpse, _| Ok(JsValue::from_serde(&tpse.get().custom_sound_atlas).unwrap()))
// }
// 
// #[wasm_bindgen]
// pub fn get_default_atlas() -> Result<JsValue, JsError> {
//   log::debug!("[TPSE] Get default atlas");
//   let state = GLOBAL_STATE.lock().unwrap();
//   let tetriojs = state.provider.provide(Asset::TetrioJS).map_err(stringify_error)?;
//   let atlas = custom_sound_atlas(tetriojs).map_err(stringify_error)?;
//   Ok(JsValue::from_serde(&atlas).unwrap())
// }
// 
// const fn default_frame_rate() -> f64 { 60.0 }
// const fn default_skyline() -> usize { 20 }
// const fn default_block_size() -> i64 { 48 }
// fn default_board_elements() -> Vec<BoardElement> {
//   BoardElement::get_draw_order().to_vec()
// }
// #[derive(Debug, serde::Deserialize)]
// struct RenderArgs {
//   frames: Vec<Vec<Vec<Option<(Piece, u8)>>>>,
//   #[serde(default)]
//   frame_duration: f64,
//   #[serde(default = "default_frame_rate")]
//   frame_rate: f64,
//   #[serde(default = "default_board_elements")]
//   board_elements: Vec<BoardElement>,
//   #[serde(default)]
//   debug_grid: bool,
//   #[serde(default = "default_skyline")]
//   skyline: usize,
//   #[serde(default = "default_block_size")]
//   block_size: i64,
//   #[serde(default)]
//   sound_effects: Vec<SoundEffectInfo<'static>>
// }
// 
// #[wasm_bindgen]
// pub fn render_video(tpse: u32, args: JsValue) -> Result<JsValue, JsError> {
//   let args: RenderArgs = args.into_serde().map_err(stringify_error)?;
//   log::debug!("[TPSE {}] Render video: {:?}", tpse, args);
//   with_tpse(tpse, |tpse, opts| {
//     let ctx = VideoContext { frame_rate: args.frame_rate };
//     let frames = render_frames(&ctx, tpse.get(), args.frames.into_iter().map(|frame| {
//       RenderOptions {
//         duration: args.frame_duration,
//         board_elements: &args.board_elements,
//         debug_grid: args.debug_grid,
//         board: frame.into(),
//         skyline: args.skyline,
//         block_size: args.block_size
//       }
//     })).map_err(|err| ImportError::with_no_context(err.into()))?;
//     let frames = frames.map(|el| {
//       let encoded = if el.image.width() == 0 || el.image.height() == 0 {
//         include_bytes!("../../assets/empty.png").to_vec()
//       } else {
//         let mut output = vec![];
//         el.image.write_to(&mut Cursor::new(&mut output), ImageOutputFormat::Png).unwrap();
//         output
//       };
//       Frame { image: encoded, min_x: el.min_x, min_y: el.min_y, max_x: el.max_x, max_y: el.max_y }
//     }).collect::<Vec<_>>();
//     let audio = render_sound_effects(&ctx, tpse.get(), &args.sound_effects).unwrap(); // todo: not unwrap
//     Ok(JsValue::from_serde(&(frames, audio.binary)).unwrap())
//   })
// }
// 
// #[wasm_bindgen]
// pub fn render_sound_effect(tpse: u32, sound: &str) -> Result<Option<Vec<f32>>, JsError> {
//   log::debug!("[TPSE {}] Render sound effect", tpse);
//   with_tpse(tpse, |tpse, opts| {
//     if tpse.cached_tetrio_atlas_decoder.is_none() {
//       let decoded = TetrioAtlasDecoder
//         ::decode_from_tpse(tpse.get())
//         .map_err(ImportError::with_no_context)?;
//       tpse.cached_tetrio_atlas_decoder = Some(decoded);
//     }
//     let decoder = tpse.cached_tetrio_atlas_decoder.as_ref().unwrap();
//     Ok(decoder.lookup(sound).map(|slice| slice.to_vec()))
//   })
// }
// 
// #[wasm_bindgen]
// pub fn render_default_sound_effect(sound: &str) -> Result<Vec<f32>, JsError> {
//   log::debug!("[TPSE] Render default sound effect");
//   let mut state = GLOBAL_STATE.lock().unwrap();
//   if state.default_context.cached_tetrio_atlas_decoder.is_none() {
//     state.default_context.cached_tetrio_atlas_decoder = Some({
//       let tetrio_js = state.provider.provide(Asset::TetrioJS)?;
//       let tetrio_ogg = state.provider.provide(Asset::TetrioOGG)?;
//       let atlas = custom_sound_atlas(tetrio_js).map_err(|err| ImportErrorType::AssetParseFailure(err))?;
//       TetrioAtlasDecoder::decode(atlas.clone(), tetrio_ogg, Some("ogg"))?
//     });
//   }
//   let decoder = state.default_context.cached_tetrio_atlas_decoder.as_ref().unwrap();
//   let samples = decoder.lookup(sound)
//     .ok_or_else(|| RenderFailure::NoSoundSoundEffect(sound.to_string()))
//     .map_err(stringify_error)?
//     .to_vec();
//   Ok(samples)
// }