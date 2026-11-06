// Stage source changes first, run this tool, then stage SOURCE-MANIFEST.json.
// Hash Git's staged blobs, not CRLF working-copy bytes or generated artifacts.
import fs from 'node:fs';import path from 'node:path';import {spawnSync} from 'node:child_process';import {createHash} from 'node:crypto';import {fileURLToPath} from 'node:url';
const repo=fileURLToPath(new URL('../../',import.meta.url)),git=process.env.ATMOS_GIT||(process.platform==='win32'?'C:/Program Files/Git/cmd/git.exe':'git');
function run(args,input){const r=spawnSync(git,args,{cwd:repo,input,windowsHide:true,maxBuffer:128*1048576});if(r.status!==0)throw Error('MANIFEST_GIT_FAILED');return r.stdout;}
const entries=run(['ls-files','--stage','-z']).toString('utf8').split('\0').filter(Boolean).map(record=>{
 const match=record.match(/^(\d+) ([a-f0-9]+) (\d)\t(.+)$/);if(!match||match[3]!=='0')throw Error('MANIFEST_UNMERGED_INDEX');return{path:match[4],git_blob_sha:match[2]};
}).filter(e=>e.path!=='SOURCE-MANIFEST.json').sort((a,b)=>a.path<b.path?-1:a.path>b.path?1:0);
const bytes=run(['cat-file','--batch'],Buffer.from(entries.map(e=>e.git_blob_sha+'\n').join('')));let offset=0;
const files=entries.map(entry=>{const end=bytes.indexOf(10,offset),header=bytes.subarray(offset,end).toString().split(' ');if(end<0||header[0]!==entry.git_blob_sha||header[1]!=='blob'||!/^\d+$/.test(header[2]))throw Error('MANIFEST_BLOB_INVALID');const size=Number(header[2]),start=end+1,body=bytes.subarray(start,start+size);if(body.length!==size||bytes[start+size]!==10)throw Error('MANIFEST_BLOB_TRUNCATED');offset=start+size+1;return{path:entry.path,bytes:size,sha256:createHash('sha256').update(body).digest('hex'),git_blob_sha:entry.git_blob_sha};});
if(offset!==bytes.length)throw Error('MANIFEST_EXTRA_BYTES');
const file=path.join(repo,'SOURCE-MANIFEST.json'),manifest=JSON.parse(fs.readFileSync(file,'utf8'));manifest.files=files;fs.writeFileSync(file,JSON.stringify(manifest,null,2)+'\n');
console.log(JSON.stringify({stagedFilesRecorded:files.length,sourceReleaseProvenancePreserved:manifest.source_release_sha256}));
