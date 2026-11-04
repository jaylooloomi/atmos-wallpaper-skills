// Explicit, reversible installed-host resource sampling. No UI automation or test-written ACKs.
// Private preimage/journal/raw samples stay outside the repository. Public output is aggregate-only.
// Run only after the existing two-hour collector has completed:
//   node desktop-host/scripts/test-resource-matrix-installed.mjs --run --output C:\private\new-matrix
// Interrupted run, after reviewing the private journal:
//   node desktop-host/scripts/test-resource-matrix-installed.mjs --recover --output C:\private\new-matrix
// Recovery never launches/replaces the host or overrides a concurrent revision. Forced exit/power loss
// cannot execute finally; durable records permit a later explicit same-session recovery, not a guarantee.
import fs from 'node:fs';
import path from 'node:path';
import {spawn, spawnSync} from 'node:child_process';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {randomBytes} from 'node:crypto';
import {hash, mergePatch, validateDesired} from '../../implementation-p4_2_5/src/contract.mjs';
import {analyzeDirectory,readJsonLines} from './analyze-soak.mjs';

const repo = fileURLToPath(new URL('../../', import.meta.url));
const phases = Object.freeze([
  {name:'single-thunder', mode:'single', effect:'thunder'},
  {name:'duplicate-rain', mode:'duplicate', effect:'rain'},
  {name:'span-rain', mode:'span', effect:'rain'}
]);
const fail = code => {throw Object.assign(Error(code), {code});};
const publicCode = error => typeof error?.code==='string'&&/^[A-Z][A-Z0-9_]{0,100}$/.test(error.code)?error.code:'MATRIX_FAILED';
const equal = (a,b) => hash(a) === hash(b);
const integer = n => Number.isSafeInteger(n) && n >= 0;
const displayId = v => typeof v === 'string' && /^display-[a-f0-9]{64}$/.test(v);
const sleep = ms => new Promise(resolve => setTimeout(resolve,ms));
const readJson = file => {
  const stat=fs.lstatSync(file);
  if(!stat.isFile() || stat.isSymbolicLink() || stat.size>8*1024*1024)fail('PRIVATE_RECORD_INVALID');
  return JSON.parse(fs.readFileSync(file,'utf8').replace(/^\uFEFF/,''));
};
function durable(file,value) {
  const temp=file+'.tmp',fd=fs.openSync(temp,'w');
  try{fs.writeFileSync(fd,JSON.stringify(value,null,2)+'\n');fs.fsyncSync(fd);}finally{fs.closeSync(fd);}
  fs.renameSync(temp,file);
}
function isInside(root,candidate) {
  const relative=path.relative(root,candidate);
  return relative==='' || relative!=='..' && !relative.startsWith('..'+path.sep) && !path.isAbsolute(relative);
}
export function parseResourceMatrixArgs(args) {
  if(args.length!==3 || !['--run','--recover'].includes(args[0]) || args[1]!=='--output' || !path.isAbsolute(args[2]))fail('MATRIX_ARGUMENT_INVALID');
  return {recover:args[0]==='--recover',output:path.resolve(args[2])};
}
function checkedDirectoryAncestors(directory,code) {
  let current=path.resolve(directory),targetExists=false;
  for(;;){
    try{const s=fs.lstatSync(current);if(s.isSymbolicLink()||!s.isDirectory())fail(code);if(current===path.resolve(directory))targetExists=true;}
    catch(e){if(e.code!=='ENOENT')throw e;}
    const parent=path.dirname(current);if(parent===current)break;current=parent;
  }
  return targetExists;
}
export function requestCollectorStop(output,phase) {
  if(!phases.some(p=>p.name===phase))fail('MATRIX_STOP_PATH_UNSAFE');
  const directory=path.join(output,phase);
  if(!checkedDirectoryAncestors(directory,'MATRIX_STOP_PATH_UNSAFE'))return false;
  const file=path.join(directory,'stop.request');
  const existingSafe=()=>{
    try{const s=fs.lstatSync(file);if(s.isSymbolicLink()||!s.isFile())fail('MATRIX_STOP_PATH_UNSAFE');return true;}
    catch(e){if(e.code==='ENOENT')return false;throw e;}
  };
  if(existingSafe())return true; // A regular existing request is sufficient; never append or overwrite it.
  let fd;
  try{fd=fs.openSync(file,'wx',0o600);fs.writeFileSync(fd,'stop\n');fs.fsyncSync(fd);return true;}
  catch(e){if(e.code==='EEXIST'&&checkedDirectoryAncestors(directory,'MATRIX_STOP_PATH_UNSAFE')&&existingSafe())return true;throw e;}
  finally{if(fd!==undefined)fs.closeSync(fd);}
}
function checkedOutput(output,recover) {
  if(!path.isAbsolute(output) || output.startsWith('\\\\') || isInside(repo,output))fail('MATRIX_PRIVATE_OUTPUT_REQUIRED');
  for(const protectedRoot of [process.env.LOCALAPPDATA&&path.join(process.env.LOCALAPPDATA,'io.atmos.wallpaper'),process.env.LOCALAPPDATA&&path.join(process.env.LOCALAPPDATA,'Atmos Wallpaper')].filter(Boolean)){
    if(isInside(protectedRoot,output))fail('MATRIX_PRIVATE_OUTPUT_REQUIRED');
  }
  checkedDirectoryAncestors(output,'MATRIX_OUTPUT_LINK_OR_FILE');
  if(recover){if(!fs.existsSync(output))fail('MATRIX_RECOVERY_MISSING');}
  else {if(fs.existsSync(output))fail('MATRIX_OUTPUT_EXISTS');fs.mkdirSync(output,{recursive:true,mode:0o700});}
}
function alive(pid){if(!integer(pid)||pid<1)return false;try{process.kill(pid,0);return true;}catch{return false;}}
function lockOutput(output) {
  const file=path.join(output,'runner.lock');
  if(fs.existsSync(file)){const old=readJson(file);if(alive(old.pid))fail('MATRIX_RUNNER_STILL_ACTIVE');fs.unlinkSync(file);}
  const fd=fs.openSync(file,'wx');try{fs.writeFileSync(fd,JSON.stringify({pid:process.pid}));fs.fsyncSync(fd);}finally{fs.closeSync(fd);}
  return ()=>{try{if(readJson(file).pid===process.pid)fs.unlinkSync(file);}catch{}};
}
function stateValid(v) {
  if(v?.ok!==true || !integer(v.revision) || typeof v.desiredHash!=='string' || !/^[a-f0-9]{64}$/.test(v.desiredHash))fail('MATRIX_STATE_INVALID');
  validateDesired(v.desired);if(hash(v.desired)!==v.desiredHash)fail('MATRIX_STATE_INVALID');return v;
}
function layoutValid(v) {
  const s=v?.settings,p=s?.layout;
  if(v?.available!==true || !integer(s?.revision) || !p || !['single','duplicate','span'].includes(p.mode) || !Array.isArray(p.targets) || p.targets.some(x=>!displayId(x)) || new Set(p.targets).size!==p.targets.length || typeof p.askOnApply!=='boolean' || (p.mode==='single'?p.targets.length!==0:p.targets.length!==2))fail('MATRIX_LAYOUT_UNSUPPORTED');
  return s;
}
async function snapshot(adapter) {
  // Slow policy CLIs must not age an already captured native heartbeat past its real five-second limit.
  // Read the host last and guard immediately; CAS still checks exact appearance/layout revisions.
  const schedule=await adapter.call(['--schedule-command','get']);
  const focus=await adapter.call(['--focus-command','get']);
  const performance=await adapter.call(['--performance-command','get']);
  const state=stateValid(await adapter.call(['--get-state']));
  const layout=layoutValid(await adapter.call(['--layout-command','get']));
  const host=await adapter.call(['--host-command','get']);
  if(!integer(host?.settings?.revision) || typeof host.settings.placement?.enabled!=='boolean' || typeof host.settings.placement.target!=='string' || !integer(schedule?.revision) || typeof schedule.policy?.enabled!=='boolean' || !integer(focus?.revision) || !['normal','focus','showcase'].includes(focus.mode) || !integer(performance?.settings?.revision))fail('MATRIX_POLICY_INVALID');
  return {state,layout,host,schedule:{revision:schedule.revision,policy:schedule.policy},focus:{revision:focus.revision,mode:focus.mode},performance:performance.settings};
}
function sameHost(s,preimage) {
  const n=s.host.runtime,age=Date.now()-n?.capturedAtMs;
  if(s.host.runtimeFresh!==true || !n || age<0 || age>=5000 || n.pid!==preimage.hostPid || n.hostSessionId!==preimage.hostSessionId)fail('MATRIX_HOST_CHANGED_OR_STALE');
  if(!equal(s.host.settings,preimage.hostSettings))fail('MATRIX_PLACEMENT_CHANGED');
}
function guards(s,preimage,journal,{pending=false}={}) {
  sameHost(s,preimage);
  if(!equal(s.schedule,preimage.schedule) || s.schedule.policy.enabled || !equal(s.focus,preimage.focus) || !equal(s.performance,preimage.performance))fail('MATRIX_CONCURRENT_POLICY_CHANGE');
  const p=journal.pending;
  const appearanceBefore=s.state.revision===journal.known.appearance.revision && s.state.desiredHash===journal.known.appearance.desiredHash;
  const appearanceAfter=pending && p?.kind==='appearance' && s.state.revision===p.body.expectedRevision+1 && s.state.desiredHash===p.afterHash;
  if(!appearanceBefore&&!appearanceAfter)fail('MATRIX_CONCURRENT_APPEARANCE_CHANGE');
  const layoutBefore=s.layout.revision===journal.known.layout.revision && equal(s.layout.layout,journal.known.layout.layout);
  const layoutAfter=pending && p?.kind==='layout' && s.layout.revision===p.body.expectedRevision+1 && s.layout.lastRequestId===p.body.requestId && equal(s.layout.layout,p.body.layout);
  if(!layoutBefore&&!layoutAfter)fail('MATRIX_CONCURRENT_LAYOUT_CHANGE');
}
function expectedTargets(preimage,journal) {
  return journal.known.layout.layout.mode==='single'?[preimage.originalTarget]:preimage.selectedDisplays.map(d=>d.id);
}
function nativeReady(s,preimage,journal) {
  const n=s.host.runtime,ids=expectedTargets(preimage,journal),targets=n?.targets;
  return n?.nativeAttached===true && n.settingsRevision===preimage.hostSettings.revision && n.layoutRevision===journal.known.layout.revision && n.configuredTargetCount===ids.length && Array.isArray(targets) && targets.length===ids.length && ids.every(id=>{
    const t=targets.find(t=>t.targetId===id),d=preimage.selectedDisplays.find(d=>d.id===id);
    return t?.scopeId==='shared' && t.nativeAttached===true && t.rendererAcknowledged===true && d && equal(t.bounds,d.bounds) && t.runtimePolicy?.paused===false;
  }) && s.state.rendererAcknowledged===true;
}
async function waitReady(adapter,preimage,journal,{signal,timeoutMs=25000}={}) {
  const until=Date.now()+timeoutMs;
  do {
    if(signal?.aborted)fail('MATRIX_INTERRUPTED');
    const s=await snapshot(adapter);guards(s,preimage,journal);
    if(nativeReady(s,preimage,journal))return s;
    await adapter.sleep(250);
  }while(Date.now()<until);
  fail('MATRIX_NATIVE_OR_ACK_TIMEOUT');
}
function desktopRestored(before,after) {
  if(!Array.isArray(before?.windows)||!Array.isArray(after?.windows))return false;
  const nonTauri=before.windows.filter(w=>w.class!=='Tauri Window');
  const shapes=v=>v.windows.filter(w=>w.class==='Tauri Window').map(({class:c,parent,visible,bounds})=>hash({class:c,parent,visible,bounds})).sort();
  return nonTauri.every(w=>after.windows.some(a=>a.handle===w.handle&&equal(a,w))) && equal(shapes(before),shapes(after));
}
function resourceCoverage(a) {
  const count=a?.measurement?.sampleCount,cpu=a?.measurement?.cpuCoverage;
  const privateSamples=a?.metrics?.all?.privateMiB?.samples,workingSetSamples=a?.metrics?.all?.workingSetMiB?.samples;
  const memorySamples=integer(privateSamples)&&integer(workingSetSamples)?Math.min(privateSamples,workingSetSamples):null;
  const gpuSamples=a?.gpu?.availableSamples;
  const fraction=n=>integer(n)&&integer(count)&&count>0&&n<=count?n/count:null;
  const cpuFraction=typeof cpu?.completeCpuFractionOfObserved==='number'&&Number.isFinite(cpu.completeCpuFractionOfObserved)&&cpu.completeCpuFractionOfObserved>=0&&cpu.completeCpuFractionOfObserved<=1?cpu.completeCpuFractionOfObserved:null;
  const gpuFraction=fraction(gpuSamples),memoryFraction=fraction(memorySamples);
  const thresholds={minimumCompleteCpuTimeFraction:.8,minimumGpuSampleFraction:.8,minimumCompleteMemorySampleFraction:.8,minimumCompleteCpuIntervals:10};
  const sufficient=cpuFraction!==null&&cpuFraction>=.8&&integer(cpu?.completeCohortIntervals)&&cpu.completeCohortIntervals>=10&&gpuFraction!==null&&gpuFraction>=.8&&memoryFraction!==null&&memoryFraction>=.8;
  const warmup=integer(a?.gpu?.warmupSamples)?a.gpu.warmupSamples:0;
  return {sufficient,thresholds,sampleCount:count??null,cpuCompleteTimeFraction:cpuFraction,cpuCompleteIntervals:cpu?.completeCohortIntervals??null,cpuPartialIntervals:cpu?.partialCohortIntervals??null,cpuObservedSeconds:cpu?.observedIntervalSeconds??null,cpuCompleteSeconds:cpu?.completeCpuSeconds??null,gpuValidSamples:gpuSamples??null,gpuValidSampleFraction:gpuFraction,gpuWarmupSamples:warmup,gpuUnavailableNonWarmupSamples:integer(count)&&integer(gpuSamples)?Math.max(0,count-gpuSamples-warmup):null,privateCompleteSamples:privateSamples??null,workingSetCompleteSamples:workingSetSamples??null,memoryCompleteSamples:memorySamples,memoryCompleteSampleFraction:memoryFraction};
}
function checkedJournal(preimage,journal) {
  if(preimage?.schemaVersion!==1 || journal?.schemaVersion!==1 || journal.preimageHash!==hash(preimage) || !/^[a-f0-9]{16}$/.test(journal.runId) || !integer(journal.sequence) || !integer(journal.known?.appearance?.revision) || !/^[a-f0-9]{64}$/.test(journal.known.appearance.desiredHash) || !integer(journal.known?.layout?.revision) || !Array.isArray(journal.phases))fail('MATRIX_RECOVERY_INVALID');
  stateValid(preimage.state);layoutValid({available:true,settings:preimage.layout});
  if(preimage.layout.layout.mode!=='single' || preimage.hostSettings?.placement?.enabled!==true || !displayId(preimage.originalTarget) || !Array.isArray(preimage.selectedDisplays) || preimage.selectedDisplays.length!==2 || preimage.selectedDisplays.some(d=>!displayId(d.id)) || new Set(preimage.selectedDisplays.map(d=>d.id)).size!==2 || preimage.selectedDisplays[0].id!==preimage.originalTarget || !integer(preimage.hostPid) || preimage.hostPid<1 || typeof preimage.hostSessionId!=='string' || !/^\d+-\d+$/.test(preimage.hostSessionId))fail('MATRIX_RECOVERY_INVALID');
  const layouts=[preimage.layout.layout,{mode:'duplicate',targets:preimage.selectedDisplays.map(d=>d.id),askOnApply:false},{mode:'span',targets:preimage.selectedDisplays.map(d=>d.id),askOnApply:false}];
  const appearanceHashes=[preimage.state.desiredHash,...['rain','thunder'].map(effect=>hash(mergePatch(preimage.state.desired,{visual:{effect,intensity:100}})))];
  if(!appearanceHashes.includes(journal.known.appearance.desiredHash) || !layouts.some(l=>equal(l,journal.known.layout.layout)))fail('MATRIX_RECOVERY_INVALID');
  if(journal.pending){const p=journal.pending;if(!['appearance','layout'].includes(p.kind) || p.body?.schemaVersion!==1 || !integer(p.body.expectedRevision) || !/^resource\.[a-f0-9]{16}\.\d+$/.test(p.body.requestId))fail('MATRIX_RECOVERY_INVALID');
    if(p.kind==='appearance' && (p.body.type!=='settings.update' || Object.keys(p.body.changes||{}).join(',')!=='visual' || Object.keys(p.body.changes.visual).sort().join(',')!=='effect,intensity' || !['rain','thunder',preimage.state.desired.visual.effect].includes(p.body.changes.visual.effect) || ![100,preimage.state.desired.visual.intensity].includes(p.body.changes.visual.intensity)))fail('MATRIX_RECOVERY_INVALID');
    if(p.kind==='appearance' && (p.body.expectedRevision!==journal.known.appearance.revision || p.afterHash!==hash(mergePatch(preimage.state.desired,p.body.changes))))fail('MATRIX_RECOVERY_INVALID');
    if(p.kind==='layout' && (p.body.expectedRevision!==journal.known.layout.revision || !layouts.some(l=>equal(l,p.body.layout))))fail('MATRIX_RECOVERY_INVALID');
  }
}
async function stopOldCollector(output,journal,adapter) {
  const active=journal.activeCollector;if(!active)return;
  if(!phases.some(p=>p.name===active.phase)||!integer(active.pid))fail('MATRIX_RECOVERY_INVALID');
  const dir=path.join(output,active.phase);
  if(alive(active.pid)){
    for(let i=0;i<80&&alive(active.pid);i++){
      requestCollectorStop(output,active.phase);
      await adapter.sleep(250);
    }
    if(alive(active.pid))fail('MATRIX_COLLECTOR_STILL_ACTIVE');
  }
  journal.activeCollector=null;
}

export async function runResourceMatrix({output,recover=false,adapter,signal}={}) {
  checkedOutput(output,recover);const unlock=lockOutput(output),preFile=path.join(output,'preimage.json'),journalFile=path.join(output,'intent.json');
  let preimage,journal,prepared=false,error=null,restoration={attempted:false,desiredHashRestored:false,layoutRestored:false,placementUnchanged:false,sameHostAlive:false,genuineAck:false,desktopMetadataRestored:false},result;
  const save=()=>durable(journalFile,journal);
  const inputFile=p=>path.join(output,p.body.requestId+'.json');
  async function invokePending() {
    const p=journal.pending;
    const current=await snapshot(adapter);guards(current,preimage,journal,{pending:true});
    if(p.kind==='appearance'){
      durable(inputFile(p),p.body);
      await adapter.call(['--apply-json',inputFile(p),'--wait-ms','10000']);
      const receipt=await adapter.call(['--get-result',p.body.requestId]);
      if(receipt.committed!==true || receipt.revision!==p.body.expectedRevision+1 || receipt.desiredHash!==p.afterHash)fail('MATRIX_RECEIPT_UNCERTAIN');
      journal.known.appearance={revision:receipt.revision,desiredHash:receipt.desiredHash};
    }else {
      await adapter.call(['--layout-command','update'],p.body);
      const next=layoutValid(await adapter.call(['--layout-command','get']));
      if(next.revision!==p.body.expectedRevision+1 || next.lastRequestId!==p.body.requestId || !equal(next.layout,p.body.layout))fail('MATRIX_RECEIPT_UNCERTAIN');
      journal.known.layout=next;
    }
    journal.pending=null;save();
  }
  async function mutate(kind,policy) {
    const current=await snapshot(adapter);guards(current,preimage,journal);
    const id='resource.'+journal.runId+'.'+(++journal.sequence);
    if(kind==='appearance'){
      const changes={visual:{effect:policy.effect,intensity:policy.intensity}},after=mergePatch(current.state.desired,changes);
      if(hash(after)===current.state.desiredHash)return;
      journal.pending={kind,body:{schemaVersion:1,type:'settings.update',requestId:id,expectedRevision:current.state.revision,changes},afterHash:hash(after)};
    }else {
      if(equal(current.layout.layout,policy))return;
      journal.pending={kind,body:{schemaVersion:1,requestId:id,expectedRevision:current.layout.revision,layout:policy}};
    }
    save();await invokePending();
  }
  try {
    if(recover){
      preimage=readJson(preFile);journal=readJson(journalFile);checkedJournal(preimage,journal);
      if(journal.status==='restored' || journal.status==='complete')fail('MATRIX_ALREADY_RESTORED');
      prepared=true;await stopOldCollector(output,journal,adapter);save();
    }else {
      // Topology/desktop queries can also take seconds; keep them before the freshness-sensitive snapshot.
      const desktop=await adapter.call(['--inspect-desktop']),ds=await adapter.call(['--list-displays']);
      const initial=await snapshot(adapter);
      if(initial.layout.layout.mode!=='single' || initial.layout.layout.targets.length || initial.host.settings.placement.enabled!==true || initial.schedule.policy.enabled || initial.focus.mode!=='normal')fail('MATRIX_SINGLE_SHARED_IDLE_REQUIRED');
      const displays=(Array.isArray(ds)?ds:ds.displays)?.filter(d=>d.selectable===true);
      const n=initial.host.runtime,originalTarget=n?.targets?.[0]?.targetId;
      if(!displays || displays.length<2 || !displayId(originalTarget) || n?.targets?.length!==1)fail('MATRIX_TWO_SELECTABLE_DISPLAYS_REQUIRED');
      const first=displays.find(d=>d.id===originalTarget),second=displays.find(d=>d.id!==originalTarget);
      if(!first||!second)fail('MATRIX_TWO_SELECTABLE_DISPLAYS_REQUIRED');
      preimage={schemaVersion:1,capturedAt:new Date().toISOString(),state:initial.state,layout:initial.layout,hostSettings:initial.host.settings,hostPid:n.pid,hostSessionId:n.hostSessionId,originalTarget,selectedDisplays:[first,second],schedule:initial.schedule,focus:initial.focus,performance:initial.performance,desktop};
      journal={schemaVersion:1,preimageHash:hash(preimage),runId:randomBytes(8).toString('hex'),sequence:0,status:'prepared',known:{appearance:{revision:initial.state.revision,desiredHash:initial.state.desiredHash},layout:initial.layout},pending:null,activeCollector:null,phases:[]};
      checkedJournal(preimage,journal);sameHost(initial,preimage);
      if(!nativeReady(initial,preimage,journal))fail('MATRIX_INITIAL_NATIVE_OR_ACK_NOT_READY');
      durable(preFile,preimage);save();prepared=true;
    }
    if(recover){journal.status='recovering';save();}
    else {
      journal.status='running';save();
      for(const phase of phases){
        if(signal?.aborted)fail('MATRIX_INTERRUPTED');
        await mutate('appearance',{effect:phase.effect,intensity:100});
        await mutate('layout',phase.mode==='single'?preimage.layout.layout:{mode:phase.mode,targets:preimage.selectedDisplays.map(d=>d.id),askOnApply:false});
        const before=await waitReady(adapter,preimage,journal,{signal});
        journal.stage=phase.name;save();
        const collected=await adapter.collect(path.join(output,phase.name),{durationSeconds:120,sampleSeconds:2,signal,expected:{hostPid:preimage.hostPid,hostSessionId:preimage.hostSessionId,appearanceRevision:journal.known.appearance.revision,layoutRevision:journal.known.layout.revision,placementRevision:preimage.hostSettings.revision,targetIds:expectedTargets(preimage,journal),effect:phase.effect,intensity:100},onStarted:pid=>{journal.activeCollector={phase:phase.name,pid};save();}});
        journal.activeCollector=null;save();
        const after=await waitReady(adapter,preimage,journal,{signal});
        const a=collected.analysis;
        const coverage=resourceCoverage(a),coverageCodes=['PROCESS_COVERAGE_PARTIAL','GPU_COUNTER_COVERAGE_PARTIAL'];
        const measurementNotes=(a?.reviewFlags||[]).filter(f=>coverageCodes.includes(f.code)),blockingReviewFlags=(a?.reviewFlags||[]).filter(f=>!coverageCodes.includes(f.code));
        const partialMeasurement=measurementNotes.length>0 || coverage.cpuCompleteTimeFraction<1 || coverage.memoryCompleteSampleFraction<1 || coverage.gpuUnavailableNonWarmupSamples>0;
        const accepted=coverage.sufficient && a?.measurement?.completionSuccessful===true && a.measurement.durationReached===true && a.measurement.sampleCount>=11 && a?.gpu?.utilizationAnalyzed===true && a?.metrics?.all?.cpu?.completeCohort?.observedSeconds>0 && a?.workload?.constantRevisionObserved===true && a?.renderers?.expectedTargetCount===(phase.mode==='single'?1:2) && a.renderers.negativeAllTargetAckSamples===0 && a.renderers.unknownAllTargetAckSamples===0 && a?.renderers?.nativeAttachmentOutages?.length===0 && a?.renderers?.ackOutages?.length===0 && Array.isArray(a.reviewFlags) && blockingReviewFlags.length===0 && collected.collectionChecks && Object.values(collected.collectionChecks).every(v=>v===true);
        journal.phases.push({passed:!!accepted,partialMeasurement,coverage,measurementNotes,blockingReviewFlags,mode:phase.mode,effect:phase.effect,intensity:100,version:typeof collected.version==='string'&&/^[0-9A-Za-z.+ -]{1,80}$/.test(collected.version)?collected.version:null,effectivePolicies:[before,after].map(s=>s.host.runtime.targets.map(t=>t.runtimePolicy)),collectionChecks:collected.collectionChecks,analysis:a});save();
        if(!accepted)fail(coverage.sufficient?'MATRIX_SAMPLE_REQUIRES_REVIEW':'MATRIX_MEASUREMENT_COVERAGE_INSUFFICIENT');
        // Return to the user's original effect before testing additional placement modes.
        if(phase.mode==='single')await mutate('appearance',{effect:preimage.state.desired.visual.effect,intensity:preimage.state.desired.visual.intensity});
      }
    }
  }catch(e){error=e;}
  finally {
    if(prepared){
      restoration.attempted=true;
      try {
        await stopOldCollector(output,journal,adapter);
        if(journal.pending)await invokePending();
        await mutate('layout',preimage.layout.layout);
        await mutate('appearance',{effect:preimage.state.desired.visual.effect,intensity:preimage.state.desired.visual.intensity});
        const after=await waitReady(adapter,preimage,journal); // Recovery deliberately ignores the stop signal.
        restoration={attempted:true,desiredHashRestored:after.state.desiredHash===preimage.state.desiredHash,layoutRestored:equal(after.layout.layout,preimage.layout.layout),placementUnchanged:equal(after.host.settings,preimage.hostSettings),sameHostAlive:true,genuineAck:nativeReady(after,preimage,journal),desktopMetadataRestored:desktopRestored(preimage.desktop,await adapter.call(['--inspect-desktop']))};
        if(!Object.values(restoration).every(Boolean))fail('MATRIX_RESTORATION_INCOMPLETE');
        journal.status=error?'restored':'complete';journal.restoration=restoration;save();
      }catch(e){journal.status='needs-recovery';journal.restoration=restoration;journal.recoveryError=e.code||'MATRIX_RESTORE_FAILED';save();if(!error)error=e;}
    }
    if(prepared){
      // A recovery journal is private, editable input, not reviewed public evidence. Never re-export its phases.
      const publicPhases=recover?[]:journal.phases,recoveredPhaseCount=recover&&integer(journal.phases.length)&&journal.phases.length<=3?journal.phases.length:null;
      result={schemaVersion:1,kind:'installed-resource-short-matrix',passed:!error&&!recover&&journal.phases.length===3,partialMeasurement:publicPhases.some(p=>p.partialMeasurement===true),recoveryOnly:recover,recovered:recover&&Object.values(restoration).every(Boolean),...(recover?{recoveredPhaseCount}:{}),status:journal.status,error:error?{code:publicCode(error)}:null,phases:publicPhases,restoration,limitations:['Each scenario is a 120-second resource sample, not a two-hour soak or leak proof.','PROCESS_COVERAGE_PARTIAL and GPU_COUNTER_COVERAGE_PARTIAL are measurement notes only when explicit CPU/GPU/memory coverage thresholds are met; all original flags and counts remain reported.','GPU is busiest accounted engine and process memory accounting, not whole-GPU percent, unique VRAM or watts.','The manager remains open; costs include the complete owned tree.','No manual visual, lock/logon, sleep, hotplug or mixed-DPI test. Renderer reports do not prove visible presentation.','CAS transactions preserve original content but advance revisions, history and receipts.','Interrupted or forced-killed runners require explicit --recover; concurrent changes or a different host session prevent automatic overwrite.','Recovery-only summaries omit private journal phase contents; recoveredPhaseCount is a bounded count, not evidence that those samples passed.']};
      durable(path.join(output,'public-summary.json'),result);
    }
    unlock();
  }
  if(!result)throw error||Object.assign(Error('MATRIX_PREFLIGHT_FAILED'),{code:'MATRIX_PREFLIGHT_FAILED'});
  return result;
}

export function installedAdapter({executable,output}={}) {
  const ps=path.join(process.env.SystemRoot,'System32/WindowsPowerShell/v1.0/powershell.exe');
  return {
    sleep,
    async call(args,input){
      let r,body;for(let i=0;i<5;i++){
        r=spawnSync(executable,args,{encoding:'utf8',windowsHide:true,input:input===undefined?undefined:JSON.stringify(input),timeout:45000,maxBuffer:2*1024*1024});
        body=undefined;try{body=JSON.parse(r.stdout);}catch{}
        if(r.status!==4&&!['STORE_LOCKED','LAYOUT_BUSY','LAYOUT_STORE_BUSY'].includes(body?.error?.code))break;
        await sleep(100);
      }
      if(r.status!==0 || body?.ok===false)fail(body?.error?.code||'MATRIX_NATIVE_COMMAND_FAILED');
      return body;
    },
    collect(dir,{durationSeconds,sampleSeconds,signal,expected,onStarted}){
      return new Promise((resolve,reject)=>{
        if(signal?.aborted)return reject(Object.assign(Error('MATRIX_INTERRUPTED'),{code:'MATRIX_INTERRUPTED'}));
        const log=fs.openSync(path.join(output,path.basename(dir)+'.collector.log'),'wx');
        const child=spawn(ps,['-NoLogo','-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File',fileURLToPath(new URL('./measure-installed-soak.ps1',import.meta.url)),'-DurationSeconds',String(durationSeconds),'-SampleSeconds',String(sampleSeconds),'-OutputDirectory',dir,'-Run'],{windowsHide:true,stdio:['ignore','pipe','pipe']});
        let timedOut=false,startFailed=false,stopError=null;
        const stop=()=>{try{requestCollectorStop(output,path.basename(dir));}catch(e){stopError=e;child.kill();}};
        const abortPoll=setInterval(()=>{if(signal?.aborted)stop();},250);
        const timeout=setTimeout(()=>{timedOut=true;stop();child.kill();},(durationSeconds+90)*1000);
        child.stdout.on('data',b=>fs.writeSync(log,b));child.stderr.on('data',b=>fs.writeSync(log,b));
        if(signal?.aborted)stop();signal?.addEventListener('abort',stop);
        child.on('spawn',()=>{try{onStarted(child.pid);}catch{startFailed=true;stop();child.kill();}});
        child.on('error',()=>{startFailed=true;});
        child.on('close',code=>{
          clearTimeout(timeout);clearInterval(abortPoll);signal?.removeEventListener('abort',stop);try{fs.closeSync(log);}catch{}
          if(stopError)return reject(stopError);
          if(code!==0||timedOut||startFailed||signal?.aborted)return reject(Object.assign(Error('MATRIX_COLLECTOR_FAILED'),{code:signal?.aborted?'MATRIX_INTERRUPTED':startFailed?'MATRIX_COLLECTOR_START_FAILED':timedOut?'MATRIX_COLLECTOR_TIMEOUT':'MATRIX_COLLECTOR_FAILED'}));
          try{
            const metadata=readJson(path.join(dir,'metadata.json')),file=path.join(dir,'samples.jsonl');
            if(fs.statSync(file).size>256*1024*1024)fail('MATRIX_SAMPLE_TOO_LARGE');
            const parsed=readJsonLines(fs.readFileSync(file,'utf8'),{preserveInvalid:true}),rows=parsed.samples;
            const collectionChecks={
              validSampleLines:rows.length>0&&parsed.invalidLines===0,
              pinnedMetadata:metadata.root?.pid===expected.hostPid&&metadata.hostSessionId===expected.hostSessionId&&equal([...(metadata.expectedTargetIds||[])].sort(),[...expected.targetIds].sort()),
              pinnedHost:rows.length>0&&rows.every(s=>s?.native?.pid===expected.hostPid&&s.native.hostSessionId===expected.hostSessionId),
              unchangedRevisions:rows.length>0&&rows.every(s=>s?.native?.layoutRevision===expected.layoutRevision&&s.native.settingsRevision===expected.placementRevision&&s.workload?.shared?.revision===expected.appearanceRevision),
              requestedWorkload:rows.length>0&&rows.every(s=>s?.workload?.shared?.effect===expected.effect&&s.workload.shared.intensity===expected.intensity),
              exactTargets:rows.length>0&&rows.every(s=>Array.isArray(s?.native?.targets)&&equal(s.native.targets.map(t=>t.targetId).sort(),[...expected.targetIds].sort())&&s.native.targets.every(t=>t.scopeId==='shared')),
              unpausedPolicy:rows.length>0&&rows.every(s=>s?.native?.targets?.length>0&&s.native.targets.every(t=>t.runtimePolicy?.paused===false))
            };
            resolve({version:metadata.version,collectionChecks,analysis:analyzeDirectory(dir,{warmupSeconds:30,endpointWindowSeconds:20})});
          }catch(e){reject(e);}
        });
      });
    }
  };
}

if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
  try {
    if(process.platform!=='win32')fail('MATRIX_WINDOWS_REQUIRED');
    const options=parseResourceMatrixArgs(process.argv.slice(2)),controller=new AbortController(),stop=()=>controller.abort();
    process.once('SIGINT',stop);process.once('SIGTERM',stop);
    try {
      const executable=path.join(process.env.LOCALAPPDATA,'Atmos Wallpaper/atmos-wallpaper.exe');
      const result=await runResourceMatrix({...options,adapter:installedAdapter({executable,output:options.output}),signal:controller.signal});
      console.log(JSON.stringify(result,null,2));if(!(result.passed||result.recovered))process.exitCode=1;
    }finally{process.removeListener('SIGINT',stop);process.removeListener('SIGTERM',stop);}
  }catch(e){console.error(JSON.stringify({passed:false,error:{code:publicCode(e)},note:'No success claimed. Private journal, if created, must be reviewed before explicit --recover.'}));process.exitCode=1;}
}
