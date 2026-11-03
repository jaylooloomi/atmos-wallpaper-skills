// Actual NSIS installation/lifecycle test. No UI automation or forged acks.
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawn,spawnSync} from 'node:child_process';
import assert from 'node:assert/strict';
import http from 'node:http';
import net from 'node:net';
import {createHash} from 'node:crypto';

if(process.platform!=='win32'||!process.argv.includes('--install-and-apply'))throw Error('Explicit --install-and-apply required');
const root=fileURLToPath(new URL('../',import.meta.url));
const bundle=path.join(root,'target/release/bundle/nsis');
const config=JSON.parse(fs.readFileSync(path.join(root,'tauri.conf.json')));
const setup=path.join(bundle,config.productName+'_'+config.version+'_x64-setup.exe');
const install=path.join(process.env.LOCALAPPDATA,'Atmos Wallpaper');
const exe=path.join(install,'atmos-wallpaper.exe');
const data=path.join(process.env.LOCALAPPDATA,'io.atmos.wallpaper');
const scratch=path.join(process.env.LOCALAPPDATA,'AtmosWallpaper/installer-validation',new Date().toISOString().replaceAll(':','-'));
fs.mkdirSync(scratch,{recursive:true});
const env={...process.env,PATH:path.join(process.env.SystemRoot,'System32')+path.delimiter+process.env.SystemRoot};
delete env.NODE_OPTIONS;delete env.NODE_PATH;delete env.PYTHONHOME;delete env.PYTHONPATH;
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
function run(file,args,options={}){const r=spawnSync(file,args,{encoding:'utf8',env,windowsHide:true,timeout:180000,maxBuffer:4*1024*1024,...options});assert.equal(r.status,0,(r.stdout||'')+(r.stderr||'')+String(r.error||''));return r.stdout}
const alive=pid=>{try{process.kill(pid,0);return true}catch{return false}};
async function until(fn,label,timeout=60000){const start=Date.now();while(Date.now()-start<timeout){const v=fn();if(v)return v;await sleep(250)}throw Error('Timeout: '+label)}
const read=file=>{try{return JSON.parse(fs.readFileSync(file))}catch{return null}};
function latestStatus(after){const dir=path.join(data,'runs');if(!fs.existsSync(dir))return null;return fs.readdirSync(dir).sort().reverse().map(n=>path.join(dir,n,'status.json')).find(f=>fs.existsSync(f)&&fs.statSync(f).mtimeMs>=after)||null}
const event=(file,kind)=>read(file)?.events.find(e=>e.kind===kind);
async function ready(after){return until(()=>{const file=latestStatus(after);const boot=file&&event(file,'runtime-ready');return boot&&boot.atMs>=after&&event(file,'renderer-ready')?.atMs>=after?{file,runtime:boot.detail,attached:event(file,'attached').detail}:null},'new installed renderer genuine ack')}
const get=(port,url,headers={})=>new Promise((resolve,reject)=>{const req=http.get({host:'127.0.0.1',port,path:url,headers},r=>{let text='';r.on('data',x=>text+=x);r.on('end',()=>resolve({status:r.statusCode,text}))});req.on('error',reject);req.setTimeout(5000,()=>req.destroy(Error('HTTP timeout')))});
async function health(port){const page=await get(port,'/manager',{'Sec-Fetch-Mode':'navigate','Sec-Fetch-Dest':'document','Sec-Fetch-Site':'none'});assert.equal(page.status,200);const token=page.text.match(/window\.atmosBridgeSession="([a-f0-9]+)"/)?.[1];assert.ok(token);const h=await get(port,'/api/health',{'X-Atmos-Session':token,'Sec-Fetch-Site':'same-origin'});assert.equal(h.status,200);return JSON.parse(h.text)}
async function closed(port){for(let i=0;i<40;i++){const open=await new Promise(r=>{const s=net.connect({host:'127.0.0.1',port});s.once('connect',()=>{s.destroy();r(true)});s.once('error',()=>r(false))});if(!open)return;await sleep(250)}throw Error('Installed core port stayed open')}
const ps=path.join(process.env.SystemRoot,'System32/WindowsPowerShell/v1.0/powershell.exe');
const executablePath=pid=>run(ps,['-NoProfile','-NonInteractive','-Command',`(Get-CimInstance Win32_Process -Filter "ProcessId=${Number(pid)}").ExecutablePath`]).trim();
const sentinel=spawn(process.execPath,['-e','setInterval(()=>{},1000)'],{windowsHide:true,stdio:'ignore'});
const checks=[];let current=null;
const baselinePort=process.env.ATMOS_BASELINE_PORT?Number(process.env.ATMOS_BASELINE_PORT):null;
assert.ok(baselinePort===null||(Number.isInteger(baselinePort)&&baselinePort>0&&baselinePort<65536));
try{
 if(fs.existsSync(exe))throw Error('Existing installation found; test refuses to overwrite it');
 if(baselinePort!==null){const baseline=await health(baselinePort);assert.ok(baseline.renderers.some(r=>r.role==='wallpaper'&&r.recent),'Original wallpaper must be working before install');}
 // Ignore installer stdio: /R launches a persistent GUI child which can inherit
 // captured pipe handles. Use NSIS's actual current-user default directory.
 const started=Date.now();run(setup,['/S','/R'],{stdio:'ignore'});
 assert.ok(fs.existsSync(exe),'Installed executable missing');
 current=await ready(started);assert.equal(current.runtime.persistent,true);assert.equal(current.runtime.boundedSeconds,null);
 assert.equal(current.attached.parent.class,'WorkerW');assert.equal(current.attached.window.visible,true);
 const payload=path.join(install,'payload');
 assert.equal(executablePath(current.runtime.pid).toLowerCase(),path.join(payload,'runtime/node/node.exe').toLowerCase());
 const installedCheck=JSON.parse(run(exe,['--check-install']));assert.equal(installedCheck.payloadVerified,true);assert.equal(installedCheck.embeddedRuntimesVerified,true);
 const probe=JSON.parse(run(path.join(payload,'runtime/node/node.exe'),[path.join(root,'scripts/probe-bundled-runtime.mjs'),payload,path.join(scratch,'synthetic-store')],{env:{...env,PYTHONHOME:'Z:\\not-an-installed-python',PYTHONPATH:'Z:\\not-installed'}}));
 checks.push({name:'silent-install-and-auto-apply',passed:true,persistent:true,bundledNodeImage:true,payloadAndRuntimeSelfCheck:true,minimalPath:true,syntheticFeatureProbe:probe});console.log(JSON.stringify(checks.at(-1)));
 // Exercise the app's public native CLI, not buttons or an automation driver.
 run(exe,['--manager']);
 let managerReady=false;for(let i=0;i<30;i++){if((await health(current.runtime.port)).renderers.some(r=>r.role==='manager'&&r.recent)){managerReady=true;break}await sleep(300)}assert.ok(managerReady,'Installed manager did not acknowledge');
 checks.push({name:'installed-manager-cli',passed:true,genuineManagerAck:true});
 const content=path.join(payload,'app/implementation-p4_2_5/dist/hifi.css');const original=fs.readFileSync(content);fs.appendFileSync(content,'\n/* deliberate installer test */');
 const corrupt=spawnSync(exe,['--check-install'],{encoding:'utf8',env,windowsHide:true,timeout:30000});fs.writeFileSync(content,original);assert.equal(corrupt.status,1);assert.match(corrupt.stderr,/mismatch/);run(exe,['--check-install']);
 checks.push({name:'payload-corruption-fails-closed',passed:true});
 const marker=path.join(data,'store/installer-preservation-marker.txt');fs.writeFileSync(marker,'synthetic preservation check');
 const settings=fs.readFileSync(path.join(data,'store/live.json'));
 const old=current;const upgraded=Date.now();run(setup,['/S','/R'],{stdio:'ignore'});current=await ready(upgraded);
 assert.notEqual(current.runtime.pid,old.runtime.pid);await until(()=>!alive(old.runtime.pid),'upgrade closes old owned runtime');await closed(old.runtime.port);assert.ok(event(old.file,'restored'));
 assert.ok(alive(sentinel.pid),'Unrelated Node process was terminated');assert.ok(fs.existsSync(marker));assert.deepEqual(fs.readFileSync(path.join(data,'store/live.json')),settings);
 checks.push({name:'running-reinstall',passed:true,oldRuntimeClosed:true,oldPortClosed:true,settingsPreserved:true,unrelatedNodePreserved:true});
 const previous=current;const uninstaller=path.join(install,'uninstall.exe');assert.ok(fs.existsSync(uninstaller));run(uninstaller,['/S']);
 await until(()=>!fs.existsSync(exe),'uninstalled application files',45000);await until(()=>!alive(previous.runtime.pid),'uninstall runtime cleanup');await closed(previous.runtime.port);
 assert.ok(fs.existsSync(marker),'Default uninstall should preserve user settings');assert.ok(alive(sentinel.pid));
 if(baselinePort!==null){const restored=await health(baselinePort);assert.ok(restored.renderers.some(r=>r.role==='wallpaper'&&r.recent),'Original wallpaper stopped after uninstall');}
 checks.push({name:'running-uninstall',passed:true,applicationRemoved:true,ownedRuntimeClosed:true,portClosed:true,userDataPreserved:true,unrelatedNodePreserved:true,originalWallpaperStillAcknowledging:baselinePort!==null?true:null});console.log(JSON.stringify(checks.at(-1)));
 fs.unlinkSync(marker);
 const finalStart=Date.now();run(setup,['/S','/R'],{stdio:'ignore'});current=await ready(finalStart);run(exe,['--check-install']);
 checks.push({name:'final-install-auto-applied',passed:true,persistent:true});
 const b=fs.readFileSync(setup);const report={schemaVersion:1,date:new Intl.DateTimeFormat('en-CA',{timeZone:'Asia/Taipei',year:'numeric',month:'2-digit',day:'2-digit'}).format(new Date()),platform:process.platform,architecture:process.arch,installer:{name:path.basename(setup),bytes:b.length,sha256:createHash('sha256').update(b).digest('hex'),signed:false},method:'Actual NSIS /S /R and native CLI + genuine renderer acks; no UI automation',checks,passed:checks.every(c=>c.passed),leftInstalledAndApplied:true,limitations:['Test host already had global runtimes and WebView2; product was verified using bundled executable paths and minimal PATH, not a pristine VM','Offline WebView2 missing-runtime branch is bundled but not executed on this host','Visual presentation needs user confirmation','No Windows 11, supported sleep, GPU/power or long-duration validation']};
 fs.writeFileSync(path.join(root,'evidence/windows-installer.json'),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({installerChecksPassed:true,checks:checks.length,leftInstalledAndApplied:true,managerPort:current.runtime.port}));
}catch(e){if(fs.existsSync(exe)){try{run(exe,['--quit'])}catch{}}throw e}
finally{sentinel.kill()}
