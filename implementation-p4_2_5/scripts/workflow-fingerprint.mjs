import fs from 'node:fs';import path from 'node:path';import {createHash} from 'node:crypto';import {spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';
const repo=fileURLToPath(new URL('../../',import.meta.url));
export function workflowTools(env=process.env){
 const pkg=JSON.parse(fs.readFileSync(path.join(repo,'implementation-p4_2_5/package-lock.json')));
 const python=env.ATMOS_PYTHON?spawnSync(env.ATMOS_PYTHON,['-B','-c','import sys,PIL;print(sys.version.split()[0]+" / Pillow "+PIL.__version__)'],{encoding:'utf8',windowsHide:true,timeout:10000}):null;
 const browser=(env.ATMOS_TEST_CHROME?[env.ATMOS_TEST_CHROME]:['C:/Program Files/Google/Chrome/Application/chrome.exe','C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe','/usr/bin/chromium']).find(p=>fs.existsSync(p));
 let browserVersion=null;if(browser&&process.platform==='win32'){
  const code='(Get-Item -LiteralPath '+"'"+browser.replaceAll("'","''")+"'"+').VersionInfo.FileVersion';
  const r=spawnSync(path.join(env.SystemRoot||'C:/Windows','System32/WindowsPowerShell/v1.0/powershell.exe'),['-NoProfile','-NonInteractive','-EncodedCommand',Buffer.from(code,'utf16le').toString('base64')],{encoding:'utf8',windowsHide:true,timeout:10000});if(r.status===0)browserVersion=r.stdout.trim();
 }
 return {node:process.version,pythonPillow:python?.status===0?python.stdout.trim():null,bundler:pkg.packages?.['node_modules/esbuild']?.version||null,browserVersion,browserBinarySha256:browser?createHash('sha256').update(fs.readFileSync(browser)).digest('hex'):null,browserFlags:['headless=new','virtual-time-budget=24000','force-device-scale-factor=1'],cacheReuseEnabled:false};
}
export function workflowFingerprint(){
 const git=process.env.ATMOS_GIT||(process.platform==='win32'?'C:/Program Files/Git/cmd/git.exe':'git');
 const result=spawnSync(git,['ls-files','--cached','--others','--exclude-standard','-z'],{cwd:repo,encoding:'utf8',windowsHide:true,maxBuffer:16*1024*1024});if(result.status!==0)throw Error('WORKFLOW_INPUT_LIST_FAILED');
 const files=[...new Set(result.stdout.split('\0').filter(p=>p&&(p==='AGENTS.md'||/^implementation-(p1|p4_2_1|p4_2_5)\/(src|scripts|tests)\//.test(p)||p.startsWith('prototype-v8.1-frozen/')||/^(implementation-p4_2_5|desktop-host)\/package(-lock)?\.json$/.test(p)||/^desktop-host\/(scripts|src|installer)\//.test(p)||/^desktop-host\/(Cargo\.(toml|lock)|tauri\.conf\.json|build\.rs)$/.test(p))))].sort();
 const hash=createHash('sha256');for(const p of files){const file=path.join(repo,p);if(!fs.existsSync(file)){hash.update(p+'\0MISSING\0');continue}if(!fs.lstatSync(file).isFile()||fs.lstatSync(file).isSymbolicLink())throw Error('UNSAFE_WORKFLOW_INPUT');hash.update(p+'\0');hash.update(fs.readFileSync(file));hash.update('\0')}
 return {sha256:hash.digest('hex'),files:files.length,node:process.version,cacheHit:false,scope:'Working-tree source, tests, scripts, frozen prototype and dependency lockfiles; telemetry only, not a cache authorization'};
}
