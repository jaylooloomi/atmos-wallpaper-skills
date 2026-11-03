// Optional developer migration check: old locally available Pillow -> bundled
// Pillow. The installed product never calls or needs the legacy executable.
import fs from 'node:fs';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {spawnSync} from 'node:child_process';
import assert from 'node:assert/strict';
const [payload,scratch,legacyPython]=process.argv.slice(2);
if(!legacyPython)throw Error('Explicit legacy Python executable required for compatibility test');
const {ControlCore}=await import(pathToFileURL(path.join(payload,'app/implementation-p4_2_5/src/portable-core.mjs')));
const bundledPython=path.join(payload,'runtime/python/python.exe');
const version=exe=>{const r=spawnSync(exe,[...(exe===legacyPython?[]:['-I']),'-B','-c','import PIL;print(PIL.__version__)'],{encoding:'utf8',windowsHide:true});assert.equal(r.status,0,r.stderr);return r.stdout.trim()};
const formats=[];
for(const format of ['PNG','JPEG','WEBP']){
 const old=new ControlCore(path.join(scratch,'old-'+format),{pythonExecutable:legacyPython});old.init();
 const fixture=spawnSync(legacyPython,['-B','-c',"import sys,io;from PIL import Image;im=Image.new('RGB',(128,96));im.putdata([(x*2,y*2,(x+y)%256) for y in range(96) for x in range(128)]);b=io.BytesIO();im.save(b,format=sys.argv[1]);sys.stdout.buffer.write(b.getvalue())",format],{windowsHide:true});assert.equal(fixture.status,0,fixture.stderr?.toString());
 fs.writeFileSync(path.join(old.store.root,'imports/fixture.bin'),fixture.stdout);
 const asset=old.stage({requestId:'old-stage',file:'fixture.bin'}).asset;
 const p=old.preview({requestId:'old-preview',expectedRevision:0,patch:{assetId:asset.assetId}});old.apply({requestId:'old-apply',expectedRevision:0,previewId:p.previewId});
 const backup=JSON.parse(fs.readFileSync(old.backup().path));
 const current=new ControlCore(path.join(scratch,'new-'+format),{pythonExecutable:bundledPython});current.init();
 const restore=current.restoreBackup({requestId:'new-restore',expectedRevision:0,backup});current.apply({requestId:'new-apply',expectedRevision:0,previewId:restore.previewId});
 assert.equal(current.status().desired.assetId,asset.assetId);assert.deepEqual(current.asset(asset.assetId).data,old.asset(asset.assetId).data);formats.push(format);
}
console.log(JSON.stringify({legacyPillow:version(legacyPython),bundledPillow:version(bundledPython),backupAssetHashesPreserved:true,formats}));
