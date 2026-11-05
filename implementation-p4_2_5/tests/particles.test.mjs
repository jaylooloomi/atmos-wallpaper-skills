import test from 'node:test';import assert from 'node:assert/strict';import fs from 'node:fs';import vm from 'node:vm';
import {particleEffects,particleFrame} from '../src/particle-model.mjs';
import {initialDesired,validateDesired} from '../src/state.mjs';
test('all five particle effects survive strict settings validation and have bounded finite animated geometry',()=>{
 for(const effect of particleEffects){const d=initialDesired();d.visual.effect=effect;assert.equal(validateDesired(d).visual.effect,effect);
  for(const [w,h]of [[320,180],[1920,1080],[7680,4320]])for(const ms of [0,700,3000,12000,7200000,1e12]){const p=particleFrame(effect,ms,100,w,h);assert.ok(p.length>0&&p.length<=400);for(const v of p){for(const key of ['x','y','r','alpha','hue'])assert.ok(Number.isFinite(v[key]));assert.ok(v.alpha>=0&&v.alpha<=1)}}
  assert.notDeepEqual(particleFrame(effect,400,80,1920,1080),particleFrame(effect,1400,80,1920,1080));assert.deepEqual(particleFrame(effect,400,0,1920,1080),[]);
 }
 assert.deepEqual(particleFrame('unknown',0,100,1920,1080),[]);assert.deepEqual(particleFrame('galaxy',NaN,100,1920,1080),[]);
});
test('particle presenter restores canvas state, draws distinct paths and obeys hidden, reduced and power policies',()=>{
 let suspended=false,hidden=false,reduced=false,draws=0,balance=0;const ctx={save(){balance++},restore(){balance--},createRadialGradient(){return {addColorStop(){}}},createLinearGradient(){return {addColorStop(){}}}};
 for(const k of ['fillRect','translate','rotate','scale','beginPath','moveTo','bezierCurveTo','fill','lineTo','stroke'])ctx[k]=()=>{draws++};
 const window={AtmosAnimationSuspended:()=>suspended},document={get hidden(){return hidden}};
 vm.runInNewContext(fs.readFileSync(new URL('../src/particle-model.mjs',import.meta.url),'utf8').replace(/\bexport /g,'')+'\n'+fs.readFileSync(new URL('../src/particle-client.js',import.meta.url),'utf8'),{window,document,matchMedia:()=>({get matches(){return reduced}})});
 for(const effect of particleEffects){const args={effect,width:1920,height:1080,intensity:100,elapsedMs:1700},before=draws;window.AtmosParticles.draw(ctx,args);assert.ok(draws>before);assert.equal(balance,0);
  for(const policy of ['suspended','hidden','reduced','paused']){const count=draws;suspended=policy==='suspended';hidden=policy==='hidden';reduced=policy==='reduced';window.AtmosParticles.draw(ctx,{...args,paused:policy==='paused'});assert.equal(draws,count)}suspended=hidden=reduced=false;
 }
});

import os from 'node:os';import path from 'node:path';import {ControlCore} from '../src/portable-core.mjs';import {exportRenderer} from '../src/export-renderer.mjs';
test('offline exports carry particle, lightning and plain-calendar runtimes with integrity manifests',t=>{const dir=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-particle-export-'));t.after(()=>fs.rmSync(dir,{recursive:true,force:true}));const core=new ControlCore(dir);core.init();for(const [i,effect]of particleEffects.entries()){const p=core.preview({requestId:'particle.preview.'+i,expectedRevision:i,patch:{visual:{effect}}});core.apply({requestId:'particle.apply.'+i,expectedRevision:i,previewId:p.previewId});const out=exportRenderer(core);assert.equal(out.snapshot.desired.visual.effect,effect);const manifest=JSON.parse(fs.readFileSync(path.join(out.directory,'p1-manifest.json')));for(const name of ['particle-client.js','lightning-client.js','calendar-client.js']){assert.ok(fs.existsSync(path.join(out.directory,name)));assert.ok(manifest.files.some(f=>f.path===name))}const loader=fs.readFileSync(path.join(out.directory,'p1-loader.js'),'utf8');assert.match(loader,/AtmosCalendar.start/);assert.match(loader,/removeAttribute\('data-atmos-loading'\)/);assert.match(loader,/atmosBootNotice/);assert.ok(loader.indexOf("'particle-client.js'")<loader.indexOf("'app.js'"));assert.ok(exportRenderer(core).existing)}});
