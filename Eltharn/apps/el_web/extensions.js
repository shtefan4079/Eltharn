import { ShaderSettingsEditor } from "./shader_settings.mjs";
export async function mountExtensions(api) {
  const DB = "ronner.extensions.v1", STORE = "data";
  const MAX_IMPORT = 32 * 1024 * 1024, MAX_PKGS = 256;
  const host = document.getElementById("eltharn_gate-launcher") || document.body;
  const launchBtn = () => document.getElementById("eltharn_gate-launch");
  let db = null, prefs = null, catalog = [], busy = false, invalid = "";
  let bypass = false;
  const shaderEditor = new ShaderSettingsEditor();
  const h = (t, txt, attrs) => {
    const e = document.createElement(t);
    if (txt != null) e.textContent = txt;
    if (attrs) for (const k in attrs) e.setAttribute(k, attrs[k]);
    return e;
  };
  let fieldset, errNode;
  const setErr = (m) => { if (errNode) errNode.textContent = m || ""; };
  const b64 = (u8) => {
    let s = ""; const CH = 0x8000;
    for (let i = 0; i < u8.length; i += CH)
      s += String.fromCharCode.apply(null, u8.subarray(i, i + CH));
    return btoa(s);
  };
  const openDb = () => new Promise((res, rej) => {
    let r;
    try { r = indexedDB.open(DB, 1); } catch (e) { rej(e); return; }
    r.onupgradeneeded = () => { r.result.createObjectStore(STORE); };
    r.onsuccess = () => res(r.result); r.onerror = () => rej(r.error);
  });
  const txDone = (tx) => new Promise((res, rej) => {
    tx.oncomplete = () => res(); tx.onerror = () => rej(tx.error); tx.onabort = () => rej(tx.error);
  });
  const txGet = (tx, k) => new Promise((res, rej) => {
    const q = tx.objectStore(STORE).get(k);
    q.onsuccess = () => res(q.result); q.onerror = () => rej(q.error);
  });
  const active = () => (prefs.profiles || []).find((p) => p.id === prefs.active_profile) || prefs.profiles[0];
  const flagKeys = () => ["mods_enabled", "shaders_enabled"];
  const modeText = (p) => {
    const [mk, sk] = flagKeys(p.flags); const m = mk && p.flags[mk], s = sk && p.flags[sk];
    return m && s ? "Mods and shaders" : m ? "Mods" : s ? "Shaders" : "Standard";
  };
  const shortHash = (c) => String(c || "").slice(7, 17);
  async function loadAll() {
    const tx = db.transaction(STORE, "readonly");
    const done = txDone(tx);
    const [p,c] = await Promise.all([txGet(tx,"prefs"),txGet(tx,"catalog")]);
    await done;
    if(c !== undefined && (!Array.isArray(c) || c.length > MAX_PKGS)) throw new Error("Saved package library is invalid");
    return {p,c:c ?? []};
  }
  async function savePrefs(next) {
    const norm = api.extension_validate_preferences(JSON.stringify(next));
    const parsed = JSON.parse(norm);
    const tx = db.transaction(STORE, "readwrite");
    const done = txDone(tx);
    try {
      const saved = await txGet(tx, "prefs");
      const previous = JSON.parse(api.extension_validate_preferences(JSON.stringify(saved ?? JSON.parse(api.extension_default_preferences()))));
      if (JSON.stringify(previous) !== JSON.stringify(prefs)) throw new Error("Profile changed in another tab. Reload Eltharn Gate before editing.");
      tx.objectStore(STORE).put(parsed, "prefs");
    } catch(e) { tx.abort(); await done.catch(()=>{}); throw e; }
    await done;
    prefs = parsed;
  }
  function referenced(cid) {
    for (const p of prefs.profiles || []) {
      for (const m of p.mods || []) if (m && m.content_id === cid) return p.name || p.id;
      if (p.shader && p.shader.package && p.shader.package.content_id === cid) return p.name || p.id;
    }
    return null;
  }
  const shaderKey = (profile) => profile?.shader
    ? JSON.stringify([profile.id, profile.shader.package, profile.shader.settings]) : null;
  async function shaderMetadata(pin, settings) {
    const tx = db.transaction(STORE, "readonly"), done = txDone(tx);
    const bytes = await txGet(tx, "package:" + pin.content_id);
    await done;
    if (!(bytes instanceof ArrayBuffer) || bytes.byteLength > MAX_IMPORT)
      throw new Error("Missing or oversized shader package");
    const metadata = JSON.parse(await api.extension_shader_settings(new Uint8Array(bytes), JSON.stringify(settings)));
    if (["id", "version", "content_id"].some(k => metadata.package[k] !== pin[k]))
      throw new Error("Saved shader package does not match the selection");
    return metadata;
  }
  function renderShaderSettings(container, profile) {
    const key = shaderKey(profile);
    if (key !== shaderEditor.key) {
      const selection = structuredClone(profile.shader);
      shaderEditor.select(key, selection?.settings, () => shaderMetadata(selection.package, {}))
        .then(changed => { if (changed) render(); });
    }
    if (key === null) return;
    const section = h("section");
    section.appendChild(h("h4", "Shader settings"));
    if (shaderEditor.state === "loading") section.appendChild(h("p", "Loading settings…"));
    else if (shaderEditor.state === "failed") section.appendChild(h("p", shaderEditor.error));
    else if (shaderEditor.state === "ready") {
      if (!shaderEditor.definitions.length) section.appendChild(h("p", "This pack has no adjustable settings."));
      for (const definition of shaderEditor.definitions) {
        const row = h("label");
        row.appendChild(h("span", definition.id + " "));
        const value = shaderEditor.value(definition.id);
        let control;
        if (definition.type === "enum") {
          control = h("select");
          for (const choice of definition.values) {
            const option = h("option", choice); option.value = choice;
            option.selected = value === choice; control.appendChild(option);
          }
          if (!definition.values.includes(value)) control.selectedIndex = -1;
        } else {
          control = h("input");
          if (definition.type === "bool") {
            control.type = "checkbox"; control.checked = value === true;
          } else {
            control.type = "number"; control.min = definition.min; control.max = definition.max;
            control.step = definition.type === "int" ? "1" : "any";
            control.value = String(value);
          }
        }
        control.setAttribute("aria-label", definition.id);
        control.oninput = () => shaderEditor.set(definition.id,
          definition.type === "bool" ? control.checked : control.value);
        row.appendChild(control); section.appendChild(row);
      }
      if (shaderEditor.definitions.length || Object.keys(shaderEditor.overrides).length) {
        const reset = h("button", "Reset to pack defaults", { type: "button" });
        reset.onclick = () => { shaderEditor.reset(); render(); };
        const apply = h("button", "Apply settings", { type: "button" });
        apply.onclick = async () => {
          const selected = structuredClone(active());
          const previous = JSON.stringify(prefs);
          busy = true; render(); setErr("");
          try {
            await shaderEditor.apply(settings => shaderMetadata(selected.shader.package, settings),
              async (settings, expectedKey) => {
                if (shaderKey(active()) !== expectedKey || JSON.stringify(prefs) !== previous)
                  throw new Error("Profile changed; settings were not saved");
                const next = structuredClone(prefs);
                next.profiles.find(p => p.id === next.active_profile).shader.settings = settings;
                await savePrefs(next);
              });
          } catch (error) { setErr(String(error?.message || error)); }
          busy = false; render();
        };
        section.appendChild(reset); section.appendChild(apply);
      }
    }
    container.appendChild(section);
  }
  async function doImport(file) {
    if (file.size > MAX_IMPORT) { setErr("ZIP over 32 MiB rejected"); return; }
    const buf = await file.arrayBuffer();
    const bytes = new Uint8Array(buf);
    let info; try { info = JSON.parse(await api.extension_inspect(bytes)); }
    catch (e) { setErr("Inspect failed: " + (e && e.message || e)); return; }
    const cid = info && info.pin && info.pin.content_id;
    if (!cid) { setErr("Inspect returned no content_id"); return; }
    const tx = db.transaction(STORE, "readwrite");
    const done = txDone(tx);
    const st = tx.objectStore(STORE);
    const q = (k) => new Promise((res, rej) => { const r = st.get(k); r.onsuccess = () => res(r.result); r.onerror = () => rej(r.error); });
    let cur;
    try { cur = JSON.parse(api.extension_validate_catalog(JSON.stringify((await q("catalog")) ?? []))); }
    catch(e) { tx.abort(); await done.catch(()=>{}); throw e; }
    const ids = new Set(cur.map((e) => e.pin.content_id));
    if (!ids.has(cid) && ids.size >= MAX_PKGS) { tx.abort(); await done.catch(()=>{}); throw new Error("Library is full (256 packages)"); }
    let next = cur;
    if (!ids.has(cid)) next = cur.concat([info]);
    st.put(bytes.buffer.slice(0), "package:" + cid); st.put(next, "catalog");
    await done;
    catalog = next;
    setErr("");
  }
  async function removePackage(cid) {
    const tx = db.transaction(STORE, "readwrite"), done = txDone(tx);
    let next;
    try {
      const [saved, entries] = await Promise.all([txGet(tx,"prefs"), txGet(tx,"catalog")]);
      const profiles = JSON.parse(api.extension_validate_preferences(JSON.stringify(saved ?? prefs)));
      if (profiles.profiles.some(p=>p.mods.some(m=>m.content_id===cid)||p.shader?.package.content_id===cid))
        throw new Error("Package is selected in a profile. Deselect it first.");
      next = JSON.parse(api.extension_validate_catalog(JSON.stringify(entries ?? []))).filter(e=>e.pin.content_id!==cid);
      tx.objectStore(STORE).put(next,"catalog");
      tx.objectStore(STORE).delete("package:"+cid);
    } catch(e) { tx.abort(); await done.catch(()=>{}); throw e; }
    await done; catalog=next; setErr("");
  }
  function render() {
    const old = fieldset; fieldset = h("fieldset");
    if (busy) fieldset.setAttribute("disabled", "");
    const det = fieldset;
    const sel = h("select"); sel.setAttribute("aria-label", "Profile");
    for (const p of prefs.profiles) { const o = h("option", p.name); o.value = p.id; if (p.id === prefs.active_profile) o.selected = true; sel.appendChild(o); }
    sel.onchange = async () => { busy = true; render(); try { const n = structuredClone(prefs); n.active_profile = sel.value; await savePrefs(n); setErr(""); } catch (e) { setErr(String(e && e.message || e)); } busy = false; render(); };
    det.appendChild(sel);
    const nm = h("input"); nm.value = ""; nm.setAttribute("placeholder", "New profile name"); nm.setAttribute("maxlength", "64");
    const mkBtn = h("button", "Create", { type: "button" });
    mkBtn.onclick = async () => {
      if ((prefs.profiles || []).length >= 16) { setErr("Max 16 profiles"); return; }
      const name = nm.value.trim() || "Profile";
      let n = 1; const ids = new Set(prefs.profiles.map((p) => p.id));
      while (ids.has("profile-" + n)) n++;
      busy = true; render();
      try { const c = structuredClone(active()); c.id = "profile-" + n; c.name = name.slice(0, 64); const nx = structuredClone(prefs); nx.profiles.push(c); nx.active_profile = c.id; await savePrefs(nx); setErr(""); }
      catch (e) { setErr(String(e && e.message || e)); }
      busy = false; render();
    };
    const delBtn = h("button", "Delete", { type: "button" });
    delBtn.onclick = async () => {
      if (prefs.profiles.length <= 1) { setErr("Keep at least one profile"); return; }
      busy = true; render();
      try { const nx = structuredClone(prefs); nx.profiles = nx.profiles.filter((p) => p.id !== nx.active_profile); nx.active_profile = nx.profiles[0].id; await savePrefs(nx); setErr(""); }
      catch (e) { setErr(String(e && e.message || e)); }
      busy = false; render();
    };
    det.appendChild(nm); det.appendChild(mkBtn); det.appendChild(delBtn);
    const p = active(); const [mk, sk] = flagKeys(p.flags);
    for (const [key, label] of [[mk, "Enable mods"], [sk, "Enable shaders"]]) {
      if (!key) continue;
      const lab = h("label"); const cb = h("input"); cb.type = "checkbox"; cb.checked = !!p.flags[key];
      cb.onchange = async () => { busy = true; render(); try { const nx = structuredClone(prefs); const ap = nx.profiles.find((x) => x.id === nx.active_profile); ap.flags[key] = cb.checked; await savePrefs(nx); setErr(""); } catch (e) { setErr(String(e && e.message || e)); } busy = false; render(); };
      lab.appendChild(cb); lab.appendChild(h("span", " " + label)); det.appendChild(lab);
    }
    det.appendChild(h("p", "Mode: " + modeText(p)));
    det.appendChild(h("p", "Changes apply on next launch."));
    const shaders = catalog.filter((e) => e.kind === "shaderpack");
    const ssel = h("select"); ssel.setAttribute("aria-label", "Shader");
    const none = h("option", "None"); none.value = "";
    if (!p.shader) none.selected = true;
    ssel.appendChild(none);
    if (p.shader && !shaders.some(s=>s.pin.content_id===p.shader.package.content_id)) {
      const missing = h("option", "Missing: "+p.shader.package.id+" "+p.shader.package.version);
      missing.value=p.shader.package.content_id; missing.selected=true; missing.disabled=true; ssel.appendChild(missing);
    }
    for (const s of shaders) {
      const o = h("option", s.name + " " + s.pin.version + " #" + shortHash(s.pin.content_id));
      o.value = s.pin.content_id; if (p.shader && p.shader.package.content_id === s.pin.content_id) o.selected = true;
      ssel.appendChild(o);
    }
    ssel.onchange = async () => {
      busy = true; render();
      try {
        const nx = structuredClone(prefs); const ap = nx.profiles.find((x) => x.id === nx.active_profile);
        if (!ssel.value) ap.shader = null;
        else { const f = shaders.find((s) => s.pin.content_id === ssel.value); ap.shader = { package: structuredClone(f.pin), settings: {} }; }
        await savePrefs(nx); setErr("");
      } catch (e) { setErr(String(e && e.message || e)); }
      busy = false; render();
    };
    det.appendChild(h("label", "Shader:")); det.appendChild(ssel);
    renderShaderSettings(det, p);
    const mods = catalog.filter((e) => e.kind === "mod");
    const inSet = new Set((p.mods || []).map((m) => m.content_id));
    for (const pin of p.mods) {
      if (!mods.some(m=>m.pin.content_id===pin.content_id))
        det.appendChild(h("p","Missing mod: "+pin.id+" "+pin.version+". Import this version again or choose another profile."));
    }
    for (const m of mods) {
      const row = h("div"); const lab = h("label"); const cb = h("input"); cb.type = "checkbox"; cb.checked = inSet.has(m.pin.content_id);
      cb.onchange = async () => {
        busy = true; render();
        try {
          const nx = structuredClone(prefs); const ap = nx.profiles.find((x) => x.id === nx.active_profile);
          if (cb.checked) { ap.mods=ap.mods.filter(x=>x.id!==m.pin.id); ap.mods.push(structuredClone(m.pin)); }
          else ap.mods = ap.mods.filter((x) => x.content_id !== m.pin.content_id);
          await savePrefs(nx); setErr("");
        } catch (e) { setErr(String(e && e.message || e)); }
        busy = false; render();
      };
      lab.appendChild(cb); lab.appendChild(h("span", " " + m.name + " " + m.pin.version));
      row.appendChild(lab);
      const rm = h("button", "Remove", { type: "button" });
      rm.onclick = async () => {
        const who = referenced(m.pin.content_id);
        if (who) { setErr("In use by profile " + who); return; }
        busy = true; render();
        try { await removePackage(m.pin.content_id); }
        catch (e) { setErr(String(e && e.message || e)); }
        busy = false; render();
      };
      row.appendChild(rm); det.appendChild(row);
    }
    for (const s of shaders) {
      const who = referenced(s.pin.content_id);
      if (who) continue;
      const row = h("div"); row.appendChild(h("span", s.name + " " + s.pin.version + " "));
      const rm = h("button", "Remove shader", { type: "button" });
      rm.onclick = async () => {
        busy = true; render();
        try { await removePackage(s.pin.content_id); }
        catch (e) { setErr(String(e && e.message || e)); }
        busy = false; render();
      };
      row.appendChild(rm); det.appendChild(row);
    }
    if ((p.mods || []).length) det.appendChild(h("p", "Selected mods need a client with mod support."));
    const file = h("input"); file.type = "file"; file.accept = ".zip"; file.hidden = true;
    file.onchange = async () => { if (file.files[0]) { busy = true; render(); try { await doImport(file.files[0]); } catch (e) { setErr(String(e && e.message || e)); } file.value = ""; busy = false; render(); } };
    const imp = h("button", "Import ZIP", { type: "button" }); imp.onclick = () => file.click();
    det.appendChild(imp); det.appendChild(file);
    const rec = h("button", "Launch once without shaders", { type: "button" });
    rec.onclick = async () => {
      const b = launchBtn(); if (!b || b.disabled) { setErr("Launch unavailable (account missing?)"); return; }
      const ap = active();
      if ((ap.mods || []).length) { setErr("Recovery refused: profile has mods"); return; }
      busy = true; render();
      try {
        setErr(""); bypass = true; try { b.click(); } finally { bypass = false; }
      } catch (e) { setErr(String(e && e.message || e)); }
      busy = false; render();
    };
    det.appendChild(rec);
    if (old && old.parentNode) old.parentNode.replaceChild(fieldset, old);
  }
  async function launch(ev) {
    if (bypass) return;
    const b = launchBtn(); if (!b) return;
    if (invalid) { ev.preventDefault(); ev.stopImmediatePropagation(); setErr(invalid); return; }
    if (busy) { ev.preventDefault(); ev.stopImmediatePropagation(); return; }
    const snap = structuredClone(active());
    const [mk, sk] = flagKeys(snap.flags);
    if (b && b.disabled) { ev.preventDefault(); ev.stopImmediatePropagation(); setErr("Launch unavailable (account missing?)"); return; }
    if (mk && sk && !snap.flags[mk] && !snap.flags[sk]) return;                                      
    ev.preventDefault(); ev.stopImmediatePropagation();
    if (!mk || !sk || (!snap.flags[mk] && !snap.flags[sk])) { bypass = true; try { b.click(); } finally { bypass = false; } return; }
    busy = true; render(); setErr("");
    try {
      const need = [];
      if (mk && snap.flags[mk]) for (const m of snap.mods || []) need.push(m.content_id);
      if (sk && snap.flags[sk] && snap.shader) need.push(snap.shader.package.content_id);
      const uniq = [...new Set(need)];
      const tx = db.transaction(STORE, "readonly"), done = txDone(tx);
      const [savedPrefs, savedCatalog, ...values] = await Promise.all([
        txGet(tx, "prefs"), txGet(tx, "catalog"),
        ...uniq.map(cid => txGet(tx, "package:" + cid)),
      ]);
      await done;
      const checkedPrefs = JSON.parse(api.extension_validate_preferences(JSON.stringify(
        savedPrefs ?? JSON.parse(api.extension_default_preferences()))));
      if (JSON.stringify(checkedPrefs) !== JSON.stringify(prefs))
        throw new Error("Profile changed in another tab. Reload Eltharn Gate before launching.");
      const checkedCatalog = api.extension_validate_catalog(JSON.stringify(savedCatalog ?? []));
      const blobs = {};
      let total = 0;
      for (let i = 0; i < uniq.length; i++) {
        const value = values[i];
        if (!(value instanceof ArrayBuffer)) throw new Error("Missing package " + shortHash(uniq[i]));
        total += value.byteLength;
        if (total > MAX_IMPORT) throw new Error("Selected packages exceed 32 MiB");
        blobs[uniq[i]] = b64(new Uint8Array(value));
      }
      const req = await api.extension_prepare_with_catalog(
        JSON.stringify(snap), JSON.stringify(blobs), checkedCatalog);
      await api.extension_configure(req);
      setErr(""); bypass = true; try { b.click(); } finally { bypass = false; }
    } catch (e) { setErr(String(e && e.message || e)); }
    busy = false; render();
  }
  const initializing=(ev)=>{ev.preventDefault();ev.stopImmediatePropagation();};
  launchBtn()?.addEventListener("click",initializing,true);
  try { db = await openDb(); }
  catch (e) { invalid = "Storage unavailable; launch blocked"; }
  if (!invalid) {
    try {
      const { p, c } = await loadAll();
      catalog = JSON.parse(api.extension_validate_catalog(JSON.stringify(c)));
      if (p == null) {
        const norm = api.extension_validate_preferences(api.extension_default_preferences());
        prefs = JSON.parse(norm);
      } else {
        try { prefs = JSON.parse(api.extension_validate_preferences(JSON.stringify(p))); }
        catch (e) { invalid = "Saved prefs corrupt; launch blocked until storage corrected"; prefs = p; }
      }
      if (!prefs || !Array.isArray(prefs.profiles) || !prefs.profiles.length) invalid = invalid || "Saved prefs corrupt; launch blocked until storage corrected";
    } catch (e) { invalid = "Storage unavailable; launch blocked"; }
  }
  const wrap = h("details"); wrap.open = true; wrap.className="eltharn_gate-extensions"; wrap.id="eltharn_gate-extensions";
  wrap.appendChild(h("summary", "Mods & shaders"));
  fieldset = h("fieldset"); errNode = h("p"); errNode.setAttribute("aria-live", "polite");
  if (invalid) errNode.textContent = invalid;
  wrap.appendChild(fieldset); wrap.appendChild(errNode); host.appendChild(wrap);
  if (!invalid) render(); else { const lb = launchBtn(); }
  const lb = launchBtn();
  if (lb) { lb.removeEventListener("click",initializing,true);lb.addEventListener("click", launch, true); }
}
