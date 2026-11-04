import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';

const root=fileURLToPath(new URL('../',import.meta.url));
const evidence=path.join(root,'evidence');
fs.mkdirSync(evidence,{recursive:true});
const env={...process.env};
if(process.platform==='win32'&&!env.ATMOS_PYTHON){
 const python=spawnSync('python',['-c','import sys; print(sys.executable)'],{encoding:'utf8',windowsHide:true});
 if(python.status===0)env.ATMOS_PYTHON=python.stdout.trim();
}
const checks=[];
for(const [name,cwd,args] of [
 ['prepare',root,['scripts/prepare.mjs','--assets-only']],
 ['product-tests',root,['--test','--test-reporter=tap','tests/*.test.mjs']],
 ['core-tests',path.resolve(root,'../implementation-p1'),['--test','--test-reporter=tap','tests/*.test.mjs']],
 ['draft-review',root,['tests/independent-draft-review.mjs']],
 ['path-review',root,['tests/independent-path-review.mjs']],
 ['cli-help',root,['bin/atmos.mjs','--help']],
 ...(!process.argv.includes('--unit-only')?[["layout-regression",root,['scripts/verify-layout.mjs']]]:[]),
]){
 const result=spawnSync(process.execPath,args,{cwd,env,encoding:'utf8',windowsHide:true,timeout:name==='layout-regression'?720000:120000,maxBuffer:8*1024*1024});
 fs.writeFileSync(path.join(evidence,name+'.log'),(result.stdout||'')+(result.stderr||'')+(result.error?String(result.error):''));
 const counts=Object.fromEntries([...((result.stdout||'').matchAll(/^# (tests|pass|fail|skipped) (\d+)$/gm))].map(m=>[m[1],Number(m[2])]));
 const check={name,exitCode:result.status,...counts};checks.push(check);console.log(JSON.stringify(check));
}
const summary={layoutRequired:!process.argv.includes('--unit-only'),platform:process.platform,node:process.version,checks,passed:checks.every(c=>c.exitCode===0)};
fs.writeFileSync(path.join(evidence,'verification.json'),JSON.stringify(summary,null,2)+'\n');
if(!summary.passed)process.exitCode=1;
