import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const script=fileURLToPath(new URL('../scripts/summarize-windows-host.mjs',import.meta.url));
test('host summary distinguishes CPU core/machine percentages, memory growth and real resume evidence',t=>{
 const dir=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-summary-'));t.after(()=>fs.rmSync(dir,{recursive:true,force:true}));
 const at=seconds=>new Date(Date.UTC(2026,10,2,17)+seconds*1000).toISOString();
 fs.writeFileSync(path.join(dir,'metadata.json'),JSON.stringify({logicalProcessors:2,sampleSeconds:10}));
 const samples=Array.from({length:40},(_,i)=>({at:at(i*10),elapsedSeconds:i*10,playerCount:1,health:{ackRecent:true,renderers:[{role:'wallpaper',recent:true}]},processes:[{id:1,startedAt:at(0),group:'server',cpuSeconds:i,privateBytes:(20+i)*1048576,workingSetBytes:100*1048576,handles:20}]}));
 fs.writeFileSync(path.join(dir,'samples.jsonl'),samples.map(s=>JSON.stringify(s)).join('\n'));
 fs.writeFileSync(path.join(dir,'windows-lifecycle-events.json'),JSON.stringify({availability:{'kernel-power':'records-found'},events:[{at:at(305),value:'Resume'}]}));
 const result=spawnSync(process.execPath,[script,dir],{encoding:'utf8'});assert.equal(result.status,0,result.stderr);
 const summary=JSON.parse(result.stdout);assert.equal(summary.completed,null);assert.equal(summary.metrics.server.cpuOneCorePercentP95,10);assert.equal(summary.metrics.server.cpuMachinePercentP95,5);assert.equal(summary.metrics.server.privateMiBMedianGrowth,18);assert.equal(summary.metrics.wallpaper.peakPrivateMiB,null);assert.equal(summary.recovery[0].secondsToObservedAck,5);assert.equal(summary.httpErrorSamples,0);
});
test('missing input fails instead of inventing successful measurements',t=>{
 const dir=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-summary-missing-'));t.after(()=>fs.rmSync(dir,{recursive:true,force:true}));
 const result=spawnSync(process.execPath,[script,dir],{encoding:'utf8'});assert.notEqual(result.status,0);assert.equal(fs.existsSync(path.join(dir,'summary.json')),false);
});
