import test from'node:test';import assert from'node:assert/strict';import{spawnSync}from'node:child_process';import{fileURLToPath}from'node:url';import path from'node:path';
test('native owned-process sampler pins executable/creation time and tracks only the actual fixture child',{skip:process.platform!=='win32'},()=>{
 const script=fileURLToPath(new URL('../../desktop-host/scripts/test-owned-processes.ps1',import.meta.url)),ps=path.join(process.env.SystemRoot,'System32/WindowsPowerShell/v1.0/powershell.exe');
 const r=spawnSync(ps,['-NoLogo','-NoProfile','-NonInteractive','-File',script],{encoding:'utf8',windowsHide:true,timeout:30000,maxBuffer:131072});assert.equal(r.status,0,r.stderr||r.stdout);const report=JSON.parse(r.stdout.trim());assert.equal(report.passed,true);assert.equal(report.checks,4);
});
