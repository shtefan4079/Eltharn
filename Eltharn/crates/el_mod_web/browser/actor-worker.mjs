import init, {admit, admit_script, layout, WorkerHost, WorkerScript} from './eltharn_mod_web.js';
import {stringifyJson} from './json.mjs';
let instance, host, script, memoryMax, failed = false, prepared = false, lastSequence = 0;
function hostCall(op, rp, rl, wp, wl) {
  const memory = instance?.exports.memory;
  if (!(memory instanceof WebAssembly.Memory)) return -9;
  const range = layout(rp, rl, wp, wl, memory.buffer.byteLength);
  if (range[0] < 0) return range[0];
  const [_, read, readLength, write, capacity] = range;
  const request = new Uint8Array(memory.buffer, read, readLength).slice();
  const frame = host.dispatch(op, request, capacity);
  const code = new DataView(frame.buffer, frame.byteOffset, 4).getInt32(0, true);
  if (code >= 0) {
    if (code > capacity || frame.length !== code + 4) throw new Error('Invalid Rust host response');
    new Uint8Array(memory.buffer, write, code).set(frame.subarray(4));
  }
  return code;
}
self.onmessage = async ({data}) => {
  const {sequence, kind, payload} = data ?? {};
  if (failed || !Number.isSafeInteger(sequence) || sequence !== lastSequence + 1) { failed = true; self.close(); return; }
  lastSequence = sequence;
  try {
    if (kind === 'prepare') {
      if (prepared) throw new Error('Actor already prepared');
      await init();
      if (payload.script) {
        const source = JSON.stringify(payload.script);
        memoryMax = admit_script(source);
        script = new WorkerScript(source, stringifyJson(payload.grants));
        prepared = true;
        self.postMessage({sequence, kind, payload: {memory_max: memoryMax}});
        return;
      }
      const bytes = new Uint8Array(payload.bytes);
      memoryMax = admit(bytes);
      host = new WorkerHost(stringifyJson(payload.grants));
      const result = await WebAssembly.instantiate(bytes, {rustore_host_v1: {call: hostCall}});
      instance = result.instance; prepared = true;
      self.postMessage({sequence, kind, payload: {memory_max: memoryMax}});
      return;
    }
    if (kind !== 'invoke' || !prepared) throw new Error('Actor not prepared or unknown operation');
    const {context, snapshot} = payload;
    if (script) {
      const started = performance.now();
      const outcome = JSON.parse(script.invoke_resources(stringifyJson(context), stringifyJson(snapshot), payload.binary_resources ?? new Uint8Array()));
      outcome.elapsed_us = Math.ceil((performance.now()-started)*1000);
      self.postMessage({sequence, kind, payload: outcome});
      if (!outcome.success || context.phase === 'Shutdown') {
        failed = true; script.free(); script = undefined; self.close();
      }
      return;
    }
    const names = {Init:'init', WorldEnter:'on_world_enter', Tick:'tick', AppTick:'app_tick', WorldExit:'on_world_exit', Shutdown:'shutdown'};
    const name = names[context.phase];
    if (!name) throw new Error('Unknown lifecycle phase');
    host.begin(stringifyJson(context), stringifyJson(snapshot));
    const started = performance.now();
    let code = null, error = null;
    try {
      const callback = instance.exports[name];
      if (context.phase === 'AppTick' || (!callback && (context.phase === 'WorldEnter' || context.phase === 'WorldExit'))) code = 0;
      else code = callback();
      if (code !== 0) error = `nonzero exit: ${code}`;
    } catch (e) { error = String(e).slice(0, 512); }
    const success = code === 0 && error === null;
    const effects = JSON.parse(host.finish(success));
    const memory_bytes = instance.exports.memory.buffer.byteLength;
    if (memory_bytes > memoryMax) throw new Error('Guest memory exceeds admitted maximum');
    self.postMessage({sequence, kind, payload: {success, error, effects, memory_bytes, fuel_used:null, elapsed_us:Math.ceil((performance.now()-started)*1000)}});
    if (!success || context.phase === 'Shutdown') { failed = true; host.free(); host = undefined; instance = undefined; self.close(); }
  } catch (e) {
    failed = true;
    self.postMessage({sequence, kind, error:String(e).slice(0, 512)});
    host?.free(); script?.free(); script = undefined; host = undefined; instance = undefined; self.close();
  }
};
