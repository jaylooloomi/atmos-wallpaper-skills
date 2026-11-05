import fs from 'node:fs';import os from 'node:os';import path from 'node:path';import {spawnSync} from 'node:child_process';import {fileURLToPath,pathToFileURL} from 'node:url';
import {summarizeLayoutReport} from './layout-regression-contract.mjs';
const root=fileURLToPath(new URL('../',import.meta.url));
export function runLayoutGate({env=process.env,run=spawnSync,reportFile=path.join(root,'evidence/layout-regression.json')}={}){
 const save=v=>{fs.mkdirSync(path.dirname(reportFile),{recursive:true});fs.writeFileSync(reportFile,JSON.stringify(v,null,2)+'\n')};save({schemaVersion:1,passed:false,status:'running'});
 try{
  const candidates=env.ATMOS_TEST_CHROME?[env.ATMOS_TEST_CHROME]:['C:/Program Files/Google/Chrome/Application/chrome.exe','C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe','/usr/bin/chromium','/usr/bin/chromium-browser','/usr/bin/google-chrome'];const chrome=candidates.find(p=>fs.existsSync(p));if(!chrome)throw Error('LAYOUT_BROWSER_REQUIRED');
  const output=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-layout-regression-')),childEnv={...env,ATMOS_TEST_CHROME:chrome,ATMOS_LAYOUT_OUTPUT:output,ATMOS_LAYOUT_GATE_REPORT:reportFile};
  let negativeDetected=false;
  for(const [args,timeout,expectedExit=0] of [[['scripts/prepare.mjs','--assets-only'],120000],[['scripts/audit-layout.mjs','--host-controls','--contrast','--native','--panel-flow','--regression','--regression-smoke','--regression-inject-overlap'],60000,1],[['scripts/audit-layout.mjs','--host-controls','--contrast','--native','--panel-flow','--regression'],600000]]){
   const negative=expectedExit===1,runEnv=negative?{...childEnv,ATMOS_LAYOUT_OUTPUT:path.join(output,'negative'),ATMOS_LAYOUT_GATE_REPORT:''}:childEnv;
   const r=run(process.execPath,args,{cwd:root,env:runEnv,encoding:'utf8',windowsHide:true,timeout,maxBuffer:16*1024*1024});process.stdout.write(r.stdout||'');fs.writeFileSync(path.join(output,negative?'negative.log':args[0].includes('audit')?'browser.log':'prepare.log'),(r.stdout||'')+(r.stderr||''));if(r.status!==expectedExit)throw Error(r.error?.code==='ETIMEDOUT'?'LAYOUT_TIMEOUT':'LAYOUT_COMMAND_FAILED');
   if(negative){const n=JSON.parse(fs.readFileSync(path.join(output,'negative/report.json'),'utf8'));negativeDetected=n.passed===false&&n.cases.length===1&&n.cases[0].regression?.states?.length===28&&n.cases[0].regression.failures.some(f=>f.code==='CONTROLS_OVERLAP');if(!negativeDetected)throw Error('LAYOUT_NEGATIVE_CONTROL_NOT_DETECTED')}
  }
  const summary=summarizeLayoutReport(JSON.parse(fs.readFileSync(path.join(output,'report.json'),'utf8')));summary.negativeControl={passed:negativeDetected,expectedOverlapRejected:negativeDetected};save(summary);return summary;
 }catch(error){const code=/^LAYOUT_[A-Z_]+$/.test(error.message)?error.message:'LAYOUT_GATE_FAILED';save({schemaVersion:1,passed:false,status:'failed',error:{code}});throw Error(code)}
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){try{console.log(JSON.stringify(runLayoutGate({reportFile:process.env.ATMOS_LAYOUT_REPORT||undefined})))}catch(e){console.error(e.message+' (set ATMOS_TEST_CHROME to a supported browser if required)');process.exitCode=1}}
