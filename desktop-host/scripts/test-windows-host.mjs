// Real native integration test. Applies a wallpaper to the primary monitor.
// Uses CLI, process ownership and native metadata only; no browser/UI driver.
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawn,spawnSync} from 'node:child_process';
import assert from 'node:assert/strict';
import net from 'node:net';
import http from 'node:http';

if(process.platform!=='win32'||!process.argv.includes('--apply-to-primary'))throw Error('Windows and explicit --apply-to-primary required');
const root=fileURLToPath(new URL('../',import.meta.url)),repo=path.dirname(root);
const exe=path.join(root,'target/debug/atmos-desktop-prototype.exe');
const python=process.env.ATMOS_PYTHON||'C:\\Python314\\python.exe';
const privateRoot=path.join(process.env.LOCALAPPDATA,'AtmosWallpaper/native-prototype-validation',new Date().toISOString().replaceAll(':','-'));
fs.mkdirSync(privateRoot,{recursive:true});
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
const inspect=()=>{const p=spawnSync(exe,['--inspect-desktop'],{encoding:'utf8',windowsHide:true});assert.equal(p.status,0,p.stderr);return JSON.parse(p.stdout)};
const alive=pid=>{try{process.kill(pid,0);return true}catch{return false}};
function profileProcesses(folder){
 const p=spawnSync(path.join(process.env.SystemRoot,'System32/WindowsPowerShell/v1.0/powershell.exe'),['-NoProfile','-Command',"$ErrorActionPreference='Stop'; @(Get-CimInstance Win32_Process -Filter \"Name='msedgewebview2.exe'\" | Where-Object { $_.CommandLine -and $_.CommandLine.Contains($env:ATMOS_TEST_PROFILE) } | Select-Object -ExpandProperty ProcessId) | ConvertTo-Json -Compress"],{encoding:'utf8',windowsHide:true,env:{...process.env,ATMOS_TEST_PROFILE:folder}});
 assert.equal(p.status,0,p.stderr);const result=p.stdout.trim()?JSON.parse(p.stdout):[];return Array.isArray(result)?result:[result];
}
async function until(fn,label,ms=45000){const start=Date.now();while(Date.now()-start<ms){const result=fn();if(result)return result;await sleep(200)}throw Error('Timed out: '+label)}
const read=file=>{try{return JSON.parse(fs.readFileSync(file))}catch{return null}};
const event=(file,kind)=>read(file)?.events.find(x=>x.kind===kind);
const args=(folder,seconds=90)=>['--runtime-root',path.join(repo,'implementation-p4_2_5'),'--node',process.execPath,'--python',python,'--store',path.join(folder,'store'),'--status',path.join(folder,'status.json'),'--run-seconds',String(seconds)];
const baseline=inspect();
const originalHandles=baseline.windows.filter(w=>w.visible).map(w=>w.handle);
const checks=[];
let active=null;
// Optional test-only baseline observer. Never controls another wallpaper app,
// writes settings or sends renderer acknowledgements.
const baselinePort=process.env.ATMOS_BASELINE_PORT?Number(process.env.ATMOS_BASELINE_PORT):null;
assert.ok(baselinePort===null||(Number.isInteger(baselinePort)&&baselinePort>0&&baselinePort<65536),'Invalid baseline port');
const get=(port,url,headers={})=>new Promise((resolve,reject)=>{const req=http.get({host:'127.0.0.1',port,path:url,headers},res=>{let body='';res.on('data',b=>body+=b);res.on('end',()=>resolve({status:res.statusCode,body}))});req.on('error',reject);req.setTimeout(3000,()=>req.destroy(Error('Baseline read timeout')))});
async function baselineHealth(){
 if(baselinePort===null)return null;
 const page=await get(baselinePort,'/manager',{'Sec-Fetch-Mode':'navigate','Sec-Fetch-Site':'none','Sec-Fetch-Dest':'document'});
 assert.equal(page.status,200);const token=page.body.match(/window\.atmosBridgeSession="([a-f0-9]+)"/)?.[1];assert.ok(token);
 const r=await get(baselinePort,'/api/health',{'Sec-Fetch-Site':'same-origin','X-Atmos-Session':token});assert.equal(r.status,200);
 return JSON.parse(r.body).renderers.some(r=>r.role==='wallpaper'&&r.recent===true);
}
async function portClosed(port){
 const started=Date.now();
 while(Date.now()-started<10000){
  const open=await new Promise((resolve,reject)=>{const s=net.connect({host:'127.0.0.1',port});s.once('connect',()=>{s.destroy();resolve(true)});s.once('error',()=>resolve(false));s.setTimeout(1000,()=>{s.destroy();reject(Error('Port check timeout'))})});
  if(!open)return;
  await sleep(200); // Windows job termination/socket teardown is asynchronous.
 }
 throw Error('Owned runtime port remained open for 10 seconds');
}
async function runCase(name,mode){
 if(baselinePort!==null)assert.equal(await baselineHealth(),true,'Original wallpaper must have a genuine recent ack before testing restoration');
 const folder=path.join(privateRoot,name);fs.mkdirSync(folder,{recursive:true});const status=path.join(folder,'status.json');
 const before=inspect();
 const child=spawn(exe,args(folder,mode==='timer'?15:90),{windowsHide:true,stdio:['pipe','pipe','pipe']});active=child;
 let stdout='',stderr='';child.stdout.on('data',x=>stdout+=x);child.stderr.on('data',x=>stderr+=x);
 const exited=new Promise(resolve=>child.once('exit',(code,signal)=>resolve({code,signal})));
 let runtime,attached;
 try{
  await until(()=>event(status,'renderer-ready'),'real renderer ack');
  runtime=event(status,'runtime-ready').detail;attached=event(status,'attached').detail;
  const current=inspect().windows.find(w=>w.handle===attached.window.handle);
  assert.ok(current?.visible,'Native window must be visible');assert.equal(current.parent,attached.parent.handle);assert.deepEqual(current.bounds,baseline.primary);assert.equal(attached.parent.class,'WorkerW');
  const webviewPids=profileProcesses(folder);assert.ok(webviewPids.length>0,'Real WebView2 processes expected');
  if(mode==='normal'){
   const duplicate=spawnSync(exe,args(path.join(folder,'duplicate')),{encoding:'utf8',windowsHide:true});
   assert.equal(duplicate.status,1);assert.match(duplicate.stderr,/already running/);
   await sleep(2500); // Observe ongoing real page updates, not only boot.
   const obs=read(path.join(folder,'store/observations/latest.json'));
   assert.equal(obs.rendererRole,'wallpaper');assert.equal(obs.stateApplied,true);assert.ok(Date.now()-Date.parse(obs.capturedAt)<5000);
   child.stdin.write('exit\n');
  } else if(mode==='crash')child.kill();
  else if(mode==='runtime-failure')process.kill(runtime.pid);
  const result=await Promise.race([exited,sleep(20000).then(()=>{throw Error('Host did not exit')})]);
  if(mode!=='crash'){
   assert.equal(result.code,mode==='runtime-failure'?2:0,stderr);
   const restored=event(status,'restored');assert.ok(restored,'Restoration journal required');
   assert.equal(restored.detail.ownWindowRemoved,true);assert.equal(restored.detail.staticWallpaperUnchanged,true);
   if(mode==='runtime-failure')assert.ok(event(status,'runtime-failed'));
  }
  await until(()=>!alive(runtime.pid),'owned backend cleanup',10000);await portClosed(runtime.port);
  await until(()=>webviewPids.every(pid=>!alive(pid)),'owned WebView2 cleanup',10000);
  const after=inspect();assert.ok(!after.windows.some(w=>w.handle===attached.window.handle),'Owned wallpaper window must disappear');
  for(const old of originalHandles)assert.ok(after.windows.some(w=>w.handle===old),'An original visible desktop window disappeared');
  if(baselinePort!==null){let restored=false;for(let i=0;i<25;i++){if(await baselineHealth()){restored=true;break}await sleep(200)}assert.ok(restored,'Original wallpaper stopped acknowledging after native exit');}
  checks.push({name,passed:true,nativeAttachment:true,genuineRendererAck:true,ownWindowRemoved:true,ownedRuntimeStopped:true,ownedWebViewProcessesStopped:true,ownedPortClosed:true,originalVisibleDesktopWindowsPreserved:true,...(baselinePort!==null?{originalWallpaperAckBeforeAndAfter:true}:{}),...(mode==='normal'?{duplicateRejected:true,ongoingRendererAck:true}:{}),...(mode==='crash'?{restorationEvidence:'OS removed owned window; original desktop windows retained; no graceful journal on forced termination'}:{staticWallpaperUnchanged:true}),exitCode:result.code});
  console.log(JSON.stringify(checks.at(-1)));
 }finally{
  if(alive(child.pid))child.kill();
  fs.writeFileSync(path.join(folder,'stdout.log'),stdout);fs.writeFileSync(path.join(folder,'stderr.log'),stderr);active=null;
 }
}
try{
 await runCase('normal-exit-and-singleton','normal');
 await runCase('bounded-timer-exit','timer');
 await runCase('backend-failure-restores','runtime-failure');
 await runCase('forced-host-termination','crash');
 const report={schemaVersion:1,date:new Intl.DateTimeFormat('en-CA',{timeZone:'Asia/Taipei',year:'numeric',month:'2-digit',day:'2-digit'}).format(new Date()),platform:process.platform,architecture:process.arch,tauri:'2.12.5',target:{selection:'primary',width:baseline.primary.width,height:baseline.primary.height},method:'real Tauri/WebView2 + Win32 metadata, process/port checks; no UI automation',livelyInvokedByTestOrHost:false,checks,passed:checks.every(x=>x.passed),limitations:['Visual correctness requires human observation','Baseline acknowledgements do not establish visible presentation','No macOS adapter or macOS test','No supported suspend on this machine','No Explorer restart, hotplug, mixed-DPI, GPU/power, long-duration or clean-machine installer validation']};
 const output=path.join(root,'evidence/windows-prototype.json');fs.mkdirSync(path.dirname(output),{recursive:true});fs.writeFileSync(output,JSON.stringify(report,null,2)+'\n');
 console.log('Native integration checks passed; sanitized evidence: '+output);
}finally{if(active&&alive(active.pid))active.kill()}
