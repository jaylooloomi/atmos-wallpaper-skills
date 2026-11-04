import {hostContract} from './host-contract.mjs';
import fs from 'node:fs';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {ControlCore} from './portable-core.mjs';
import {commandPatch,executeCommand} from './commands.mjs';
import {hash,mergePatch,requestId} from './contract.mjs';
import {createScopeRegistry,readLayout,displayId} from './scope-registry.mjs';
import {watchCommandFile} from './agent-watch.mjs';
import {agentHelp,commandSchema} from './agent-contract.mjs';

const fail=code=>{throw Object.assign(Error(code),{code})};
const actions=new Map([['--agent-help',0],['--agent-schema',0],['--host-schema',0],['--get-state',0],['--apply-json',1],['--validate-json',1],['--get-result',1],['--watch-json',1]]);
export function parseAgentArgs(args){
 const action=args[0];if(!actions.has(action))fail('ARGUMENT_INVALID');let at=1,value;
 if(actions.get(action)){value=args[at++];if(!value||value.startsWith('--'))fail('ARGUMENT_INVALID')}
 let store,target,wait=action==='--apply-json'?3000:0,pollMs=250,settleMs=750,durationMs=0;const seen=new Set();
 while(at<args.length){const key=args[at++],v=args[at++];if(seen.has(key)||v===undefined)fail('ARGUMENT_INVALID');seen.add(key);
  if(key==='--store'){if(!path.isAbsolute(v))fail('ARGUMENT_INVALID');store=v}
  else if(key==='--target'&&!['--agent-help','--agent-schema','--host-schema'].includes(action)){if(!displayId(v))fail('TARGET_INVALID');target=v}
  else if(key==='--wait-ms'&&['--apply-json','--get-result'].includes(action)){if(!/^\d+$/.test(v)||Number(v)>30000)fail('ARGUMENT_INVALID');wait=Number(v)}
  else if(action==='--watch-json'&&['--poll-ms','--settle-ms','--duration-ms'].includes(key)){if(!/^\d+$/.test(v)||!Number.isSafeInteger(Number(v)))fail('ARGUMENT_INVALID');const n=Number(v);if(key==='--poll-ms'){if(n<100||n>5000)fail('ARGUMENT_INVALID');pollMs=n}else if(key==='--settle-ms'){if(n<500||n>10000)fail('ARGUMENT_INVALID');settleMs=n}else{if(n<0||n>86400000)fail('ARGUMENT_INVALID');durationMs=n}}
  else fail('ARGUMENT_INVALID');
 }
 if(action==='--watch-json'&&settleMs<2*pollMs)fail('ARGUMENT_INVALID');
 return {action,value,store,wait,...(target?{target}:{}),...(action==='--watch-json'?{pollMs,settleMs,durationMs}:{})};
}
export function readCommandFile(file,store){
 if(!path.isAbsolute(file)||(process.platform==='win32'&&!/^[A-Za-z]:[\\/]/.test(file)))fail('INPUT_PATH_INVALID');
 let fd;try{
  if(!fs.lstatSync(file).isFile())fail('INPUT_NOT_REGULAR');
  const real=fs.realpathSync(file),root=fs.realpathSync(store),relative=path.relative(root,real);
  if(relative===''||(!relative.startsWith('..'+path.sep)&&relative!=='..'&&!path.isAbsolute(relative)))fail('INTERNAL_STORE_INPUT_DENIED');
  fd=fs.openSync(file,fs.constants.O_RDONLY|fs.constants.O_NOFOLLOW|fs.constants.O_NONBLOCK);const st=fs.fstatSync(fd);
  if(!st.isFile())fail('INPUT_NOT_REGULAR');if(st.size>32768)fail('BODY_TOO_LARGE');
  const b=Buffer.alloc(32769);let size=0,n;while(size<b.length&&(n=fs.readSync(fd,b,size,b.length-size,null)))size+=n;if(size>32768)fail('BODY_TOO_LARGE');
  try{return JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(b.subarray(0,size)))}catch{fail('INVALID_JSON')}
 }finally{if(fd!==undefined)fs.closeSync(fd)}
}
function status(core,id,now,routing={}){return core.store.locked(()=>{
 const live=core.store.live();let receipt=null;
 if(id){requestId(id);if(id.length>80)fail('INVALID_REQUEST_ID');const key='ui.'+id+'.apply';receipt=Object.hasOwn(live.receipts,key)?live.receipts[key]:null;const file='requests/'+hash(key)+'.json';if(!receipt&&fs.existsSync(core.store.path(file)))receipt=core.store.read(file);if(receipt?.uncertain)fail('RECOVERED_OUTCOME_UNKNOWN')}
 const target=id?receipt?.result:live;const revision=target?.revision??null,desiredHash=target?.desiredHash??null;
 const superseded=!!target&&(revision!==live.revision||desiredHash!==live.desiredHash);
 const reports=(routing.expectedTargets||[null]).map(targetId=>{let observation=null;try{observation=core.store.read(targetId?'observations/wallpaper-'+targetId+'.json':'observations/wallpaper.json')}catch{}const age=now-Date.parse(observation?.capturedAt);const expectedId=targetId||routing.targetId;const matched=!!target&&!superseded&&observation?.rendererRole==='wallpaper'&&observation?.source==='local-bridge'&&observation?.stateApplied===true&&observation.revision===revision&&observation.desiredHash===desiredHash&&age>=0&&age<4000&&(!expectedId||(observation.targetId===expectedId&&typeof observation.hostSessionId==='string'&&observation.hostSessionId.length>0));return {targetId:expectedId||null,acknowledged:matched,capturedAt:matched?observation.capturedAt:null};});
 const ack=reports.length>0&&reports.every(r=>r.acknowledged),capturedAt=ack?reports.map(r=>r.capturedAt).sort()[0]:null;
 return {apiVersion:1,ok:true,...(id?{requestId:id,found:!!receipt,committed:!!receipt}:{desired:live.desired}),revision,desiredHash,currentRevision:live.revision,superseded,rendererAcknowledged:ack,acknowledgement:{source:ack?'wallpaper-renderer':'none',capturedAt:capturedAt,freshForMs:4000,trust:'renderer-reported; not OS or visual verification'},hostVisibility:'unknown',...(routing.mode?{targetMode:routing.mode,targetScope:routing.scope,selectedTarget:routing.targetId||null,targetAcknowledgements:reports}:{})};
})}
export async function runAgent(args,{defaultStore,now=()=>Date.now(),sleep=ms=>new Promise(r=>setTimeout(r,ms)),signal,emit=()=>{}}={}){
 const options=parseAgentArgs(args);
 if(options.action==='--agent-help')return {apiVersion:1,ok:true,guide:agentHelp};
 if(options.action==='--agent-schema')return commandSchema;
 if(options.action==='--host-schema')return hostContract;
 const store=options.store||defaultStore;if(!store)fail('STORE_UNAVAILABLE');let core=new ControlCore(store);core.init();const layout=readLayout(core.store.root).layout;let routing={mode:layout.mode,scope:'shared'};
 if(layout.mode==='independent'){if(!options.target)fail('TARGET_REQUIRED');if(!layout.targets.includes(options.target))fail('TARGET_INACTIVE');core=createScopeRegistry(store).ensure(options.target);routing={mode:layout.mode,scope:options.target,targetId:options.target};}
 else if(['duplicate','span'].includes(layout.mode)){if(options.target&&!layout.targets.includes(options.target))fail('TARGET_INACTIVE');routing={mode:layout.mode,scope:'shared',expectedTargets:layout.targets};}
 else if(options.target)fail('TARGET_INACTIVE');
 if(options.action==='--watch-json')return watchCommandFile({file:options.value,store,pollMs:options.pollMs,settleMs:options.settleMs,durationMs:options.durationMs},{signal,emit,now,apply:command=>{if(JSON.stringify(readLayout(store).layout)!==JSON.stringify(layout))fail('TARGET_SCOPE_CHANGED');commandPatch(command);const result=executeCommand(core,command);return {...status(core,command.requestId,now(),routing),replayed:result.replayed===true,waitedMs:0}}});
 if(options.action==='--get-state')return status(core,null,now(),routing);
 let id=options.value,replayed=false;
 if(['--apply-json','--validate-json'].includes(options.action)){
  const command=readCommandFile(options.value,store),patch=commandPatch(command);id=command.requestId;
  if(options.action==='--validate-json')return core.store.locked(()=>{const live=core.store.live();core.assertRevision(live,command.expectedRevision);mergePatch(live.desired,patch);return {apiVersion:1,ok:true,valid:true,committed:false,requestId:id,revision:live.revision,note:'Merged-state and revision validation only; not reserved and not a receipt replay check.'}});
  replayed=executeCommand(core,command).replayed===true;
 }
 const start=now();let result=status(core,id,now(),routing);
 while(result.committed&&!result.rendererAcknowledged&&!result.superseded&&now()-start<options.wait){await sleep(Math.min(200,options.wait-(now()-start)));result=status(core,id,now(),routing)}
 return {...result,...(options.action==='--apply-json'?{replayed}:{}),waitedMs:Math.max(0,now()-start)};
}
export function agentError(error){
 const code=typeof error?.code==='string'?error.code:'LOCAL_OPERATION_FAILED';
 const conflict=['TARGET_SCOPE_CHANGED','REVISION_CONFLICT','REQUEST_CONFLICT','RECOVERED_OUTCOME_UNKNOWN','PREVIEW_STALE','PREVIEW_EXPIRED','PREVIEW_CANCELLED'];
 const invalid=['TARGET_REQUIRED','TARGET_INVALID','TARGET_INACTIVE','ARGUMENT_INVALID','INPUT_PATH_INVALID','INPUT_NOT_REGULAR','INTERNAL_STORE_INPUT_DENIED','BODY_TOO_LARGE','INVALID_JSON','COMMAND_INVALID','INVALID_REQUEST_ID','INVALID_REVISION','UNKNOWN_FIELD','INVALID_STATE','EFFECT_REMOVED','FIXED_SETTING'];
 const exitCode=code==='STORE_LOCKED'?4:conflict.includes(code)?3:invalid.includes(code)?2:1;
 return {exitCode,output:{apiVersion:1,ok:false,error:{code:exitCode===1?'LOCAL_OPERATION_FAILED':code,retryable:code==='STORE_LOCKED'},committed:null,note:'No success claimed. Query --get-result before retrying an unknown commit outcome.'}};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
 if(process.env.ATMOS_AGENT_WATCH_GATE==='1'){const {createInterface}=await import('node:readline');const input=createInterface({input:process.stdin});let opened=false;for await(const line of input){opened=line==='start';break}input.close();if(!opened){console.log(JSON.stringify({apiVersion:1,ok:false,error:{code:'WATCH_PARENT_CLOSED'}}));process.exit(1)}}
 const controller=new AbortController(),stop=()=>controller.abort();process.once('SIGINT',stop);process.once('SIGTERM',stop);
 try{const defaultStore=process.env.LOCALAPPDATA?path.join(process.env.LOCALAPPDATA,'io.atmos.wallpaper/store'):undefined;const args=process.argv.slice(2),result=await runAgent(args,{defaultStore,signal:controller.signal,emit:value=>console.log(JSON.stringify({apiVersion:1,...value}))});if(args[0]!=='--watch-json')console.log(JSON.stringify(result))}catch(e){const failure=agentError(e);console.log(JSON.stringify(failure.output));process.exitCode=failure.exitCode}finally{process.removeListener('SIGINT',stop);process.removeListener('SIGTERM',stop)}
}
