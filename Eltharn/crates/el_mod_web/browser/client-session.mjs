import {stringifyJson} from './json.mjs';
const PREPARE_TIMEOUT_MS = 20000;
const PHASE_TIMEOUT_MS = 10000;
const INPUT_CAP_BYTES = 1 << 20;
const MAX_DEFINITIONS = 16;

function byteLength(s) {
  return typeof TextEncoder !== "undefined"
    ? new TextEncoder().encode(s).length
    : s.length;
}

export class WebSession {
  generation = 0;
  running = false;
  activePhase = null;

  constructor(worker) {
    this._worker = worker;
    this._seq = 0;
    this._pending = null;
    this._report = null;
    this._contentInput = null;
    this._appInput = null;
    this._keybindingInput = null;
    this._disposed = false;
    this._dead = false;
    this._onVisibility = () => {
      try {
        if (typeof document !== "undefined" && document.visibilityState === "hidden") this.dispose();
      } catch {              }
    };
    this._onMessage = (ev) => this._handleMessage(ev);
    this._onFailure = (err) => this._fail(err);
    worker.onmessage = this._onMessage;
    worker.onerror = (ev) => this._onFailure(ev?.error ?? ev?.message ?? new Error("scheduler error"));
    worker.onmessageerror = (ev) => this._onFailure(ev?.data ?? new Error("scheduler messageerror"));
    if (typeof document !== "undefined" && document.addEventListener) {
      document.addEventListener("visibilitychange", this._onVisibility);
    }
  }

  static async prepare(definitions, side = 1) {
    if (!Array.isArray(definitions)) throw new TypeError("definitions must be an array");
    if (!definitions.length) return null;
    if (globalThis.document?.visibilityState === "hidden") throw new Error("tab-suspended");
    if (definitions.length > MAX_DEFINITIONS) throw new RangeError("max 16 definitions");
    const worker = new Worker(new URL("./scheduler-worker.mjs", import.meta.url), { type: "module" });
    const session = new WebSession(worker);
    try {
      const report = await session._request("prepare", { definitions, side }, PREPARE_TIMEOUT_MS);
      session._report = report;
      return session;
    } catch (err) {
      session.dispose();
      throw err;
    }
  }

  report() {
    if (this._disposed) throw new Error("disposed");
    if (this._report == null) throw new Error("no report cached");
    return this._report;
  }

  setContentInput(jsonString) {
    if (this._disposed) throw new Error("disposed");
    if (this.running) throw new Error("busy: idle only");
    if (typeof jsonString !== "string") throw new TypeError("input must be a JSON string");
    if (byteLength(jsonString) > INPUT_CAP_BYTES) throw new RangeError("input exceeds 1MiB");
    this._contentInput = jsonString;
  }

  async phase(phase) {
    if (this._disposed) throw new Error("disposed");
    if (this._dead) throw new Error("scheduler dead");
    if (phase === "Init") throw new Error("Init already done in prepare");
    if (!["WorldEnter", "Tick", "AppTick", "WorldExit", "Shutdown"].includes(phase)) throw new RangeError(`invalid phase: ${String(phase)}`);
    if (this.running) throw new Error("busy: one pending phase");
    this.running = true;
    this.activePhase = phase;
    const input = phase === "Tick" ? this._contentInput : phase === "AppTick" ? this._appInput : null;
    const keybindings = ["Tick", "AppTick"].includes(phase) ? this._keybindingInput : null;
    this._contentInput = null;
    this._appInput = null;
    this._keybindingInput = null;
    try {
      const report = await this._request("phase", { phase, input, keybindings }, PHASE_TIMEOUT_MS);
      this._report = report;
      return report;
    } catch (error) {
      this._fail(error);
      throw error;
    } finally {
      this.running = false;
      this.activePhase = null;
    }
  }

  cancelTick() {
    if (this._disposed || this._dead) return false;
    if (!this.running || !['Tick','AppTick'].includes(this.activePhase)) return false;
    const pending = this._pending;
    if (!pending) return false;
    if (this.generation >= Number.MAX_SAFE_INTEGER) { this._fail(new Error("generation exhausted")); return false; }
    this.generation += 1;
    try {
      this._worker?.postMessage({ kind: "cancel", target: pending.sequence });
    } catch {                                     }
    return true;
  }

  dispose() {
    if (this._disposed) return;
    this._disposed = true;
    this.generation += 1;
    const pending = this._pending;
    this._pending = null;
    if (pending) {
      clearTimeout(pending.timer);
      pending.reject(new Error("disposed"));
    }
    this._removeVisibility();
    try {
      this._worker?.postMessage({ kind: "dispose" });
    } catch {              }
    try {
      this._worker?.terminate();
    } catch {              }
    if (this._worker) { this._worker.onmessage = null; this._worker.onerror = null; this._worker.onmessageerror = null; }
    this._worker = null;
    this._report = null;
    this._contentInput = null;
    this._appInput = null;
    this._keybindingInput = null;
    this.running = false;
    this.activePhase = null;
  }

  _nextSequence() {
    if (this._seq >= Number.MAX_SAFE_INTEGER) throw new Error("sequence exhausted");
    this._seq += 1;
    return this._seq;
  }
  setAppInput(jsonString) {
    if (this._disposed || this.running) throw new Error('App input requires an idle session');
    if (typeof jsonString !== 'string' || byteLength(jsonString) > 262144) throw new Error('Invalid app input');
    this._appInput = jsonString;
  }
  setKeybindingInput(jsonString) {
    if (this._disposed || this.running) throw new Error('Keybinding input requires an idle session');
    if (typeof jsonString !== 'string' || byteLength(jsonString) > 262144) throw new Error('Invalid keybinding input');
    this._keybindingInput = jsonString;
  }

  _request(kind, payload, timeoutMs) {
    if (this._disposed) return Promise.reject(new Error("disposed"));
    if (this._dead) return Promise.reject(new Error("scheduler dead"));
    if (this._pending) return Promise.reject(new Error("busy: one pending RPC"));
    const sequence = this._nextSequence();
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this._fail(new Error(`${kind} timed out`));

      }, timeoutMs);
      this._pending = { sequence, kind, resolve, reject, timer };
      try {
        this._worker.postMessage({ sequence, kind, payload });
      } catch (err) {
        this._fail(err);
      }
    });
  }

  _handleMessage(ev) {
    if (this._disposed) return;
    const msg = ev?.data, pending = this._pending;
    if (!pending || !msg || msg.sequence !== pending.sequence || msg.kind !== pending.kind) {
      this._fail(new Error("foreign-duplicate-late scheduler reply")); return;
    }
    if (typeof msg.error === "string") { this._fail(new Error(msg.error.slice(0, 512))); return; }
    try {
      if (!msg.payload || typeof msg.payload !== "object" || byteLength(stringifyJson(msg.payload)) > 8 * 1024 * 1024) throw new Error("invalid scheduler report");
    } catch (error) { this._fail(error); return; }
    this._pending = null;
    clearTimeout(pending.timer);
    pending.resolve(msg.payload);
  }

  _fail(error) {
    if (this._disposed) return;
    const pending = this._pending;
    this._pending = null;
    if (pending) {
      clearTimeout(pending.timer);
      pending.reject(new Error(String(error).slice(0, 512)));
    }
    this._dead = true;
    this.dispose();
  }

  _removeVisibility() {
    try {
      if (typeof document !== "undefined" && document.removeEventListener) {
        document.removeEventListener("visibilitychange", this._onVisibility);
      }
    } catch {              }
  }
}
