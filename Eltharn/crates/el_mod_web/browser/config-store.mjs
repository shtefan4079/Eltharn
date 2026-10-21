import {stringifyJson} from './json.mjs';
const DB = 'rustore-mod-config-v1';
const TABLE = 'configs';
const bytes = value => new TextEncoder().encode(stringifyJson(value)).length;
const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
function owner(key) {
  if (typeof key !== 'string' || !/^[a-z0-9][a-z0-9_.-]{0,62}:[a-z0-9][a-z0-9_.-]{0,62}\/(client|server|shared)\/(client|server)$/.test(key)
      || /^(minecraft|eltharn|rustore):/.test(key)) throw new Error('Invalid configuration owner');
}
export function storageOwner(actor, side) {
  if (side !== 1 && side !== 2) throw new Error('Invalid runtime side');
  const key = `${actor}/${side === 1 ? 'client' : 'server'}`;
  owner(key); return key;
}
function name(key) { if (!/^[A-Za-z0-9_.-]{1,64}$/.test(key)) throw new Error('Invalid configuration key'); }
function value(v) {
  let count = 0; const pending = [[v, 0]];
  while (pending.length) {
    const [v, depth] = pending.pop();
    if (++count > 128 || depth > 8) throw new Error('Configuration nesting/element limit');
    if (v === null || typeof v === 'boolean') continue;
    if (typeof v === 'number') { if (!Number.isFinite(v)) throw new Error('Invalid configuration number'); continue; }
    if (typeof v === 'string') { if (v.length > 8192) throw new Error('Configuration string limit'); continue; }
    if (Array.isArray(v)) { if (v.length > 128) throw new Error('Configuration array limit'); pending.push(...v.map(x => [x, depth + 1])); continue; }
    if (!object(v) || Object.keys(v).length > 64) throw new Error('Invalid configuration value');
    for (const [k, x] of Object.entries(v)) { name(k); pending.push([x, depth + 1]); }
  }
  if (bytes(v) > 8192) throw new Error('Configuration value exceeds 8 KiB');
}
export function validateRecord(record) {
  if (!object(record) || Object.keys(record).sort().join(',') !== 'revision,values'
      || !Number.isSafeInteger(record.revision) || record.revision < 0 || !object(record.values)
      || Object.keys(record.values).length > 64) throw new Error('Invalid configuration record');
  for (const [k, v] of Object.entries(record.values)) { name(k); value(v); }
  if (bytes(record) > 32768) throw new Error('Configuration exceeds 32 KiB');
  return record;
}
const empty = () => ({revision:0, values:{}});
function recordFromEnvelope(envelope, key) {
  if (envelope === undefined) return empty();
  if (!object(envelope) || Object.keys(envelope).sort().join(',') !== 'owner,record,schema_version'
      || envelope.schema_version !== 1 || envelope.owner !== key) throw new Error('Configuration schema/owner mismatch');
  return validateRecord(envelope.record);
}
export class ConfigStore {
  constructor(db) { this.db = db; this.transactions = new Set(); this.closed = false; }
  static open() {
    return new Promise((resolve, reject) => {
      const request = indexedDB.open(DB, 1);
      let done = false;
      const timer = setTimeout(() => { done = true; reject(new Error('Configuration database open deadline')); }, 5000);
      request.onupgradeneeded = () => request.result.createObjectStore(TABLE);
      request.onerror = () => { clearTimeout(timer); done = true; reject(request.error); };
      request.onblocked = () => { clearTimeout(timer); done = true; reject(new Error('Configuration database upgrade blocked')); };
      request.onsuccess = () => {
        clearTimeout(timer);
        if (done) { request.result.close(); return; }
        const store = new ConfigStore(request.result);
        request.result.onversionchange = () => store.close();
        resolve(store);
      };
    });
  }
  transact(key, next = null, expected = null) {
    owner(key);
    if (next) validateRecord(next);
    if (this.closed) return Promise.reject(new Error('Configuration store closed'));
    return new Promise((resolve, reject) => {
      const tx = this.db.transaction(TABLE, next ? 'readwrite' : 'readonly', next ? {durability:'strict'} : undefined);
      this.transactions.add(tx);
      let result, failure;
      const timer = setTimeout(() => { failure = new Error('Configuration transaction deadline'); try {tx.abort();} catch {} }, 5000);
      const finish = () => { clearTimeout(timer); this.transactions.delete(tx); };
      tx.oncomplete = () => { finish(); resolve(result); };
      tx.onabort = () => { finish(); reject(failure || tx.error || new Error('Configuration transaction aborted')); };
      tx.onerror = () => {};
      const table = tx.objectStore(TABLE), request = table.get(key);
      request.onsuccess = () => {
        try {
          const current = recordFromEnvelope(request.result, key);
          if (!next) { result = current; return; }
          if (current.revision !== expected) { result = {record:current, conflict:true}; return; }
          table.put({schema_version:1, owner:key, record:next}, key);
          result = {record:next, conflict:false};
        } catch (error) { failure = error; tx.abort(); }
      };
    });
  }
  load(key) { return this.transact(key); }
  compareAndSet(write) {
    if (!object(write) || !Number.isSafeInteger(write.expected) || write.expected < 0 || write.expected >= Number.MAX_SAFE_INTEGER)
      return Promise.reject(new Error('Invalid configuration revision'));
    return this.transact(write.owner, {revision:write.expected + 1, values:write.values}, write.expected);
  }
  close() {
    if (this.closed) return;
    this.closed = true;
    for (const tx of this.transactions) { try {tx.abort();} catch {} }
    this.db.close();
  }
}
