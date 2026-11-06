// Explicit finite native watcher sessions in fresh synthetic stores only.
import test from 'node:test';import assert from 'node:assert/strict';import fs from 'node:fs';import os from 'node:os';import path from 'node:path';
import {spawn,spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';
const repo=fileURLToPath(new URL('../../',import.meta.url)),exe=path.join(repo,'desktop-host/target/debug/atmos-wallpaper.exe'),legacy=path.join(repo,'implementation-p4_2_5/src/agent-cli.mjs');
assert.equal(process.platform,'win32');assert.ok(fs.existsSync(exe));assert.equal(fs.existsSync(path.join(path.dirname(exe),'payload')),false);
const env={...process.env,PATH:'',NODE_OPTIONS:'--invalid-node-option',NODE_PATH:'Z:/missing-runtime'};
const command=(id='watch.1',revision=0,effect='rain')=>({schemaVersion:1,type:'settings.update',requestId:id,expectedRevision:revision,changes:{visual:{effect}}});
function fixture(t){const base=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-native-watch-')),root=path.join(base,'store'),file=path.join(base,'intent.json');t.after(()=>fs.rmSync(base,{recursive:true,force:true}));return{base,root,file};}
const args=(f,duration=1200)=>['--watch-json',f.file,'--store',f.root,'--poll-ms','100','--settle-ms','500','--duration-ms',String(duration)];
function finite(f,{useNative=true,duration=1200}={}){const r=spawnSync(useNative?exe:process.execPath,useNative?args(f,duration):[legacy,...args(f,duration)],{env:useNative?env:process.env,encoding:'utf8',windowsHide:true,timeout:duration+8000,maxBuffer:1048576});assert.ifError(r.error);assert.equal(r.stderr,'');return{exit:r.status,lines:r.stdout.trim().split(/\r?\n/).map(s=>JSON.parse(s))};}
function normalized(lines){return lines.map(line=>{const v=structuredClone(line);if(v.event==='watch-stopped'){assert.ok(v.elapsedMs>=0);delete v.elapsedMs;}return v;});}
function active(f,duration,started){return new Promise((resolve,reject)=>{
 const child=spawn(exe,args(f,duration),{env,windowsHide:true,stdio:['ignore','pipe','pipe']});let buffer='',stderr='';const lines=[],timers=[];
 const timer=setTimeout(()=>{child.kill();reject(Error('NATIVE_WATCH_TIMEOUT'));},duration+10000);
 child.stdout.on('data',part=>{buffer+=part;for(;;){const at=buffer.indexOf('\n');if(at<0)break;const line=JSON.parse(buffer.slice(0,at));buffer=buffer.slice(at+1);lines.push(line);if(line.event==='watch-started')started?.(delay=>{const id=setTimeout(delay.fn,delay.ms);timers.push(id);});}});
 child.stderr.on('data',b=>stderr+=b);child.on('error',reject);child.on('exit',exit=>{clearTimeout(timer);timers.forEach(clearTimeout);try{assert.equal(stderr,'');assert.equal(buffer,'');resolve({exit,lines});}catch(e){reject(e);}});
 });}

test('finite native watcher applies exact input once and legacy watcher replays the same receipt',t=>{
 const f=fixture(t);fs.writeFileSync(f.file,JSON.stringify(command()));
 const result=finite(f);assert.equal(result.exit,0);assert.equal(result.lines[0].event,'watch-started');assert.equal(result.lines.at(-1).reason,'duration');
 const successes=result.lines.filter(v=>v.event==='result');assert.equal(successes.length,1);assert.equal(successes[0].replayed,false);assert.equal(successes[0].rendererAcknowledged,false);
 const old=finite(f,{useNative:false});const expected=structuredClone(result.lines);expected.find(v=>v.event==='result').replayed=true;
 assert.deepEqual(normalized(old.lines),normalized(expected));assert.equal(JSON.parse(fs.readFileSync(path.join(f.root,'live.json'))).revision,1);
});

test('invalid UTF-8 and missing-file errors remain deduplicated JSONL inputs',t=>{
 for(const bytes of [Buffer.from([0xff]),null]){const f=fixture(t);if(bytes)fs.writeFileSync(f.file,bytes);const n=finite(f),j=finite(f,{useNative:false});assert.equal(n.exit,0);assert.deepEqual(normalized(n.lines),normalized(j.lines));assert.equal(n.lines.filter(v=>v.event==='input-error').length,1);}
});

test('partial writes settle; atomic replacements commit each changed intent once',async t=>{
 const f=fixture(t);fs.writeFileSync(f.file,'{');
 const result=await active(f,3200,schedule=>{
  schedule({ms:200,fn:()=>{const tmp=f.file+'.tmp';fs.writeFileSync(tmp,JSON.stringify(command()));fs.renameSync(tmp,f.file);}});
  schedule({ms:1800,fn:()=>{const tmp=f.file+'.tmp';fs.writeFileSync(tmp,JSON.stringify(command('watch.2',1,'snow')));fs.renameSync(tmp,f.file);}});
 });
 assert.equal(result.exit,0);assert.equal(result.lines.filter(v=>v.event==='result').length,2);assert.equal(result.lines.filter(v=>v.event==='input-error').length,0);
 const live=JSON.parse(fs.readFileSync(path.join(f.root,'live.json')));assert.equal(live.revision,2);assert.equal(live.desired.visual.effect,'snow');
});

test('owner busy retries stop after the bounded backoff and never rewrite intent',async t=>{
 const f=fixture(t);const body=JSON.stringify(command());fs.writeFileSync(f.file,body);
 const result=await active(f,3000,()=>fs.writeFileSync(path.join(f.root,'.lock'),JSON.stringify({pid:process.pid})));
 assert.equal(result.exit,0);const errors=result.lines.filter(v=>v.event==='command-error');assert.equal(errors.length,1);assert.equal(errors[0].error.code,'STORE_LOCKED');assert.equal(errors[0].committed,null);
 assert.equal(JSON.parse(fs.readFileSync(path.join(f.root,'live.json'))).revision,0);assert.equal(fs.readFileSync(f.file,'utf8'),body);fs.unlinkSync(path.join(f.root,'.lock'));
});

test('watch target mapping changes block subsequent commits until reconciliation',async t=>{
 const f=fixture(t);fs.writeFileSync(f.file,JSON.stringify(command()));
 const result=await active(f,1200,()=>fs.writeFileSync(path.join(f.root,'display-layout.json'),JSON.stringify({schemaVersion:1,revision:1,lastRequestId:'fixture.layout',layout:{mode:'duplicate',targets:['display-'+'a'.repeat(64)],askOnApply:false}})));
 assert.equal(result.exit,0);assert.equal(result.lines.filter(v=>v.event==='result').length,0);assert.equal(result.lines.find(v=>v.event==='command-error').error.code,'TARGET_SCOPE_CHANGED');assert.equal(JSON.parse(fs.readFileSync(path.join(f.root,'live.json'))).revision,0);
});
