// Native candidate vs legacy focus policy. Synthetic host-status fixture only:
// it authorizes policy tests, never renderer ACK/native attachment/visibility claims.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {runAppearance} from '../../implementation-p4_2_5/src/appearance-cli.mjs';
import {ControlCore} from '../../implementation-p4_2_5/src/portable-core.mjs';
const repo=fileURLToPath(new URL('../../',import.meta.url)),exe=path.join(repo,'desktop-host/target/debug/atmos-wallpaper.exe');
assert.equal(process.platform,'win32');assert.ok(fs.existsSync(exe));assert.equal(fs.existsSync(path.join(path.dirname(exe),'payload')),false);
const environment={...process.env,PATH:'',NODE_OPTIONS:'--invalid-node-option',NODE_PATH:'Z:/missing-runtime'};
const args=(root,action='get')=>['--focus-command',action,'--store',root];
const input=(id,revision,mode='focus')=>({schemaVersion:1,requestId:id,expectedRevision:revision,mode});
function fixture(t){const root=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-native-focus-diff-'));t.after(()=>fs.rmSync(root,{recursive:true,force:true}));const core=new ControlCore(root);core.init();const session='123-456';const host=(extra={})=>fs.writeFileSync(path.join(root,'host-status.json'),JSON.stringify({schemaVersion:1,pid:process.pid,capturedAtMs:Date.now(),hostSessionId:session,...extra}));host();return{root,core,session,host};}
function native(arguments_,body){const r=spawnSync(exe,arguments_,{env:environment,input:body===undefined?undefined:JSON.stringify(body),encoding:'utf8',windowsHide:true,timeout:10000});assert.ifError(r.error);assert.equal(r.stderr,'');return{exit:r.status,value:JSON.parse(r.stdout)};}
async function js(arguments_,body){try{return{exit:0,value:await runAppearance(arguments_,{input:body})};}catch(e){const code=e.code||'LOCAL_OPERATION_FAILED';return{exit:code==='STORE_LOCKED'?4:/CONFLICT/.test(code)?3:/INVALID|REQUIRED|INACTIVE|SHARED|BODY_TOO_LARGE/.test(code)?2:1,value:{apiVersion:1,ok:false,error:{code,retryable:code==='STORE_LOCKED'},committed:null}};}}

test('native focus CLI and JS share receipts, conflicts and owner state without changing appearance',async t=>{
 const f=fixture(t),before=fs.readFileSync(path.join(f.root,'live.json'));
 assert.deepEqual(native(args(f.root)),await js(args(f.root)));
 const first=native(args(f.root,'update'),input('native.1',0));assert.equal(first.exit,0);
 assert.deepEqual(await js(args(f.root,'update'),input('native.1',0)),{exit:0,value:{...first.value,result:{...first.value.result,replayed:true}}});
 const second=await js(args(f.root,'update'),input('js.2',1,'showcase'));assert.equal(second.exit,0);
 assert.deepEqual(native(args(f.root,'update'),input('js.2',1,'showcase')),{exit:0,value:{...second.value,result:{...second.value.result,replayed:true}}});
 for(const body of [input('stale',0),input('native.1',0,'normal'),input('constructor',2),{...input('bad',2),target:'shared'},{...input('bad',2),mode:'exec'}])assert.deepEqual(native(args(f.root,'update'),body),await js(args(f.root,'update'),body));
 assert.deepEqual(native(args(f.root)),await js(args(f.root)));
 assert.deepEqual(fs.readFileSync(path.join(f.root,'live.json')),before);
 assert.equal(fs.existsSync(path.join(f.root,'observations','wallpaper.json')),false);
});

test('host restart resets focus and receipts, with no retired presentation policy mutation',async t=>{
 const f=fixture(t);assert.equal(native(args(f.root,'update'),input('before-restart',0)).exit,0);
 const isolated=path.join(f.root,'focus-screensaver-runtime.json');fs.writeFileSync(isolated,'{"fixture":"must remain unchanged"}\n');
 const before=fs.readFileSync(isolated);f.host({hostSessionId:'234-567'});
 assert.deepEqual(native(args(f.root)),await js(args(f.root)));assert.equal(native(args(f.root)).value.mode,'normal');
 assert.equal(native(args(f.root,'update'),input('before-restart',0,'showcase')).exit,0);
 assert.deepEqual(native(args(f.root)),await js(args(f.root)));assert.deepEqual(fs.readFileSync(isolated),before);
});

test('stale, stopped, dead or malformed host evidence never enables focus CLI',async t=>{
 const f=fixture(t);
 for(const extra of [{capturedAtMs:Date.now()-20000},{capturedAtMs:Date.now()+20000},{state:'stopped'},{pid:0},{pid:4294967295},{hostSessionId:'not-a-host-session'}]){
  f.host(extra);assert.deepEqual(native(args(f.root)),await js(args(f.root)));assert.equal(native(args(f.root)).value.error.code,'FOCUS_HOST_UNAVAILABLE');
 }
 f.host();fs.writeFileSync(path.join(f.root,'.lock'),JSON.stringify({pid:process.pid}));
 assert.deepEqual(native(args(f.root,'update'),input('locked',0)),await js(args(f.root,'update'),input('locked',0)));fs.unlinkSync(path.join(f.root,'.lock'));
 assert.deepEqual(native([...args(f.root),'--target','shared']),await js([...args(f.root),'--target','shared']));
});
