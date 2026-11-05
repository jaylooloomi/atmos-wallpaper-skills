// Read-only validation against the installed native CLI; never applies an effect.
import fs from 'node:fs';import path from 'node:path';import os from 'node:os';import assert from 'node:assert/strict';import {spawnSync} from 'node:child_process';
const exe=path.join(process.env.LOCALAPPDATA,'Atmos Wallpaper/atmos-wallpaper.exe');
const run=args=>{const r=spawnSync(exe,args,{encoding:'utf8',windowsHide:true,timeout:30000,maxBuffer:1024*1024});let value;try{value=JSON.parse(r.stdout)}catch{throw Error('Installed CLI JSON invalid')}assert.equal(r.status,0,'Installed CLI '+args[0]+' failed: '+String(value.error?.code||'UNKNOWN'));return value};
const effects=['meteor','firefly','sakura','fireworks','galaxy'],before=run(['--get-state']),schema=run(['--agent-schema']);
for(const effect of effects)assert.ok(schema.properties.changes.properties.visual.properties.effect.enum.includes(effect));
const dir=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-installed-particle-validation-'));
for(const effect of effects){const file=path.join(dir,effect+'.json');fs.writeFileSync(file,JSON.stringify({schemaVersion:1,type:'settings.update',requestId:'validate.particles.'+effect,expectedRevision:before.revision,changes:{visual:{effect}}}));const r=run(['--validate-json',file]);assert.equal(r.valid,true);assert.equal(r.committed,false)}
const after=run(['--get-state']);assert.equal(after.revision,before.revision);assert.equal(after.desiredHash,before.desiredHash);
const report={schemaVersion:1,date:new Date().toLocaleDateString('en-CA',{timeZone:'Asia/Taipei'}),passed:true,effects,checks:{installedAgentSchemaContainsAllEffects:true,installedMergedStateValidationPassed:true,noEffectApplied:true,desiredRevisionAndHashPreserved:true},limitations:['Read-only CLI/schema validation, not a visual or GPU performance test.','The configured effect and information-saving preference were not changed.']};
fs.writeFileSync(process.env.ATMOS_PARTICLE_REPORT||new URL('../evidence/particle-agent-installed-2026-11-06.json',import.meta.url),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
