// Pure simulated-machine safety checks. These never invoke an installed app, GPU or desktop API.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {hash,mergePatch,initialDesired} from '../src/contract.mjs';
import {runResourceMatrix,parseResourceMatrixArgs,requestCollectorStop} from '../../desktop-host/scripts/test-resource-matrix-installed.mjs';

function createMock(output) {
  const ds=[{id:'display-'+'a'.repeat(64),selectable:true,primary:true,bounds:{x:0,y:0,width:1920,height:1080}},{id:'display-'+'b'.repeat(64),selectable:true,primary:false,bounds:{x:-1920,y:0,width:1920,height:1080}}];
  const initial=initialDesired();initial.visual.effect='rain';initial.visual.intensity=61;initial.widgets.todos=[{id:'private-todo',text:'PRIVATE_FIXTURE_CONTENT',done:false}];
  let desired=initial,revision=7,layout={schemaVersion:1,revision:3,lastRequestId:null,layout:{mode:'single',targets:[],askOnApply:true}};
  const host={schemaVersion:1,revision:4,placement:{enabled:true,target:'primary'}},receipts=new Map(),actions=[],collected=[];
  const schedule={revision:2,policy:{enabled:false}},focus={revision:0,mode:'normal'},performance={schemaVersion:1,revision:1,lastRequestId:null,performance:{fps:30,pauseBattery:false,pauseFullscreen:false}};
  const desktop={windows:[{handle:100,parent:0,class:'WorkerW',visible:true,bounds:ds[0].bounds},{handle:101,parent:100,class:'Tauri Window',visible:true,bounds:ds[0].bounds}]};
  const copy=v=>JSON.parse(JSON.stringify(v));
  const api={
    actions,collected,initialHash:hash(initial),originalLayout:copy(layout.layout),schedule,focus,performance,hostPid:987654,hostSessionId:'987654-12345678',rendererAck:true,
    dump:()=>({desired,revision,layout,receipts:[...receipts]}),
    load:v=>{desired=copy(v.desired);revision=v.revision;layout=copy(v.layout);receipts.clear();for(const[k,r]of v.receipts||[])receipts.set(k,copy(r));},
    changeUserAppearance:()=>{desired=mergePatch(desired,{visual:{intensity:42}});revision++;},
    changeUserLayout:()=>{layout={...layout,revision:layout.revision+1,lastRequestId:'user.concurrent',layout:{...layout.layout,askOnApply:true}};},
    sleep:async()=>{},
    async call(args,input){
      actions.push({args:[...args],input:copy(input??null)});
      const ids=layout.layout.mode==='single'?[ds[0].id]:layout.layout.targets;
      if(args[0]==='--get-state')return {ok:true,revision,desiredHash:hash(desired),desired:copy(desired),rendererAcknowledged:api.rendererAck};
      if(args[0]==='--layout-command'){
        if(args[1]==='update'){
          if(!receipts.has(input.requestId)){
            if(input.expectedRevision!==layout.revision)throw Object.assign(Error('conflict'),{code:'LAYOUT_REVISION_CONFLICT'});
            layout={schemaVersion:1,revision:layout.revision+1,lastRequestId:input.requestId,layout:copy(input.layout)};receipts.set(input.requestId,copy(layout));
          }
        }
        return {available:true,settings:copy(layout)};
      }
      if(args[0]==='--host-command')return {available:true,settings:copy(host),runtimeFresh:true,runtime:{pid:api.hostPid,hostSessionId:api.hostSessionId,capturedAtMs:Date.now(),settingsRevision:host.revision,layoutRevision:layout.revision,nativeAttached:true,configuredTargetCount:ids.length,targets:ids.map(id=>({targetId:id,scopeId:'shared',bounds:ds.find(d=>d.id===id).bounds,nativeAttached:true,rendererAcknowledged:api.rendererAck,runtimePolicy:{paused:false,fps:30,onBattery:false,fullscreenCovered:false,reason:'active'}}))}};
      if(args[0]==='--schedule-command')return copy(schedule);
      if(args[0]==='--focus-command')return copy(focus);
      if(args[0]==='--performance-command')return {available:true,settings:copy(performance)};
      if(args[0]==='--list-displays')return copy(ds);
      if(args[0]==='--inspect-desktop')return copy(desktop);
      if(args[0]==='--apply-json'){
        const body=JSON.parse(fs.readFileSync(args[1],'utf8'));
        if(!receipts.has(body.requestId)){
          if(body.expectedRevision!==revision)throw Object.assign(Error('conflict'),{code:'REVISION_CONFLICT'});
          desired=mergePatch(desired,body.changes);revision++;
          receipts.set(body.requestId,{committed:true,revision,desiredHash:hash(desired),rendererAcknowledged:true});
        }
        return copy(receipts.get(body.requestId));
      }
      if(args[0]==='--get-result')return copy(receipts.get(args[1])||{found:false,committed:false});
      throw Error('Unexpected mock command '+args[0]);
    },
    async collect(dir,options){
      assert.ok(fs.existsSync(path.join(output,'preimage.json')),'Preimage must exist before changed state is sampled');
      const intent=JSON.parse(fs.readFileSync(path.join(output,'intent.json'),'utf8'));
      assert.equal(intent.pending,null);assert.equal(intent.known.appearance.desiredHash,hash(desired));
      options.onStarted(0);collected.push({name:path.basename(dir),effect:desired.visual.effect,intensity:desired.visual.intensity,layout:copy(layout.layout),duration:options.durationSeconds,interval:options.sampleSeconds});
      return {version:'0.7.1',collectionChecks:{pinnedHost:true,requestedWorkload:true,unpausedPolicy:true},analysis:{measurement:{completionSuccessful:true,durationReached:true,sampleCount:61,cpuCoverage:{completeCpuFractionOfObserved:1,completeCohortIntervals:60,partialCohortIntervals:0,observedIntervalSeconds:120,completeCpuSeconds:120}},gpu:{utilizationAnalyzed:true,availableSamples:60,warmupSamples:1,busiestOwnedEnginePercent:{mean:12}},metrics:{all:{cpu:{completeCohort:{observedSeconds:120}},privateMiB:{samples:61},workingSetMiB:{samples:61}}},reviewFlags:[],workload:{constantRevisionObserved:true},renderers:{expectedTargetCount:idsForCount(),negativeAllTargetAckSamples:0,unknownAllTargetAckSamples:0,nativeAttachmentOutages:[],ackOutages:[]}}};
    }
  };
  function idsForCount(){return layout.layout.mode==='single'?1:2;}
  return api;
}
function fixture(t){const root=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-matrix-safety-'));t.after(()=>fs.rmSync(root,{recursive:true,force:true}));return{root,output:path.join(root,'private-run')};}

test('matrix requires explicit action, an absolute fresh output, and rejects repository output before CLI access',async t=>{
  const {output}=fixture(t);let calls=0;
  assert.throws(()=>parseResourceMatrixArgs(['--output',output]),/MATRIX_ARGUMENT_INVALID/);
  assert.throws(()=>parseResourceMatrixArgs(['--run','--output','relative']),/MATRIX_ARGUMENT_INVALID/);
  const repo=fileURLToPath(new URL('../../',import.meta.url));
  await assert.rejects(runResourceMatrix({output:path.join(repo,'private-matrix-forbidden'),adapter:{call(){calls++;}}}),/MATRIX_PRIVATE_OUTPUT_REQUIRED/);
  fs.mkdirSync(output);await assert.rejects(runResourceMatrix({output,adapter:{call(){calls++;}}}),/MATRIX_OUTPUT_EXISTS/);assert.equal(calls,0);
});
test('all scenarios preserve private state and original placement without launching or exiting the host',async t=>{
  const {output}=fixture(t),mock=createMock(output),result=await runResourceMatrix({output,adapter:mock});
  assert.equal(result.passed,true);assert.equal(result.status,'complete');assert.ok(Object.values(result.restoration).every(Boolean));
  assert.deepEqual(mock.collected.map(p=>[p.name,p.effect,p.intensity,p.layout.mode,p.duration,p.interval]),[['single-thunder','thunder',100,'single',120,2],['duplicate-rain','rain',100,'duplicate',120,2],['span-rain','rain',100,'span',120,2]]);
  assert.equal(hash(mock.dump().desired),mock.initialHash);assert.deepEqual(mock.dump().layout.layout,mock.originalLayout);
  assert.ok(mock.dump().revision>7);assert.ok(mock.dump().layout.revision>3);
  assert.ok(!mock.actions.some(a=>['--quit','--manager','--host-command update'].includes(a.args.join(' '))));
  const published=JSON.stringify(result);for(const secret of ['PRIVATE_FIXTURE_CONTENT','private-todo','987654','12345678','display-'+ 'a'.repeat(64),output])assert.ok(!published.includes(secret),'Public result leaked '+secret);
});
test('slow policy and topology reads precede the fresh host heartbeat instead of artificially expiring it',async t=>{
  t.mock.timers.enable({apis:['Date'],now:Date.now()});t.after(()=>t.mock.timers.reset());
  const {output}=fixture(t),mock=createMock(output),call=mock.call;
  const slow=['--schedule-command','--focus-command','--performance-command','--inspect-desktop','--list-displays'];
  mock.call=async(args,input)=>{const value=await call(args,input);if(slow.includes(args[0]))t.mock.timers.tick(2000);return value;};
  const result=await runResourceMatrix({output,adapter:mock});assert.equal(result.passed,true);assert.ok(result.restoration.genuineAck);
  const args=mock.actions.map(a=>a.args.join(' '));
  assert.deepEqual(args.slice(0,8),['--inspect-desktop','--list-displays','--schedule-command get','--focus-command get','--performance-command get','--get-state','--layout-command get','--host-command get']);
  for(let i=0;i<args.length;i++)if(args[i]==='--host-command get')assert.deepEqual(args.slice(i-5,i),['--schedule-command get','--focus-command get','--performance-command get','--get-state','--layout-command get']);
});
test('a genuinely five-second-old or older native heartbeat still fails without mutation',async t=>{
  t.mock.timers.enable({apis:['Date'],now:Date.now()});t.after(()=>t.mock.timers.reset());
  for(const age of [5000,6500]){
    const {output}=fixture(t),mock=createMock(output),call=mock.call;
    mock.call=async(args,input)=>{const value=await call(args,input);if(args[0]==='--host-command')value.runtime.capturedAtMs=Date.now()-age;return value;};
    await assert.rejects(runResourceMatrix({output,adapter:mock}),/MATRIX_HOST_CHANGED_OR_STALE/);
    assert.equal(mock.actions.filter(a=>a.args[0]==='--apply-json'||a.args[1]==='update').length,0);
  }
});
test('collector failure restores original content using CAS and reports the failed sample',async t=>{
  const {output}=fixture(t),mock=createMock(output);mock.collect=async()=>{throw Object.assign(Error('synthetic collector failure'),{code:'MATRIX_COLLECTOR_FAILED'});};
  const result=await runResourceMatrix({output,adapter:mock});assert.equal(result.passed,false);assert.equal(result.status,'restored');assert.equal(result.error.code,'MATRIX_COLLECTOR_FAILED');assert.equal(hash(mock.dump().desired),mock.initialHash);assert.ok(result.restoration.genuineAck);
});
test('partial GPU coverage cannot pass and its anonymous failed aggregate is preserved',async t=>{
  const {output}=fixture(t),mock=createMock(output),collect=mock.collect;
  mock.collect=async(...args)=>{const r=await collect(...args);r.analysis.gpu.availableSamples=20;return r;};
  const result=await runResourceMatrix({output,adapter:mock});assert.equal(result.passed,false);assert.equal(result.phases.length,1);assert.equal(result.phases[0].passed,false);assert.equal(result.phases[0].analysis.gpu.availableSamples,20);assert.equal(result.status,'restored');assert.equal(hash(mock.dump().desired),mock.initialHash);
});
test('coverage gaps above explicit CPU/GPU/memory thresholds remain visible without claiming a product fault',async t=>{
  const {output}=fixture(t),mock=createMock(output),collect=mock.collect;
  mock.collect=async(...args)=>{const r=await collect(...args),a=r.analysis;a.measurement.cpuCoverage={completeCpuFractionOfObserved:.96,completeCohortIntervals:57,partialCohortIntervals:3,observedIntervalSeconds:120,completeCpuSeconds:115.2};a.gpu.availableSamples=56;a.metrics.all.privateMiB.samples=58;a.metrics.all.workingSetMiB.samples=58;a.reviewFlags=[{code:'PROCESS_COVERAGE_PARTIAL',severity:'review',partialSamples:3},{code:'GPU_COUNTER_COVERAGE_PARTIAL',severity:'review',samplesWithPartialOwnedTree:3}];return r;};
  const result=await runResourceMatrix({output,adapter:mock});assert.equal(result.passed,true);assert.equal(result.partialMeasurement,true);assert.equal(result.error,null);
  for(const p of result.phases){assert.equal(p.partialMeasurement,true);assert.equal(p.coverage.sufficient,true);assert.equal(p.coverage.cpuCompleteTimeFraction,.96);assert.equal(p.coverage.memoryCompleteSamples,58);assert.equal(p.measurementNotes.length,2);assert.deepEqual(p.measurementNotes,p.analysis.reviewFlags);assert.equal(p.blockingReviewFlags.length,0);}
});
test('CPU time, minimum complete intervals, and memory coverage each independently block insufficient measurement',async t=>{
  for(const kind of ['cpu-time','cpu-intervals','memory']){
    const {output}=fixture(t),mock=createMock(output),collect=mock.collect;
    mock.collect=async(...args)=>{const r=await collect(...args),a=r.analysis;if(kind==='cpu-time')a.measurement.cpuCoverage.completeCpuFractionOfObserved=.79;if(kind==='cpu-intervals')a.measurement.cpuCoverage.completeCohortIntervals=9;if(kind==='memory')a.metrics.all.workingSetMiB.samples=48;return r;};
    const result=await runResourceMatrix({output,adapter:mock});assert.equal(result.passed,false,kind);assert.equal(result.error.code,'MATRIX_MEASUREMENT_COVERAGE_INSUFFICIENT');assert.equal(result.phases[0].coverage.sufficient,false);assert.equal(result.status,'restored');assert.equal(hash(mock.dump().desired),mock.initialHash);
  }
});
test('non-coverage analyzer flags still block a complete sample',async t=>{
  const {output}=fixture(t),mock=createMock(output),collect=mock.collect;
  mock.collect=async(...args)=>{const r=await collect(...args);r.analysis.reviewFlags=[{code:'ACK_OUTAGE_REVIEW',severity:'review',longestSeconds:15}];return r;};
  const result=await runResourceMatrix({output,adapter:mock});assert.equal(result.passed,false);assert.equal(result.error.code,'MATRIX_SAMPLE_REQUIRES_REVIEW');assert.equal(result.phases[0].coverage.sufficient,true);assert.equal(result.phases[0].measurementNotes.length,0);assert.equal(result.phases[0].blockingReviewFlags[0].code,'ACK_OUTAGE_REVIEW');assert.equal(result.status,'restored');
});
test('concurrent appearance changes survive failure and automatic restoration refuses to overwrite them',async t=>{
  const {output}=fixture(t),mock=createMock(output),collect=mock.collect;let changed;
  mock.collect=async(...args)=>{const r=await collect(...args);mock.changeUserAppearance();changed=mock.dump();return r;};
  const result=await runResourceMatrix({output,adapter:mock});assert.equal(result.passed,false);assert.equal(result.status,'needs-recovery');assert.equal(result.error.code,'MATRIX_CONCURRENT_APPEARANCE_CHANGE');assert.deepEqual(mock.dump(),changed);assert.equal(result.restoration.desiredHashRestored,false);
});
test('concurrent layout changes are not silently reconciled to a fresh revision',async t=>{
  const {output}=fixture(t),mock=createMock(output),collect=mock.collect;let changed;
  mock.collect=async(...args)=>{const r=await collect(...args);if(mock.collected.length===2){mock.changeUserLayout();changed=mock.dump();}return r;};
  const result=await runResourceMatrix({output,adapter:mock});assert.equal(result.status,'needs-recovery');assert.equal(result.error.code,'MATRIX_CONCURRENT_LAYOUT_CHANGE');assert.deepEqual(mock.dump(),changed);
});
test('a different host PID or session blocks subsequent mutation and restoration',async t=>{
  for(const kind of ['pid','session']){
    const {output}=fixture(t),mock=createMock(output),collect=mock.collect;let previousMutations;
    const mutations=()=>mock.actions.filter(a=>a.args[0]==='--apply-json'||a.args[1]==='update').length;
    mock.collect=async(...args)=>{const r=await collect(...args);if(kind==='pid')mock.hostPid++;else mock.hostSessionId='987654-99999999';previousMutations=mutations();return r;};
    const result=await runResourceMatrix({output,adapter:mock});assert.equal(result.status,'needs-recovery');assert.equal(result.error.code,'MATRIX_HOST_CHANGED_OR_STALE');assert.equal(mutations(),previousMutations);assert.equal(result.restoration.genuineAck,false);
  }
});
test('a failed collector without a genuine final renderer report cannot claim successful restoration',async t=>{
  const {output}=fixture(t),mock=createMock(output);
  mock.collect=async()=>{mock.rendererAck=false;throw Object.assign(Error('synthetic collector failure'),{code:'MATRIX_COLLECTOR_FAILED'});};
  mock.sleep=async()=>{throw Object.assign(Error('simulated wait timeout'),{code:'MATRIX_NATIVE_OR_ACK_TIMEOUT'});};
  const result=await runResourceMatrix({output,adapter:mock});assert.equal(result.status,'needs-recovery');assert.equal(result.restoration.genuineAck,false);assert.equal(result.passed,false);
});
test('concurrent schedule, focus, or performance policy changes do not trigger an automatic overwrite',async t=>{
  for(const policy of ['schedule','focus','performance']){
    const {output}=fixture(t),mock=createMock(output),collect=mock.collect;let previousMutations;
    const mutations=()=>mock.actions.filter(a=>a.args[0]==='--apply-json'||a.args[1]==='update').length;
    mock.collect=async(...args)=>{const r=await collect(...args);mock[policy].revision++;previousMutations=mutations();return r;};
    const result=await runResourceMatrix({output,adapter:mock});assert.equal(result.status,'needs-recovery');assert.equal(result.error.code,'MATRIX_CONCURRENT_POLICY_CHANGE');assert.equal(mutations(),previousMutations);
  }
});
test('unknown mutation response replays its persisted request instead of inventing a new intent',async t=>{
  const {output}=fixture(t),mock=createMock(output),call=mock.call;let lost=true;
  mock.call=async(args,input)=>{const result=await call(args,input);if(args[0]==='--apply-json'&&lost){lost=false;throw Object.assign(Error('lost response'),{code:'MATRIX_NATIVE_COMMAND_FAILED'});}return result;};
  const result=await runResourceMatrix({output,adapter:mock});assert.equal(result.status,'restored');assert.equal(hash(mock.dump().desired),mock.initialHash);
  const applied=mock.actions.filter(a=>a.args[0]==='--apply-json');assert.equal(applied[0].args[1],applied[1].args[1]);assert.ok(result.restoration.genuineAck);
});
test('active schedules are rejected before any mutation and no restore rewrites their state',async t=>{
  const {output}=fixture(t),mock=createMock(output);mock.schedule.policy.enabled=true;
  await assert.rejects(runResourceMatrix({output,adapter:mock}),/MATRIX_SINGLE_SHARED_IDLE_REQUIRED/);assert.equal(mock.actions.filter(a=>a.args[0]==='--apply-json'||a.args[1]==='update').length,0);
});
test('abrupt isolated process exit after commit leaves a recoverable intent; explicit recovery restores it',async t=>{
  const {root,output}=fixture(t),runner=pathToFileURL(fileURLToPath(new URL('../../desktop-host/scripts/test-resource-matrix-installed.mjs',import.meta.url))).href,contract=pathToFileURL(fileURLToPath(new URL('../src/contract.mjs',import.meta.url))).href;
  const childFile=path.join(root,'abrupt-mock.mjs');
  fs.writeFileSync(childFile,`import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';import {hash,mergePatch,initialDesired} from ${JSON.stringify(contract)};import {runResourceMatrix} from ${JSON.stringify(runner)};const createMock=${createMock.toString()};const output=process.argv[2],mock=createMock(output),call=mock.call;mock.call=async(args,input)=>{const r=await call(args,input);if(args[0]==='--apply-json'){fs.writeFileSync(path.join(output,'simulated-machine.json'),JSON.stringify(mock.dump()));process.exit(29);}return r;};await runResourceMatrix({output,adapter:mock});`);
  const child=spawnSync(process.execPath,[childFile,output],{windowsHide:true,encoding:'utf8',timeout:10000});assert.equal(child.status,29,child.stderr);
  const pending=JSON.parse(fs.readFileSync(path.join(output,'intent.json'),'utf8'));assert.equal(pending.pending.kind,'appearance');assert.equal(pending.status,'running');
  const mock=createMock(output);mock.load(JSON.parse(fs.readFileSync(path.join(output,'simulated-machine.json'),'utf8')));
  const result=await runResourceMatrix({output,recover:true,adapter:mock});assert.equal(result.recovered,true);assert.equal(result.recoveryOnly,true);assert.equal(result.passed,false);assert.equal(hash(mock.dump().desired),mock.initialHash);assert.deepEqual(mock.dump().layout.layout,mock.originalLayout);assert.equal(mock.collected.length,0);
});
test('invalid recovery intent cannot reach mutation code or publish its contents',async t=>{
  const {output}=fixture(t),mock=createMock(output);mock.collect=async()=>{throw Object.assign(Error('fail'),{code:'MATRIX_COLLECTOR_FAILED'});};
  await runResourceMatrix({output,adapter:mock});const file=path.join(output,'intent.json'),j=JSON.parse(fs.readFileSync(file,'utf8'));j.status='needs-recovery';j.known.appearance.desiredHash='c'.repeat(64);fs.writeFileSync(file,JSON.stringify(j));
  const previous=mock.actions.length,publicBefore=fs.readFileSync(path.join(output,'public-summary.json'),'utf8');
  await assert.rejects(runResourceMatrix({output,recover:true,adapter:mock}),/MATRIX_RECOVERY_INVALID/);assert.equal(mock.actions.length,previous);assert.equal(fs.readFileSync(path.join(output,'public-summary.json'),'utf8'),publicBefore);
});
test('recovery-only output never publishes editable private phase contents',async t=>{
  const {output}=fixture(t),mock=createMock(output);mock.collect=async()=>{throw Object.assign(Error('fail'),{code:'MATRIX_COLLECTOR_FAILED'});};
  await runResourceMatrix({output,adapter:mock});const file=path.join(output,'intent.json'),j=JSON.parse(fs.readFileSync(file,'utf8'));
  j.status='needs-recovery';j.phases=[{notes:'RECOVERY_PRIVATE_SECRET',pid:987654,privatePath:output}];fs.writeFileSync(file,JSON.stringify(j));
  const result=await runResourceMatrix({output,recover:true,adapter:mock}),serialized=JSON.stringify(result),saved=fs.readFileSync(path.join(output,'public-summary.json'),'utf8');
  assert.equal(result.recovered,true);assert.deepEqual(result.phases,[]);assert.equal(result.recoveredPhaseCount,1);
  for(const text of [serialized,saved])for(const secret of ['RECOVERY_PRIVATE_SECRET','987654',output])assert.ok(!text.includes(secret));
});
test('stop requests use exclusive creation and do not overwrite an existing regular request',t=>{
  const {output}=fixture(t);fs.mkdirSync(path.join(output,'single-thunder'),{recursive:true});
  const file=path.join(output,'single-thunder','stop.request');assert.equal(requestCollectorStop(output,'single-thunder'),true);assert.equal(fs.readFileSync(file,'utf8'),'stop\n');
  fs.writeFileSync(file,'KEEP_EXISTING_REQUEST');assert.equal(requestCollectorStop(output,'single-thunder'),true);assert.equal(fs.readFileSync(file,'utf8'),'KEEP_EXISTING_REQUEST');
  assert.equal(requestCollectorStop(output,'duplicate-rain'),false);assert.throws(()=>requestCollectorStop(output,'../outside'),/MATRIX_STOP_PATH_UNSAFE/);
});
test('recovery refuses a phase directory junction or symlink without touching its external destination',async t=>{
  const {root,output}=fixture(t),mock=createMock(output);mock.collect=async()=>{throw Object.assign(Error('fail'),{code:'MATRIX_COLLECTOR_FAILED'});};
  await runResourceMatrix({output,adapter:mock});const outside=path.join(root,'external-directory');fs.mkdirSync(outside);const sentinel=path.join(outside,'stop.request');fs.writeFileSync(sentinel,'EXTERNAL_SENTINEL');
  const phase=path.join(output,'single-thunder');try{fs.symlinkSync(outside,phase,process.platform==='win32'?'junction':'dir');}catch(e){if(['EPERM','EACCES','ENOTSUP'].includes(e.code)){t.skip('Directory-link creation unavailable for this test account');return;}throw e;}
  const file=path.join(output,'intent.json'),j=JSON.parse(fs.readFileSync(file,'utf8'));j.status='needs-recovery';j.activeCollector={phase:'single-thunder',pid:process.pid};fs.writeFileSync(file,JSON.stringify(j));const prior=mock.actions.length;
  const result=await runResourceMatrix({output,recover:true,adapter:mock});assert.equal(result.recovered,false);assert.equal(result.status,'needs-recovery');assert.equal(result.error.code,'MATRIX_STOP_PATH_UNSAFE');assert.equal(mock.actions.length,prior);assert.equal(fs.readFileSync(sentinel,'utf8'),'EXTERNAL_SENTINEL');
});
test('a linked stop.request is rejected rather than followed or overwritten',t=>{
  const {root,output}=fixture(t),phase=path.join(output,'single-thunder');fs.mkdirSync(phase,{recursive:true});
  const sentinel=path.join(root,'external-file');fs.writeFileSync(sentinel,'EXTERNAL_SENTINEL');
  try{fs.symlinkSync(sentinel,path.join(phase,'stop.request'),'file');}catch(e){if(['EPERM','EACCES','ENOTSUP'].includes(e.code)){t.skip('File symlink creation unavailable for this test account');return;}throw e;}
  assert.throws(()=>requestCollectorStop(output,'single-thunder'),/MATRIX_STOP_PATH_UNSAFE/);assert.equal(fs.readFileSync(sentinel,'utf8'),'EXTERNAL_SENTINEL');
});
