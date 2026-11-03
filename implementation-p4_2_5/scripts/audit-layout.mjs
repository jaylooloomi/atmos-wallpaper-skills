import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import http from 'node:http';
import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {ControlCore} from '../src/portable-core.mjs';
import {createBridge} from '../src/router.mjs';
const root=fileURLToPath(new URL('../',import.meta.url));
const chrome=process.env.ATMOS_TEST_CHROME||'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe';
if(!fs.existsSync(chrome))throw Error('Set ATMOS_TEST_CHROME to an installed Chromium executable');
const temp=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-layout-'));
const core=new ControlCore(path.join(temp,'store'));core.init();
const output=process.env.ATMOS_LAYOUT_OUTPUT?path.resolve(process.env.ATMOS_LAYOUT_OUTPUT):path.join(root,'evidence','layout');fs.mkdirSync(output,{recursive:true});
const auditScript=`(()=>{let attempts=0;function measure(){const scene=document.querySelector('.scene-picker-top'),cards=[...document.querySelectorAll('.scene-picker-top [data-scene]')];if(!scene||cards.length!==6||cards.some(e=>!e.querySelector('img')?.complete)||!window.AtmosCalendar){if(attempts++<100){setTimeout(measure,100);return}return report({error:'PAGE_NOT_READY'})}document.fonts.ready.then(()=>setTimeout(()=>{const box=e=>{const r=e.getBoundingClientRect();return {left:r.left,right:r.right,top:r.top,bottom:r.bottom,width:r.width,height:r.height}};const sceneBox=box(scene),row=document.getElementById('previewSettingsRow');report({width:innerWidth,height:innerHeight,pageWidth:document.documentElement.scrollWidth,pageClientWidth:document.documentElement.clientWidth,scene:sceneBox,columns:getComputedStyle(document.getElementById('sceneOptions')).gridTemplateColumns.split(' ').length,cards:cards.map(e=>({...box(e),contentWidth:e.scrollWidth,clientWidth:e.clientWidth,title:e.textContent.trim()})),toolbar:{...box(row),contentWidth:row.scrollWidth,clientWidth:row.clientWidth},side:box(document.querySelector('.side')),sceneBeforePreview:sceneBox.bottom<=box(row).top,horizontalScrollContainers:[...document.querySelectorAll('body *')].filter(e=>e.clientWidth>0&&e.getBoundingClientRect().height>0&&['auto','scroll'].includes(getComputedStyle(e).overflowX)&&e.scrollWidth>e.clientWidth+1).map(e=>e.id||e.className||e.tagName),overflows:[...document.querySelectorAll('body>*,main>*,#previewSettingsRow>*')].filter(e=>getComputedStyle(e).display!=='none').filter(e=>{const r=e.getBoundingClientRect();return r.left<-.5||r.right>innerWidth+.5}).map(e=>e.id||e.className||e.tagName)})},250))}function report(result){const e=parent.document.createElement('pre');e.id='layout-results';e.hidden=true;e.textContent=JSON.stringify(result);parent.document.body.append(e)}measure()})();`;
let bridge;
const server=http.createServer(async(req,res)=>{try{
 const url=new URL(req.url,'http://127.0.0.1');
 if(url.pathname==='/audit-shell'){const width=Number(url.searchParams.get('width')),height=Number(url.searchParams.get('height'));res.setHeader('Content-Type','text/html');res.end('<!doctype html><html><body style="margin:0;background:#ddd"><iframe style="display:block;border:0;width:'+width+'px;height:'+height+'px" src="/manager"></iframe></body></html>');return}
 if(url.pathname==='/layout-audit.js'){res.setHeader('Content-Type','text/javascript');res.end(auditScript);return}
 const parts=[];for await(const part of req)parts.push(part);const body=Buffer.concat(parts),headers=new Headers();for(const [key,value]of Object.entries(req.headers))if(value!==undefined)headers.set(key,Array.isArray(value)?value.join(','):value);
 const request=new Request(origin+req.url,{method:req.method,headers,...(body.length?{body}:{})});const response=await bridge.handle(request,{rawPath:req.url});
 res.writeHead(response.status,Object.fromEntries(response.headers));let bytes=Buffer.from(await response.arrayBuffer());if(url.pathname==='/manager'&&response.status===200)bytes=Buffer.from(bytes.toString().replace('</html>','<script src="/layout-audit.js"></script></html>'));res.end(bytes);
 }catch{res.writeHead(500);res.end('Fixture error')}});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const origin='http://127.0.0.1:'+server.address().port;
bridge=createBridge({store:path.join(temp,'store'),staticRoot:path.join(root,'dist'),prototypeRoot:path.resolve(root,'../prototype-v8.1-frozen'),origin});
const cases=process.argv.includes('--quick')?[[390,844,'neon'],[1280,900,'natural']]:[320,360,390,600,768,1000,1100,1101,1280,1440,1876].flatMap(width=>['natural','neon','warm','minimal'].map(theme=>[width,width<700?844:1080,theme]));
const results=[];
try{for(const [width,height,theme]of cases){
 const state=core.status(),id='layout-'+width+'-'+theme;const preview=core.preview({requestId:id,expectedRevision:state.revision,patch:{visual:{theme,locale:'en'}}});core.apply({requestId:id+'-apply',expectedRevision:state.revision,previewId:preview.previewId});
 const profile=path.join(temp,id);const screenshot=(width===390&&theme==='neon')||(width===1876&&theme==='natural');
 const args=['--headless=new','--no-first-run','--no-default-browser-check','--disable-extensions','--disable-background-networking','--disable-sync','--force-device-scale-factor=1','--user-data-dir='+profile,'--window-size='+Math.max(800,width)+','+height,'--virtual-time-budget=12000','--dump-dom',...(screenshot?['--screenshot='+path.join(output,id+'.png')]:[]),origin+'/audit-shell?width='+width+'&height='+height];
 const stdout=await new Promise((resolve,reject)=>{let text='';const child=spawn(chrome,args,{windowsHide:true,stdio:['ignore','pipe','pipe']});const timeout=setTimeout(()=>{child.kill();reject(Error('CHROMIUM_TIMEOUT'))},30000);child.stdout.on('data',chunk=>text+=chunk);child.stderr.resume();child.once('error',reject);child.once('exit',code=>{clearTimeout(timeout);code===0?resolve(text):reject(Error('CHROMIUM_EXIT_'+code))})});
 const match=stdout.match(/<pre id="layout-results" hidden(?:="")?>([\s\S]*?)<\/pre>/);let result;
 try{result=JSON.parse(match?.[1]?.replaceAll('&quot;','"').replaceAll('&amp;','&').replaceAll('&lt;','<').replaceAll('&gt;','>'))}catch{result={error:'NO_MEASUREMENT'}}
 const failures=[];if(result.error)failures.push(result.error);else{
  if(result.width!==width)failures.push('WRONG_VIEWPORT');if(result.pageWidth>result.pageClientWidth+1)failures.push('PAGE_HORIZONTAL_OVERFLOW');
  if(result.toolbar.contentWidth>result.toolbar.clientWidth+1)failures.push('TOOLBAR_HORIZONTAL_SCROLL');
  if(result.horizontalScrollContainers.length)failures.push('EXTRA_HORIZONTAL_SCROLL');if(!result.sceneBeforePreview)failures.push('SCENE_ORDER');if(result.overflows.length)failures.push('OFFSCREEN_SECTION');
  for(const card of result.cards)if(card.width<85||card.left<-.5||card.right>width+.5||card.contentWidth>card.clientWidth+1)failures.push('CARD_SQUEEZED_OR_OVERFLOW');
 }
 results.push({width,height,theme,...result,failures});console.log(JSON.stringify({width,theme,failures,pageWidth:result.pageWidth,columns:result.columns}));
}}finally{await new Promise(resolve=>server.close(resolve))}
fs.writeFileSync(path.join(output,'report.json'),JSON.stringify({kind:'Isolated Chromium CLI render; synthetic fixture only; no user browser/desktop automation',cases:results,passed:results.every(r=>!r.failures.length)},null,2)+'\n');
if(results.some(r=>r.failures.length))process.exitCode=1;
