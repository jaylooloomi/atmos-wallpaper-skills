import fs from 'node:fs';
import path from 'node:path';
import {spawn,spawnSync} from 'node:child_process';

// Only terminate the process tree created by this invocation, never process names.
function terminate(child){
 if(!child.pid)return;
 if(process.platform==='win32')spawnSync(path.join(process.env.SystemRoot||'C:/Windows','System32/taskkill.exe'),['/PID',String(child.pid),'/T','/F'],{windowsHide:true,stdio:'ignore'});
 else child.kill('SIGKILL');
}
export function atomicJson(file,value){
 fs.mkdirSync(path.dirname(file),{recursive:true});const temporary=file+'.'+process.pid+'.tmp';
 fs.writeFileSync(temporary,JSON.stringify(value,null,2)+'\n');fs.renameSync(temporary,file);
}
export async function runProcess(executable,args,{cwd,env=process.env,timeout=120000,logFile,onOutput,signal,stream=true,maxBuffer=8*1024*1024}={}){
 const start=performance.now();if(logFile)fs.mkdirSync(path.dirname(logFile),{recursive:true});
 const log=logFile?fs.openSync(logFile,'w'):null;const output={stdout:[],stderr:[]};let outputBytes=0,error=null,timedOut=false,aborted=false,overflow=false;
 return new Promise(resolve=>{
  const child=spawn(executable,args,{cwd,env,windowsHide:true,stdio:['ignore','pipe','pipe']});
  const cancel=()=>{aborted=true;terminate(child)};const interrupt=()=>cancel();
  const timer=setTimeout(()=>{timedOut=true;terminate(child)},timeout);
  signal?.addEventListener('abort',cancel,{once:true});process.once('SIGINT',interrupt);process.once('SIGTERM',interrupt);
  if(signal?.aborted)cancel();
  const consume=(kind,bytes)=>{
   if(log!==null)fs.writeSync(log,bytes);if(stream)(kind==='stdout'?process.stdout:process.stderr).write(bytes);
   onOutput?.(kind,bytes);
   outputBytes+=bytes.length;if(outputBytes>maxBuffer){if(!overflow){overflow=true;terminate(child)}return}
   output[kind].push(bytes);
  };
  child.stdout.on('data',b=>consume('stdout',b));child.stderr.on('data',b=>consume('stderr',b));
  child.once('error',e=>{error={code:e.code||'SPAWN_ERROR',message:e.message}});
  child.once('close',(code,exitSignal)=>{
   clearTimeout(timer);signal?.removeEventListener('abort',cancel);process.removeListener('SIGINT',interrupt);process.removeListener('SIGTERM',interrupt);
   if(log!==null)fs.closeSync(log);
   if(timedOut)error={code:'ETIMEDOUT'};else if(aborted)error={code:'ABORTED'};else if(overflow)error={code:'MAX_BUFFER'};
   resolve({status:code,signal:exitSignal,stdout:Buffer.concat(output.stdout).toString(),stderr:Buffer.concat(output.stderr).toString(),error,timedOut,aborted,elapsedMs:Math.round(performance.now()-start)});
  });
 });
}
