import test from 'node:test';import assert from 'node:assert/strict';import fs from 'node:fs';import vm from 'node:vm';import {parseHTML} from 'linkedom';
test('removed mode/playlist controls do not mount or mutate saved policy; focus still synchronizes',async()=>{
 for(const wallpaper of [false,true]){
  const {window}=parseHTML('<html><body data-render-mode="'+(wallpaper?'wallpaper':'manager')+'"><aside class="side"><div class="panel-scroll"></div></aside></body></html>');let mode='normal';const calls=[],intervals=[],events=[];
  const context=vm.createContext({window,document:window.document,AbortSignal,fetch:async(url,options)=>{calls.push({url,method:options.method});return {ok:true,json:async()=>({mode})}},setInterval:fn=>intervals.push(fn),addEventListener:(event,fn)=>events.push({event,fn})});
  for(const name of ['focus-client.js','schedule-client.js'])vm.runInContext(fs.readFileSync(new URL('../src/'+name,import.meta.url),'utf8'),context);
  window.AtmosFocus.start({wallpaper,headers:()=>({'X-Atmos-Session':'fixture'})});window.AtmosSchedule.start({wallpaper,headers:()=>({})});await new Promise(r=>setImmediate(r));
  assert.equal(window.document.querySelector('.side .panel-scroll').children.length,0);assert.equal(window.document.querySelector('[data-mode],[data-save],[data-enabled]'),null);assert.equal(window.document.body.dataset.atmosFocus,'normal');assert.ok(calls.every(c=>c.method==='GET'&&c.url==='/api/host/focus'));
  mode='focus';await intervals[0]();assert.equal(window.document.body.dataset.atmosFocus,'focus');mode='showcase';await intervals[0]();assert.equal(window.document.body.dataset.atmosFocus,'showcase');assert.equal(events.length,1);
  window.AtmosFocus.start({wallpaper,headers:()=>({})});assert.equal(intervals.length,1);
 }
});
