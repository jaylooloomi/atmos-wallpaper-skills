import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
const fail=code=>{throw Object.assign(Error(code),{code})};
const fingerprint=bytes=>createHash('sha256').update(bytes).digest('hex');
function outside(file,store){
 if(!path.isAbsolute(file)||(process.platform==='win32'&&!/^[A-Za-z]:[\\/]/.test(file)))fail('INPUT_PATH_INVALID');
 const real=fs.realpathSync(file),root=fs.realpathSync(store),relative=path.relative(root,real);
 if(relative===''||relative==='.'||(!relative.startsWith('..'+path.sep)&&relative!=='..'&&!path.isAbsolute(relative)))fail('INTERNAL_STORE_INPUT_DENIED');
}
// Read the exact bytes that are later parsed and submitted. No second command-file read.
export function watchSnapshot(file,store){
 outside(file,store);if(!fs.lstatSync(file).isFile())fail('INPUT_NOT_REGULAR');let fd;
 try{fd=fs.openSync(file,fs.constants.O_RDONLY|fs.constants.O_NOFOLLOW|fs.constants.O_NONBLOCK);const before=fs.fstatSync(fd,{bigint:true});if(!before.isFile())fail('INPUT_NOT_REGULAR');if(before.size>32768n)fail('BODY_TOO_LARGE');const bytes=Buffer.alloc(32769);let n=0,x;while(n<bytes.length&&(x=fs.readSync(fd,bytes,n,bytes.length-n,null)))n+=x;if(n>32768)fail('BODY_TOO_LARGE');const after=fs.fstatSync(fd,{bigint:true}),current=fs.lstatSync(file,{bigint:true});if(!current.isFile()||before.dev!==after.dev||before.ino!==after.ino||before.size!==after.size||before.mtimeNs!==after.mtimeNs||after.dev!==current.dev||after.ino!==current.ino||after.size!==current.size||after.mtimeNs!==current.mtimeNs)fail('INPUT_CHANGING');return {bytes:bytes.subarray(0,n),key:fingerprint(bytes.subarray(0,n))};}
 finally{if(fd!==undefined)fs.closeSync(fd)}
}
function codeOf(e){return typeof e?.code==='string'&&/^[A-Z][A-Z0-9_]{0,79}$/.test(e.code)?e.code:'LOCAL_OPERATION_FAILED'}
export class FileCommandWatcher{
 constructor({file,store,settleMs=750,apply,emit=()=>{}}){this.file=file;this.store=store;this.settleMs=settleMs;this.apply=apply;this.emit=emit;this.candidate=null;this.attempted=null;this.readError=null;this.retry=0;this.retryAt=0;}
 async poll(now=Date.now()){
  let snapshot;try{snapshot=watchSnapshot(this.file,this.store);this.readError=null;}catch(e){const code=codeOf(e);this.candidate=null;if(this.readError!==code){this.readError=code;this.emit({event:'input-error',ok:false,error:{code,retryable:code==='ENOENT'||code==='INPUT_CHANGING'},committed:false})}return;}
  if(snapshot.key!==this.candidate?.key){this.candidate={...snapshot,since:now};this.retry=0;this.retryAt=0;return;}
  if(now-this.candidate.since<this.settleMs||snapshot.key===this.attempted||now<this.retryAt)return;
  let command;try{command=JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(snapshot.bytes));}catch{this.attempted=snapshot.key;this.emit({event:'input-error',inputHash:snapshot.key,ok:false,error:{code:'INVALID_JSON',retryable:false},committed:false});return;}
  try{const result=await this.apply(command);this.attempted=snapshot.key;this.retry=0;this.emit({event:'result',inputHash:snapshot.key,...result});}
  catch(e){const code=codeOf(e);if(code==='STORE_LOCKED'&&this.retry<3){this.retryAt=now+250*2**this.retry++;return;}this.attempted=snapshot.key;this.emit({event:'command-error',inputHash:snapshot.key,ok:false,error:{code,retryable:code==='STORE_LOCKED'},committed:null,note:'Inspect receipt after an unknown outcome. Conflicts require a new reconciled intent; the watcher never rewrites revisions.'});}
 }
}
export async function watchCommandFile(options,{apply,emit=()=>{},signal,now=()=>Date.now(),sleep}={}){
 const{file,store,pollMs=250,settleMs=750,durationMs=0}=options;
 // Validate the configured path before waiting; a missing file may be created later.
 if(!path.isAbsolute(file)||(process.platform==='win32'&&!/^[A-Za-z]:[\\/]/.test(file)))fail('INPUT_PATH_INVALID');
 const root=fs.realpathSync(store),parent=fs.realpathSync(path.dirname(file)),relative=path.relative(root,parent);
 if(relative===''||(!relative.startsWith('..'+path.sep)&&relative!=='..'&&!path.isAbsolute(relative)))fail('INTERNAL_STORE_INPUT_DENIED');
 const watcher=new FileCommandWatcher({file,store,settleMs,apply,emit});const started=now();emit({apiVersion:1,event:'watch-started',ok:true,pollMs,settleMs,durationMs,autoStart:false});
 const pause=sleep||((ms)=>new Promise(resolve=>{const timer=setTimeout(done,ms);function done(){clearTimeout(timer);signal?.removeEventListener('abort',done);resolve()}signal?.addEventListener('abort',done,{once:true});if(signal?.aborted)done()}));
 while(!signal?.aborted&&(!durationMs||now()-started<durationMs)){await watcher.poll(now());if(signal?.aborted)break;const remaining=durationMs?durationMs-(now()-started):pollMs;if(remaining<=0)break;await pause(Math.min(pollMs,remaining));}
 const stopped={apiVersion:1,event:'watch-stopped',ok:true,reason:signal?.aborted?'signal':'duration',elapsedMs:Math.max(0,now()-started)};emit(stopped);return stopped;
}
