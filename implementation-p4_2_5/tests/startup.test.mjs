import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createStartupService} from '../src/startup-service.mjs';
import {createBridge} from '../src/router.mjs';
import {ControlCore} from '../src/portable-core.mjs';
const enabled=value=>({available:true,enabled:value,scope:'current-user',trigger:'logon'});
test('standalone bridge has no native startup capability and executes nothing',async()=>{
 const service=createStartupService({executable:null,run:()=>{throw Error('must not execute')}});assert.equal((await service.status()).available,false);await assert.rejects(service.set(true));
});
test('startup service invokes only fixed native arguments and hides executable paths',async()=>{
 const calls=[];const service=createStartupService({platform:'win32',executable:'C:\\Program Files\\Atmos\\atmos-wallpaper.exe',run:async(file,args)=>{calls.push([file,args]);return JSON.stringify({...enabled(args[1]!=='disable'),command:'C:\\PRIVATE\\host.exe'})}});
 assert.equal((await service.status()).enabled,true);assert.equal((await service.set(false)).enabled,false);assert.deepEqual(calls.map(c=>c[1]),[['--autostart','status'],['--autostart','disable']]);assert.ok(!JSON.stringify(await service.status()).includes('PRIVATE'));await assert.rejects(service.set('enable'));
});
test('startup writes serialize across clients and a failed operation does not poison retries',async()=>{
 const calls=[];let release;const service=createStartupService({platform:'win32',executable:'C:\\Atmos\\atmos-wallpaper.exe',run:async(file,args)=>{calls.push(args[1]);if(calls.length===1)await new Promise(r=>release=r);return JSON.stringify(enabled(args[1]==='enable'))}});
 const a=service.set(true),b=service.set(false);await new Promise(r=>setImmediate(r));assert.deepEqual(calls,['enable']);release();assert.equal((await a).enabled,true);assert.equal((await b).enabled,false);
 let count=0;const retry=createStartupService({platform:'win32',executable:'C:\\Atmos\\atmos-wallpaper.exe',run:async()=>{if(++count===1)throw Error('denied');return JSON.stringify(enabled(false))}});await assert.rejects(retry.set(true));assert.equal((await retry.set(false)).enabled,false);
});
function setup(t,service){const dir=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-startup-'));t.after(()=>fs.rmSync(dir,{recursive:true,force:true}));new ControlCore(dir).init();const root=fileURLToPath(new URL('../',import.meta.url)),origin='http://127.0.0.1:4173';const bridge=createBridge({store:dir,staticRoot:path.join(root,'dist'),prototypeRoot:path.resolve(root,'../prototype-v8.1-frozen'),origin,startupService:service});
 const call=(body,role='manager',extra={},method=body===undefined?'GET':'POST')=>{const headers={Host:'127.0.0.1:4173',Origin:origin,'Sec-Fetch-Site':'same-origin','Content-Type':'application/json','X-Atmos-Session':role==='manager'?bridge.sessionToken:bridge.wallpaperSessionToken,...extra};for(const k of Object.keys(headers))if(headers[k]===null)delete headers[k];return bridge.handle(new Request(origin+'/api/host/startup',{method,headers,...(body!==undefined?{body:JSON.stringify(body)}:{})}));};return {call,core:new ControlCore(dir)};
}
test('startup API permits manager only and never mutates wallpaper desired state',async t=>{
 let value=false;const {call,core}=setup(t,{status:async()=>enabled(value),set:async v=>enabled(value=v)});assert.equal((await call(undefined,'wallpaper')).status,403);assert.equal((await call({enabled:true},'wallpaper')).status,403);assert.equal((await(await call()).json()).enabled,false);assert.equal((await(await call({enabled:true})).json()).enabled,true);assert.equal(core.status().revision,0);
});
test('startup OS writes reject cross-origin, missing sessions and extra command/path fields',async t=>{
 let writes=0;const {call}=setup(t,{status:async()=>enabled(false),set:async()=>{writes++;return enabled(true)}});
 for(const [body,headers] of [[{enabled:true},{Origin:'https://attacker.test'}],[{enabled:true},{Origin:null}],[{enabled:true},{'X-Atmos-Session':'wrong'}],[{enabled:true},{'Sec-Fetch-Site':'cross-site'}],[{enabled:true},{'Content-Type':'text/plain'}],[{enabled:'true'},{}],[{enabled:true,path:'C:/evil.exe'},{}],[{enabled:true,command:'enable'},{}]])assert.ok((await call(body,'manager',headers)).status>=400);
 assert.equal((await call(undefined,'manager',{},'DELETE')).status,405);assert.equal(writes,0);
});
test('missing/failed native integration reports unavailable or failure, not a saved setting',async t=>{
 const a=setup(t,null);assert.equal((await(await a.call()).json()).available,false);assert.equal((await a.call({enabled:true})).status,503);
 const b=setup(t,{status:async()=>{throw Error('private native error')},set:async()=>{throw Error('private native error')}});const response=await b.call({enabled:true});assert.equal(response.status,503);assert.ok(!(await response.text()).includes('private'));
});
