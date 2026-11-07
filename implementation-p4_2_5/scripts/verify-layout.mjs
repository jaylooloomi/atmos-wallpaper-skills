import fs from 'node:fs';import os from 'node:os';import path from 'node:path';import {fileURLToPath,pathToFileURL} from 'node:url';
import {summarizeLayoutReport} from './layout-regression-contract.mjs';import {runProcess,atomicJson} from './process-runner.mjs';import {workflowFingerprint} from './workflow-fingerprint.mjs';import {assertArtifact} from './artifact.mjs';
const root=fileURLToPath(new URL('../',import.meta.url));
export async function runLayoutGate({env=process.env,run=runProcess,reportFile=path.join(root,'evidence/layout-regression.json'),artifact}={}){
 const start=performance.now(),phases=[];let input;const save=v=>atomicJson(reportFile,{...v,elapsedMs:Math.round(performance.now()-start),phases,...(input?{input}:{})});save({schemaVersion:1,passed:false,status:'running'});
 try{
  const candidates=env.ATMOS_TEST_CHROME?[env.ATMOS_TEST_CHROME]:['C:/Program Files/Google/Chrome/Application/chrome.exe','C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe','/usr/bin/chromium','/usr/bin/chromium-browser','/usr/bin/google-chrome'];const chrome=candidates.find(p=>fs.existsSync(p));if(!chrome)throw Error('LAYOUT_BROWSER_REQUIRED');
  if(artifact)assertArtifact(artifact,{quick:true});input=workflowFingerprint();const output=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-layout-regression-')),childEnv={...env,...(artifact?{ATMOS_FRONTEND_ARTIFACT:artifact.dist}:{}),ATMOS_TEST_CHROME:chrome,ATMOS_LAYOUT_OUTPUT:output,ATMOS_LAYOUT_GATE_REPORT:reportFile};let negativeDetected=false;
  for(const [name,args,timeout,expectedExit=0] of [...(artifact?[]:[['prepare',['scripts/prepare.mjs','--assets-only'],120000]]),['layout-negative',['scripts/audit-layout.mjs','--host-controls','--contrast','--native','--panel-flow','--regression','--regression-smoke','--regression-inject-overlap'],60000,1],['layout-positive',['scripts/audit-layout.mjs','--host-controls','--contrast','--native','--panel-flow','--regression'],600000]]){
   const negative=expectedExit===1,runEnv=negative?{...childEnv,ATMOS_LAYOUT_OUTPUT:path.join(output,'negative'),ATMOS_LAYOUT_GATE_REPORT:''}:childEnv;
   console.log(JSON.stringify({stage:name,event:'start',inputFingerprint:input.sha256,cacheHit:false}));
   const r=await run(process.execPath,args,{cwd:root,env:runEnv,timeout,logFile:path.join(output,name+'.log'),maxBuffer:16*1024*1024});
   const phase={name,exitCode:r.status,expectedExit,elapsedMs:r.elapsedMs??null,timedOut:r.timedOut??false,aborted:r.aborted??false};phases.push(phase);console.log(JSON.stringify({event:'complete',...phase}));
   if(r.error||r.status!==expectedExit)throw Error(r.error?.code==='ETIMEDOUT'?'LAYOUT_TIMEOUT':r.aborted?'LAYOUT_ABORTED':'LAYOUT_COMMAND_FAILED');
   if(negative){const n=JSON.parse(fs.readFileSync(path.join(output,'negative/report.json'),'utf8'));negativeDetected=n.passed===false&&n.cases.length===1&&n.cases[0].regression?.states?.length===28&&n.cases[0].regression.failures.some(f=>f.code==='CONTROLS_OVERLAP');if(!negativeDetected)throw Error('LAYOUT_NEGATIVE_CONTROL_NOT_DETECTED')}
  }
  if(workflowFingerprint().sha256!==input.sha256)throw Error('LAYOUT_INPUT_CHANGED');
  if(artifact)assertArtifact(artifact,{quick:true});const summary=summarizeLayoutReport(JSON.parse(fs.readFileSync(path.join(output,'report.json'),'utf8')));summary.negativeControl={passed:negativeDetected,expectedOverlapRejected:negativeDetected};save(summary);return summary;
 }catch(error){const code=/^LAYOUT_[A-Z_]+$/.test(error.message)?error.message:'LAYOUT_GATE_FAILED';save({schemaVersion:1,passed:false,status:'failed',error:{code}});throw Error(code)}
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){try{console.log(JSON.stringify(await runLayoutGate({reportFile:process.env.ATMOS_LAYOUT_REPORT||undefined})))}catch(e){console.error(e.message+' (set ATMOS_TEST_CHROME to a supported browser if required)');process.exitCode=1}}
