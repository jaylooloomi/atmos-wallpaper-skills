import fs from 'node:fs';
import path from 'node:path';
import {Worker} from 'node:worker_threads';
import {spawnSync} from 'node:child_process';
import {fetchCalendar,subscriptionURL} from './calendar-fetch.mjs';
const fail=code=>{throw Object.assign(new Error(code),{code})};
export function parseIsolated(text,options){return new Promise((resolve,reject)=>{
 const worker=new Worker(new URL('./calendar-worker.mjs',import.meta.url),{workerData:{text,options},execArgv:[],resourceLimits:{maxOldGenerationSizeMb:96}});
 const timer=setTimeout(()=>{worker.terminate();reject(Object.assign(Error('ICS_TOO_COMPLEX'),{code:'ICS_TOO_COMPLEX'}))},3000);
 worker.once('message',message=>{clearTimeout(timer);worker.terminate();message.ok?resolve(message.result):reject(Object.assign(Error(message.code),{code:message.code}))});
 worker.once('error',()=>{clearTimeout(timer);reject(Object.assign(Error('ICS_INVALID'),{code:'ICS_INVALID'}))});
 worker.once('exit',code=>{clearTimeout(timer);if(code!==0)reject(Object.assign(Error('ICS_INVALID'),{code:'ICS_INVALID'}))});
})}
function protect(bytes,decrypt=false){
 if(process.platform!=='win32')return bytes;
 const action=decrypt?'Unprotect':'Protect';
 const script=`Add-Type -AssemblyName System.Security; $b=[Convert]::FromBase64String([Console]::In.ReadToEnd()); $r=[Security.Cryptography.ProtectedData]::${action}($b,$null,[Security.Cryptography.DataProtectionScope]::CurrentUser); [Console]::Write([Convert]::ToBase64String($r))`;
 const result=spawnSync(path.join(process.env.SystemRoot,'System32/WindowsPowerShell/v1.0/powershell.exe'),['-NoProfile','-NonInteractive','-EncodedCommand',Buffer.from(script,'utf16le').toString('base64')],{input:bytes.toString('base64'),encoding:'utf8',windowsHide:true,timeout:8000,maxBuffer:4*1024*1024});
 if(result.status!==0||result.error)fail('ICS_STORAGE_FAILED');return Buffer.from(result.stdout.trim(),'base64');
}
export function createCalendarService({store,now=Date.now,fetcher=fetchCalendar,parser=parseIsolated}={}){
 const dir=store.path('calendar');fs.mkdirSync(dir,{recursive:true,mode:0o700});
 const file=store.path('calendar/connection.bin');let source=null,lastError=null,cached=null,parsedAt=0,busy=false,refreshing=null;
 try{if(fs.existsSync(file)){const bytes=fs.readFileSync(file);if(bytes.length>3*1048576)fail('ICS_STORAGE_FAILED');source=JSON.parse(protect(bytes,true).toString('utf8'))}}catch{lastError='ICS_STORAGE_FAILED'}
 const save=value=>store.atomic('calendar/connection.bin',protect(Buffer.from(JSON.stringify(value))));
 const safeError=e=>/^ICS_[A-Z_]+$/.test(e.code||e.message)?e.code||e.message:'ICS_FETCH_FAILED';
 async function change(type,body){
  if(busy||refreshing)fail('ICS_BUSY');busy=true;
  try{
   if(type==='disconnect'){save(null);source=null;cached=null;lastError=null;return status()}
   const input=type==='refresh'?source:body;
   if(!input)fail('ICS_NOT_CONFIGURED');
   const url=type==='subscribe'?subscriptionURL(body.url).href:input.url;
   const text=url?await fetcher(url):input.text;
   if(typeof text!=='string'||Buffer.byteLength(text)>1048576)fail('ICS_TOO_LARGE');
   const result=await parser(text,{now:now()});
   const value={type:url?'subscription':'file',name:String(input.name||'我的日曆').slice(0,80),url:url||null,text,updatedAt:new Date(now()).toISOString()};
   save(value);source=value;cached=result;parsedAt=now();lastError=null;return status();
  }catch(e){lastError=safeError(e);throw Object.assign(Error(lastError),{code:lastError})}finally{busy=false}
 }
 function status(){return {configured:!!source,type:source?.type||null,name:source?.name||null,updatedAt:source?.updatedAt||null,lastError,events:cached?.events||[],truncated:cached?.truncated||false,zone:cached?.zone||null}}
 async function events(){
  if(source&&!busy&&!refreshing&&(parsedAt===0||now()-parsedAt>60000)){
   refreshing=(async()=>{try{
    if(source.url&&now()-Date.parse(source.updatedAt)>=15*60000){refreshing=null;await change('refresh',source);return}
    cached=await parser(source.text,{now:now()});parsedAt=now();
   }catch(e){lastError=safeError(e);parsedAt=now()}finally{refreshing=null}})();
   await refreshing;
  }
  if(refreshing&&!cached)await refreshing;
  return status();
 }
 return {change,events,status};
}
