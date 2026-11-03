import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';

if(process.platform!=='win32')throw Error('Windows x64 payload build only');
const root=fileURLToPath(new URL('../',import.meta.url)),repo=path.dirname(root);
const payload=path.join(root,'payload'),cache=path.join(process.env.LOCALAPPDATA,'AtmosWallpaper/installer-build-cache');
const python=process.env.ATMOS_BUILD_PYTHON||'C:\\Python314\\python.exe';
const git=process.env.ATMOS_GIT||'C:\\Program Files\\Git\\cmd\\git.exe';
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
function extract(zip,destination,mode='all'){
 run(python,['-I','-B','-c',String.raw`import pathlib,sys,zipfile
source,dest,mode=sys.argv[1:]; dest=pathlib.Path(dest); dest.mkdir(parents=True,exist_ok=True)
with zipfile.ZipFile(source) as z:
 for name in z.namelist():
  p=pathlib.PurePosixPath(name)
  if p.is_absolute() or '..' in p.parts or any(':' in part or '\\' in part for part in p.parts): raise ValueError('Unsafe archive path')
  if name.endswith('/'): continue
  if mode=='node':
   if p.name not in ('node.exe','LICENSE') or len(p.parts)!=2: continue
   target=dest/p.name
  else: target=dest.joinpath(*p.parts)
  target.parent.mkdir(parents=True,exist_ok=True); target.write_bytes(z.read(name))
`,zip,destination,mode]);
}
fs.mkdirSync(cache,{recursive:true});
// Recursive removal is restricted to this exact generated directory; never a
// caller-supplied path, parent, junction or runtime/user data directory.
if(path.resolve(payload)!==path.resolve(root,'payload')||fs.realpathSync(root)!==path.resolve(root).replace(/[\\/]$/,''))throw Error('Unexpected payload root');
if(fs.existsSync(payload)){if(fs.lstatSync(payload).isSymbolicLink())throw Error('Refuse payload junction');fs.rmSync(payload,{recursive:true})}
fs.mkdirSync(payload,{recursive:true});
const downloads=await Promise.all(Object.entries(pins).filter(([k])=>k!=='schemaVersion').map(async([k,p])=>[k,await download(p)]));
const archives=Object.fromEntries(downloads);
extract(archives.node,path.join(payload,'runtime/node'),'node');
extract(archives.python,path.join(payload,'runtime/python'));
extract(archives.pillow,path.join(payload,'runtime/python/site-packages'));
fs.writeFileSync(path.join(payload,'runtime/python/python314._pth'),'python314.zip\n.\nsite-packages\n');
// Package only source files tracked by Git, never local stores, tests or logs.
const gitList=spawnSync(git,['ls-files','-z'],{cwd:repo,encoding:'utf8',windowsHide:true});
const tracked=gitList.status===0?gitList.stdout.split('\0').filter(Boolean):JSON.parse(fs.readFileSync(path.join(repo,'SOURCE-MANIFEST.json'))).files.map(f=>f.path);
const prefixes=['implementation-p4_2_5/src/','implementation-p1/src/','implementation-p4_2_1/src/','prototype-v8.1-frozen/'];
for(const file of tracked.filter(f=>prefixes.some(p=>f.startsWith(p)))){
 if(file.includes('\\')||path.posix.normalize(file)!==file)throw Error('Unsafe published source path');
 copy(path.join(repo,file),path.join(payload,'app',file));
}
const product=path.join(payload,'app/implementation-p4_2_5');
for(const name of ['package.json','package-lock.json'])copy(path.join(repo,'implementation-p4_2_5',name),path.join(product,name));
run(process.execPath,[path.join(repo,'implementation-p4_2_5/scripts/prepare.mjs'),'--assets-only']);
fs.cpSync(path.join(repo,'implementation-p4_2_5/dist'),path.join(product,'dist'),{recursive:true});
const npmCli=path.join(path.dirname(process.execPath),'node_modules/npm/bin/npm-cli.js');
run(process.execPath,[npmCli,'ci','--omit=dev','--ignore-scripts','--no-audit','--no-fund'],{cwd:product});
// Isolated embedded Python must be able to import its bundled Pillow codecs.
const embedded=path.join(payload,'runtime/python/python.exe');
const check=run(embedded,['-I','-B','-c','import sys,PIL;from PIL import Image,features;assert features.check("webp");print(sys.version.split()[0]+" / Pillow "+PIL.__version__)'],{env:{SystemRoot:process.env.SystemRoot,PATH:path.join(process.env.SystemRoot,'System32')}}).trim();
const notices=path.join(payload,'licenses');fs.mkdirSync(notices,{recursive:true});
copy(path.join(root,'installer/NOTICE.txt'),path.join(notices,'NOTICE.txt'));
copy(path.join(payload,'runtime/node/LICENSE'),path.join(notices,'NODE-LICENSE.txt'));
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
const files=walk(payload).map(p=>{const b=fs.readFileSync(path.join(payload,p));return {path:p,bytes:b.length,sha256:sha(b)}});
const manifest={schemaVersion:1,architecture:'x86_64',versions:{node:pins.node.version,python:pins.python.version,pillow:pins.pillow.version},files};
fs.writeFileSync(path.join(payload,'payload-manifest.json'),JSON.stringify(manifest,null,2)+'\n');
console.log(JSON.stringify({payloadPrepared:true,files:files.length,bytes:files.reduce((s,x)=>s+x.bytes,0),embeddedPython:check,systemRuntimeRequired:false}));
