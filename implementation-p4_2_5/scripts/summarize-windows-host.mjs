import fs from 'node:fs';
import path from 'node:path';
const dir=process.argv[2];
if(!dir)throw Error('Usage: node scripts/summarize-windows-host.mjs ABSOLUTE_MEASUREMENT_DIRECTORY');
const read=name=>JSON.parse(fs.readFileSync(path.join(dir,name),'utf8').replace(/^\uFEFF/,''));
const lines=name=>fs.existsSync(path.join(dir,name))?fs.readFileSync(path.join(dir,name),'utf8').trim().split(/\r?\n/).filter(Boolean).map(line=>JSON.parse(line.replace(/^\uFEFF/,''))):[];
const metadata=read('metadata.json'),samples=lines('samples.jsonl');
const windowsEvents=fs.existsSync(path.join(dir,'windows-lifecycle-events.json'))?read('windows-lifecycle-events.json'):null;
const events=[...lines('events.jsonl'),...(windowsEvents?.events||[])].sort((a,b)=>Date.parse(a.at)-Date.parse(b.at));
const percentile=(values,p)=>values.length?[...values].sort((a,b)=>a-b)[Math.floor((values.length-1)*p)]:null;
const metrics={};
for(const group of ['server','wallpaper']){
 const points=samples.map(sample=>({at:Date.parse(sample.at),elapsed:sample.elapsedSeconds,processes:(sample.processes||[]).filter(p=>p.group===group)}));
 const memory=points.filter(p=>p.processes.length).map(p=>({elapsed:p.elapsed,privateMiB:p.processes.reduce((sum,x)=>sum+x.privateBytes,0)/1048576,workingSetMiB:p.processes.reduce((sum,x)=>sum+x.workingSetBytes,0)/1048576,handles:p.processes.reduce((sum,x)=>sum+x.handles,0)}));
 const cpu=[];
 for(let i=1;i<points.length;i++){
  const prev=points[i-1],current=points[i],seconds=(current.at-prev.at)/1000;
  if(seconds<=0||seconds>metadata.sampleSeconds*3||!current.processes.length)continue;
  const old=new Map(prev.processes.map(p=>[p.id+':'+p.startedAt,p.cpuSeconds]));
  let delta=0;for(const p of current.processes){const before=old.get(p.id+':'+p.startedAt);if(before!==undefined)delta+=Math.max(0,p.cpuSeconds-before)}
  cpu.push(delta/seconds*100);
 }
 const steady=memory.filter(p=>p.elapsed>=120),first=steady.slice(0,10),last=steady.slice(-10);
 metrics[group]={samples:memory.length,peakPrivateMiB:percentile(memory.map(p=>p.privateMiB),1),peakWorkingSetMiB:percentile(memory.map(p=>p.workingSetMiB),1),cpuOneCorePercentP50:percentile(cpu,.5),cpuOneCorePercentP95:percentile(cpu,.95),cpuMachinePercentP95:percentile(cpu,.95)===null?null:percentile(cpu,.95)/metadata.logicalProcessors,privateMiBMedianGrowth:first.length&&last.length?percentile(last.map(p=>p.privateMiB),.5)-percentile(first.map(p=>p.privateMiB),.5):null,handleMedianGrowth:first.length&&last.length?percentile(last.map(p=>p.handles),.5)-percentile(first.map(p=>p.handles),.5):null};
}
const recovery=events.filter(e=>['SessionUnlock','Resume'].includes(e.value)).map(event=>{const at=Date.parse(event.at),next=samples.find(s=>Date.parse(s.at)>=at&&s.health?.ackRecent&&s.health.renderers?.some(r=>r.role==='wallpaper'&&r.recent));return {event:event.value,at:event.at,nextRecentWallpaperAckAt:next?.at??null,secondsToObservedAck:next?(Date.parse(next.at)-at)/1000:null}});
const completed=fs.existsSync(path.join(dir,'complete.json'))?read('complete.json'):null;
const result={metadata,completed,sampleCount:samples.length,observedSpanSeconds:samples.length>1?(Date.parse(samples.at(-1).at)-Date.parse(samples[0].at))/1000:0,httpErrorSamples:samples.filter(s=>s.httpError).length,noRecentAckSamples:samples.filter(s=>!s.health?.ackRecent).length,missingPlayerSamples:samples.filter(s=>s.playerCount!==1).length,metrics,windowsEventLogAvailability:windowsEvents?.availability??null,events,recovery,limitations:['CPU percentages distinguish one core from the whole machine.','Private-memory growth is the difference of first/last ten medians after two-minute warm-up, not proof that no leak exists.','Polling/lock/sleep gaps and Lively background pause can suppress acknowledgments.','GPU, frame rate, energy and battery draw are not measured by this collector.']};
fs.writeFileSync(path.join(dir,'summary.json'),JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify(result,null,2));
