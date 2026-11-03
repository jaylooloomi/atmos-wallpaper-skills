// Run with the INSTALLED bundled Node, a minimal PATH, and only synthetic data.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {spawnSync} from 'node:child_process';
const [payload,scratch]=process.argv.slice(2);
const python=path.join(payload,'runtime/python/python.exe');
const product=path.join(payload,'app/implementation-p4_2_5');
const {ControlCore}=await import(pathToFileURL(path.join(product,'src/portable-core.mjs')));
const {createCalendarService}=await import(pathToFileURL(path.join(product,'src/calendar-service.mjs')));
const core=new ControlCore(scratch,{pythonExecutable:python});core.init();
const formats=[];let last;
for(const format of ['PNG','JPEG','WEBP']){
 const image=spawnSync(python,['-I','-B','-c',"import io,sys;from PIL import Image;b=io.BytesIO();Image.new('RGB',(8,6),(20,80,160)).save(b,format=sys.argv[1]);sys.stdout.buffer.write(b.getvalue())",format],{windowsHide:true,maxBuffer:1024*1024});assert.equal(image.status,0,image.stderr?.toString());
 const file='測試 image.'+format.toLowerCase();fs.writeFileSync(path.join(scratch,'imports',file),image.stdout);
 const staged=core.stage({requestId:'stage-'+format,file});assert.equal(staged.asset.width,8);assert.equal(staged.asset.height,6);last=staged.asset.assetId;formats.push(format);
}
const preview=core.preview({requestId:'preview-asset',expectedRevision:0,patch:{assetId:last}});
core.apply({requestId:'apply-asset',expectedRevision:0,previewId:preview.previewId});
const backup=core.backup();
const restored=core.restoreBackup({requestId:'restore-asset',expectedRevision:1,backup:JSON.parse(fs.readFileSync(backup.path))});assert.ok(restored.previewId);
const next=new Date(Date.now()+86400000).toISOString().slice(0,10).replaceAll('-','');
const text=['BEGIN:VCALENDAR','VERSION:2.0','PRODID:-//Atmos//Installer test//EN','BEGIN:VEVENT','UID:installer-test','DTSTAMP:20261103T000000Z','DTSTART;VALUE=DATE:'+next,'SUMMARY:安裝版日曆測試','END:VEVENT','END:VCALENDAR',''].join('\r\n');
const calendar=createCalendarService({store:core.store});await calendar.change('import',{name:'Synthetic install test',text});
const reopened=createCalendarService({store:core.store});const events=await reopened.events();assert.equal(events.configured,true);assert.equal(events.events.length,1);
assert.ok(!fs.readFileSync(path.join(scratch,'calendar/connection.bin')).includes(Buffer.from('BEGIN:VCALENDAR')),'Windows calendar should be DPAPI encrypted');
console.log(JSON.stringify({node:process.version,pythonAndPillowBundled:true,imageFormats:formats,unicodePaths:true,assetApplyAndBackupRestore:true,icsImportWorkerAndDpapiRestart:true}));
