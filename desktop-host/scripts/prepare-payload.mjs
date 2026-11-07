import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';

import {assertVerified,verifyWorkflow,disposeVerification} from '../../implementation-p4_2_5/scripts/workflow.mjs';
import {payloadPolicy} from './payload-integrity.mjs';
export async function preparePayload({proof}={}){
const verified=assertVerified(proof);
if(process.platform!=='win32')throw Error('Windows x64 payload build only');
const root=fileURLToPath(new URL('../',import.meta.url)),repo=path.dirname(root);
const payload=path.join(root,'payload');payloadPolicy.invalidate(payload);const cache=path.join(process.env.LOCALAPPDATA,'AtmosWallpaper/installer-build-cache');
const python=process.env.ATMOS_BUILD_PYTHON||'C:\\Python314\\python.exe';
const cargo=path.join(process.env.USERPROFILE,'.cargo/bin/cargo.exe');
const pins=JSON.parse(fs.readFileSync(path.join(root,'installer/runtime-lock.json')));
const sha=b=>createHash('sha256').update(b).digest('hex');
function run(exe,args,options={}){const r=spawnSync(exe,args,{encoding:'utf8',windowsHide:true,maxBuffer:32*1024*1024,...options});if(r.status!==0)throw Error((r.stdout||'')+(r.stderr||'')+(r.error||''));return r.stdout}
function copy(from,to){fs.mkdirSync(path.dirname(to),{recursive:true});fs.copyFileSync(from,to)}
function walk(dir,prefix=''){return fs.readdirSync(dir,{withFileTypes:true}).flatMap(e=>{if(e.isSymbolicLink())throw Error('Symlink in payload');const p=path.posix.join(prefix,e.name);return e.isDirectory()?walk(path.join(dir,e.name),p):[p]}).sort()}
async function download(pin){
 const name=path.basename(new URL(pin.url).pathname),file=path.join(cache,name);
 if(!fs.existsSync(file)){const response=await fetch(pin.url,{signal:AbortSignal.timeout(180000)});if(!response.ok)throw Error('Download failed: '+pin.url);fs.writeFileSync(file,Buffer.from(await response.arrayBuffer()))}
 if(sha(fs.readFileSync(file))!==pin.sha256)throw Error('Pinned archive hash mismatch: '+name);
 return file;
}
function extract(zip,destination){
 run(python,['-I','-B','-c',String.raw`import pathlib,sys,zipfile
source,dest=sys.argv[1:]; dest=pathlib.Path(dest); dest.mkdir(parents=True,exist_ok=True)
with zipfile.ZipFile(source) as z:
 for name in z.namelist():
  p=pathlib.PurePosixPath(name)
  if p.is_absolute() or '..' in p.parts or any(':' in part or '\\' in part for part in p.parts): raise ValueError('Unsafe archive path')
  if name.endswith('/'): continue
  target=dest.joinpath(*p.parts)
  target.parent.mkdir(parents=True,exist_ok=True); target.write_bytes(z.read(name))
`,zip,destination]);
}
fs.mkdirSync(cache,{recursive:true});
// Recursive removal is restricted to this exact generated directory; never a
// caller-supplied path, parent, junction or runtime/user data directory.
if(path.resolve(payload)!==path.resolve(root,'payload')||fs.realpathSync(root)!==path.resolve(root).replace(/[\\/]$/,''))throw Error('Unexpected payload root');
if(fs.existsSync(payload)){if(fs.lstatSync(payload).isSymbolicLink())throw Error('Refuse payload junction');fs.rmSync(payload,{recursive:true})}
fs.mkdirSync(payload,{recursive:true});
copy(path.join(root,'installer/CPAL-0.18.2-LICENSE.txt'),path.join(payload,'licenses/CPAL-0.18.2-LICENSE.txt'));
const downloads=await Promise.all(Object.entries(pins).filter(([k])=>k!=='schemaVersion').map(async([k,p])=>[k,await download(p)]));
const archives=Object.fromEntries(downloads);
extract(archives.python,path.join(payload,'runtime/python'));
extract(archives.pillow,path.join(payload,'runtime/python/site-packages'));
fs.writeFileSync(path.join(payload,'runtime/python/python314._pth'),'python314.zip\n.\nsite-packages\n');
const product=path.join(payload,'app/implementation-p4_2_5');
for(const file of ['implementation-p1/src/decode-image.py','prototype-v8.1-frozen/manifest.json','SOURCE-MANIFEST.json']){
 copy(path.join(repo,file),path.join(payload,'app',file));
}
fs.cpSync(path.join(repo,'prototype-v8.1-frozen/assets'),path.join(payload,'app/prototype-v8.1-frozen/assets'),{recursive:true});
fs.cpSync(verified.artifact.dist,path.join(product,'dist'),{recursive:true});
for(const file of walk(path.join(product,'dist')))fs.chmodSync(path.join(product,'dist',file),0o644);
// Isolated embedded Python must be able to import its bundled Pillow codecs.
const embedded=path.join(payload,'runtime/python/python.exe');
const check=run(embedded,['-I','-B','-c','import sys,PIL;from PIL import Image,features;assert features.check("webp");print(sys.version.split()[0]+" / Pillow "+PIL.__version__)'],{env:{SystemRoot:process.env.SystemRoot,PATH:path.join(process.env.SystemRoot,'System32')}}).trim();
const notices=path.join(payload,'licenses');fs.mkdirSync(notices,{recursive:true});
copy(path.join(root,'installer/NOTICE.txt'),path.join(notices,'NOTICE.txt'));
copy(path.join(payload,'runtime/python/LICENSE.txt'),path.join(notices,'PYTHON-LICENSE.txt'));
const metadata=JSON.parse(run(cargo,['metadata','--locked','--format-version','1','--filter-platform','x86_64-pc-windows-msvc'],{cwd:root}));
const licenses=[];
for(const pkg of metadata.packages.filter(p=>p.source?.startsWith('registry+'))){
 const dir=path.dirname(pkg.manifest_path),names=fs.readdirSync(dir).filter(n=>/^(LICENSE|COPYING|NOTICE)([.-].*)?$/i.test(n)&&fs.statSync(path.join(dir,n)).isFile());
 if(pkg.license_file){const local=path.resolve(dir,pkg.license_file);if(local.startsWith(dir+path.sep)&&fs.existsSync(local)&&!names.includes(pkg.license_file))names.push(pkg.license_file)}
 for(const name of names)copy(path.join(dir,name),path.join(notices,'rust',pkg.name+'-'+pkg.version,path.basename(name)));
 licenses.push({name:pkg.name,version:pkg.version,license:pkg.license,notices:names.map(n=>'rust/'+pkg.name+'-'+pkg.version+'/'+path.basename(n))});
}
fs.writeFileSync(path.join(notices,'RUST-DEPENDENCIES.json'),JSON.stringify(licenses,null,2)+'\n');
copy(path.join(root,'installer/runtime-lock.json'),path.join(notices,'RUNTIME-SOURCES.json'));
const manifest={schemaVersion:1,architecture:'x86_64',versions:{python:pins.python.version,pillow:pins.pillow.version}};
const sealed=payloadPolicy.seal(proof,payload,manifest);
console.log(JSON.stringify({payloadPrepared:true,files:sealed.files.length,bytes:sealed.files.reduce((s,x)=>s+x.bytes,0),embeddedPython:check,systemRuntimeRequired:false}));

return {payload,files:sealed.files.length};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){let proof;try{({proof}=await verifyWorkflow());await preparePayload({proof})}catch(e){console.error(e.message);process.exitCode=1}finally{if(proof)disposeVerification(proof)}}
