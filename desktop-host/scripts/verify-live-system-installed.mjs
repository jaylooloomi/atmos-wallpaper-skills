// Read-only installed-host verification. No synthetic telemetry or renderer ACKs.
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const root=fileURLToPath(new URL('../',import.meta.url));
const exe=path.join(process.env.LOCALAPPDATA,'Atmos Wallpaper/atmos-wallpaper.exe');
const state=spawnSync(exe,['--host-command','get'],{encoding:'utf8',windowsHide:true,timeout:30000});
assert.equal(state.status,0);const host=JSON.parse(state.stdout);assert.ok(host.runtimeFresh&&host.runtime?.nativeAttached);
const runs=path.join(process.env.LOCALAPPDATA,'io.atmos.wallpaper/runs');
const run=fs.readdirSync(runs).find(n=>n.endsWith('-'+host.runtime.pid));assert.ok(run);
const journal=JSON.parse(fs.readFileSync(path.join(runs,run,'status.json')));
const port=journal.events.find(e=>e.kind==='runtime-ready').detail.port;
const get=(route,headers)=>new Promise((resolve,reject)=>{
 const req=http.get('http://127.0.0.1:'+port+route,{headers},res=>{let text='';res.on('data',b=>text+=b);res.on('end',()=>{try{assert.equal(res.statusCode,200);resolve(text)}catch(e){reject(e)}})});
 req.setTimeout(10000,()=>req.destroy(Error('Installed API timeout')));req.on('error',reject);
});
const page=await get('/manager',{'Sec-Fetch-Mode':'navigate','Sec-Fetch-Dest':'document','Sec-Fetch-Site':'none'});
const token=page.match(/window\.atmosBridgeSession="([a-f0-9]+)"/)[1];
const headers={'X-Atmos-Session':token,'Sec-Fetch-Site':'same-origin'};
const samples=[];
for(let i=0;i<6;i++){
 if(i)await new Promise(r=>setTimeout(r,2600));
 const s=JSON.parse(await get('/api/providers/system',headers));
 assert.equal(s.available,true);assert.equal(s.source,'windows-native-readonly');assert.equal(s.stale,false);
 assert.ok(s.ageMs>=0&&s.ageMs<15000);assert.ok(Number.isFinite(s.cpuPercent)&&s.cpuPercent>=0&&s.cpuPercent<=100);
 assert.ok(s.memory.usedBytes>=0&&s.memory.usedBytes<=s.memory.totalBytes);
 assert.ok(s.network?.available);assert.ok(['active-physical-interfaces','active-nonloopback-interfaces'].includes(s.network.scope));
 for(const key of ['receiveBytesPerSecond','sendBytesPerSecond'])assert.ok(Number.isFinite(s.network[key])&&s.network[key]>=0);
 assert.equal(s.gpu,null);assert.equal(s.power,null);samples.push(s);
}
assert.equal(new Set(samples.map(s=>s.sampledAtMs)).size,samples.length,'Samples must advance, not replay one cached value');
const range=fn=>({min:Math.min(...samples.map(fn)),max:Math.max(...samples.map(fn))});
const report={schemaVersion:1,date:new Date().toLocaleDateString('en-CA',{timeZone:'Asia/Taipei'}),passed:true,method:'Six read-only authenticated installed /api/providers/system calls, separated by 2.6 seconds; genuine native Windows telemetry',samples:samples.length,distinctSampleTimes:samples.length,observedSeconds:(samples.at(-1).sampledAtMs-samples[0].sampledAtMs)/1000,cpuPercent:range(s=>s.cpuPercent),memoryUsedGiB:range(s=>s.memory.usedBytes/1073741824),receiveBytesPerSecond:range(s=>s.network.receiveBytesPerSecond),sendBytesPerSecond:range(s=>s.network.sendBytesPerSecond),networkScopes:[...new Set(samples.map(s=>s.network.scope))],checks:{nativeHostAttached:true,freshSamples:true,networkAvailable:true,noGpuOrPowerClaims:true},limitations:['Short data-path check, not a soak, visual proof or benchmark.','CPU uses the primary processor group. Network is interface throughput, not internet speed.','No adapter identity, session secret, private settings or raw series is exported.']};
fs.writeFileSync(process.env.ATMOS_SYSTEM_OUTPUT||path.join(root,'evidence/live-system-samples-2026-11-06.json'),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
