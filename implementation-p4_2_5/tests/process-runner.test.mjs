import test from 'node:test';import assert from 'node:assert/strict';import fs from 'node:fs';import os from 'node:os';import path from 'node:path';import {runProcess,atomicJson} from '../scripts/process-runner.mjs';
test('streams both channels before completion, preserves raw UTF8/logs and exit status',async t=>{
 const dir=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-runner-'));t.after(()=>fs.rmSync(dir,{recursive:true,force:true}));const logFile=path.join(dir,'raw.log');let resolveFirst;const first=new Promise(r=>resolveFirst=r);
 const pending=runProcess(process.execPath,['-e',`process.stdout.write('early');process.stderr.write('error-channel');const b=Buffer.from('天氣');process.stdout.write(b.subarray(0,1));setTimeout(()=>{process.stdout.write(b.subarray(1));process.exitCode=7},500)`],{stream:false,logFile,onOutput:()=>resolveFirst(true)});
 assert.equal(await first,true);const result=await pending;assert.equal(result.status,7);assert.equal(result.stdout,'early天氣');assert.equal(result.stderr,'error-channel');assert.equal(result.timedOut,false);assert.ok(result.elapsedMs>=400);assert.ok(fs.readFileSync(logFile).includes(Buffer.from('error-channel')));
});
test('timeout kills the owned subprocess, and is distinct from a normal failure',async()=>{const r=await runProcess(process.execPath,['-e','setInterval(()=>{},1000)'],{stream:false,timeout:100});assert.equal(r.timedOut,true);assert.equal(r.error.code,'ETIMEDOUT');assert.notEqual(r.status,0)});
test('spawn errors and aborts cannot be reported as successful completion',async()=>{
 const missing=await runProcess(path.join(os.tmpdir(),'atmos-no-such-executable-'+Date.now()),[],{stream:false});assert.equal(missing.error.code,'ENOENT');assert.notEqual(missing.status,0);
 const controller=new AbortController();const pending=runProcess(process.execPath,['-e','setInterval(()=>{},1000)'],{stream:false,signal:controller.signal});controller.abort();const aborted=await pending;assert.equal(aborted.aborted,true);assert.equal(aborted.timedOut,false);assert.equal(aborted.error.code,'ABORTED');
});
test('output overflow is a failure and atomic reports replace previous success',async t=>{
 const r=await runProcess(process.execPath,['-e',"process.stdout.write('x'.repeat(4096))"],{stream:false,maxBuffer:16});assert.equal(r.error.code,'MAX_BUFFER');
 const d=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-report-'));t.after(()=>fs.rmSync(d,{recursive:true,force:true}));const file=path.join(d,'report.json');atomicJson(file,{passed:true});atomicJson(file,{passed:false,status:'running'});assert.equal(JSON.parse(fs.readFileSync(file)).passed,false);assert.deepEqual(fs.readdirSync(d),['report.json']);
});
test('timeout cleans up the owned descendant tree on Windows',{skip:process.platform!=='win32'},async()=>{
 const code=`const {spawn}=require('node:child_process');const c=spawn(process.execPath,['-e','setInterval(()=>{},1000)'],{windowsHide:true,stdio:'ignore'});console.log(c.pid);setInterval(()=>{},1000)`;
 const r=await runProcess(process.execPath,['-e',code],{stream:false,timeout:700});const descendant=Number(r.stdout.trim());assert.ok(Number.isSafeInteger(descendant)&&descendant>0);assert.equal(r.timedOut,true);assert.throws(()=>process.kill(descendant,0));
});
