import {execFile} from 'node:child_process';
import path from 'node:path';
import fs from 'node:fs';
const unavailable=()=>({available:false,enabled:false,scope:'current-user',trigger:'logon'});
export function createStartupService({executable=null,platform=process.platform,run=null}={}){
 const supported=platform==='win32'&&typeof executable==='string'&&path.win32.isAbsolute(executable)&&path.win32.basename(executable).toLowerCase()==='atmos-wallpaper.exe'&&(run||fs.existsSync(executable));
 const execute=run||((file,args)=>new Promise((resolve,reject)=>execFile(file,args,{windowsHide:true,timeout:15000,maxBuffer:16384},(error,stdout)=>error?reject(error):resolve(stdout))));
 let queue=Promise.resolve();
 async function invoke(action){
  const value=JSON.parse(await execute(executable,['--autostart',action]));
  if(typeof value?.available!=='boolean'||typeof value.enabled!=='boolean'||value.scope!=='current-user'||value.trigger!=='logon')throw Error('STARTUP_RESPONSE_INVALID');
  // Native executable paths stay local; only the setting crosses the bridge.
  return {available:value.available,enabled:value.enabled,scope:value.scope,trigger:value.trigger};
 }
 return {
  async status(){return supported?invoke('status'):unavailable()},
  set(enabled){if(!supported)return Promise.reject(Error('STARTUP_UNAVAILABLE'));if(typeof enabled!=='boolean')return Promise.reject(Error('STARTUP_INVALID'));
   const result=queue.then(()=>invoke(enabled?'enable':'disable'));queue=result.catch(()=>{});return result;
  }
 };
}
