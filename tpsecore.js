const tpse_import_handlers = new Map();
/**
 * Registers a handler for import-related events from a given tpse handle.
 * The handler lives for one import cycle and is automatically removed once finished.
 * @param {number} tpse_id the id of the tpse to register the handlers for
 * @param {function(number, object)} import_finished called with an exit code and the additional import result information when a queued import finishes 
 * @param {function(object)} decide called when the importer needs an external decision on how to proceed due to multiple import options
 * @param {function(object)} log_sink sink for recieving logs specific to this tpse
 */
export function registerTPSEImportEventHandler(tpse_id, import_finished, decide, log_sink) {
  tpse_import_handlers.set(tpse_id, { import_finished, decide, log_sink });
}

let event_bus_listeners = [];
/**
 * Registers a general event listener
 * The listener can unregister itself by returning false
 */
export function registerListener(listener) {
  event_bus_listeners.push(listener);
}
function sendEvent(event) {
  event_bus_listeners = event_bus_listeners.filter(handler => handler(event) !== false);
}

let externTPSEHandler = null;
/**
 * Registers the extern TPSE handler.
 * 
 * For each callback, the first parameter is the extern tpse id as returned by `allocate_extern_tpse`
 * and the second is the key. The third parameter of `set` is the value to set as json (i.e. a string).
 * The return value of `get` is the retrieved value as json (i.e. a string).
 * 
 * Any of the callbacks may return a rejected promise to propagate an error back to tpsecore.
 * 
 * @param {function(number, number): Promise<String?>} get
 * @param {function(number, number, String): Promise} set
 * @param {function(number, number): Promise} remove
 */
export function setExternalTPSEHandler(get, set, remove) {
  externTPSEHandler = { get, set, remove };
}

let panic_reported = false;
let async_runtime_interval = null;
let cached_assets = {};

let samples = {};
let textures = {};
/**
 * @param {number|object} id index of texture in textures, or a texture object directly
 * @param {function} map a mapping function that accepts the texture, skipped if the texture is error
 * @param {boolean} make_mut creates a copy of a copy-on-write texture if necessary before calling map
 * @param {[number,number,number,number]?} slice propagated slice information, for internal use 
 * @param {number} depth counter of how many texture lookup indirections have been made, for internal use
 * @returns {any} value returned from map function, or an error
 */
function getTexture(id, map, make_mut=false, slice=null, depth=0) {
  let tex = typeof id == 'object' ? id : textures[id];
  if (!tex) {
    console.error("texture", id, "not found");
    return { kind: 'error', error: new Error("no such texture " + id) };
  }
  
  // console.log(`wasm accelerator> ${'  '.repeat(depth)}- getTexture`, { id, tex, make_mut, slice, depth })
  
  switch (tex.kind) {
    case 'error':
      return { kind: 'error', error: tex.error };
      
    case 'canvas':
      if (!slice) slice = [0, 0, tex.canvas.width, tex.canvas.height];
      return map({ texture: tex.canvas, immutable: false, slice });
    
    case 'texture':
      if (!slice) slice = [0, 0, tex.texture.width, tex.texture.height];
      if (make_mut) {
        throw new Error("is this even used? It's not correct, either way. FIXME");
        let canvas = new OffscreenCanvas(slice[2], slice[3]);
        canvas.getContext('2d').drawImage(tex.texture, slice[0], slice[1], slice[2], slice[3], 0, 0, slice[2], slice[3]);
        tex.texture = canvas;
        tex.kind = 'canvas';
        return getTexture(source, map, make_mut, null, depth+1);
      }
      return map({ texture: tex.texture, immutable: true, slice });
      
    case 'slice':
      let { source, x, y, w, h } = tex;
      if (slice) {
        let [x2, y2, w2, h2] = slice;
        x += x2;
        y += y2;
        w = h2;
        h = h2;
      }
      return getTexture(source, map, make_mut, [x, y, w, h], depth+1);
      
    default:
      console.error(tex);
      throw new Error("object of unknown type found in texture list");
  }
}

let tpsecore_url = import.meta.url.replace(/\/[^\/]+$/, '/tpsecore.wasm');
console.log("loading tpsecore.wasm from", tpsecore_url);
const wasm = await WebAssembly.instantiateStreaming(fetch(tpsecore_url), {
  tpsecore: {
    report_import_done(tpse, status, flag_data_ptr, flag_data_len) {
      let flags = JSON.parse(new TextDecoder().decode(new Uint8Array(tpsecore.memory.buffer, flag_data_ptr, flag_data_len)));
      console.log("import", tpse, "done with status", status);
      tpse_import_handlers.get(tpse)?.import_finished(status, flags);
      tpse_import_handlers.delete(tpse);
    },
    report_migration_done(tpse, status) {
      console.log("report_migration_done", tpse, status);
      sendEvent({ kind: 'migration_done', tpse, status });
    },
    report_frame_render_done(tpse, nonce, status, ptr, len) {
      console.log(`report_frame_render_done`, tpse, nonce, status, ptr, len);
      // buffer deallocation needs to happen asynchronously, otherwise we'll trigger reentrant mutex panics
      setTimeout(() => {
        let buffer = new Uint8Array(tpsecore.memory.buffer, ptr, len);
        
        // when status=0 it's the result buffer, when status=2 it's null, otherwise it's an error string
        if (status == 0) {
          sendEvent({ kind: 'frame_render_done', tpse, nonce, buffer: buffer.slice() });
        } else if (status == 2) {
          sendEvent({ kind: 'frame_render_done', tpse, nonce, no_content: true })
        } else {
          let error = new Error('render_frame failed: ' + new TextDecoder().decode(buffer));
          sendEvent({ kind: 'frame_render_done', tpse, nonce, error });
        }
        
        // buffer is cloned (via slice) or decoded into a string at this point
        tpsecore.deallocate_buffer(ptr);
      });
    },
    set_runtime_sleeping(sleeping) {
      clearInterval(async_runtime_interval);
      if (!sleeping) {
        async_runtime_interval = setInterval(() => {
          tpsecore.tick_async();
        });
      }
    },
    log(level, ptr, len) {
      let msg = new TextDecoder('utf-8').decode(new Uint8Array(tpsecore.memory.buffer, ptr, len));
      let logger = null;
      switch (level) {
        case 1: logger = console.error; break;
        case 2: logger = console.warn ; break;
        case 3: logger = console.info ; break;
        case 4: logger = console.debug; break;
        case 5: logger = console.debug; break;
        default: logger = console.log ; break;
      }
      logger(new Date(), "wasm>", msg);
    },
    import_log(tpse, ptr, len) {
      let json = new TextDecoder('utf-8').decode(new Uint8Array(tpsecore.memory.buffer, ptr, len));
      console.log("import log", json);
      let msg = JSON.parse(json);
      tpse_import_handlers.get(tpse)?.log_sink(msg);
      console.debug(new Date(), `wasm (tpse ${tpse})>`, msg);
    },
    report_panic() {
      if (panic_reported) return;
      panic_reported = true;
      console.trace("tpsecore panic");
      sendEvent({ kind: 'panic' });
    },
    async tpse_get(extern_tpse_id, key_ptr, key_len, wake_id) {
      console.log(`tpse_get`, { extern_tpse_id, key_ptr, key_len });
      try {
        let key = new TextDecoder().decode(new Uint8Array(tpsecore.memory.buffer, key_ptr, key_len));
        if (!externTPSEHandler) {
          throw new Error("no extern TPSE handler registered");
        }
        /** @type {String?} */
        let result = await externTPSEHandler.get(extern_tpse_id, key);
        
        if (!result) {
          tpsecore.provide_wakeable_two(wake_id, BigInt(1), BigInt(0));
          return;
        }
        
        let encoded = new TextEncoder().encode(result);
        let ptr = tpsecore.allocate_buffer(encoded.length);
        new Uint8Array(tpsecore.memory.buffer, ptr, encoded.length).set(encoded);
        
        tpsecore.provide_wakeable_two(wake_id, BigInt(0), BigInt(ptr));
      } catch(ex) {
        let encoded = new TextEncoder().encode(ex);
        let ptr = tpsecore.allocate_buffer(encoded.length);
        new Uint8Array(tpsecore.memory.buffer, ptr, encoded.length).set(encoded);
        tpsecore.provide_wakeable_two(wake_id, BigInt(2), BigInt(ptr));
      }
    },
    async tpse_set(extern_tpse_id, key_ptr, key_len, data_ptr, data_len, wake_id) {
      console.log(`tpse_set`, { extern_tpse_id, key_ptr, key_len, data_ptr, data_len });
      try {
        let key = new TextDecoder().decode(new Uint8Array(tpsecore.memory.buffer, key_ptr, key_len));
        let data = new TextDecoder().decode(new Uint8Array(tpsecore.memory.buffer, data_ptr, data_len));
        if (!externTPSEHandler) {
          throw new Error("no extern TPSE handler registered");
        }
        await externTPSEHandler.set(extern_tpse_id, key, data);
        tpsecore.provide_wakeable_one(wake_id, BigInt(0));
      } catch(ex) {
        let encoded = new TextEncoder().encode(ex);
        let ptr = tpsecore.allocate_buffer(encoded.length);
        new Uint8Array(tpsecore.memory.buffer, ptr, encoded.length).set(encoded);
        tpsecore.provide_wakeable_one(wake_id, BigInt(ptr));
      }
    },
    async tpse_delete(extern_tpse_id, key_ptr, key_len, wake_id) {
      console.log(`tpse_delete`, { extern_tpse_id, key_ptr, data_len });
      try {
        let key = new TextDecoder().decode(new Uint8Array(tpsecore.memory.buffer, key_ptr, key_len));
        if (!externTPSEHandler) {
          throw new Error("no extern TPSE handler registered");
        }
        await externTPSEHandler.remove(extern_tpse_id, key);
        tpsecore.provide_wakeable_one(wake_id, BigInt(0));
      } catch(ex) {
        let encoded = new TextEncoder().encode(ex);
        let ptr = tpsecore.allocate_buffer(encoded.length);
        new Uint8Array(tpsecore.memory.buffer, ptr, encoded.length).set(encoded);
        tpsecore.provide_wakeable_one(wake_id, BigInt(ptr));
      }
    }
  },
  wasm_decision_maker: {
    async decide(tpse_id, data, len, wake_id) {
      let options = JSON.parse(new TextDecoder().decode(new Uint8Array(tpsecore.memory.buffer, data, len)));
      
      try {
        let handler = tpse_import_handlers.get(tpse_id);
        if (!handler) throw new Error("no decision handler registered for tpse");
        let result = await handler.decide(options);
        
        let encoded = new TextEncoder().encode(JSON.stringify(result));
        let ptr = tpsecore.allocate_buffer(encoded.length);
        new Uint8Array(tpsecore.memory.buffer, ptr, encoded.length).set(encoded);
        
        tpsecore.provide_wakeable_two(wake_id, BigInt(0), BigInt(ptr));
      } catch(ex) {
        let encoded = new TextEncoder().encode(ex);
        let ptr = tpsecore.allocate_buffer(encoded.length);
        new Uint8Array(tpsecore.memory.buffer, ptr, encoded.length).set(encoded);
        tpsecore.provide_wakeable_two(wake_id, BigInt(1), BigInt(ptr));
      }
    }
  },
  wasm_accelerator_texture: {
    async flush_command_buffer(ptr, len, wake_id) {
      console.log("wasm accelerator> command buffer flush requested", ptr, len, wake_id);
      let start = Date.now();
      let view = new DataView(tpsecore.memory.buffer, ptr, len);
      let flushedCount = 0;
      let offset = 0;
      
      function read(count) {
        // console.log(`wasm accelerator> command buffer read ${count} bytes at position ${offset} (total length: ${len})`);
        let old_offset = offset;
        offset += count;
        return old_offset;
      }
      function toCanvas(drawable, slice) {
        if (!slice) slice = [0, 0, drawable.width, drawable.height];
        let [x, y, w, h] = slice;
        let canvas = new OffscreenCanvas(w, h);
        canvas.getContext('2d').drawImage(drawable, x, y, w, h, 0, 0, w, h);
        return canvas;
      }
      
      while (offset < len) {
        flushedCount++;
        let command = view.getUint8(read(1));
        let handle_id = Number(view.getBigUint64(read(8)));
        // console.log("wasm accelerator> - begin command processing with view", textures, command, handle_id);
        try {
        switch (command) {
          case 0: { // drop texture
            // console.log("wasm accelerator> - flush_command_buffer: drop texture", handle_id);
            delete textures[handle_id]
            break;
          }
          case 1: { // new_texture
            let width = view.getUint32(read(4));
            let height = view.getUint32(read(4));
            // console.log("wasm accelerator> - flush_command_buffer: new_texture", { width, height });
            textures[handle_id] = { kind: 'canvas', canvas: new OffscreenCanvas(width, height) };
            break;
          }
          case 2: { // decode_texture
            let ptr = Number(view.getBigUint64(read(8)));
            let len = Number(view.getBigUint64(read(8)));
            // console.log("wasm accelerator> - flush_command_buffer: decode_texture", { ptr, len });
            try {
              let decoder = new ImageDecoder({
                data: new Uint8Array(tpsecore.memory.buffer, ptr, len),
                type: "image/png" // todo: actually detect image type
              });
              let { image } = await decoder.decode();
              // console.log("wasm accelerator> got decoded", image, image.width, image.codedWidth);
              image.width = image.codedWidth;
              image.height = image.codedHeight;
              textures[handle_id] = { kind: 'texture', texture: image };
            } catch(ex) {
              console.error("wasm accelerator> - image decoding failed:", ex);
              textures[handle_id] = { kind: 'error', error: ex };
            }
            break;
          }
          case 3: { // create_copy
            let new_id = Number(view.getBigUint64(read(8)));
            // console.log("wasm accelerator> - flush_command_buffer: create_copy", { new_id });
            if (textures[handle_id].kind == 'canvas') {
              textures[new_id] = { kind: 'canvas', canvas: toCanvas(textures[handle_id].canvas) }
            } else {
              textures[new_id] = textures[handle_id]; // copy on write
            }
            break;
          }
          case 4: { // slice
            let new_id = Number(view.getBigUint64(read(8)));
            let x = view.getUint32(read(4));
            let y = view.getUint32(read(4));
            let w = view.getUint32(read(4));
            let h = view.getUint32(read(4));
            // console.log("wasm accelerator> - flush_command_buffer: slice", { new_id, x, y, w, h });
            if (!textures[handle_id]) {
              debugger;
              throw new Error("wasm accelerator> slice source invalid: got " + handle_id);
            }
            textures[new_id] = { kind: 'slice', source: textures[handle_id], x, y, w, h };
            break;
          }
          case 5: { // resized
            let new_id = Number(view.getBigUint64(read(8)));
            let nw = view.getUint32(read(4));
            let nh = view.getUint32(read(4));
            // console.log("wasm accelerator> - flush_command_buffer: resized", { new_id, nw, nh });
            // todo: make resizing lazily pass through destination coordiantes
            // so that we can avoid intermediate texture copies
            textures[new_id] = getTexture(handle_id, ({ texture, slice: [x, y, w, h] }) => {
              let canvas = new OffscreenCanvas(nw, nh);
              canvas.getContext('2d').drawImage(texture, x, y, w, h, 0, 0, nw, nh);
              return { kind: 'canvas', canvas };
            });
            break;
          }
          case 6: { // tinted
            let new_id = Number(view.getBigUint64(read(8)));
            let r = view.getUint8(read(1)) / 255;
            let g = view.getUint8(read(1)) / 255;
            let b = view.getUint8(read(1)) / 255;
            let a = view.getUint8(read(1)) / 255;
            // console.log("wasm accelerator> - flush_command_buffer: tinted", { new_id, r, g, b, a });
            textures[new_id] = getTexture(handle_id, ({ texture, slice: [x, y, w, h] }) => {
              let canvas = new OffscreenCanvas(w, h);
              let ctx = canvas.getContext('2d');
              ctx.drawImage(texture, x, y, w, h, 0, 0, w, h);
              ctx.globalCompositeOperation = 'multiply';
              ctx.fillStyle = `rgba(${r}, ${g}, ${b}, ${a})`;
              ctx.fillRect(0, 0, w, h);
              return { kind: 'canvas', canvas };
            });
            break;
          }
          case 7: { // overlay
            let overlay_id = Number(view.getBigUint64(read(8)));
            let draw_at_x = Number(view.getBigInt64(read(8)));
            let draw_at_y = Number(view.getBigInt64(read(8)));
            // console.log("wasm accelerator> - flush_command_buffer: overlay", { overlay_id, draw_at_x, draw_at_y });
            let result = getTexture(handle_id, ({ texture, slice: [x, y, _w, _h] }) => {
              return getTexture(overlay_id, ({ texture: overlay_texture, slice: [x2, y2, w2, h2] }) => {
                texture.getContext('2d').drawImage(overlay_texture, x2, y2, w2, h2, draw_at_x+x, draw_at_y+y, w2, h2);
                return null
              });
            }, true);
            if (result?.error)
              textures[handle_id] = result;
            break;
          }
          case 8: { // draw_line
            let x1 = view.getFloat32(read(4));
            let y1 = view.getFloat32(read(4));
            let x2 = view.getFloat32(read(4));
            let y2 = view.getFloat32(read(4));
            let r = view.getUint8(read(1)) / 255;
            let g = view.getUint8(read(1)) / 255;
            let b = view.getUint8(read(1)) / 255;
            let a = view.getUint8(read(1)) / 255;
            // console.log("wasm accelerator> - flush_command_buffer: draw_line", { x1, y1, x2, y2, r, g, b, a });
            let result = getTexture(handle_id, ({ texture, slice: [x, y, _w, _h] }) => {
              let ctx = texture.getContext('2d');
              // todo: ensure line remains within slice boundaries
              ctx.strokeStyle = `rgba(${r}, ${g}, ${b}, ${a})`;
              ctx.beginPath();
              ctx.moveTo(x1+x, y1+y);
              ctx.lineTo(x2+x, y2+y);
              ctx.strokePath();
              return null;
            }, true);
            if (result.error)
              textures[handle_id] = result;
            break;
          }
          case 9: { // draw_text
            let r = view.getUint8(read(1)) / 255;
            let g = view.getUint8(read(1)) / 255;
            let b = view.getUint8(read(1)) / 255;
            let a = view.getUint8(read(1)) / 255;
            let ox = view.getInt32(read(4));
            let oy = view.getInt32(read(4));
            let fontsize = view.getFloat32(read(4));
            let ptr = Number(view.getBigUint64(read(8)));
            let len = Number(view.getBigUint64(read(8)));
            // console.log("wasm accelerator> - flush_command_buffer: draw_text", { r, g, b, a, ox, oy, fontsize, ptr, len });
            let result = getTexture(handle_id, ({ texture, slice: [x, y, _w, _h] }) => {
              // todo: ensure text remains within slice boundaries
              let string = new TextDecoder().decode(new Uint8Array(tpsecore.memory.buffer, ptr, len));
              ctx.fillStyle = `rgba(${r}, ${g}, ${b}, ${a})`;
              ctx.font = `${fontsize}px sans-serif`;
              texture.getContext('2d').fillText(string, x+ox, y+oy);
              return null;
            }, true);
            if (result.error)
              textures[handle_id] = result;
            break;
          }
        }
        } catch(ex) {
          console.error("wasm accelerator> error processing command buffer", ex);
          return;
        }
      }
      
      console.log(`wasm accelerator> - command buffer flush (${flushedCount} items, ${offset} bytes flushed in ${Date.now() - start}ms) completed, waking task ${wake_id}`);
      let result = tpsecore.provide_wakeable_zero(wake_id);
      if (result != 0) console.error("wasm accelerator> - flush_command_buffer failed to provide asynchronous return value: " + result);
    },
    fetch_dimensions(id, code_ptr, width_ptr, height_ptr) {
      let view = new DataView(tpsecore.memory.buffer)
      let result = getTexture(id, ({ slice: [_x, _y, w, h] }) => {
        console.log("wasm accelerator> fetch_dimensions", { id, w, h, code_ptr, width_ptr, height_ptr });
        view.setBigUint64(code_ptr, BigInt(0), true);
        view.setUint32(width_ptr, w, true);
        view.setUint32(height_ptr, h, true);
      });
      if (result?.error) {
        let buffer = new TextEncoder().encode(result.error);
        let ptr = tpsecore.allocate_buffer(buffer.length);
        new Uint8Array(tpsecore.memory.buffer, ptr, buffer.length).set(buffer);
        view.setBigUint64(code_ptr, BigInt(ptr), true);
      }
    },
    async fraction_opaque(id, code_ptr) {
      let view = new DataView(tpsecore.memory.buffer);
      let value = null;
      let result = getTexture(id, ({ texture, slice: [x, y, w, h] }) => {
        console.log("wasm accelerator> fraction_opaque", { id });
        
        // todo: ensure texture is a canvas so we can extract data directly rather than
        // always making an intermediate copy.
        let canvas = new OffscreenCanvas(w, h);
        let ctx = canvas.getContext('2d');
        ctx.drawImage(texture, x, y, w, h, 0, 0, w, h);
        let data = ctx.getImageData(0, 0, w, h).data;
        let opaque = 0;
        for (let i = 0; i < data.length; i += 4)
          if (data[i + 3] > 0)
            opaque += 1;
        
        view.setBigUint64(code_ptr, BigInt(0), true);
        value = opaque / (w * h);
      });
      if (result?.error) {
        let buffer = new TextEncoder().encode(result.error);
        let ptr = tpsecore.allocate_buffer(buffer.length);
        new Uint8Array(tpsecore.memory.buffer, ptr, buffer.length).set(buffer);
        view.setBigUint64(code_ptr, BigInt(ptr), true);
      }
      return value;
    },
    async encode_png(id, wake_id) {
      try {
        let source_texture = getTexture(id, tex => tex);
        if (source_texture.error)
          throw new Error("source texture in error state: " + source_texture.error);
        let { texture, slice: [x, y, w, h] } = source_texture;
        let canvas = new OffscreenCanvas(w, h);
        canvas.getContext('2d').drawImage(texture, x, y, w, h, 0, 0, w, h);
        let blob = await canvas.convertToBlob();
        let reader = new FileReader(blob);
        await new Promise((res, rej) => {
          reader.onload = res;
          reader.onerror = rej;
          reader.readAsArrayBuffer(blob);
        });
        let buffer = new Uint8Array(reader.result);
        let ptr = tpsecore.allocate_buffer(buffer.length);
        new Uint8Array(tpsecore.memory.buffer, ptr, buffer.length).set(buffer);
        let result = tpsecore.provide_wakeable_two(wake_id, BigInt(0), BigInt(ptr));
        if (result != 0) console.error("wasm accelerator> encode_png failed to provide asynchronous return value: " + result);
      } catch(ex) {
        console.error("wasm accelerator> encode_png failed:", ex);
        let buffer = new TextEncoder().encode(ex);
        let ptr = tpsecore.allocate_buffer(buffer.length);
        new Uint8Array(tpsecore.memory.buffer, ptr, buffer.length).set(buffer);
        let result = tpsecore.provide_wakeable_two(wake_id, BigInt(1), BigInt(ptr));
        if (result != 0) console.error("wasm accelerator> encode_png failed to provide asynchronous return value: " + result);
        return;
      }
    }
  },
  wasm_accelerator_audio: {
    new_from_samples(id, sample_ptr, sample_len) {
      let view = new DataView(tpsecore.memory.buffer, sample_ptr, sample_len);
      let buffer = new Float32Array(sample_len/4);
      for (let i = 0; i < sample_len/4; i++) {
        buffer.set(i, view.getFloat32(i*4, true));
      }
      samples[id] = { buffer, start: 0, len: buffer.length };
    },
    async decode_audio(id, buffer_ptr, buffer_len, ext_ptr, ext_len, wake_id) {
      let _extension = ext_ptr && new TextDecoder().decode(new Uint8Array(tpsecore.memory.buffer, ext_ptr, ext_len));
      let buffer = new Uint8Array(tpsecore.memory.buffer, buffer_ptr, buffer_len).slice();
      try {
        if (globalThis.TPSECORE_EXTERNAL_AUDIO_DECODE) {
          let decoded = await globalThis.TPSECORE_EXTERNAL_AUDIO_DECODE(buffer);
          samples[id] = { buffer: decoded, start: 0, len: decoded.length };
          let result = tpsecore.provide_wakeable_one(wake_id, BigInt(0));
          if (result != 0) console.error("wasm accelerator> decode_audio failed to provide asynchronous return value: " + result);
        } else if (globalThis.AudioContext) {
          let ctx = new AudioContext({ sampleRate: 48000 });
          let decoded = await ctx.decodeAudioData(buffer.buffer);
          if (decoded.numberOfChannels != 2)
            throw new Error("channel counts other than 2 are not supported, found " + decoded.numberOfChannels);
          let decoded_buffer = new Float32Array(decoded.length * 2);
          let c0 = decoded.getChannelData(0);
          let c1 = decoded.getChannelData(1);
          for (let i = 0; i < decoded.length; i++) {
            decoded_buffer[i*2+0] = c0[i];
            decoded_buffer[i*2+1] = c1[i];
          }
          
          samples[id] = { buffer: decoded_buffer, start: 0, len: decoded_buffer.length };
          let result = tpsecore.provide_wakeable_one(wake_id, BigInt(0));
          if (result != 0) console.error("wasm accelerator> decode_audio failed to provide asynchronous return value: " + result);
        } else {
          throw new Error("wasm audio decode has no suitable implementations for this context");
        }
      } catch(ex) {
        console.error("wasm accelerator> decode_audio failed:", ex);
        let buffer = new TextEncoder().encode(ex);
        let ptr = tpsecore.allocate_buffer(buffer.length);
        new Uint8Array(tpsecore.memory.buffer, ptr, buffer.length).set(buffer);
        let result = tpsecore.provide_wakeable_one(wake_id, BigInt(ptr));
        if (result != 0) console.error("wasm accelerator> decode_audio failed to provide asynchronous return value: " + result);
      }
    },
    slice(source_id, id, start, len) {
      let source = samples[source_id];
      if (!source) {
        samples[id] = { error: new Error("attempt to slice unknown buffer: " + source_id) };
        return;
      }
      if (source.error) {
        samples[id] = { error: new Error("attempt to slice buffer in error state: " + source.error) };
        return;
      }
      samples[id] = { buffer: source.buffer, start: source.start + start, len: len };
    },
    length(id) {
      let source = samples[id];
      if (!source) return 0;
      if (source.error) return 0;
      return source.len;
    },
    read(id, buf_ptr, buf_len) {
      let source = samples[id];
      if (!source) return 1;
      if (source.error) return 1;
      
      let stride = 4*2; // (f32, f32)
      let view = new DataView(tpsecore.memory.buffer, buf_ptr, buf_len);
      for (let i = source.start; i < Math.min(source.end, buf_len/stride); i++) {
        view.setFloat32((i - start)*stride, source.buffer[i], true);
      }
      return 0;
    },
    async encode_ogg(id_buf_ptr, id_buf_len, wake_id) {
      try {
        let id_view = new DataView(tpsecore.memory.buffer, id_buf_ptr, id_buf_len);
        /** @type {Float32Array[]} */
        let in_buffers = [];
        let total_len_bytes = 0;
        for (let i = 0; i < id_buf_len/4; i++) {
          let id = id_view.getUint32(i*4, true);
          let source = samples[id];
          if (!source) {
            err = new Error("unknown source " + id);
            break;
          }
          if (source.error) {
            err = new Error("source " + id + " in error state: " + source.error);
            break;
          }
          
          // slice/subarray just returns empty views for the exact same code and I have no idea why (???)
          // manual copying it is...
          if (source.start + source.len > source.buffer.length)
            throw new Error(`audio slice overflows its underlying buffer size (${source.start} + ${source.len} -> ${source.start + source.len} > ${source.buffer.length})`);
          let copy = new Float32Array(source.len);
          for (let i = 0; i < source.len; i++)
            copy[i] = source.buffer[source.start + i];
          
          in_buffers.push(copy);
          total_len_bytes += copy.byteLength;
        }
        
        let encoded_data_ptr = null;
        
        // use OggVorbisEncoder.js, if it's available
        if (globalThis.OggVorbisEncoder) {
          let encoder = new OggVorbisEncoder(48000, 2, 1); // quality is 0.5 in tetrioplus
          let last_pause = Date.now();
          let is_worker = (
            typeof DedicatedWorkerGlobalScope !== 'undefined' &&
            self instanceof DedicatedWorkerGlobalScope
          );
          for (let buffer of in_buffers) {
            let left = new Float32Array(buffer.length/2);
            let right = new Float32Array(buffer.length/2);
            for (let i = 0; i < buffer.length/2; i += 1) {
              left[i] = buffer[i*2]
              right[i] = buffer[i*2+1];
            }
            encoder.encode([left, right]);
            
            // give control back to the browser for a bit so it doesn't seem quite as frozen
            if (Date.now() - last_pause > 100 && !is_worker) {
              last_pause = Date.now();
              await new Promise(res => setTimeout(res, 1));
            }
          }
          let blob = encoder.finish(["audio/ogg"]);
          let buf = new Uint8Array(await blob.arrayBuffer());
          encoded_data_ptr = tpsecore.allocate_buffer(buf.byteLength);
          new Uint8Array(tpsecore.memory.buffer, encoded_data_ptr, buf.byteLength).set(buf);
        }
        // Would be nice to use the new AudioEncoder API, but firefox mobile doesn't support it
        // (and also I can't get ogg header checksums working...)
        else if (false && globalThis.AudioEncoder) {
          let chunks = [];
          let encoded_len = 0;
          let page_seq = 0;
          let error = null;
          let encoder = new AudioEncoder({
            output: chunk => {
              console.log("Got chunk of length", chunk.byteLength);
              
              let segment_count = Math.ceil(chunk.byteLength / 255);
              let extra_segment = chunk.byteLength % 255 == 0;
              if (extra_segment) segment_count += 1;
              
              if (segment_count > 255)
                throw new Error("multipage segments not implemented");
              
              let page = new Uint8Array(24+segment_count+chunk.byteLength);
              let view = new DataView(page.buffer);
              let write_string = (offset, str) => {
                for (let i = 0; i < str.length; i++)
                  view.setUint8(offset + i, str.charCodeAt(i));
              }
              // https://en.wikipedia.org/wiki/Ogg#Page_structure
              write_string(0, "OggS");
              view.setUint8(4, 0); // version
              view.setUint8(5, 0); // header type, populated later
              
              // convert microseconds to sample
              let sample_position = (BigInt(chunk.timestamp) * BigInt(48000)) / BigInt(1_000_000);
              view.setBigUint64(6, sample_position, true); // granule position
              view.setUint32(14, 0, true); // bitstream serial
              view.setUint32(18, page_seq++, true); // page sequence number
              view.setUint32(22, 0, true); // checksum, populated later
              
              view.setUint8(23, segment_count, true); // page segments
              for (let i = 0; i < segment_count; i++) { // page segment table
                view.setUint8(24+i, Math.min(chunk.byteLength - i*255, 255));
              }
              
              chunk.copyTo(page.subarray(24+segment_count, 24+segment_count+chunk.byteLength));
              
              chunks.push(page);
              encoded_len += page.byteLength;
            },
            error: err => error = err
          });
          encoder.configure({
            codec: 'opus',
            opus: { format: 'ogg' },
            sampleRate: 48000,
            numberOfChannels: 2,
          });
          let enc_offset = 0;
          for (let chunk of in_buffers) {
            encoder.encode(new AudioData({
              format: 'f32',
              sampleRate: 48000,
              numberOfChannels: 2,
              numberOfFrames: chunk.length / 2,
              timestamp: enc_offset / (48000 * 2) * 1_000_000,
              data: chunk
            }));
            enc_offset += chunk.length;
          }
          await encoder.flush();
          if (error) throw error;
          
          let header = new DataView(chunks[0].buffer); // set beginning of stream marker
          header.setUint8(5, header.getUint8(5) | 0x02); 
          let trailer = new DataView(chunks[chunks.length-1].buffer); // set end of stream marker
          trailer.setUint8(5, trailer.getUint8(5) | 0x04);
          for (let chunk of chunks) { // checksums
            // can't figure this bit out, no actual implementation of crc32() provided
            // always ends up throwing bad crc errors
            let checksum = crc32(chunk);
            new DataView(chunk.buffer).setUint32(22, checksum, true);
          }
          
          encoded_data_ptr = tpsecore.allocate_buffer(encoded_len);
          let target = new Uint8Array(tpsecore.memory.buffer, encoded_data_ptr, encoded_len);
          let copy_offset = 0;
          for (let chunk of chunks) {
            target.subarray(copy_offset, copy_offset+chunk.byteLength).set(chunk);
            copy_offset += chunk.byteLength;
          }
        }
        // lightweight fallback wav muxing
        else {
          console.warn("using fallback wav muxing - this will result in very large sound effect file sizes!");
          let length = 44 + total_len_bytes;
          encoded_data_ptr = tpsecore.allocate_buffer(length);
          let view = new DataView(tpsecore.memory.buffer, encoded_data_ptr, length);
          let write_string = (offset, str) => {
            for (let i = 0; i < str.length; i++)
              view.setUint8(offset + i, str.charCodeAt(i));
          }
          
          // We decode the audio data using AudioContext.decodeAudioData,
          // which automatically resamples to the AudioContext's sample rate.
          // the default is 48000 but we also explicitly set it because that
          // matches the sample rate of the ogg file embedded in tetrio.opus.rsd
          let sampleRate = 48000;
          let bitDepth = 32;
          let byteDepth = bitDepth/8;
          let channels = 2;
          // https://en.wikipedia.org/wiki/WAV#WAV_file_header
          write_string(0, 'RIFF');
          view.setUint32(4, 44 - 8 + total_len_bytes, true);
          write_string(8, 'WAVEfmt ');
          view.setUint32(16, 16, true); // BlocSize
          view.setUint16(20, 3, true); // AudioFormat (3=float)
          view.setUint16(22, channels, true); // NbrChannels
          view.setUint32(24, sampleRate, true); // Frequency
          view.setUint32(28, sampleRate * byteDepth * channels, true); // BytePerSec
          view.setUint16(32, channels * byteDepth, true); // BytePerBloc
          view.setUint16(34, bitDepth, true); // BitsPerSample
          write_string(36, 'data');
          view.setUint32(40, total_len_bytes, true); // DataSize
          let offset = 44;
          for (let chunk of in_buffers) {
            for (let float of chunk) {
              view.setFloat32(offset, float, true);
              offset += 4;
            }
          }
        }
        
        let result = tpsecore.provide_wakeable_two(wake_id, BigInt(0), BigInt(encoded_data_ptr));
        if (result != 0) console.error("wasm accelerator> encode_ogg failed to provide asynchronous return value: " + result);
      } catch(ex) {
        console.error("wasm accelerator> encode_ogg failed:", ex);
        debugger;
        let buffer = new TextEncoder().encode(ex);
        let ptr = tpsecore.allocate_buffer(buffer.length);
        new Uint8Array(tpsecore.memory.buffer, ptr, buffer.length).set(buffer);
        let result = tpsecore.provide_wakeable_two(wake_id, BigInt(1), BigInt(ptr));
        if (result != 0) console.error("wasm accelerator> encode_ogg failed to provide asynchronous return value: " + result);
      }
    },
    drop(id) {
      delete samples[id];
    },
  },
  wasm_accelerator_asset: {
    async fetch_asset(asset_id) {
      if (cached_assets[asset_id]) {
        console.log("fetch_asset", asset_id, "already cached, providing.");
        tpsecore.provide_asset(asset_id, cached_assets[asset_id]);
        return;
      }
      
      console.log("fetch_asset", asset_id);
      let asset = null;
      let electron = globalThis?.location?.href?.startsWith('tetrio-plus-internal://');
      let backend = (
        // environment-defined route
        globalThis.TPSECORE_EXTERNAL_ASSET_BACKEND_FORMATTER ? globalThis.TPSECORE_EXTERNAL_ASSET_BACKEND_FORMATTER :
        // this route bypasses CORS on TETR.IO Desktop
        electron ? (folder, asset) => `tetrio-plus://tetrio-plus/${folder}/${asset}?bypass-tetrio-plus` :
        // general fallback mainly used from within tetrio plus on firefox,
        // where CORS are relaxed due to explicit site privilege in extension manifest
        (folder, asset) => `https://tetr.io/${folder}/${asset}?bypass-tetrio-plus`
      );
      switch(asset_id) {
        case 0: asset = backend('js', 'tetrio.js'); break;
        case 1: asset = backend('sfx', 'tetrio.opus.rsd'); break;
        case 2: throw new Error("unknown asset #" + asset_id);
      }
      try {
        let body = new Uint8Array(await fetch(asset).then(res => res.arrayBuffer()));
        console.log("fetch_asset", asset_id, "done, got", body.length, "bytes");
        let ptr = tpsecore.allocate_buffer(body.length);
        new Uint8Array(tpsecore.memory.buffer, ptr, body.length).set(body);
        tpsecore.provide_asset(asset_id, ptr);
        cached_assets[asset_id] = ptr;
        console.log("asset marked provided");
      } catch(ex) {
        console.error("fetch_asset", asset_id, "failed:", ex);
        tpsecore.provide_asset(asset_id, 0);
      }
    }
  }
});
export const tpsecore = wasm.instance.exports;