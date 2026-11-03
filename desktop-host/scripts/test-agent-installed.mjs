// Explicit installed native CLI integration. No UI automation or fabricated ack.
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';import {spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';
if(process.platform!=='win32'||!process.argv.includes('--apply-and-restore'))throw Error('Explicit --apply-and-restore required');
const root=fileURLToPath(new URL('../',import.meta.url)),exe=path.join(process.env.LOCALAPPDATA,'Atmos Wallpaper/atmos-wallpaper.exe');
const scratch=path.join(process.env.LOCALAPPDATA,'AtmosWallpaper/installer-validation/agent-'+Date.now());fs.mkdirSync(scratch,{recursive:true});
const run=(args,code=0)=>{let r;for(let attempt=0;attempt<5;attempt++){r=spawnSync(exe,args,{encoding:'utf8',windowsHide:true,timeout:45000,maxBuffer:2*1024*1024});if(r.status!==4||code===4)break;Atomics.wait(new Int32Array(new SharedArrayBuffer(4)),0,0,100)}assert.equal(r.status,code,r.stdout+r.stderr+String(r.error||''));return JSON.parse(r.stdout)};
const write=(name,c)=>{const file=path.join(scratch,name+'.json');fs.writeFileSync(file,JSON.stringify(c));return file};
const command=(id,revision,changes)=>({schemaVersion:1,type:'settings.update',requestId:id,expectedRevision:revision,changes});
const checks=[];
const help=run(['--agent-help']);assert.equal(help.guide.apiVersion,1);assert.equal(run(['--agent-schema']).$id,'urn:atmos:agent:settings-update:1');checks.push({name:'native-machine-discovery',passed:true});
const isolated=path.join(scratch,'isolated-store'),options=['--store',isolated];assert.equal(run(['--get-state',...options]).revision,0);
const file=write('isolated',command('agent.isolated',0,{visual:{effect:'snow'}}));assert.equal(run(['--validate-json',file,...options]).valid,true);assert.equal(run(['--get-state',...options]).revision,0);
const first=run(['--apply-json',file,'--wait-ms','0',...options]);assert.equal(first.committed,true);assert.equal(first.rendererAcknowledged,false);assert.equal(first.revision,1);
assert.equal(run(['--apply-json',file,'--wait-ms','0',...options]).replayed,true);assert.equal(run(['--get-result','agent.isolated',...options]).revision,1);
assert.equal(run(['--apply-json',write('conflict',command('agent.stale',0,{visual:{theme:'neon'}})),...options],3).error.code,'REVISION_CONFLICT');
assert.equal(run(['--apply-json',write('reuse',command('agent.isolated',0,{visual:{theme:'neon'}})),...options],3).error.code,'REQUEST_CONFLICT');
assert.equal(run(['--apply-json',write('invalid',command('agent.invalid',1,{visual:{intensity:101}})),...options],2).error.code,'INVALID_STATE');
assert.equal(run(['--get-state',...options]).revision,1);assert.equal(run(['--get-result','not-found',...options]).found,false);checks.push({name:'native-validation-replay-conflicts-and-errors',passed:true,isolatedStore:true,noFakeAcknowledgement:true});
const before=run(['--get-state']),id='agent.live.'+Date.now();let applied=null,restored=null;
try{
 const change=write('live-change',command(id,before.revision,{visual:{intensity:before.desired.visual.intensity===100?99:100}}));
 applied=run(['--apply-json',change,'--wait-ms','10000']);assert.equal(applied.committed,true);assert.equal(applied.rendererAcknowledged,true,'Real wallpaper must report the committed JSON state');
 const queried=run(['--get-result',id,'--wait-ms','1000']);assert.equal(queried.committed,true);assert.equal(queried.rendererAcknowledged,true);checks.push({name:'live-native-json-commit-and-genuine-wallpaper-ack',passed:true});
}finally{
 if(!applied?.committed){const receipt=run(['--get-result',id]);if(receipt.committed)applied=receipt}
 if(applied?.committed){const current=run(['--get-state']);assert.equal(current.revision,applied.revision,'Concurrent user change detected; refusing to overwrite it');restored=run(['--apply-json',write('restore',command(id+'.restore',current.revision,{visual:{intensity:before.desired.visual.intensity}})),'--wait-ms','10000']);assert.equal(restored.committed,true);assert.equal(restored.rendererAcknowledged,true,'Restored state needs genuine wallpaper acknowledgement');const after=run(['--get-state']);assert.equal(after.desiredHash,before.desiredHash,'Original desired state was not restored');checks.push({name:'restore-original-desired-and-genuine-wallpaper-ack',passed:true})}
}
const ps=spawnSync('C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe',['-NoProfile','-NonInteractive','-Command',`[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); $OutputEncoding=[Console]::OutputEncoding; $atmosExe='${exe.replaceAll("'","''")}'; $state=& $atmosExe --get-state | Out-String | ConvertFrom-Json; if(!$state.ok){exit 1}; Write-Output $state.desired.widgets.city; Write-Output 'JSON_PIPELINE_OK'`],{encoding:'utf8',windowsHide:true,timeout:45000});assert.equal(ps.status,0,ps.stderr);assert.match(ps.stdout,/JSON_PIPELINE_OK/);assert.ok(ps.stdout.includes(before.desired.widgets.city));checks.push({name:'documented-powershell-utf8-json-pipeline',passed:true});
const evidence={schemaVersion:1,date:'2026-11-04',version:JSON.parse(fs.readFileSync(path.join(root,'tauri.conf.json'))).version,method:'Installed native CLI, real core CAS/receipts and genuine wallpaper-role reports; no UI automation',checks,limitations:['Renderer acknowledgement is not OS presentation or manual visual proof.','No new reboot, sleep, multi-monitor, GPU or long-duration measurement.','No automatic file watcher is enabled.']};fs.writeFileSync(path.join(root,'evidence/agent-api.json'),JSON.stringify(evidence,null,2)+'\n');console.log(JSON.stringify({passed:true,checks:checks.length,originalDesiredRestored:true}));
