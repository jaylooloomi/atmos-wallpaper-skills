// Differential source-candidate checks only. Never uses the installed executable,
// default store, user browser, real providers, renderer ACKs or desktop startup.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {spawn,spawnSync} from 'node:child_process';
import {once} from 'node:events';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {runAgent,agentError} from '../../implementation-p4_2_5/src/agent-cli.mjs';
import {runAppearance} from '../../implementation-p4_2_5/src/appearance-cli.mjs';
import {ControlCore} from '../../implementation-p4_2_5/src/portable-core.mjs';

const repo=fileURLToPath(new URL('../../',import.meta.url));
const exe=process.env.ATMOS_NATIVE_CANDIDATE||path.join(repo,'desktop-host/target/debug/atmos-wallpaper.exe');
const legacy=path.join(repo,'implementation-p4_2_5/src/appearance-cli.mjs');
assert.equal(process.platform,'win32','Windows native candidate required');
assert.ok(fs.existsSync(exe),'Build the isolated source candidate first');
assert.equal(fs.existsSync(path.join(path.dirname(exe),'payload')),false,'Candidate must have no bundled runtime/payload for this check');
const nativeEnv={...process.env,PATH:'',NODE_OPTIONS:'--invalid-node-option-should-never-be-read',NODE_PATH:'Z:/missing-runtime'};
const hash=s=>createHash('sha256').update(s).digest('hex');
const intent=(id,revision,city='台北',enabled=true)=>({schemaVersion:1,requestId:id,expectedRevision:revision,weather:{enabled,city}});
function fixture(t){const base=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-native-provider-diff-')),root=path.join(base,'store');t.after(()=>fs.rmSync(base,{recursive:true,force:true}));return{base,root};}
function native(args,body){
 const result=spawnSync(exe,args,{input:typeof body==='string'||Buffer.isBuffer(body)?body:body===undefined?undefined:JSON.stringify(body),encoding:'utf8',windowsHide:true,env:nativeEnv,timeout:10000,maxBuffer:1048576});
 assert.ifError(result.error);assert.equal(result.stderr,'');return{exit:result.status,value:JSON.parse(result.stdout)};
}
const args=(root,action='get')=>['--providers-command',action,'--store',root];
async function js(arguments_,input){
 try{return{exit:0,value:await runAppearance(arguments_,{input})};}
 catch(e){const code=e.code||'LOCAL_OPERATION_FAILED';const exit=code==='STORE_LOCKED'?4:/CONFLICT/.test(code)?3:/INVALID|REQUIRED|INACTIVE|SHARED|BODY_TOO_LARGE/.test(code)?2:1;return{exit,value:{apiVersion:1,ok:false,error:{code,retryable:code==='STORE_LOCKED'},committed:null}};}
}

test('embedded catalogs equal JS contracts without Node/payload/store',async()=>{
 for(const command of ['--agent-help','--agent-schema','--host-schema','--appearance-help']){
  const expected=command==='--appearance-help'?await runAppearance([command]):await runAgent([command]);
  assert.deepEqual(native([command]),{exit:0,value:expected});
  if(command!=='--appearance-help')assert.deepEqual(native([command,'--store','/unused-root']),{exit:0,value:expected});
  const invalid=[command,'--exec','anything'];
  const expectedError=command==='--appearance-help'?await js(invalid):agentError(Object.assign(Error(),{code:'ARGUMENT_INVALID'}));
  assert.deepEqual(native(invalid),command==='--appearance-help'?expectedError:{exit:expectedError.exitCode,value:expectedError.output});
 }
});

test('native and legacy writers alternate and replay each other without changing appearance/backups',async t=>{
 const{root}=fixture(t);const core=new ControlCore(root);core.init();
 fs.writeFileSync(path.join(root,'backups','keep.json'),'{"privateFixture":"unchanged"}\n');
 const live=fs.readFileSync(path.join(root,'live.json')),backup=fs.readFileSync(path.join(root,'backups','keep.json'));
 assert.deepEqual(native(args(root)),await js(args(root)));
 let revision=0;
 for(const [i,city] of ['台北','東京','倫敦','紐約'].entries()){
  const command=intent('native.'+i,revision++,city,i%2===0);
  const first=native(args(root,'update'),command);assert.equal(first.exit,0);
  assert.deepEqual(await js(args(root,'update'),command),{exit:0,value:{...first.value,result:{...first.value.result,replayed:true}}});
 }
 assert.deepEqual(fs.readFileSync(path.join(root,'live.json')),live);
 assert.deepEqual(fs.readFileSync(path.join(root,'backups','keep.json')),backup);
});

test('JS receipt fingerprints, numeric spellings and native pending recovery are compatible',async t=>{
 const{root}=fixture(t);await js(args(root));
 let revision=0;
 for(const city of ['台北','東京','倫敦','紐約']){
  const body=intent('js.'+revision,revision,city);
  const first=await js(args(root,'update'),body);assert.equal(first.exit,0);
  const replay=native(args(root,'update'),JSON.stringify(body).replace('"schemaVersion":1','"schemaVersion":1.0').replace('"expectedRevision":'+revision,'"expectedRevision":'+revision+'.0'));
  assert.deepEqual(replay,{exit:0,value:{...first.value,result:{...first.value.result,replayed:true}}});
  const name=path.join(root,'requests','provider.'+hash(body.requestId)+'.json');
  const receipt=JSON.parse(fs.readFileSync(name));fs.writeFileSync(name,JSON.stringify({...receipt,pending:true}));
  assert.deepEqual(native(args(root,'update'),body),replay);
  assert.equal(JSON.parse(fs.readFileSync(name)).pending,undefined);
  revision++;
 }
 assert.equal(native(args(root)).value.settings.revision,revision);
 const next=intent('before-pointer',revision);
 // Same exact pending receipt before the policy pointer was written: retry commits once.
 const fp=hash(JSON.stringify({expectedRevision:revision,requestId:next.requestId,schemaVersion:1,weather:{city:'台北',enabled:true}}));
 fs.writeFileSync(path.join(root,'requests','provider.'+hash(next.requestId)+'.json'),JSON.stringify({fingerprint:fp,pending:true,result:{committed:true,revision:revision+1}}));
 assert.equal(native(args(root,'update'),next).value.result.revision,revision+1);
 assert.equal((await js(args(root,'update'),next)).value.result.replayed,true);
});

test('native policy retains legacy validation, conflicts, owner lock and path denial',async t=>{
 const{root,base}=fixture(t);await js(args(root));
 const first=intent('once',0);assert.equal(native(args(root,'update'),first).exit,0);
 for(const body of [intent('stale',0),intent('once',0,'東京'),{...intent('invalid',1),unexpected:true},intent('invalid',1,'invalid'),{...intent('invalid',1),expectedRevision:0.5}]){
  assert.deepEqual(native(args(root,'update'),body),await js(args(root,'update'),body));
 }
 fs.writeFileSync(path.join(root,'.lock'),JSON.stringify({pid:process.pid}));
 assert.deepEqual(native(args(root,'update'),intent('busy',1)),await js(args(root,'update'),intent('busy',1)));
 fs.unlinkSync(path.join(root,'.lock'));
 const child=spawn(process.execPath,['-e','process.exit(0)'],{windowsHide:true});const dead=child.pid;await once(child,'exit');
 fs.writeFileSync(path.join(root,'.lock'),JSON.stringify({pid:dead}));
 assert.equal(native(args(root,'update'),intent('after-crash',1)).exit,0);assert.equal(fs.existsSync(path.join(root,'.lock')),false);
 for(const body of ['{',Buffer.from([0xff]),' '.repeat(16385)]){
  const r=native(args(root,'update'),body);assert.equal(r.exit,2);assert.equal(r.value.error.code,body.length>16384?'BODY_TOO_LARGE':'INVALID_JSON');
 }
 const link=path.join(base,'linked');fs.symlinkSync(root,link,'junction');
 assert.equal(native(args(link)).value.error.code,'PATH_REJECTED');
 for(const extra of [['--target','shared'],['--store',root],['--exec','cmd']])assert.equal(native([...args(root),...extra]).value.error.code,'ARGUMENT_INVALID');
 assert.equal(native(['--providers-command','get','--store','/rooted']).value.error.code,'PATH_REJECTED');
 assert.equal(native(args(root)).value.settings.revision,2);
});

test('simultaneous JS and native writers share one CAS lock',async t=>{
 const{root}=fixture(t);await js(args(root));
 const calls=Array.from({length:12},(_,i)=>new Promise((resolve,reject)=>{
  const useNative=i%2===0,child=spawn(useNative?exe:process.execPath,useNative?args(root,'update'):[legacy,...args(root,'update')],{windowsHide:true,env:useNative?nativeEnv:process.env,stdio:['pipe','pipe','pipe']});
  let stdout='',stderr='';child.stdout.on('data',b=>stdout+=b);child.stderr.on('data',b=>stderr+=b);child.on('error',reject);child.on('exit',exit=>{try{assert.equal(stderr,'');resolve({exit,value:JSON.parse(stdout)});}catch(e){reject(e);}});child.stdin.end(JSON.stringify(intent('race.'+i,0)));
 }));
 const results=await Promise.all(calls);
 assert.equal(results.filter(r=>r.exit===0).length,1);
 assert.ok(results.every(r=>[0,3,4].includes(r.exit)));
 assert.equal(native(args(root)).value.settings.revision,1);
 assert.equal(fs.existsSync(path.join(root,'.lock')),false);
});
