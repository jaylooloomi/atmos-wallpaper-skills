import {execFile} from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import {normalizeMonitorMetadata} from './monitor-adapter.mjs';
const id=/^(primary|display-[a-f0-9]{64})$/;
export function validHostUpdate(v){return !!v&&!Array.isArray(v)&&Object.keys(v).sort().join(',')==='expectedRevision,placement,schemaVersion'&&v.schemaVersion===1&&Number.isSafeInteger(v.expectedRevision)&&v.expectedRevision>=0&&!!v.placement&&!Array.isArray(v.placement)&&Object.keys(v.placement).sort().join(',')==='enabled,target'&&typeof v.placement.enabled==='boolean'&&typeof v.placement.target==='string'&&id.test(v.placement.target)}
export function createDisplayService({executable=null,store=null,platform=process.platform,run=null,now=Date.now}={}){
 if(store!==null&&(!path.win32.isAbsolute(store)||store.startsWith('\\\\')))throw Error('HOST_STORE_INVALID');
 const supported=platform==='win32'&&typeof executable==='string'&&path.win32.isAbsolute(executable)&&path.win32.basename(executable).toLowerCase()==='atmos-wallpaper.exe'&&(run||fs.existsSync(executable));
 const execute=run||((file,args,input)=>new Promise((resolve,reject)=>{const child=execFile(file,args,{windowsHide:true,timeout:15000,maxBuffer:131072},(error,stdout,stderr)=>{if(error){let nativeCode;try{nativeCode=JSON.parse(stdout).error?.code}catch{}const code=String(nativeCode||stderr).match(/\b(HOST_REVISION_CONFLICT|HOST_SETTINGS_BUSY|HOST_SETTINGS_INVALID|TARGET_OFFLINE_OR_AMBIGUOUS|TARGET_IDENTITY_UNRESOLVED)\b/)?.[1];reject(Object.assign(Error(code||'HOST_OPERATION_FAILED'),{code:code||'HOST_OPERATION_FAILED'}))}else resolve(stdout)});child.stdin.end(input||'')}));
 let latest=null,queue=Promise.resolve(),inflight=null;
 async function invoke(action,input){const v=JSON.parse(await execute(executable,['--host-command',action,...(store?['--store',store]:[])],input?JSON.stringify(input):''));if(v?.schemaVersion!==1||v.available!==true||v.settings?.schemaVersion!==1||!Array.isArray(v.displays)||v.displays.length>64||!validHostUpdate({schemaVersion:1,expectedRevision:v.settings?.revision,placement:v.settings?.placement}))throw Error('HOST_RESPONSE_INVALID');
  for(const d of v.displays)if(typeof d.id!=='string'||!/^display-[a-f0-9]{64}$/.test(d.id)||typeof d.name!=='string'||typeof d.deviceName!=='string'||typeof d.primary!=='boolean'||typeof d.selectable!=='boolean'||![d.bounds?.x,d.bounds?.y,d.bounds?.width,d.bounds?.height].every(Number.isInteger)||d.bounds.width<=0||d.bounds.height<=0)throw Error('HOST_RESPONSE_INVALID');
  const runtime=v.runtime&&Number.isFinite(v.runtime.capturedAtMs)&&Math.abs(now()-v.runtime.capturedAtMs)<5000?v.runtime:null;
  latest={...v,runtime,runtimeFresh:!!runtime};return latest;
 }
 return {async status(){if(!supported)return {available:false,displays:[],settings:null,runtime:null,runtimeFresh:false};if(!inflight)inflight=invoke('get').catch(e=>{latest=null;throw e}).finally(()=>{inflight=null});return inflight},
  update(v){if(!supported)return Promise.reject(Object.assign(Error('HOST_UNAVAILABLE'),{code:'HOST_UNAVAILABLE'}));if(!validHostUpdate(v))return Promise.reject(Object.assign(Error('HOST_SETTINGS_INVALID'),{code:'HOST_SETTINGS_INVALID'}));const result=queue.then(()=>invoke('update',v));queue=result.catch(()=>{});return result},
  monitor(){if(!latest)return normalizeMonitorMetadata({schemaVersion:1,monitors:[]},{source:'unavailable'});const target=latest.settings.placement.target==='primary'?latest.displays.find(d=>d.primary):latest.displays.find(d=>d.id===latest.settings.placement.target&&d.selectable);return normalizeMonitorMetadata({schemaVersion:1,monitors:latest.displays.map(d=>({id:d.deviceName,primary:d.primary,width:d.bounds.width,height:d.bounds.height,scalePercent:d.scalePercent}))},{targetId:target?.deviceName||'unresolved',source:run?'fixture':'windows-native-readonly',capturedAt:new Date(now()).toISOString()})}
 };
}
