import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
const root=fileURLToPath(new URL('../',import.meta.url));
const env={...process.env,PATH:path.join(process.env.USERPROFILE,'.cargo/bin')+path.delimiter+path.dirname(process.execPath)+path.delimiter+process.env.PATH,CARGO_BUILD_JOBS:process.env.CARGO_BUILD_JOBS||'6'};
function run(exe,args){const result=spawnSync(exe,args,{cwd:root,env,stdio:'inherit',windowsHide:true});if(result.status!==0)process.exit(result.status||1)}
if(!process.argv.includes('--reuse-payload'))run(process.execPath,['scripts/prepare-payload.mjs']);
run(process.execPath,['node_modules/@tauri-apps/cli/tauri.js','build','--bundles','nsis','--','--locked']);
const dir=path.join(root,'target/release/bundle/nsis');
const config=JSON.parse(fs.readFileSync(path.join(root,'tauri.conf.json')));
const artifacts=fs.readdirSync(dir).filter(n=>n===config.productName+'_'+config.version+'_x64-setup.exe').map(name=>{const b=fs.readFileSync(path.join(dir,name));return {name,bytes:b.length,sha256:createHash('sha256').update(b).digest('hex')}});
if(artifacts.length!==1)throw Error('Expected one NSIS setup executable');
fs.writeFileSync(path.join(dir,'SHA256SUMS.txt'),artifacts.map(x=>x.sha256+'  '+x.name).join('\n')+'\n');
console.log(JSON.stringify({windowsInstallers:artifacts,unsigned:true}));
