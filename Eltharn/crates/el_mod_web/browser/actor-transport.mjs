const LIMITS_V1 = { version: 1, prepareMs: 15000, callbackMs: 250, maxBytes: 1 << 20 };

export class ActorTransport {
  constructor(url, opts = {}) {
    this.url = url;
    this.W = opts.WorkerClass ?? Worker;
    this.setT = (opts.clock?.setTimeout ?? setTimeout).bind(globalThis);
    this.clearT = (opts.clock?.clearTimeout ?? clearTimeout).bind(globalThis);
    this.limits = LIMITS_V1;
    this.now = opts.clock?.now ?? (() => performance.now());
    this.seq = 0; this.pending = null; this.worker = null;
    this.failed = null; this.disposed = false;
    this._onMsg = (e) => this._handle(e?.data);
    this._onErr = (e) => this._fail(e?.error ?? e?.message ?? e);
    this._onVis = () => { if (this._hidden()) this._fail(new Error('tab-suspended')); };
  }
  get started() { return !!this.worker; }
  start(payload) { return this._rpc('prepare', payload, this.limits.prepareMs, true); }
  invoke(payload) { return this._rpc('invoke', payload, this.limits.callbackMs, false); }
  _hidden() { return globalThis.document?.visibilityState === 'hidden'; }
  _rpc(kind, payload, budgetMs, isFirst) {
    if (this.disposed) return Promise.reject(new Error('disposed'));
    if (this.failed) return Promise.reject(this.failed);
    if (this.pending) return Promise.reject(new Error('one-outstanding-only'));
    if (isFirst && this.worker) return Promise.reject(new Error('already-started'));
    if (typeof SharedArrayBuffer !== 'undefined' && payload instanceof SharedArrayBuffer) return Promise.reject(new Error('no-sab'));
    if (this._hidden()) return Promise.reject(new Error('tab-suspended'));
    if (this.seq + 1 > Number.MAX_SAFE_INTEGER) { this._fail(new Error('seq-exhausted')); return Promise.reject(this.failed); }
    if (!this.worker) {
      if (!isFirst) return Promise.reject(new Error('not-started'));
      try { this._ensureWorker(); } catch (e) { return Promise.reject(e); }
    }
    const sequence = ++this.seq;
    return new Promise((resolve, reject) => {
      const timer = this.setT(() => this._fail(new Error(kind + '-deadline')), budgetMs);
      this.pending = { sequence, kind, resolve, reject, timer, deadline: this.now() + budgetMs };
      try { this.worker.postMessage({ sequence, kind, payload }); }
      catch (e) { this._fail(e); }
    });
  }
  _ensureWorker() {
    const loc = globalThis.location;
    if (loc && new URL(this.url, loc.href).origin !== loc.origin) throw new Error('same-origin-only');
    const w = new this.W(this.url, { type: 'module' });
    w.addEventListener('message', this._onMsg);
    w.addEventListener('messageerror', this._onErr);
    w.addEventListener('error', this._onErr);
    this.worker = w;
    globalThis.document?.addEventListener('visibilitychange', this._onVis);
  }
  _handle(msg) {
    const p = this.pending;
    if (!p || !msg || msg.sequence !== p.sequence || msg.kind !== p.kind) {
      this._fail(new Error('foreign-duplicate-late')); return;
    }
    if (this.now() > p.deadline) { this._fail(new Error(p.kind + '-deadline')); return; }
    if (typeof msg.error === 'string') { this._fail(new Error(msg.error.slice(0,512))); return; }
    let n = 0;
    try { n = new TextEncoder().encode(JSON.stringify(msg.payload) ?? "").length; } catch { this._fail(new Error('unserializable')); return; }
    if (n > this.limits.maxBytes) { this._fail(new Error('response-too-large')); return; }
    this.clearT(p.timer); this.pending = null; p.resolve(msg.payload);
  }
  _fail(err) {
    if (this.disposed) return;
    const e = err instanceof Error ? err : new Error(String(err));
    if (!this.failed) this.failed = e;
    const p = this.pending; this.pending = null;
    if (p) { this.clearT(p.timer); p.reject(e); }
    if (this.worker) {
      try { this.worker.removeEventListener('message', this._onMsg); } catch {}
      try { this.worker.removeEventListener('messageerror', this._onErr); } catch {}
      try { this.worker.removeEventListener('error', this._onErr); } catch {}
      try { this.worker.terminate(); } catch {}
      this.worker = null;
    }
    try { globalThis.document?.removeEventListener('visibilitychange', this._onVis); } catch {}
  }
  dispose() {
    if (this.disposed) return;
    this._fail(new Error('disposed'));
    this.disposed = true;
  }
}
