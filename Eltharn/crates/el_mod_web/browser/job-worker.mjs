let prepared = false, sequence = 0;
self.onmessage = async ({data}) => {
  const {kind, payload} = data;
  if (data.sequence !== ++sequence) {self.close(); return;}
  try {
    if (kind === 'prepare' && !prepared) {
      prepared = true; self.postMessage({sequence, kind, payload:{ready:true}}); return;
    }
    if (!prepared || kind !== 'invoke' || !Array.isArray(payload.data) || payload.data.length > 16384
      || payload.data.some(n=>!Number.isInteger(n)||n<0||n>255)) throw new Error('Invalid job');
    const result = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', new Uint8Array(payload.data))));
    self.postMessage({sequence, kind, payload:result});
  } catch(e) {self.postMessage({sequence, kind, error:String(e).slice(0,512)}); self.close();}
};
