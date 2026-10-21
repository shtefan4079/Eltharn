import { WebSession } from './session.mjs';


let session = null;
let busy = false;
let activeSequence = 0;
let activePhaseName = null;
let lastSequence = 0;
let generation = 0;
let closed = false;

function schedulerInfo() {
  if (!session) return { actor_workers: 0, job_workers: 0 };
  const actors = Array.isArray(session.actors)
    ? session.actors.filter((a) => a && a.started).length
    : 0;
  const jobs = session.jobWorker && session.jobWorker.started ? 1 : 0;
  return { actor_workers: actors, job_workers: jobs };
}

function withScheduler(report) {
  const base = report && typeof report === 'object' ? report : {};
  base.scheduler = schedulerInfo();
  return base;
}

function reply(msg) {
  try { self.postMessage(msg); } catch {}
}

async function fail(sequence, kind, e) {
  if (closed) return;
  try {
    reply({ sequence, kind, error: String(e).slice(0, 512) });
  } catch {}
  await disposeAndClose();
}

async function disposeAndClose() {
  if (closed) {
    generation++;
    return;
  }
  closed = true;
  generation++;
  busy = false;
  activeSequence = 0;
  activePhaseName = null;
  const s = session;
  session = null;
  if (s) {
    try { await s.dispose(); } catch {}
  }
  try { self.close(); } catch {}
}

async function handle(msg) {
  if (closed) return;
  if (!msg || typeof msg !== 'object') return;
  if (msg.kind === 'cancel') {
    const t = msg.target;
    if (t === activeSequence && busy && session && session.running && ['Tick','AppTick'].includes(activePhaseName)) {
      try { await session.cancelTick(); } catch {}
    }
    return;                          
  }
  if (msg.kind === 'dispose') {
    await disposeAndClose();
    return;                                                         
  }
  const { sequence, kind, payload } = msg;
  if (!Number.isSafeInteger(sequence) || sequence !== lastSequence + 1) {
    await fail(sequence, kind, new Error('bad sequence'));
    return;
  }
  if (lastSequence >= Number.MAX_SAFE_INTEGER) {
    await fail(sequence, kind, new Error('sequence exhausted'));
    return;
  }
  if (busy) {
    await fail(sequence, kind, new Error('one rpc at once'));
    return;
  }
  lastSequence = sequence;
  if (kind === 'prepare') {
    if (session !== null) {
      await fail(sequence, kind, new Error('already prepared'));
      return;
    }
    const gen = generation;
    busy = true;
    activeSequence = sequence;
    activePhaseName = null;
    try {
      const defs = payload ? payload.definitions : undefined;
      const side = payload ? payload.side : undefined;
      const s = await WebSession.prepare(defs, side);
      if (gen !== generation) { s?.dispose(); return; }                              
      if (!s) throw new Error("empty scheduler session");
      session = s;
      busy = false;
      activeSequence = 0;
      reply({ sequence, kind, payload: withScheduler(s.report()) });
    } catch (e) {
      if (gen !== generation) return;
      await fail(sequence, kind, e);
    }
    return;
  }
  if (kind === 'phase') {
    if (session === null) {
      await fail(sequence, kind, new Error('not prepared'));
      return;
    }
    const gen = generation;
    busy = true;
    activeSequence = sequence;
    activePhaseName = payload ? payload.phase : null;
    try {
      const name = payload ? payload.phase : undefined;
      const input = payload ? payload.input : null;
      const keybindings = payload ? payload.keybindings : null;
      if (name === 'Tick' && typeof input === 'string') session.setContentInput(input);
      if (name === 'AppTick' && typeof input === 'string') session.setAppInput(input);
      if (['Tick','AppTick'].includes(name) && typeof keybindings === 'string') session.setKeybindingInput(keybindings);
      let report = await session.phase(name);
      if (gen !== generation) return;                                         
      if (!report) report = session.report();
      busy = false;
      activeSequence = 0;
      activePhaseName = null;
      reply({ sequence, kind, payload: withScheduler(report) });
    } catch (e) {
      if (gen !== generation) return;
      await fail(sequence, kind, e);
    }
    return;
  }
  await fail(sequence, kind, new Error('unknown kind'));
}

self.onmessage = (ev) => { void handle(ev.data); };
