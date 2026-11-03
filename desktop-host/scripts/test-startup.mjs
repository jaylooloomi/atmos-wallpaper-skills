// Real registry + installed HTTP/CLI + NSIS tests. Never reboots/logs off the
// user or automates desktop/browser UI. Login invocation is tested separately
// from an actual Windows sign-in, which is explicitly not claimed here.
import fs from 'node:fs';import path from 'node:path';import {fileURLToPath} from 'node:url';import {spawn,spawnSync} from 'node:child_process';import assert from 'node:assert/strict';import http from 'node:http';import {createHash} from 'node:crypto';
if(process.platform!=='win32'||!process.argv.includes('--configure-login-startup'))throw Error('Explicit --configure-login-startup required');
const root=fileURLToPath(new URL('../',import.meta.url)),config=JSON.parse(fs.readFileSync(path.join(root,'tauri.conf.json')));
const setup=path.join(root,'target/release/bundle/nsis',config.productName+'_'+config.version+'_x64-setup.exe');
const dir=path.join(process.env.LOCALAPPDATA,'Atmos Wallpaper'),exe=path.join(dir,'atmos-wallpaper.exe'),data=path.join(process.env.LOCALAPPDATA,'io.atmos.wallpaper');
const env={...process.env,PATH:path.join(process.env.SystemRoot,'System32')+path.delimiter+process.env.SystemRoot};delete env.NODE_OPTIONS;delete env.PYTHONPATH;delete env.PYTHONHOME;
const ps=path.join(process.env.SystemRoot,'System32/WindowsPowerShell/v1.0/powershell.exe');
function run(file,args,options={}){const r=spawnSync(file,args,{env,encoding:'utf8',windowsHide:true,timeout:180000,maxBuffer:4*1024*1024,...options});assert.equal(r.status,0,(r.stderr||'')+(r.stdout||'')+String(r.error||''));return r.stdout}
const sleep=ms=>new Promise(r=>setTimeout(r,ms));const alive=pid=>{try{process.kill(pid,0);return true}catch{return false}};
async function until(fn,label,timeout=90000){const start=Date.now();while(Date.now()-start<timeout){const v=fn();if(v)return v;await sleep(250)}throw Error('Timeout: '+label)}
const read=f=>{try{return JSON.parse(fs.readFileSync(f))}catch{return null}};
function current(after=0){const runs=path.join(data,'runs');if(!fs.existsSync(runs))return null;for(const name of fs.readdirSync(runs).sort().reverse()){const file=path.join(runs,name,'status.json'),value=read(file);const boot=value?.events.find(e=>e.kind==='runtime-ready');if(boot?.atMs>=after&&value.events.some(e=>e.kind==='renderer-ready'))return {file,boot:boot.detail}}return null}
const registered=()=>run(ps,['-NoProfile','-NonInteractive','-Command',"$k=[Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\\Microsoft\\Windows\\CurrentVersion\\Run');if($k){$v=$k.GetValue('AtmosWallpaper',$null);if($null -ne $v){[Console]::Write($v)};$k.Close()}"]).trim();
function otherEntriesHash(){const text=run(ps,['-NoProfile','-NonInteractive','-Command',"$k=Get-Item 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Run' -ErrorAction SilentlyContinue;$v=@();if($k){$v=@($k.GetValueNames()|Where-Object {$_ -ne 'AtmosWallpaper'}|Sort-Object|ForEach-Object {[pscustomobject]@{name=$_;kind=$k.GetValueKind($_).ToString();value=$k.GetValue($_)}})};ConvertTo-Json -InputObject $v -Compress -Depth 4"]);return createHash('sha256').update(text).digest('hex')}
function request(port,url,token,body){const origin='http://127.0.0.1:'+port;return new Promise((resolve,reject)=>{const headers={Origin:origin,'Sec-Fetch-Site':token?'same-origin':'none','Sec-Fetch-Mode':token?'cors':'navigate','Sec-Fetch-Dest':token?'empty':'document',...(token?{'X-Atmos-Session':token}:{}),...(body!==undefined?{'Content-Type':'application/json'}:{})};const req=http.request(origin+url,{method:body===undefined?'GET':'POST',headers},res=>{let text='';res.on('data',x=>text+=x);res.on('end',()=>resolve({status:res.statusCode,text}))});req.on('error',reject);req.setTimeout(20000,()=>req.destroy(Error('API timeout')));req.end(body===undefined?undefined:JSON.stringify(body))})}
async function tokenFor(app,route='/manager'){const page=await request(app.boot.port,route);assert.equal(page.status,200);return page.text.match(/window\.atmosBridgeSession="([a-f0-9]+)"/)?.[1]}
async function api(app,body){const r=await request(app.boot.port,'/api/host/startup',await tokenFor(app),body);assert.equal(r.status,200,r.text);return JSON.parse(r.text)}
async function install(){const at=Date.now();run(setup,['/S','/R'],{stdio:'ignore'});return until(()=>current(at),'new installed renderer ack')}
async function quit(app){run(exe,['--quit']);await until(()=>!alive(app.boot.pid),'owned core closure',10000);assert.equal(read(app.file).events.find(e=>e.kind==='restored').detail.ownWindowRemoved,true)}
async function launch(args){const at=Date.now();const child=spawn(exe,args,{env,windowsHide:true,stdio:'ignore'});child.unref();return until(()=>current(at),'background startup renderer ack')}
function record(name,detail={}){checks.push({name,passed:true,...detail});console.log(JSON.stringify(checks.at(-1)))}
const checks=[];let app;
assert.ok(fs.existsSync(exe),'Upgrade test requires an existing installed Atmos');
const settings=fs.readFileSync(path.join(data,'store/live.json'));const others=otherEntriesHash();
const sentinel=spawn(process.execPath,['-e','setInterval(()=>{},1000)'],{windowsHide:true,stdio:'ignore'});
try{
 // Explicit integration-test baseline: reset only Atmos's new preference, not
 // other startup entries or Windows Startup Apps approval. Final state is on.
 run(ps,['-NoProfile','-NonInteractive','-Command',"$k=[Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\\AtmosWallpaper',$true);if($k){$k.DeleteValue('StartupEnabled',$false);$k.Close()}"]);
 app=await install();assert.deepEqual(fs.readFileSync(path.join(data,'store/live.json')),settings);run(exe,['--check-install']);
 const initial=await api(app);assert.equal(initial.available,true);assert.equal(initial.enabled,true);assert.equal(registered(),JSON.parse(run(exe,['--autostart','status'])).command);
 record('installer-initializes-current-user-logon-registration',{settingsPreserved:true});
 assert.equal((await api(app,{enabled:false})).enabled,false);assert.equal(registered(),'');assert.equal((await api(app,{enabled:true})).enabled,true);assert.ok(registered());
 const denied=await request(app.boot.port,'/api/host/startup',await tokenFor(app,'/wallpaper'),{enabled:false});assert.equal(denied.status,403);assert.equal((await api(app)).enabled,true);
 record('manager-api-on-off-and-wallpaper-denied',{actualRegistryVerified:true,visualDesiredUnchanged:fs.readFileSync(path.join(data,'store/live.json')).equals(settings)});
 const command=registered();await quit(app);assert.equal(registered(),command);
 const match=command.match(/^"([^"]+)" (--autostart-launch)$/);assert.ok(match);assert.equal(match[1].toLowerCase(),exe.toLowerCase());
 app=await launch([match[2]]);assert.equal((await api(app)).enabled,true);
 const health=JSON.parse((await request(app.boot.port,'/api/health',await tokenFor(app))).text);assert.ok(health.renderers.some(r=>r.role==='wallpaper'&&r.recent));assert.ok(!health.renderers.some(r=>r.role==='manager'));
 const pid=app.boot.pid;run(exe,['--autostart-launch']);assert.equal(current().boot.pid,pid);record('registered-command-background-apply',{genuineWallpaperAck:true,noManagerRenderer:true,duplicateDoesNotRestart:true,actualWindowsLogin:false});
 await api(app,{enabled:false});await quit(app);app=await launch([]);assert.equal((await api(app)).enabled,false);assert.equal(registered(),'');
 app=await install();assert.equal((await api(app)).enabled,false);assert.equal(registered(),'');record('disabled-choice-survives-restart-and-reinstall');
 await api(app,{enabled:true});const old=app;run(path.join(dir,'uninstall.exe'),['/S'],{stdio:'ignore'});await until(()=>!fs.existsSync(exe),'uninstalled executable');await until(()=>!alive(old.boot.pid),'uninstalled core');assert.equal(registered(),'');assert.ok(fs.existsSync(path.join(data,'store/live.json')));assert.ok(alive(sentinel.pid));record('uninstall-removes-own-startup-entry',{userDataPreserved:true,unrelatedNodePreserved:true});
 app=await install();assert.equal((await api(app)).enabled,true);assert.deepEqual(fs.readFileSync(path.join(data,'store/live.json')),settings);assert.equal(otherEntriesHash(),others);run(exe,['--manager']);
 record('reinstall-restores-enabled-choice',{otherStartupEntriesUnchanged:true,leftInstalledEnabledAndApplied:true});
 const bytes=fs.readFileSync(setup);const report={schemaVersion:1,date:new Intl.DateTimeFormat('en-CA',{timeZone:'Asia/Taipei',year:'numeric',month:'2-digit',day:'2-digit'}).format(new Date()),version:config.version,installer:{name:path.basename(setup),bytes:bytes.length,sha256:createHash('sha256').update(bytes).digest('hex')},method:'Actual HKCU registry, installed manager HTTP, native CLI and NSIS; no UI automation',checks,passed:true,limitations:['Exact registered command executed successfully; no actual Windows reboot/logoff performed','Windows Startup Apps/policy can override Run registration and are not modified','Visual UI confirmation recorded separately','No fresh VM, Windows 11 or long-duration validation']};fs.writeFileSync(path.join(root,'evidence/windows-startup.json'),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({passed:true,startupEnabled:true,managerPort:app.boot.port}));
}finally{sentinel.kill()}
