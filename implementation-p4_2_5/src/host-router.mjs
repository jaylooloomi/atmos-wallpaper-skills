import {readBounded} from './library-paths.mjs';
import {createScheduleService} from './schedule-core.mjs';
import {spanGeometry} from './span-layout.mjs';
// Native host only: target identity is separate from appearance revisions.
import fs from 'node:fs';
import path from 'node:path';
import {createBridge} from './router.mjs';
import {createScopeRegistry,readLayout,displayId} from './scope-registry.mjs';
import {createCalendarService} from './calendar-service.mjs';
const error=(code,status=400)=>Response.json({ok:false,error:{code}},{status,headers:{'Cache-Control':'no-store'}});
export function createHostBridge(options){
 const registry=createScopeRegistry(options.store,options.coreOptions),views=new Map();let calendar;
 const calendarProvider=()=>calendar??=createCalendarService({store:registry.owner.store});
 const clock=options.now||Date.now;
 function nativeStatus(){try{const v=readBounded(registry.owner.store.path('host-status.json'),131072);const age=clock()-v.capturedAtMs;if(v.schemaVersion!==1||typeof options.hostSessionId!=='string'||!options.hostSessionId||v.hostSessionId!==options.hostSessionId||!Number.isFinite(age)||age<0||age>=5000||!Array.isArray(v.targets))return null;return v}catch{return null}}
 const runtimePolicy=id=>{const v=nativeStatus();return (id?v?.targets.find(t=>t.targetId===id):v?.targets[0])?.runtimePolicy||null};
 const base=createBridge({...options,automationEnabled:false,calendarProvider,runtimePolicyProvider:()=>runtimePolicy(null),projectionProvider:()=>projection(null)});
 function layout(){if(options.presentation==='screensaver'){
 let v={schemaVersion:1,screensaver:{mode:'duplicate',targets:[]}};const file=path.join(options.store,'screensaver-settings.json');if(fs.existsSync(file)){if(fs.statSync(file).size>16384)throw Error('SC_POLICY_INVALID');v=JSON.parse(fs.readFileSync(file))}const p=v.screensaver;if(v.schemaVersion!==1||!p||!['independent','duplicate','span'].includes(p.mode)||!Array.isArray(p.targets)||p.targets.length>16||p.targets.some(id=>!displayId(id))||new Set(p.targets).size!==p.targets.length)throw Error('SC_POLICY_INVALID');return{mode:p.mode,targets:p.targets.length?p.targets:(options.displayService?.latest?.()?.displays||[]).filter(d=>d.selectable).sort((a,b)=>Number(b.primary)-Number(a.primary)).map(d=>d.id),askOnApply:false};
 }return readLayout(options.store).layout}
 if(options.presentation!=='screensaver'){const timer=setInterval(()=>{try{const l=layout(),ids=l.mode==='independent'?l.targets:['shared'];for(const id of ids){const c=id==='shared'?registry.owner:registry.get(id);if(fs.existsSync(c.store.path('live.json')))createScheduleService(c).tick();}}catch{}},5000);timer.unref();}

 function projection(id){const l=layout(),ds=options.displayService?.latest?.()?.displays||[];let selected=id;if(l.mode==='single'){let host;try{host=readBounded(registry.owner.store.path('host-settings.json'))}catch{host=options.displayService?.latest?.()?.settings}const target=host?.placement?.target||'primary';selected=target==='primary'?ds.find(d=>d.primary&&d.selectable)?.id||null:target}const d=ds.find(d=>d.id===selected&&d.selectable);let revision;if(options.presentation==='screensaver'){try{revision=readBounded(registry.owner.store.path('screensaver-settings.json')).revision}catch{revision=0}}else revision=readLayout(options.store).revision;return {layoutRevision:revision,mode:l.mode,targetId:selected,targetBounds:d?.bounds||null,scalePercent:d?.scalePercent||null,span:l.mode==='span'&&selected?spanGeometry(ds,l.targets,selected):null};}
 function view(id,l){if(!displayId(id)||!l.targets.includes(id))throw Object.assign(Error('TARGET_INACTIVE'),{code:'TARGET_INACTIVE'});const scopeId=l.mode==='independent'?id:'shared',key=id+':'+scopeId;if(!views.has(key)){
 const root=scopeId==='shared'?options.store:registry.get(id).store.root;
 views.set(key,createBridge({...options,automationEnabled:false,store:root,token:base.sessionToken,wallpaperToken:base.wallpaperSessionToken,screensaverToken:base.screensaverSessionToken,coreOptions:{...options.coreOptions,...(scopeId==='shared'?{}:{sharedRoot:options.store})},runtimePolicyProvider:()=>runtimePolicy(id),projectionProvider:()=>projection(id),routing:{targetId:id,scopeId},observationFile:options.presentation==='screensaver'?'observations/screensaver-'+id+'.json':scopeId==='shared'?'observations/wallpaper-'+id+'.json':'observations/wallpaper.json',calendarProvider,monitorProvider:options.displayService?.monitorFor?()=>{const m=options.displayService.monitorFor(id),current=layout();if(current.mode==='span')m.span=spanGeometry(options.displayService.latest()?.displays||[],current.targets,id);return m}:options.monitorProvider}));
 }return views.get(key)}
 function health(){
 const l=layout(),native=nativeStatus(),topology=options.displayService?.latest?.(),host=(()=>{try{return readBounded(registry.owner.store.path('host-settings.json'))}catch{return topology?.settings||{revision:0,placement:{enabled:true,target:'primary'}}}})();
 const enabled=host.placement?.enabled===true;let ids=l.mode==='single'?[host.placement.target==='primary'?topology?.displays?.find(d=>d.primary&&d.selectable)?.id||native?.targets?.[0]?.targetId||null:host.placement.target]:l.targets;
 if(!enabled)ids=[];const expectedLayout=options.presentation==='screensaver'?null:readLayout(options.store).revision;
 const nativeCurrent=!!native&&(expectedLayout===null||native.layoutRevision===expectedLayout)&&native.settingsRevision===host.revision;
 const perTarget=ids.map(id=>{const scopeId=l.mode==='independent'?id:'shared',dir=scopeId==='shared'?registry.owner.store.root:registry.owner.store.path('targets/'+id),name=l.mode==='single'||scopeId!=='shared'?'wallpaper.json':'wallpaper-'+id+'.json';let acknowledged=false,observed=null,revision=null,desiredHash=null,errorCode=null;
 try{const live=readBounded(path.join(dir,'live.json'),16*1048576);revision=live.revision;desiredHash=live.desiredHash;const snapshot=l.mode==='single'?base.snapshot():view(id,l).snapshot();observed=readBounded(path.join(dir,'observations',name),16384);const age=clock()-Date.parse(observed.capturedAt);acknowledged=options.presentation!=='screensaver'&&typeof options.hostSessionId==='string'&&options.hostSessionId.length>0&&observed.hostSessionId===options.hostSessionId&&observed.targetId===(l.mode==='single'?null:id)&&observed.rendererRole==='wallpaper'&&observed.source==='local-bridge'&&observed.stateApplied===true&&observed.assetLoaded===true&&observed.revision===revision&&observed.desiredHash===desiredHash&&observed.assetSha256===snapshot.artwork.sha256&&observed.projectionId===snapshot.projectionId&&Number.isFinite(age)&&age>=0&&age<4000;
 }catch(e){errorCode=e.code||'TARGET_STATE_UNAVAILABLE'}const nativeTarget=nativeCurrent?native.targets.find(t=>t.targetId===id):null;return{targetId:id,scopeId,revision,desiredHash,rendererAcknowledged:acknowledged,nativeAttached:nativeTarget?nativeTarget.nativeAttached===true:null,online:topology?topology.displays.some(d=>d.id===id&&d.selectable):null,lastAckAt:acknowledged?Date.parse(observed.capturedAt):null,renderer:acknowledged?{rendererId:observed.rendererId,role:'wallpaper',revision,desiredHash,assetSha256:observed.assetSha256,capturedAtMs:Date.parse(observed.capturedAt),recent:true}:null,error:errorCode?{code:errorCode}:nativeTarget?.error?{code:nativeTarget.error}:null,hostVisibility:'unknown'};});
 const all=perTarget.length>0&&perTarget.every(t=>t.targetId!==null&&t.online!==false&&t.rendererAcknowledged);return{schemaVersion:1,source:'local-bridge',hostVisibility:'unknown',mode:l.mode,enabled,configuredTargetCount:ids.length,rendererRecentlyAcknowledged:all,allTargetsAcknowledged:all,nativeRecentlyReported:!!native,nativeAllAttached:nativeCurrent&&perTarget.length>0&&perTarget.every(t=>t.nativeAttached===true),lastAckAt:all?Math.min(...perTarget.map(t=>t.lastAckAt)):null,perTarget,renderers:[...perTarget.flatMap(t=>t.renderer?[t.renderer]:[]),...[base,...views.values()].flatMap(b=>b.rendererReports().filter(r=>r.role==='manager'))],capabilities:{snapshot:true,ack:true,httpDesiredWrites:true,hostConnected:false},trust:'renderer acknowledgements and native attachment reports are separate; visibility remains unknown'};
 }
 function defaultTarget(l){const d=options.displayService?.latest?.()?.displays||[];return l.targets.find(id=>d.some(x=>x.id===id&&x.primary))||l.targets[0]}
 return {...base,registry,async handle(req,context={}){try{
 const u=new URL(req.url),p=u.pathname;
 // Validate before any profile creation or request side effect.
 if(u.origin!==options.origin||req.headers.get('Host')!==u.host)return error('HOST_DENIED',403);
 if(u.search)return error('QUERY_NOT_ALLOWED',400);
 if(req.headers.has('Origin')&&req.headers.get('Origin')!==options.origin)return error('ORIGIN_DENIED',403);
 if(p.startsWith('/api/')){
  if(req.headers.get('Sec-Fetch-Site')!=='same-origin')return error('FETCH_METADATA_DENIED',403);
  if(!base.roleOf(req))return error('SESSION_DENIED',401);
  if(p==='/api/health'){if(req.method!=='GET')return error('METHOD_DENIED',405);const value=health();value.capabilities.httpDesiredWrites=base.roleOf(req)==='manager';return Response.json(value,{headers:{'Cache-Control':'no-store','X-Content-Type-Options':'nosniff'}});}
  if(p.startsWith('/api/host/')||p.startsWith('/api/calendar/'))return base.handle(req,context);
  const l=layout(),id=req.headers.get('X-Atmos-Target');
  if(l.mode==='single'){if(id)return error('SCOPE_CHANGED',401);return base.handle(req,context)}
  if(!id)return error('TARGET_REQUIRED',409);
  const bridge=view(id,l),scope=l.mode==='independent'?id:'shared';
  if(req.headers.get('X-Atmos-Scope')!==scope)return error('SCOPE_CHANGED',401);
  if(scope!=='shared')registry.ensure(id);
  return bridge.handle(req,context);
 }
 const match=p.match(/^\/(manager|wallpaper|screensaver)\/(display-[a-f0-9]{64})$/);
 if(match?.[1]==='screensaver'&&options.presentation!=='screensaver')return error('ROLE_DENIED',403);
 if(match||p==='/manager'||p==='/wallpaper'||p==='/'){
  const l=layout();if(l.mode==='single')return match?error('TARGET_INACTIVE',409):base.handle(req,context);
  const kind=(match?.[1]==='screensaver'?'wallpaper':match?.[1])||(p==='/wallpaper'?'wallpaper':'manager'),id=match?.[2]||defaultTarget(l),bridge=view(id,l);
  const url=new URL('/'+kind,options.origin);return bridge.handle(new Request(url,req),{rawPath:'/'+kind});
 }
 return base.handle(req,context);
 }catch(e){return error(e.code||'LOCAL_OPERATION_FAILED',e.code==='STORE_LOCKED'?409:503)}}};
}
