let jsonModule;
async function encodeJson(value) {
  jsonModule ??= import(new URL('./mods/json.mjs', document.baseURI).href);
  return (await jsonModule).stringifyJson(value);
}
let session = null;
let starting = false;
let generation = 0;
export async function startMods(configText, buffers) {
  if (session || starting) throw new Error('Mods already configured');
  const configs = JSON.parse(configText);
  if (!Array.isArray(configs) || !Array.isArray(buffers) || configs.length !== buffers.length) {
    throw new Error('Mod configuration/source count mismatch');
  }
  if (!configs.length) return encodeJson({actors:[],resources:[0,0,0,0,0],keybindings:{bindings:{}},overlay:{messages:{}}});
  const definitions = configs.map((c, i) => {
    if (c.language === 'rhai') return {...c, script:JSON.parse(buffers[i])};
    if (c.language === 'wasm') return {...c, bytes:buffers[i]};
    throw new Error(`Unsupported mod language: ${String(c.language)}`);
  });
  const preparingGeneration = generation;
  starting = true;
  try {
    const {WebSession} = await import(new URL('./mods/client-session.mjs', document.baseURI).href);
    if (preparingGeneration !== generation) throw new Error('Mod preparation cancelled');
    const prepared = await WebSession.prepare(definitions,1,{1:10,2:20});
    if (preparingGeneration !== generation) {
      prepared?.dispose();
      throw new Error('Mod preparation cancelled');
    }
    session = prepared;
    return encodeJson(session.report());
  } finally {
    if (preparingGeneration === generation) starting = false;
  }
}
export async function phaseMods(phase,count,input='{}',keybindings='{}') {
  if (!session) throw new Error('Mods not configured');
  if(!Number.isInteger(count)||count<1||count>10||(phase!=='Tick'&&count!==1))throw new Error('Invalid lifecycle batch');
  const generation=session.generation;let report;
  if(phase==='Tick')session.setContentInput(input);
  if(phase==='AppTick')session.setAppInput(input);
  if(['Tick','AppTick'].includes(phase))session.setKeybindingInput(keybindings);
  const network=[];
  for(let i=0;i<count;i++){
    report=await session.phase(phase);
    if(generation!==session.generation){network.length=0;break;}
    if(Array.isArray(report.network))network.push(...report.network);
  }
  if(report)report.network=network;
  return encodeJson(report);
}
export function cancelModTick() {if(session?.running&&['Tick','AppTick'].includes(session.activePhase))session.cancelTick();}
export function disposeMods() {generation++;session?.dispose();session=null;starting=false;}
