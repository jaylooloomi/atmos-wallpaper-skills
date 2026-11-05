// Counter-only export. Native window labels, handles and raw timestamps never leave the local run.
import {quantile} from './analyze-soak.mjs';
const MiB=1048576, valid=n=>Number.isSafeInteger(n)&&n>=0;
function metric(input,key,windowSeconds=300){
 const p=input.filter(p=>typeof p[key]==='number'&&Number.isFinite(p[key])).sort((a,b)=>a.t-b.t),values=p.map(p=>p[key]);
 const first=p[0],last=p.at(-1),span=first&&last?last.t-first.t:0,disjoint=span>=windowSeconds*2;
 const opening=disjoint?p.filter(p=>p.t<first.t+windowSeconds).map(p=>p[key]):[],closing=disjoint?p.filter(p=>p.t>last.t-windowSeconds).map(p=>p[key]):[];
 let linear=null,theil=null;
 if(p.length>=3&&span>0){const avgT=p.reduce((n,p)=>n+p.t,0)/p.length,avgV=values.reduce((n,v)=>n+v,0)/p.length,xx=p.reduce((n,p)=>n+(p.t-avgT)**2,0);if(xx>0)linear=p.reduce((n,p)=>n+(p.t-avgT)*(p[key]-avgV),0)/xx*3600;const small=p.length<=120?p:Array.from({length:120},(_,i)=>p[Math.round(i*(p.length-1)/119)]),slopes=[];for(let i=0;i<small.length;i++)for(let j=i+1;j<small.length;j++)if(small[j].t>small[i].t)slopes.push((small[j][key]-small[i][key])/(small[j].t-small[i].t)*3600);theil=quantile(slopes,.5)}
 const a=quantile(opening,.5),b=quantile(closing,.5);
 return {samples:p.length,first:values[0]??null,last:values.at(-1)??null,min:values.length?Math.min(...values):null,median:quantile(values,.5),peak:values.length?Math.max(...values):null,spanSeconds:span,openingWindowMedian:a,closingWindowMedian:b,medianWindowGrowth:a!==null&&b!==null?b-a:null,endpointWindowSeconds:windowSeconds,windowsDisjoint:disjoint,linearPerHour:linear,theilSenPerHour:theil};
}
export function summarizeRendererMemory(samples,{warmupSeconds=300,freshnessMs=90000}={}){
 const views=new Map(),coverage={eligibleSamples:0,missingFrames:0,unavailableObservations:0,invalidObservations:0,omittedWindows:0};
 for(const s of samples){const t=s.monotonicSeconds??s.elapsedSeconds;if(typeof t!=='number'||!Number.isFinite(t)||t<warmupSeconds)continue;coverage.eligibleSamples++;const now=Date.parse(s.at),native=s.native?.rendererMemory;
  if(!Number.isFinite(now)||native?.schemaVersion!==1||native.method!=='read-only-cdp-no-gc'||!Array.isArray(native.views)){coverage.missingFrames++;continue}
  const seen=new Set();for(const v of native.views){if(!['manager','wallpaper'].includes(v.role)||typeof v.windowLabel!=='string'||v.windowLabel.length>256||seen.has(v.windowLabel)){coverage.invalidObservations++;continue}seen.add(v.windowLabel);
   if(!views.has(v.windowLabel)){if(views.size>=32){coverage.omittedWindows++;continue}views.set(v.windowLabel,{role:v.role,validSamples:0,heap:new Map(),dom:new Map()})}const series=views.get(v.windowLabel);if(series.role!==v.role){coverage.invalidObservations++;continue}
   if(v.available!==true){coverage.unavailableObservations++;continue}
   const h=v.heap,d=v.dom,fresh=p=>p&&valid(p.capturedAtMs)&&now-p.capturedAtMs>=-1000&&now-p.capturedAtMs<=freshnessMs;
   if(!fresh(h)||!fresh(d)||!['usedBytes','totalBytes'].every(k=>valid(h[k])&&h[k]<=68719476736)||h.usedBytes>h.totalBytes||!['documents','nodes','jsEventListeners'].every(k=>valid(d[k])&&d[k]<=100000000)||[h.embedderUsedBytes,h.backingStorageBytes].some(n=>n!==undefined&&(!valid(n)||n>68719476736))){coverage.invalidObservations++;continue}
   series.validSamples++;
   const ht=t-(now-h.capturedAtMs)/1000,dt=t-(now-d.capturedAtMs)/1000;
   if(ht>=warmupSeconds&&!series.heap.has(h.capturedAtMs))series.heap.set(h.capturedAtMs,{t:ht,usedMiB:h.usedBytes/MiB,totalMiB:h.totalBytes/MiB,...(h.embedderUsedBytes!==undefined?{embedderMiB:h.embedderUsedBytes/MiB}:{}),...(h.backingStorageBytes!==undefined?{backingMiB:h.backingStorageBytes/MiB}:{})});
   if(dt>=warmupSeconds&&!series.dom.has(d.capturedAtMs))series.dom.set(d.capturedAtMs,{t:dt,documents:d.documents,nodes:d.nodes,jsEventListeners:d.jsEventListeners});
  }
 }
 const ordinal={manager:0,wallpaper:0};return {schemaVersion:1,method:'Fixed native WebView2 Runtime.getHeapUsage and Memory.getDOMCounters; no GC, JS evaluation, debugger port or UI automation',warmupExcludedSeconds:warmupSeconds,freshnessMs,coverage,views:[...views.values()].map(v=>({role:v.role,windowOrdinal:++ordinal[v.role],validCounterSamples:v.validSamples,counterSampleCoverage:coverage.eligibleSamples?v.validSamples/coverage.eligibleSamples:0,heap:Object.fromEntries(['usedMiB','totalMiB','embedderMiB','backingMiB'].map(k=>[k,metric([...v.heap.values()],k)])),dom:Object.fromEntries(['documents','nodes','jsEventListeners'].map(k=>[k,metric([...v.dom.values()],k)]))})),limitations:['Heap counters describe ordinary GC-cycle observations, not retained heap after a forced collection. They cannot independently prove zero leaks.','Each metric deduplicates native captures; missing, stale, failed and invalid probes are not filled with zero.','Process private bytes also include native allocator, browser and driver resources; these counters are complementary.']};
}
