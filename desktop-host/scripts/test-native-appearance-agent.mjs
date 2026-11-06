// Differential tests of the isolated source candidate, never an installed app.
import test from 'node:test';import assert from 'node:assert/strict';
import fs from 'node:fs';import os from 'node:os';import path from 'node:path';
import {spawn,spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';
import {runAgent,agentError} from '../../implementation-p4_2_5/src/agent-cli.mjs';
import {ControlCore} from '../../implementation-p4_2_5/src/portable-core.mjs';
import {Store as BaseStore} from '../../implementation-p1/src/store.mjs';
import {initialDesired as v1} from '../../implementation-p1/src/state.mjs';
import {initialDesired as v2} from '../../implementation-p4_2_5/src/state-v2.mjs';
import {initialDesired as v3} from '../../implementation-p4_2_5/src/state-v3.mjs';
import {hash} from '../../implementation-p4_2_5/src/contract.mjs';
const repo=fileURLToPath(new URL('../../',import.meta.url)),exe=path.join(repo,'desktop-host/target/debug/atmos-wallpaper.exe');
const legacy=path.join(repo,'implementation-p4_2_5/src/agent-cli.mjs'),a='display-'+'a'.repeat(64),b='display-'+'b'.repeat(64);
assert.equal(process.platform,'win32');assert.ok(fs.existsSync(exe));assert.equal(fs.existsSync(path.join(path.dirname(exe),'payload')),false);
const env={...process.env,PATH:'',NODE_OPTIONS:'--invalid-node-option',NODE_PATH:'Z:/missing-runtime'};
function native(args){const r=spawnSync(exe,args,{encoding:'utf8',windowsHide:true,env,timeout:10000,maxBuffer:1048576});assert.ifError(r.error);assert.equal(r.stderr,'');return{exit:r.status,value:JSON.parse(r.stdout)};}
async function js(args){try{return{exit:0,value:await runAgent(args)};}catch(e){const v=agentError(e);return{exit:v.exitCode,value:v.output};}}
function fixture(t){const base=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-native-appearance-diff-')),root=path.join(base,'store'),file=path.join(base,'intent.json');t.after(()=>fs.rmSync(base,{recursive:true,force:true}));return{base,root,file,write:v=>fs.writeFileSync(file,typeof v==='string'?v:JSON.stringify(v))};}
const args=(f,action='--get-state',value,extra=[])=>[action,...(value?[value]:[]),'--store',f.root,...extra];
const command=(id,revision,changes={visual:{effect:'rain'}})=>({schemaVersion:1,type:'settings.update',requestId:id,expectedRevision:revision,changes});
function stable(r){const copy=structuredClone(r);if(copy.value.waitedMs!==undefined){assert.ok(Number.isInteger(copy.value.waitedMs)&&copy.value.waitedMs>=0&&copy.value.waitedMs<10000);delete copy.value.waitedMs;}return copy;}
async function equal(arguments_){assert.deepEqual(stable(native(arguments_)),stable(await js(arguments_)));}

test('native initialize, validate, apply, replay and JS-written receipts share exact state hashes',async t=>{
 const f=fixture(t);await equal(args(f));
 f.write(command('native.1',0,{visual:{clockX:0.10000000000000002,effect:'meteor'},widgets:{todos:[{id:'\ud800',text:'\udfff🙂',done:false}]}}));
 await equal(args(f,'--validate-json',f.file));
 const first=native(args(f,'--apply-json',f.file,['--wait-ms','0']));assert.equal(first.exit,0);assert.equal(first.value.committed,true);assert.equal(first.value.rendererAcknowledged,false);
 const replay=await js(args(f,'--apply-json',f.file,['--wait-ms','0']));assert.deepEqual(stable(replay),stable({...first,value:{...first.value,replayed:true}}));
 await equal(args(f));await equal(args(f,'--get-result','native.1'));
 f.write(command('js.2',1,{visual:{background:'coast'}}));const second=await js(args(f,'--apply-json',f.file,['--wait-ms','0']));assert.equal(second.exit,0);
 assert.deepEqual(stable(native(args(f,'--apply-json',f.file,['--wait-ms','0']))),stable({...second,value:{...second.value,replayed:true}}));
 await equal(args(f));const live=JSON.parse(fs.readFileSync(path.join(f.root,'live.json')));assert.equal(live.revision,2);assert.equal(live.desiredHash,hash(live.desired));
 assert.equal(live.desired.widgets.todos[0].text,'\udfff🙂');assert.equal(fs.existsSync(path.join(f.root,'observations','wallpaper.json')),false);
});

test('all existing desired formats migrate with byte-preserved source backups and matching receipts',async t=>{
 for(const initial of [v1,v2,v3]){
  const f=fixture(t),other={...f,root:path.join(f.base,'legacy-store')};new BaseStore(f.root);new BaseStore(other.root);
  const desired=initial();if(!desired.formatVersion)desired.visual.effect='tornado';
  const old={schemaVersion:1,revision:5,desired,desiredHash:hash(desired),history:[],receipts:{}};
  const raw=JSON.stringify(old,null,2)+'\n';
  for(const root of[f.root,other.root]){fs.writeFileSync(path.join(root,'live.json'),raw);fs.writeFileSync(path.join(root,'revision.json'),'{"last":5}\n');}
  assert.deepEqual(native(args(f)),await js(args(other)));
  assert.deepEqual(JSON.parse(fs.readFileSync(path.join(f.root,'live.json'))),JSON.parse(fs.readFileSync(path.join(other.root,'live.json'))));
  const backups=fs.readdirSync(path.join(f.root,'backups'));assert.equal(backups.length,1);assert.equal(fs.readFileSync(path.join(f.root,'backups',backups[0]),'utf8'),raw);
  assert.deepEqual(JSON.parse(fs.readFileSync(path.join(f.root,'compatibility-notice.json'))),JSON.parse(fs.readFileSync(path.join(other.root,'compatibility-notice.json'))));
 }
});

test('independent targets share the owner lock/material but isolate revisions and receipts',async t=>{
 const f=fixture(t);await equal(args(f));
 fs.writeFileSync(path.join(f.root,'display-layout.json'),JSON.stringify({schemaVersion:1,revision:1,lastRequestId:'layout.1',layout:{mode:'independent',targets:[a,b],askOnApply:false}}));
 await equal(args(f));assert.equal(native(args(f)).value.error.code,'TARGET_REQUIRED');
 await equal(args(f,'--get-state',undefined,['--target',a]));await equal(args(f,'--get-state',undefined,['--target',b]));
 f.write(command('target.a',1,{visual:{background:'coast'}}));assert.equal(native(args(f,'--apply-json',f.file,['--target',a,'--wait-ms','0'])).exit,0);
 await equal(args(f,'--get-state',undefined,['--target',a]));await equal(args(f,'--get-state',undefined,['--target',b]));
 assert.equal(native(args(f,'--get-state',undefined,['--target',a])).value.desired.visual.background,'coast');
 assert.equal(native(args(f,'--get-state',undefined,['--target',b])).value.desired.visual.background,'forest');
 assert.equal(JSON.parse(fs.readFileSync(path.join(f.root,'live.json'))).revision,0);
 assert.equal(native(args(f,'--apply-json',path.join(f.root,'live.json'),['--target',a,'--wait-ms','0'])).value.error.code,'INTERNAL_STORE_INPUT_DENIED');
});

test('native Agent preserves validation/conflict/uncertain outcome/lock errors without overwriting state',async t=>{
 const f=fixture(t);await equal(args(f));
 for(const c of [command('bad',0,{visual:{paused:true}}),command('bad',0,{visual:{effect:'tornado'}}),command('bad',0,{widgets:{todos:[{id:'a',text:'x',done:false},{id:'a',text:'y',done:false}]}}),{...command('bad',0),expectedRevision:0.5},{...command('bad',0),extra:'field'},command('bad',0,{visual:{unexpected:1}})]){f.write(c);await equal(args(f,'--apply-json',f.file,['--wait-ms','0']));}
 f.write(command('once',0));assert.equal(native(args(f,'--apply-json',f.file,['--wait-ms','0'])).exit,0);
 for(const c of [command('stale',0),command('once',0,{visual:{background:'coast'}})]){f.write(c);await equal(args(f,'--apply-json',f.file,['--wait-ms','0']));}
 const live=JSON.parse(fs.readFileSync(path.join(f.root,'live.json')));live.receipts['ui.once.apply'].uncertain=true;fs.writeFileSync(path.join(f.root,'live.json'),JSON.stringify(live));await equal(args(f,'--get-result','once'));
 fs.writeFileSync(path.join(f.root,'.lock'),JSON.stringify({pid:process.pid}));await equal(args(f));fs.unlinkSync(path.join(f.root,'.lock'));
 for(const text of ['{',' '.repeat(32769)]){f.write(text);await equal(args(f,'--apply-json',f.file,['--wait-ms','0']));}
 assert.equal(JSON.parse(fs.readFileSync(path.join(f.root,'live.json'))).revision,1);
});

test('synthetic observation fixtures preserve manager denial and all-target acknowledgement semantics',async t=>{
 const f=fixture(t);await equal(args(f));const live=JSON.parse(fs.readFileSync(path.join(f.root,'live.json')));
 fs.writeFileSync(path.join(f.root,'display-layout.json'),JSON.stringify({schemaVersion:1,revision:1,lastRequestId:'layout.1',layout:{mode:'duplicate',targets:[a,b],askOnApply:false}}));
 const observation={source:'local-bridge',rendererRole:'manager',stateApplied:true,revision:live.revision,desiredHash:live.desiredHash,capturedAt:new Date().toISOString(),hostSessionId:'synthetic-fixture-only'};
 const name=id=>path.join(f.root,'observations','wallpaper-'+id+'.json');fs.writeFileSync(name(a),JSON.stringify({...observation,targetId:a}));fs.writeFileSync(name(b),JSON.stringify({...observation,targetId:b}));
 await equal(args(f));assert.equal(native(args(f)).value.rendererAcknowledged,false);
 for(const id of[a,b])fs.writeFileSync(name(id),JSON.stringify({...observation,rendererRole:'wallpaper',targetId:id,capturedAt:new Date().toISOString()}));
 await equal(args(f));assert.equal(native(args(f)).value.rendererAcknowledged,true);assert.equal(native(args(f)).value.hostVisibility,'unknown');
 fs.unlinkSync(name(b));await equal(args(f));assert.equal(native(args(f)).value.rendererAcknowledged,false);
});

test('concurrent native and JS Agent transactions produce one CAS commit',async t=>{
 const f=fixture(t);await equal(args(f));
 const results=await Promise.all(Array.from({length:10},(_,i)=>new Promise((resolve,reject)=>{
  const file=path.join(f.base,'race-'+i+'.json');fs.writeFileSync(file,JSON.stringify(command('race.'+i,0)));
  const useNative=i%2===0,arguments_=args(f,'--apply-json',file,['--wait-ms','0']),child=spawn(useNative?exe:process.execPath,useNative?arguments_:[legacy,...arguments_],{windowsHide:true,env:useNative?env:process.env,stdio:['ignore','pipe','pipe']});let out='',err='';child.stdout.on('data',b=>out+=b);child.stderr.on('data',b=>err+=b);child.on('error',reject);child.on('exit',exit=>{try{assert.equal(err,'');resolve({exit,value:JSON.parse(out)});}catch(e){reject(e);}});
 })));
 assert.equal(results.filter(r=>r.exit===0).length,1);assert.ok(results.every(r=>[0,3,4].includes(r.exit)));assert.equal(native(args(f)).value.revision,1);
});

test('custom PNG bytes and relocated library ownership survive native edits; offline never falls back',async t=>{
 const f=fixture(t);await equal(args(f));
 const bytes=fs.readFileSync(path.join(repo,'prototype-v8.1-frozen/assets/nature-twilight-clean.png')),id=hash(bytes),library=path.join(f.base,'library'),uuid='12345678-1234-1234-1234-123456789abc';
 fs.mkdirSync(path.join(library,'assets'),{recursive:true});
 fs.writeFileSync(path.join(library,'.atmos-library.json'),JSON.stringify({schemaVersion:1,id:uuid,storeKey:hash(fs.realpathSync(f.root).toLowerCase())}));
 fs.writeFileSync(path.join(f.root,'library-config.json'),JSON.stringify({schemaVersion:1,revision:1,root:library,id:uuid}));
 fs.writeFileSync(path.join(library,'assets',id+'.png'),bytes);fs.writeFileSync(path.join(library,'assets',id+'.json'),JSON.stringify({assetId:id,sourceHash:id,bytes:bytes.length,width:1920,height:1080,format:'PNG'}));
 const live=JSON.parse(fs.readFileSync(path.join(f.root,'live.json')));live.desired.assetId=id;live.desiredHash=hash(live.desired);fs.writeFileSync(path.join(f.root,'live.json'),JSON.stringify(live));
 f.write(command('custom.theme',0,{visual:{theme:'warm'}}));assert.equal(native(args(f,'--apply-json',f.file,['--wait-ms','0'])).exit,0);
 await equal(args(f,'--apply-json',f.file,['--wait-ms','0']));assert.equal(native(args(f)).value.desired.assetId,id);assert.deepEqual(fs.readFileSync(path.join(library,'assets',id+'.png')),bytes);
 const offline=path.join(f.base,'offline-library');fs.renameSync(library,offline);f.write(command('offline.edit',1,{visual:{clockX:44}}));await equal(args(f,'--apply-json',f.file,['--wait-ms','0']));
 assert.equal(native(args(f,'--apply-json',f.file,['--wait-ms','0'])).exit,1);assert.equal(fs.existsSync(library),false);assert.equal(native(args(f)).value.revision,1);
});
