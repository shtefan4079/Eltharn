import {ActorTransport} from './actor-transport.mjs';
import {ConfigStore, storageOwner} from './config-store.mjs';
import {stringifyJson} from './json.mjs';

export class WebSession {
  static async prepare(definitions, side=1) {
    if (definitions.length === 0) return null;
    const api = await import('./eltharn_mod_web.js'); await api.default();
    const policy=JSON.parse(api.runtime_policy());
    if (definitions.length > policy.max_actors) throw new Error('Actor limit');
    let total = 0;
    const configs = definitions.map(d=>{
      const memory_max = d.script ? api.admit_script(JSON.stringify(d.script)) : api.admit(d.bytes); total += memory_max;
      if (total > policy.memory_total) throw new Error('Aggregate declared memory limit');
      return {key:d.key,dependencies:d.dependencies,grants:d.grants,memory_max,required:d.required};
    });
    const session = new WebSession(new api.Coordinator(stringifyJson(configs), side));
    session.side = side;
    try {
      const owners = configs.filter(c => c.grants.some(g => g === 'config.read' || g === 'config.write')).map(c => c.key);
      const records = {};
      if (owners.length) {
        session.configStore = await ConfigStore.open();
        for (const owner of owners) records[owner] = await session.configStore.load(storageOwner(owner, side));
      }
      session.controller.initialize_config(stringifyJson(records));
      for (let i=0;i<definitions.length;i++) {
        const actor=new ActorTransport(new URL('./actor-worker.mjs',import.meta.url));session.actors.push(actor);
        const result=await actor.start({bytes:definitions[i].bytes,script:definitions[i].script,grants:configs[i].grants});
        if (result.memory_max!==configs[i].memory_max) throw new Error(`${configs[i].key}: memory profile mismatch`);
      }
      await session.phase('Init');return session;
    } catch(e) {session.dispose();throw e;}
  }
  constructor(controller) {
    this.controller=controller;this.actors=[];this.disposed=false;this.running=false;
    this.jobWorker=null;this.jobQueue=[];this.jobRunning=false;this.jobGeneration=0;this.jobErrors=[];
    this.generation=0;this.activePhase=null;this.cancelRequested=false;
    this.configStore=null;
  }
  async phase(phase) {
    if(this.disposed||this.running)throw new Error('Session disposed or busy');
    this.running=true;this.activePhase=phase;const generation=this.generation;
    try {
      if(phase==='WorldExit'||phase==='Shutdown')this.stopJobs();
      this.controller.begin(phase);
      for(;;) {
        const invocation=JSON.parse(this.controller.next());if(!invocation)break;
        invocation.binary_resources=this.controller.binary_resources();
        const {request}=invocation;
        for(const field of ['epoch','tick','callback_id','sequence']) {
          if(!Number.isSafeInteger(request[field]))throw new Error('Web transport counter exhausted');
        }
        const actor=this.actors[request.actor];let outcome;
        try {outcome=await actor.invoke(invocation);}
        catch(e){outcome={success:false,error:String(e).slice(0,512),effects:{commands:[],logs:[],calls:0},fuel_used:null,memory_bytes:0,elapsed_us:0};}
        if(this.disposed)throw new Error('Disposed phase');
        if(generation!==this.generation)return JSON.parse(this.controller.report());
        const disabled=JSON.parse(this.controller.complete(stringifyJson({...outcome,request})));
        for(const rank of disabled)this.actors[rank]?.dispose();
        if(this.cancelRequested){
          const cancelled=JSON.parse(this.controller.cancel_tick());
          for(const rank of cancelled)this.actors[rank]?.dispose();
          this.cancelRequested=false;this.generation++;this.running=false;this.activePhase=null;
          return JSON.parse(this.controller.report());
        }
      }
      this.activePhase = null; this.cancelRequested = false;
      for (const write of JSON.parse(this.controller.config_writes())) {
        let receipt = null, error = null;
        try { receipt = await this.configStore.compareAndSet({...write,owner:storageOwner(write.owner,this.side)}); }
        catch (e) { error = String(e).slice(0,128); }
        if (this.disposed) throw new Error('Disposed configuration persistence');
        this.controller.acknowledge_config(stringifyJson(write), stringifyJson(receipt), error);
      }
      const report=JSON.parse(this.controller.report());
      for(let i=0;i<report.actors.length;i++) {
        if(['Disabled','Terminated'].includes(report.actors[i].state))this.actors[i]?.dispose();
      }
      const jobs=JSON.parse(this.controller.take_jobs());
      if(this.jobQueue.length+jobs.length>32)throw new Error('Job queue limit');
      this.jobQueue.push(...jobs);this.pumpJobs();
      return report;
    } catch(e) {
      if(generation===this.generation)this.dispose();
      throw e;
    } finally {if(generation===this.generation){this.running=false;this.activePhase=null;}}
  }
  setContentInput(input) {
    if(this.disposed||this.running)throw new Error('Session disposed or busy');
    this.controller.set_content_input(input);
  }
  setAppInput(input) {
    if(this.disposed||this.running)throw new Error('Session disposed or busy');
    this.controller.set_app_input(input);
  }
  setKeybindingInput(input) {
    if(this.disposed||this.running)throw new Error('Session disposed or busy');
    this.controller.set_keybinding_input(input);
  }
  cancelTick() {
    if(!this.running||!['Tick','AppTick'].includes(this.activePhase))throw new Error('No tick in flight');
    this.cancelRequested=true;this.stopJobs();
  }
  async pumpJobs() {
    if(this.jobRunning||this.disposed||!this.jobQueue.length)return;
    const generation=this.jobGeneration;this.jobRunning=true;
    try {
      if(!this.jobWorker){this.jobWorker=new ActorTransport(new URL('./job-worker.mjs',import.meta.url));await this.jobWorker.start({});}
      while(this.jobQueue.length&&!this.disposed&&generation===this.jobGeneration){
        const job=this.jobQueue.shift();
        const result=await this.jobWorker.invoke({data:job.data});
        if(this.disposed||generation!==this.jobGeneration)return;
        try {this.controller.complete_job(stringifyJson(job),new Uint8Array(result));}
        catch(e){this.jobErrors.push({sequence:job.sequence,epoch:job.epoch,error:String(e).slice(0,512)});if(this.jobErrors.length>64)this.jobErrors.shift();}
      }
    }catch(e){if(!this.disposed&&generation===this.jobGeneration){this.jobErrors.push({error:String(e).slice(0,512)});this.dispose();}}
    finally{if(generation===this.jobGeneration)this.jobRunning=false;}
  }
  stopJobs(){this.jobGeneration++;this.jobWorker?.dispose();this.jobWorker=null;this.jobQueue=[];this.jobRunning=false;}
  report(){if(this.disposed)throw new Error('Session disposed');return {...JSON.parse(this.controller.report()),job_errors:this.jobErrors};}
  dispose(){if(this.disposed)return;this.disposed=true;this.generation++;this.running=false;this.activePhase=null;this.cancelRequested=false;this.actors.forEach(a=>a.dispose());this.stopJobs();this.configStore?.close();this.controller.free();}
}
