import {layoutRegressionCases} from './layout-regression-contract.mjs';
import {layoutHostFixtures} from './layout-host-fixtures.mjs';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import http from 'node:http';
import {runProcess} from './process-runner.mjs';
import {fileURLToPath} from 'node:url';
import {ControlCore} from '../src/portable-core.mjs';
import {createBridge} from '../src/router.mjs';
const root=fileURLToPath(new URL('../',import.meta.url)),frontend=process.env.ATMOS_FRONTEND_ARTIFACT||path.join(root,'dist');
const chrome=process.env.ATMOS_TEST_CHROME||'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe';
if(!fs.existsSync(chrome))throw Error('Set ATMOS_TEST_CHROME to an installed Chromium executable');
const temp=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-layout-'));
const output=process.env.ATMOS_LAYOUT_OUTPUT?path.resolve(process.env.ATMOS_LAYOUT_OUTPUT):path.join(root,'evidence','layout');fs.mkdirSync(output,{recursive:true});
const auditScript=fs.readFileSync(new URL('./layout-regression-probe.js',import.meta.url),'utf8')+'\n'+`(()=>{let resizeWarnings=0;addEventListener('error',event=>{if(event.message.startsWith('ResizeObserver loop')){resizeWarnings++;return}const e=parent.document.createElement('pre');e.id='layout-results';e.hidden=true;e.textContent=JSON.stringify({error:'FIXTURE_SCRIPT_ERROR',message:event.message});parent.document.body.append(e)});let attempts=0;function measure(){const scene=document.querySelector('.scene-picker-top'),cards=[...document.querySelectorAll('.scene-picker-top [data-scene]')];if(document.documentElement.hasAttribute('data-atmos-loading')||!scene||cards.length!==6||cards.some(e=>!e.querySelector('img')?.complete)||!window.AtmosCalendar||(${process.argv.includes('--host-controls')}&&(!document.querySelector('.audio-settings')||!window.AtmosSettingsTabs))){if(attempts++<300){setTimeout(measure,100);return}return report({error:'PAGE_NOT_READY',loading:document.documentElement.hasAttribute('data-atmos-loading'),bootMessage:document.getElementById('atmosBootMessage')?.textContent,fonts:document.fonts.status})}if(${process.argv.includes('--panel-flow')}){if(${process.argv.includes('--diagnostic-quick')}){if(${JSON.stringify(process.env.ATMOS_QUICK_WIDGET||'clock')}==='audio')window.AtmosSettingsTabs.select('system');else window.AtmosSettingsTabs.selectWidget(${JSON.stringify(process.env.ATMOS_QUICK_WIDGET||'clock')});}else window.AtmosInspector.open('clock');}document.fonts.ready.then(()=>setTimeout(()=>{const box=e=>{const r=e.getBoundingClientRect();return {left:r.left,right:r.right,top:r.top,bottom:r.bottom,width:r.width,height:r.height}};const sceneBox=box(scene),row=document.getElementById('previewSettingsRow');report({widgetPanels:window.AtmosWidgetPanelRegression.measure(),accordionNavigation:window.AtmosAccordionRegression.measure(),hostControls:${process.argv.includes('--host-controls')}?['.audio-settings','.library-settings','.provider-settings'].map(selector=>({selector,visible:!!document.querySelector(selector)&&!document.querySelector(selector).hidden})):[],width:innerWidth,height:innerHeight,pageWidth:document.documentElement.scrollWidth,pageClientWidth:document.documentElement.clientWidth,scene:sceneBox,columns:getComputedStyle(document.getElementById('sceneOptions')).gridTemplateColumns.split(' ').length,cards:cards.map(e=>({...box(e),contentWidth:e.scrollWidth,clientWidth:e.clientWidth,title:e.textContent.trim()})),toolbar:{...box(row),contentWidth:row.scrollWidth,clientWidth:row.clientWidth,minInputWidth:Math.min(...[...row.querySelectorAll('input')].map(e=>e.getBoundingClientRect().width)),inlineLabels:[...document.querySelectorAll('#previewResolutionControls>label')].every(label=>{const range=document.createRange();range.selectNodeContents(label.firstChild);const text=range.getBoundingClientRect(),control=label.querySelector('input,select').getBoundingClientRect();return Math.abs(text.top+text.height/2-control.top-control.height/2)<2}),singleRow:(()=>{const centers=[...row.querySelectorAll('input,select,button')].map(e=>{const r=e.getBoundingClientRect();return r.top+r.height/2});return Math.max(...centers)-Math.min(...centers)<2})()},side:box(document.querySelector('.side')),toolbarInPreviewColumn:row.parentElement===document.querySelector('main>section:first-child'),brandHeader:{display:getComputedStyle(document.querySelector('body>header')).display,height:document.querySelector('body>header').getBoundingClientRect().height,native:document.body.dataset.nativeManager==='true'},sceneBeforePreview:sceneBox.bottom<=box(row).top,horizontalScrollContainers:[...document.querySelectorAll('body *')].filter(e=>e.clientWidth>0&&e.getBoundingClientRect().height>0&&['auto','scroll'].includes(getComputedStyle(e).overflowX)&&e.scrollWidth>e.clientWidth+1).map(e=>e.id||e.className||e.tagName),overflows:[...document.querySelectorAll('body>*,main>*,#previewSettingsRow>*')].filter(e=>getComputedStyle(e).display!=='none').filter(e=>{const r=e.getBoundingClientRect();return r.left<-.5||r.right>innerWidth+.5}).map(e=>e.id||e.className||e.tagName)})},250))}function toolbarContrast(){const bar=document.querySelector('.bridge-commands'),surface=getComputedStyle(bar).backgroundColor,canvas=document.createElement('canvas');canvas.width=canvas.height=1;const ctx=canvas.getContext('2d'),rgba=color=>{ctx.clearRect(0,0,1,1);ctx.fillStyle=color;ctx.fillRect(0,0,1,1);return [...ctx.getImageData(0,0,1,1).data]},blend=(fg,bg,opacity=1)=>{const a=fg[3]/255*opacity;return fg.slice(0,3).map((v,i)=>v*a+bg[i]*(1-a))},luminance=color=>color.slice(0,3).map(v=>{v/=255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4}).reduce((sum,v,i)=>sum+v*[.2126,.7152,.0722][i],0),ratio=(a,b)=>{const x=luminance(a),y=luminance(b);return (Math.max(x,y)+.05)/(Math.min(x,y)+.05)},bg=rgba(surface),measure=el=>{const c=getComputedStyle(el),button=el.tagName==='BUTTON',back=button?blend(rgba(c.backgroundColor),bg):bg,front=blend(rgba(c.color),back,Number(c.opacity));return {element:el.id||el.tagName.toLowerCase(),color:c.color,background:button?c.backgroundColor:surface,opacity:Number(c.opacity),fontSize:c.fontSize,contrast:ratio(front,back)}};const text=[...bar.querySelectorAll('strong,#draftStatus,small,#localeControl>span,#appLocale')].map(measure);const buttons=[...bar.querySelectorAll('button')].map(el=>{const disabled=el.disabled;el.disabled=false;const measured=measure(el);el.disabled=disabled;return measured});return {text,enabledButtons:buttons}}function toolbarTransitions(){const original=document.body.dataset.theme;try{return ['natural','neon','warm','minimal','natural'].map(theme=>{document.body.dataset.theme=theme;return {theme,...toolbarContrast()}})}finally{document.body.dataset.theme=original}}function noteMetrics(){const scene=document.getElementById('scene').getBoundingClientRect(),d=window.atmosCanvasDimensions;return {removed:!document.querySelector('#monitorStatus,.preview-explanation'),height:scene.height,naturalHeight:scene.width/(d.width/d.height),sceneRatio:scene.width/scene.height,targetRatio:d.width/d.height}}function report(result){if(${process.argv.includes('--panel-flow')}){const rect=e=>{const r=e.getBoundingClientRect();return {top:r.top,bottom:r.bottom,height:r.height}};const items=[...document.querySelector('.panel-scroll').children].filter(e=>getComputedStyle(e).display!=='none'&&e.getBoundingClientRect().height>0);result.panelFlow={overlaps:items.slice(1).filter((e,i)=>rect(e).top<rect(items[i]).bottom-1).map(e=>e.id||e.className),escapedContainers:['.side','.component-section','#widgetManager','#componentInspector','.inspect-card.is-open'].filter(selector=>{const e=document.querySelector(selector);return e&&[...e.children].filter(c=>!c.hidden&&getComputedStyle(c).display!=='none').some(c=>rect(c).bottom>rect(e).bottom+2)}),nestedPropsScroll:[...document.querySelectorAll('.inspect-props:not([hidden])')].some(e=>e.scrollHeight>e.clientHeight+2),retiredScreensaverAbsent:!document.querySelector('.screensaver-settings,[data-settings-panel=screensaver]')};}const bar=document.querySelector('.bridge-commands'),before=bar.getBoundingClientRect(),initialShadow=getComputedStyle(bar).boxShadow,initialStuck=bar.dataset.stuck;scrollTo(0,Math.max(600,before.top+100));dispatchEvent(new Event('scroll'));setTimeout(()=>{const after=bar.getBoundingClientRect(),button=bar.querySelector('#applyDraft'),b=button.getBoundingClientRect();result.applyToolbar={position:getComputedStyle(bar).position,scrollY,top:after.top,width:after.width,height:after.height,contentWidth:bar.scrollWidth,clientWidth:bar.clientWidth,buttonOnTop:!!document.elementFromPoint(b.left+b.width/2,b.top+b.height/2)?.closest('.bridge-commands'),initialTop:before.top,initialShadow,initialStuck,stuck:bar.dataset.stuck,shadow:getComputedStyle(bar).boxShadow};const finish=()=>{if(${process.argv.includes('--regression')})result.regression=window.AtmosLayoutRegression.measure({injectOverlap:${process.argv.includes('--regression-inject-overlap')}});if(${process.argv.includes('--contrast')})result.toolbarContrast=toolbarTransitions();result.resizeObserverWarnings=resizeWarnings;if(${process.argv.includes('--note')}){const viewport=document.querySelector('.preview-viewport');scrollTo(0,scrollY+viewport.getBoundingClientRect().top-200)}else if(!${process.argv.includes('--gap')})scrollTo(0,0);const e=parent.document.createElement('pre');e.id='layout-results';e.hidden=true;e.textContent=JSON.stringify(result);parent.document.body.append(e)};const noteBefore=noteMetrics();if(${process.argv.includes('--note')}){window.AtmosInspector.open('weather');window.atmosSchedulePanelAlign?.();setTimeout(()=>{const scene=document.getElementById('scene').getBoundingClientRect(),d=window.atmosCanvasDimensions;result.explanation={...noteMetrics(),beforeHeight:noteBefore.height,sceneRatio:scene.width/scene.height,targetRatio:d.width/d.height};finish()},200)}else{result.explanation=noteBefore;finish()}},100)}measure()})();`;
async function openFixture(store){
let bridge;
const server=http.createServer(async(req,res)=>{try{
 const url=new URL(req.url,'http://127.0.0.1');
 if(url.pathname==='/audit-shell'){const width=Number(url.searchParams.get('width')),height=Number(url.searchParams.get('height'));res.setHeader('Content-Type','text/html');res.end('<!doctype html><html><body style="margin:0;background:#ddd"><iframe style="display:block;border:0;width:'+width+'px;height:'+height+'px" src="/manager"></iframe></body></html>');return}
 if(url.pathname==='/native-manager-fixture.js'){res.setHeader('Content-Type','text/javascript');res.end('window.__ATMOS_NATIVE_MANAGER__=true;');return}
 // Synthetic player geometry only: no YouTube request, playback or ACK is fabricated.
 if(url.pathname==='/youtube-client.js'){let code=fs.readFileSync(path.join(frontend,'youtube-client.js'),'utf8');if(code.split('frame.src=embed;').length!==2)throw Error('YouTube fixture hook missing');if(code.split('attachPlayer(current,token);').length!==2)throw Error('YouTube API fixture hook missing');code=code.replace('frame.src=embed;',"frame.dataset.syntheticFixture='true';").replace('attachPlayer(current,token);','');res.setHeader('Content-Type','text/javascript');res.end(code);return}
 if(url.pathname==='/layout-audit.js'){res.setHeader('Content-Type','text/javascript');res.end(auditScript);return}
 const parts=[];for await(const part of req)parts.push(part);const body=Buffer.concat(parts),headers=new Headers();for(const [key,value]of Object.entries(req.headers))if(value!==undefined)headers.set(key,Array.isArray(value)?value.join(','):value);
 const request=new Request(origin+req.url,{method:req.method,headers,...(body.length?{body}:{})});const response=await bridge.handle(request,{rawPath:req.url});
 res.writeHead(response.status,Object.fromEntries(response.headers));let bytes=Buffer.from(await response.arrayBuffer());if(url.pathname==='/manager'&&response.status===200)bytes=Buffer.from(bytes.toString().replace('</html>','<script src="/layout-audit.js"></script></html>'));if(process.argv.includes('--native')&&url.pathname==='/manager'&&response.status===200)bytes=Buffer.from(bytes.toString().replace('<style>','<script src="/native-manager-fixture.js"></script><style>'));res.end(bytes);
 }catch{res.writeHead(500);res.end('Fixture error')}});
await new Promise((resolve,reject)=>{
 server.once('error',reject);
 server.listen(0,'127.0.0.1',resolve);
});
const origin='http://127.0.0.1:'+server.address().port;
async function close(){
 server.closeAllConnections();
 await new Promise(resolve=>server.close(resolve));
}
try{
 bridge=createBridge({store,staticRoot:frontend,prototypeRoot:path.resolve(root,'../prototype-v8.1-frozen'),origin,...(process.argv.includes('--host-controls')?layoutHostFixtures():{})});
 return {origin,close};
}catch(error){
 await close();
 throw error;
}
}
const cases=process.argv.includes('--diagnostic-quick')?[[390,844,'neon','zh-TW'],[1280,900,'natural','en']]:process.argv.includes('--regression')?layoutRegressionCases:process.argv.includes('--panel-flow-quick')?[[1280,900,'neon']]:process.argv.includes('--panel-flow')?[320,390,768,1000,1100,1280,1876].flatMap(w=>['natural','neon'].map(t=>[w,900,t])):process.argv.includes('--contrast')?[[320,160,'minimal'],[390,844,'neon'],[1280,900,'natural'],[1280,900,'warm'],[1876,1080,'minimal']]:process.argv.includes('--alignment')?[[320,844,'natural'],[390,844,'neon'],[1280,900,'natural'],[1876,1080,'natural']]:process.argv.includes('--gap')?[[320,160,'minimal'],[320,844,'natural'],[390,844,'neon'],[1280,900,'natural']]:process.argv.includes('--toolbar')?[[320,844,'natural'],[390,844,'neon'],[1280,900,'natural']]:process.argv.includes('--quick')?[[390,844,'neon'],[1280,900,'natural']]:[320,360,390,600,768,1000,1100,1101,1280,1440,1876].flatMap(width=>['natural','neon','warm','minimal'].map(theme=>[width,width<700?844:1080,theme]));
async function auditViewport([width,height,theme,caseLocale],signal){
 const caseStart=performance.now();console.log(JSON.stringify({event:'viewport-start',width,height,theme,cacheHit:false}));
 const id='layout-'+width+'-'+theme;
 const store=path.join(temp,id,'store');
 const core=new ControlCore(store);
 core.init();
 const locale=caseLocale||(process.argv.includes('--zh')?'zh-TW':'en');
 const state=core.status(),preview=core.preview({requestId:id,expectedRevision:state.revision,patch:{visual:{theme,locale}}});core.apply({requestId:id+'-apply',expectedRevision:state.revision,previewId:preview.previewId});
 const {origin,close}=await openFixture(store);
 try{
 const profile=path.join(temp,id,'profile');const screenshot=(width===1280&&theme==='natural')||(width===390&&theme==='neon')||(width===1876&&theme==='natural');
 const args=['--headless=new','--no-first-run','--no-default-browser-check','--disable-extensions','--disable-background-networking','--disable-sync','--force-device-scale-factor=1','--user-data-dir='+profile,'--window-size='+Math.max(800,width)+','+height,'--virtual-time-budget=24000','--dump-dom',...(screenshot?['--screenshot='+path.join(output,id+'.png')]:[]),origin+'/audit-shell?width='+width+'&height='+height];
 const childResult=await runProcess(chrome,args,{stream:false,timeout:30000,signal,logFile:path.join(output,id+'.browser.log')});if(childResult.error||childResult.status!==0)throw Error(childResult.timedOut?'CHROMIUM_TIMEOUT':childResult.aborted?'CHROMIUM_ABORTED':'CHROMIUM_EXIT_'+childResult.status);const stdout=childResult.stdout;
 const match=stdout.match(/<pre id="layout-results" hidden(?:="")?>([\s\S]*?)<\/pre>/);let result;
 try{result=JSON.parse(match?.[1]?.replaceAll('&quot;','"').replaceAll('&amp;','&').replaceAll('&lt;','<').replaceAll('&gt;','>'))}catch{result={error:'NO_MEASUREMENT'}}
 const failures=[];if(result.widgetPanels?.passed!==true)failures.push('WIDGET_PANEL_REGRESSION');if(result.accordionNavigation?.passed!==true)failures.push('SETTINGS_ACCORDION_REGRESSION');if(process.argv.includes('--regression')&&result.regression?.passed!==true)failures.push('COMPONENT_THEME_LAYOUT_REGRESSION');if(result.error)failures.push(result.error);else{
  if(result.panelFlow&&(result.panelFlow.overlaps.length||result.panelFlow.escapedContainers.length||result.panelFlow.nestedPropsScroll||!result.panelFlow.retiredScreensaverAbsent))failures.push('SETTINGS_PANEL_CONTENT_OVERLAP');if(process.argv.includes('--host-controls')&&result.hostControls.some(c=>!c.visible))failures.push('HOST_CONTROL_HIDDEN');if(process.argv.includes('--native')?(result.brandHeader.display!=='none'||result.brandHeader.height!==0||!result.brandHeader.native):(result.brandHeader.display==='none'||result.brandHeader.native))failures.push('BRAND_HEADER_VISIBILITY');if(process.argv.includes('--contrast')&&result.toolbarContrast.some(state=>[...state.text,...state.enabledButtons].some(item=>item.contrast<4.5)))failures.push('TOOLBAR_TEXT_LOW_CONTRAST');if(result.resizeObserverWarnings>4)failures.push('REPEATED_RESIZE_WARNINGS');if(!result.explanation.removed||Math.abs(result.explanation.height-result.explanation.naturalHeight)>1)failures.push('PREVIEW_RATIO_OR_HINT_REMOVAL_INVALID');if(process.argv.includes('--note')&&(Math.abs(result.explanation.height-result.explanation.beforeHeight)>1||Math.abs(result.explanation.sceneRatio-result.explanation.targetRatio)>.01))failures.push('PANEL_CHANGED_PREVIEW_GEOMETRY');if(result.width!==width)failures.push('WRONG_VIEWPORT');if(result.pageWidth>result.pageClientWidth+1)failures.push('PAGE_HORIZONTAL_OVERFLOW');
  if(result.applyToolbar.initialTop<=12||result.applyToolbar.initialShadow!=='none'||result.applyToolbar.initialStuck!=='false')failures.push('APPLY_TOOLBAR_FLOATS_AT_START');if(result.applyToolbar.stuck!=='true'||result.applyToolbar.shadow==='none')failures.push('APPLY_TOOLBAR_PIN_STATE');if(result.applyToolbar.position!=='sticky'||result.applyToolbar.scrollY<=result.applyToolbar.initialTop||Math.abs(result.applyToolbar.top-12)>1||!result.applyToolbar.buttonOnTop)failures.push('APPLY_TOOLBAR_NOT_STICKY');if(result.applyToolbar.contentWidth>result.applyToolbar.clientWidth+1||result.applyToolbar.top+result.applyToolbar.height>height-11)failures.push('APPLY_TOOLBAR_OVERFLOW');
  if(!result.toolbarInPreviewColumn||(width>1000&&Math.abs(result.toolbar.top-result.side.top)>1))failures.push('PREVIEW_PANEL_TOP_MISALIGNED');if(result.toolbar.minInputWidth<60)failures.push('PREVIEW_INPUT_SQUEEZED');if(width>=1700&&(!result.toolbar.inlineLabels||!result.toolbar.singleRow))failures.push('PREVIEW_NOT_SINGLE_ROW');
  if(result.toolbar.contentWidth>result.toolbar.clientWidth+1)failures.push('TOOLBAR_HORIZONTAL_SCROLL');
  if(result.horizontalScrollContainers.length)failures.push('EXTRA_HORIZONTAL_SCROLL');if(!result.sceneBeforePreview)failures.push('SCENE_ORDER');if(result.overflows.length)failures.push('OFFSCREEN_SECTION');
  for(const card of result.cards)if(card.width<85||card.left<-.5||card.right>width+.5||card.contentWidth>card.clientWidth+1)failures.push('CARD_SQUEEZED_OR_OVERFLOW');
 }
 const elapsedMs=Math.round(performance.now()-caseStart);
 console.log(JSON.stringify({event:'viewport-complete',elapsedMs,width,theme,failures,pageWidth:result.pageWidth,columns:result.columns}));
 return {elapsedMs,width,height,theme,locale,...result,failures};
 }finally{await close()}
}

const selectedCases=process.argv.includes('--regression-smoke')?cases.slice(0,1):cases;
// Isolate each viewport in its own store, HTTP origin and Chromium profile.
// Keep the negative control and other diagnostic modes serial.
const concurrency=process.argv.includes('--regression')&&selectedCases.length>1
 ? Math.min(3,Math.max(1,Math.floor(os.availableParallelism()/4)))
 : 1;
const results=[];
const controller=new AbortController();
const interrupt=()=>controller.abort();
process.once('SIGINT',interrupt);
process.once('SIGTERM',interrupt);
try{
 for(let index=0;index<selectedCases.length;index+=concurrency){
  if(controller.signal.aborted)throw Error('CHROMIUM_ABORTED');
  let failure;
  const batch=await Promise.allSettled(selectedCases.slice(index,index+concurrency).map(async viewport=>{
   try{return await auditViewport(viewport,controller.signal)}
   catch(error){failure??=error;controller.abort();throw error}
  }));
  if(failure)throw failure;
  if(controller.signal.aborted)throw Error('CHROMIUM_ABORTED');
  results.push(...batch.map(item=>item.value));
  if(process.argv.includes('--regression')&&process.env.ATMOS_LAYOUT_GATE_REPORT)fs.writeFileSync(process.env.ATMOS_LAYOUT_GATE_REPORT,JSON.stringify({schemaVersion:1,passed:false,status:'running',completedCases:results.length,expectedCases:selectedCases.length})+'\n');
 }
}finally{
 process.removeListener('SIGINT',interrupt);
 process.removeListener('SIGTERM',interrupt);
 const resolved=fs.realpathSync(temp);
 if(path.dirname(resolved)!==fs.realpathSync(os.tmpdir())||!path.basename(resolved).startsWith('atmos-layout-'))throw Error('LAYOUT_TEMP_CLEANUP_UNSAFE');
 await fs.promises.rm(resolved,{recursive:true,force:true,maxRetries:8,retryDelay:200});
}
fs.writeFileSync(path.join(output,'report.json'),JSON.stringify({kind:'Isolated Chromium CLI render; synthetic fixture only; no user browser/desktop automation',cases:results,passed:results.every(r=>!r.failures.length)},null,2)+'\n');
if(results.some(r=>r.failures.length))process.exitCode=1;
