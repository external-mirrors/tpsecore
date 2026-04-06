// todo: finish rewrites of the below

use std::ptr::null;
use std::sync::Arc;

use crate::accel::wasm_texture_handle::WasmTextureHandle;
use crate::import::radiance::parse_radiance_sound_definition;
use crate::import::skin_splicer::Piece;
use crate::render::{BoardElement, FrameInfo, RenderContext, RenderOptions};
use crate::wasm::asynch::spawn;
use crate::wasm::{BUFFER_STATE, TPSE_STATE, TPSEStatus, report_frame_render_done};

/// Returns the sound effect atlas from the given tpse slot
///
/// Return value: null if no such tpse or if the tpse is busy importing or external, otherwise buffer
/// containing serialized atlas. The returned buffer should be deallocated via [deallocate_buffer].
#[unsafe(no_mangle)]
pub extern "C" fn get_atlas(tpse: u32) -> *const u8 {
  let state = TPSE_STATE.lock().unwrap();
  let Some(tpse) = state.tpses.get(&tpse) else { return null() };
  let TPSEStatus::IdleInternal(tpse) = &tpse.status else { return null() };
  let buffer = serde_json::to_vec(&tpse.custom_sound_atlas).unwrap();
  drop(state);
  
  
  let mut state = BUFFER_STATE.lock().unwrap();
  let id = state.next_id();
  state.buffers.insert(id, buffer.into());
  state.buffers.get(&id).unwrap().as_ptr()
}

/// Parses the atlas out of an rsd file
///
/// Return value: null if no such buffer or if parsing fails (see logs), otherwise buffer containing serialized atlas
/// The returned buffer should be deallocated via [deallocate_buffer].
#[unsafe(no_mangle)]
pub extern "C" fn parse_radiance_atlas(rsd_buffer: *mut u8) -> *const u8 {
  let mut state = BUFFER_STATE.lock().unwrap();
  let Some(id) = state.lookup_buffer(rsd_buffer) else { return null() };
  let rsd_buffer = state.buffers.get(&id).unwrap();
  
  let atlas = match parse_radiance_sound_definition(&rsd_buffer[..]) {
    Ok(atlas) => atlas,
    Err(err) => {
      log::error!("failed to parse radiance file: {err}");
      return null();
    }
  };
  
  let parsed_buffer = serde_json::to_vec(&atlas.to_old_style_atlas()).unwrap();
  
  let id = state.next_id();
  state.buffers.insert(id, parsed_buffer.into());
  state.buffers.get(&id).unwrap().as_ptr()
}


/// Prepares render data for a given tpse, which involves decoding assets into directly useable buffers.
/// When no longer necessary, data can be discarded with [discard_render_data]. Data is also freed when
/// the TPSE is deallocated.
/// Return codes: 0=ok, 1=no such tpse, 2=loading failed, 3=tpse external or busy importing
#[unsafe(no_mangle)]
pub extern "C" fn prepare_render_data(tpse_id: u32) -> u32 {
  let mut state = TPSE_STATE.lock().unwrap();
  let Some(tpse) = state.tpses.get_mut(&tpse_id) else { return 1 };
  let TPSEStatus::IdleInternal(tpse_data) = &tpse.status else { return 3 };
  match RenderContext::try_from_tpse(&tpse_data) {
    Err(_err) => {
      // todo: report error text
      2
    }
    Ok(ctx) => {
      tpse.render_data = Some(ctx);
      0
    }
  }
}

/// Throws away decoded buffers to free up memory
/// Return codes: 0=ok, 1=no such tpse
#[unsafe(no_mangle)]
pub extern "C" fn discard_render_data(tpse_id: u32) -> u32 {
  let mut state = TPSE_STATE.lock().unwrap();
  let Some(tpse) = state.tpses.get_mut(&tpse_id) else { return 1 };
  tpse.render_data = None;
  0
}

const fn default_skyline() -> usize { 20 }
const fn default_block_size() -> i64 { 48 }
fn default_board_elements() -> Vec<BoardElement> {
  BoardElement::get_draw_order().to_vec()
}
#[derive(Debug, serde::Deserialize)]
struct RenderFrameArgs {
  board_state: Vec<Vec<Option<(Piece, u8)>>>,
  #[serde(default = "default_board_elements")]
  board_elements: Vec<BoardElement>,
  #[serde(default)]
  debug_grid: bool,
  #[serde(default = "default_skyline")]
  skyline: usize,
  #[serde(default = "default_block_size")]
  block_size: i64
}

// return code 0=queued 1=no such tpse 2=tpse lacks render data 3=no such argument buffer 4=unparseable arguments
#[unsafe(no_mangle)]
pub extern "C" fn render_frame(tpse_id: u32, argument_buffer: *mut u8, nonce: u64) -> u32 {
  let tpse_state = TPSE_STATE.lock().unwrap();
  let Some(tpse) = tpse_state.tpses.get(&tpse_id) else { return 1 };
  let Some(ctx) = tpse.render_data.clone() else { return 2 };
  drop(tpse_state);
  
  let buffer_state = BUFFER_STATE.lock().unwrap();
  let Some(id) = buffer_state.lookup_buffer(argument_buffer) else { return 2 };
  let argument_buffer = buffer_state.buffers.get(&id).unwrap();
  let args: RenderFrameArgs = match serde_json::from_slice(&argument_buffer) {
    Ok(res) => res,
    Err(err) => {
      log::error!("failed to parse render_video arguments: {err:?}");
      return 4;
    }
  };
  drop(buffer_state);
  
  spawn(async move {
    let frame = ctx.render_frame(&FrameInfo {
      real_time: 0.0, // todo
      render_options: &RenderOptions {
        board_elements: &args.board_elements,
        debug_grid: args.debug_grid,
        board: args.board_state.into(),
        skyline: args.skyline,
        block_size: args.block_size
      }
    }).await;
    
    let alloc = |buffer: Arc<[u8]>| {
      let len = buffer.len();
      let mut state = BUFFER_STATE.lock().unwrap();
      let id = state.next_id();
      state.buffers.insert(id, buffer);
      let ptr = state.buffers.get_mut(&id).unwrap().as_ptr();
      (ptr, len)
    };
    
    match frame {
      Err(err) => {
        let (ptr, len) = alloc(err.to_string().into_bytes().into());
        unsafe { report_frame_render_done(tpse_id, nonce, 1, ptr, len); }
      },
      Ok(None) => {
        unsafe { report_frame_render_done(tpse_id, nonce, 2, null(), 0); }
      },
      #[cfg(feature = "wasm_rendering")]
      Ok(Some(frame)) => {
        WasmTextureHandle::force_flush().await;
        let id: u32 = frame.image.id().try_into().unwrap(); // todo: handle >32bit values
        unsafe { report_frame_render_done(tpse_id, nonce, 3, id as *const u8, 0); }
      },
      #[cfg(not(feature = "wasm_rendering"))]
      Ok(Some(frame)) => {
        match frame.image.encode_png().await {
          Err(err) => {
            let (ptr, len) = alloc(err.to_string().into_bytes().into());
            unsafe { report_frame_render_done(tpse_id, nonce, 1, ptr, len); }
          },
          Ok(buffer) => {
            let (ptr, len) = alloc(buffer);
            unsafe { report_frame_render_done(tpse_id, nonce, 0, ptr, len); }
          },
        }
      }
    };
  });
  0
}
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