import {previewStaticRoot} from './preview-static-root.mjs';
if(process.env.ATMOS_PRESENTATION==='screensaver')throw Error('SCREENSAVER_REMOVED');
import {createDailyFeeds} from './daily-feeds.mjs';

import {createPerformanceService} from './performance-service.mjs';
import {createProvidersService} from './providers-service.mjs';
import {createLayoutService} from './layout-service.mjs';
import {createAudioService} from './audio-service.mjs';
import {createLibraryService} from './library-service.mjs';
import {createDisplayService} from './display-service.mjs';
import {createStartupService} from './startup-service.mjs';
import {readMonitorMetadata} from './monitor-adapter.mjs';
// Windows-only entry. Execution requires separate explicit user authorization; never launch in cloud.
import {runtimeOptions} from './runtime-options.mjs';
import fs from 'node:fs';import {ControlCore} from './portable-core.mjs';import {validateLocalRoot} from './portable-store.mjs';
import {createLocalServer} from './http-transport.mjs';import path from 'node:path';import {fileURLToPath} from 'node:url';import {createHostBridge} from './host-router.mjs';
if(process.platform!=='win32')throw Error('Windows entry only; cloud must use official managed preview.');if(!process.argv.includes('--start-local-service'))throw Error('Explicit --start-local-service required; no listener started.');if(!process.env.ATMOS_STORE||!process.env.ATMOS_PYTHON)throw Error('Explicit ATMOS_STORE and ATMOS_PYTHON required.');validateLocalRoot(process.env.ATMOS_STORE);if(!path.isAbsolute(process.env.ATMOS_PYTHON)||!fs.existsSync(process.env.ATMOS_PYTHON))throw Error('Configured existing Python executable missing; no install performed.');
const {port}=runtimeOptions(process.argv.slice(2).filter(v=>v!=='--start-local-service'),process.env);
const displays=createDisplayService({store:process.env.ATMOS_DESKTOP_HOST_STORE||null,executable:process.env.ATMOS_DESKTOP_HOST_EXE||null});await displays.status().catch(()=>{});const topologyPoll=setInterval(()=>displays.status().catch(()=>{}),3000);topologyPoll.unref();
const root=fileURLToPath(new URL('../',import.meta.url)),origin='http://127.0.0.1:'+port;const core=new ControlCore(process.env.ATMOS_STORE,{pythonExecutable:process.env.ATMOS_PYTHON});core.init();const library=createLibraryService({root:process.env.ATMOS_STORE,executable:process.env.ATMOS_DESKTOP_HOST_EXE||null});const dailyFeeds=createDailyFeeds({root:process.env.ATMOS_STORE||path.join(root,'demo-store')});dailyFeeds.start();const bridge=createHostBridge({dailyFeedsService:dailyFeeds,presentation:'wallpaper',performanceService:createPerformanceService({store:process.env.ATMOS_STORE,executable:process.env.ATMOS_DESKTOP_HOST_EXE||null}),providersService:createProvidersService({root:process.env.ATMOS_STORE,executable:process.env.ATMOS_DESKTOP_HOST_EXE||null}),layoutService:createLayoutService({store:process.env.ATMOS_STORE,executable:process.env.ATMOS_DESKTOP_HOST_EXE||null}),hostSessionId:process.env.ATMOS_HOST_SESSION_ID||null,audioService:createAudioService({store:process.env.ATMOS_STORE,executable:process.env.ATMOS_DESKTOP_HOST_EXE||null}),libraryService:library,displayService:displays,monitorProvider:()=>displays.monitor(),startupService:createStartupService({executable:process.env.ATMOS_DESKTOP_HOST_EXE||null}),monitorMetadata:readMonitorMetadata({enabled:true,targetId:process.env.ATMOS_TARGET_MONITOR||null}),store:process.env.ATMOS_STORE||path.join(root,'demo-store'),staticRoot:previewStaticRoot({requested:process.env.ATMOS_PREVIEW_STATIC_ROOT,defaultRoot:path.join(root,'dist'),nativeExecutable:process.env.ATMOS_DESKTOP_HOST_EXE}),prototypeRoot:path.resolve(root,'../prototype-v8.1-frozen'),origin});
const server=createLocalServer({bridge,origin,trace:process.env.ATMOS_TRACE_REQUESTS==='1'?event=>console.log(JSON.stringify(event)):undefined});
server.listen({host:'127.0.0.1',port},()=>console.log('Atmos Windows local runtime ready at '+origin+' (Windows loopback only; session stays in memory; host unknown)'));
for(const sig of ['SIGINT','SIGTERM'])process.once(sig,()=>server.close(()=>process.exit(0)));
