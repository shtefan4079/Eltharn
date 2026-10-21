let audioCtx = null;
const _cancellable_sounds = new Map();
let listenerPosition = [0, 0, 0];
let listenerForward = [0, 0, -1];
let directionalAudioEnabled = false;

function setParam(param, value, ctx) {
    if (typeof param.setValueAtTime === 'function') param.setValueAtTime(value, ctx.currentTime);
    else param.value = value;
}

function applyListener(ctx) {
    const listener = ctx.listener;
    if (!listener) return;
    if (listener.positionX) {
        for (const name of ['positionX', 'positionY', 'positionZ']) setParam(listener[name], 0, ctx);
    } else if (listener.setPosition) listener.setPosition(0, 0, 0);
    if (listener.forwardX) {
        for (const [name, value] of [
            ['forwardX', listenerForward[0]], ['forwardY', 0], ['forwardZ', listenerForward[2]],
            ['upX', 0], ['upY', 1], ['upZ', 0],
        ]) setParam(listener[name], value, ctx);
    } else if (listener.setOrientation) listener.setOrientation(...listenerForward, 0, 1, 0);
}

function applyPosition(entry) {
    if (!entry.panner) return;
    const position = entry.position.map((value, axis) => value - listenerPosition[axis]);
    const panner = entry.panner;
    if (panner.positionX) {
        for (const [axis, name] of ['positionX', 'positionY', 'positionZ'].entries()) {
            setParam(panner[name], position[axis], audioCtx);
        }
    } else panner.setPosition(...position);
}

function applyVolume(entry) {
    if (entry.gain) setParam(entry.gain.gain, entry.volume, audioCtx);
    if (entry.audio) entry.audio.volume = Math.min(1, entry.volume);
}

function getAudioContext() {
    if (!audioCtx) {
        const AudioCtx = typeof globalThis !== 'undefined' && (globalThis.AudioContext || globalThis.webkitAudioContext);
        if (AudioCtx) {
            try {
                audioCtx = new AudioCtx();
                applyListener(audioCtx);
            } catch (e) {}
        }
    }
    if (audioCtx && audioCtx.state === 'suspended') {
        audioCtx.resume().catch(() => {});
    }
    return audioCtx;
}

if (typeof globalThis !== 'undefined' && typeof globalThis.addEventListener === 'function') {
    const unlock = () => {
        if (audioCtx && audioCtx.state === 'suspended') {
            audioCtx.resume().catch(() => {});
        }
    };
    globalThis.addEventListener('click', unlock, { passive: true });
    globalThis.addEventListener('keydown', unlock, { passive: true });
    globalThis.addEventListener('mousedown', unlock, { passive: true });
}

if (typeof globalThis !== 'undefined') {
    globalThis._active_sounds = globalThis._active_sounds || new Set();
}

function mimeForBytes(copy) {
    if (copy.length >= 12 &&
        copy[0] === 0x52 && copy[1] === 0x49 && copy[2] === 0x46 && copy[3] === 0x46 &&
        copy[8] === 0x57 && copy[9] === 0x41 && copy[10] === 0x56 && copy[11] === 0x45) {
        return 'audio/wav';
    }
    return 'audio/ogg';
}

function fallbackHtml5Audio(copy, pitch, id, entry) {
    if (typeof Audio === 'undefined') {
        if (typeof id === 'number' && entry && _cancellable_sounds.get(id) === entry) {
            _cancellable_sounds.delete(id);
        }
        return false;
    }
    if (typeof id === 'number' && entry && _cancellable_sounds.get(id) !== entry) {
        return false;
    }
    const mime = mimeForBytes(copy);
    const blob = new Blob([copy], { type: mime });
    const url = URL.createObjectURL(blob);
    const audio = new Audio(url);
    entry.audio = audio;
    applyVolume(entry);
    if (pitch >= 0.1 && pitch <= 10.0) {
        audio.playbackRate = pitch;
    }
    let cleaned = false;
    const cleanup = () => {
        if (cleaned) return;
        cleaned = true;
        entry.audio = null;
        if (globalThis._active_sounds) {
            globalThis._active_sounds.delete(audio);
        }
        try {
            URL.revokeObjectURL(url);
        } catch (e) {}
        if (typeof id === 'number' && entry && _cancellable_sounds.get(id) === entry) {
            _cancellable_sounds.delete(id);
        }
    };
    if (entry) {
        entry.stop = () => {
            try {
                audio.pause();
            } catch (e) {}
            cleanup();
        };
    }
    if (globalThis._active_sounds) {
        globalThis._active_sounds.add(audio);
    }
    audio.onended = cleanup;
    audio.onerror = cleanup;
    try {
        const p = audio.play();
        if (p && typeof p.catch === 'function') {
            p.catch(cleanup);
        }
    } catch (e) {
        cleanup();
        return false;
    }
    return true;
}

export function play_sound_bytes_js(bytes, volume, pitch, id, positional = false, x = 0, y = 0, z = 0) {
    try {
        if (typeof id !== 'number' || !Number.isSafeInteger(id) || id <= 0) {
            return false;
        }
        if (_cancellable_sounds.has(id)) {
            return false;
        }
        if (![volume, x, y, z].every(Number.isFinite)) return false;
        const copy = new Uint8Array(bytes.length);
        copy.set(bytes);
        const entry = { stop: () => {}, volume: Math.max(0, volume),
            position: positional ? [x, y, z] : null, source: null, gain: null, panner: null, audio: null };
        const ctx = getAudioContext();
        if (ctx) {
            _cancellable_sounds.set(id, entry);
            ctx.decodeAudioData(copy.buffer.slice(0)).then((audioBuffer) => {
                if (_cancellable_sounds.get(id) !== entry) return;
                const source = ctx.createBufferSource();
                entry.source = source;
                source.buffer = audioBuffer;
                if (pitch >= 0.1 && pitch <= 10.0) {
                    source.playbackRate.value = pitch;
                }
                const gainNode = ctx.createGain();
                entry.gain = gainNode;
                let ended = false;
                const cleanup = () => {
                    if (ended) return;
                    ended = true;
                    if (_cancellable_sounds.get(id) === entry) {
                        _cancellable_sounds.delete(id);
                    }
                    try {
                        source.disconnect();
                    } catch (e) {}
                    try {
                        gainNode.disconnect();
                    } catch (e) {}
                    try {
                        if (entry.panner) entry.panner.disconnect();
                    } catch (e) {}
                    entry.source = null;
                    entry.gain = null;
                    entry.panner = null;
                };
                entry.stop = () => {
                    try {
                        source.stop();
                    } catch (e) {}
                    cleanup();
                };
                source.onended = cleanup;
                applyVolume(entry);
                if (entry.position) {
                    const panner = ctx.createPanner();
                    entry.panner = panner;
                    try {
                        panner.panningModel = directionalAudioEnabled ? 'HRTF' : 'equalpower';
                    } catch (e) {
                        panner.panningModel = 'equalpower';
                    }
                    panner.distanceModel = 'linear';
                    panner.rolloffFactor = 0;
                    applyPosition(entry);
                    source.connect(panner);
                    panner.connect(gainNode);
                } else source.connect(gainNode);
                gainNode.connect(ctx.destination);
                try { source.start(0); } catch (error) { stop_sound_js(id); throw error; }
            }).catch((err) => {
                if (_cancellable_sounds.get(id) !== entry) return;
                try {
                    for (const node of [entry.source, entry.panner, entry.gain]) {
                        if (node) node.disconnect();
                    }
                    entry.source = null; entry.panner = null; entry.gain = null;
                    fallbackHtml5Audio(copy, pitch, id, entry);
                }
                catch { stop_sound_js(id); }
            });
            return true;
        } else {
            if (typeof Audio === 'undefined') return false;
            _cancellable_sounds.set(id, entry);
            const ok = fallbackHtml5Audio(copy, pitch, id, entry);
            if (!ok) {
                if (_cancellable_sounds.get(id) === entry) {
                    _cancellable_sounds.delete(id);
                }
                return false;
            }
            return true;
        }
    } catch (e) {
        stop_sound_js(id);
        console.error('[Audio] Exception in play_sound_bytes_js:', e);
        return false;
    }
}

export function update_listener_js(x, y, z, yaw) {
    if (![x, y, z, yaw].every(Number.isFinite)) return;
    listenerPosition = [x, y, z];
    listenerForward = [-Math.sin(yaw), 0, Math.cos(yaw)];
    if (audioCtx) applyListener(audioCtx);
    for (const entry of _cancellable_sounds.values()) applyPosition(entry);
}

export function set_directional_audio_js(enabled) {
    directionalAudioEnabled = !!enabled;
    for (const entry of _cancellable_sounds.values()) {
        if (!entry.panner) continue;
        try {
            entry.panner.panningModel = directionalAudioEnabled ? 'HRTF' : 'equalpower';
        } catch (e) {
            entry.panner.panningModel = 'equalpower';
        }
    }
}

export function set_sound_position_js(id, x, y, z) {
    const entry = _cancellable_sounds.get(id);
    if (!entry || !entry.position || ![x, y, z].every(Number.isFinite)) return;
    entry.position = [x, y, z];
    applyPosition(entry);
}

export function set_sound_volume_js(id, volume) {
    const entry = _cancellable_sounds.get(id);
    if (!entry || !Number.isFinite(volume)) return;
    entry.volume = Math.max(0, volume);
    applyVolume(entry);
}

export function stop_sound_js(id) {
    try {
        const entry = _cancellable_sounds.get(id);
        if (!entry) return;
        _cancellable_sounds.delete(id);
        try {
            entry.stop();
        } catch (e) {}
    } catch (e) {}
}

export function is_sound_active_js(id) {
    try {
        return _cancellable_sounds.has(id);
    } catch (e) {
        return false;
    }
}

export function stop_all_sounds_js() {
    try {
        const keys = Array.from(_cancellable_sounds.keys());
        for (const key of keys) {
            stop_sound_js(key);
        }
    } catch (e) {}
}
