import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import {parseHTML} from 'linkedom';
function setup(wallpaper=false){
 const {window}=parseHTML('<html><body><main><div id="calendar"></div><section class="calendar-section"><button id="google">Google</button></section></main></body></html>');
 const state={calendarMode:'ics',locale:'zh-TW'},calls=[];let marked=false;
 window.atmosBridgeApp={getState:()=>state,update:patch=>Object.assign(state,patch)};window.atmosBridgeChanged=()=>{marked=true};
 const data={configured:true,type:'file',name:'Test',events:[{title:'<img src=x onerror=alert(1)>私人行程',start:'2026-11-03T02:00:00Z',end:'2026-11-03T03:00:00Z',allDay:false}]};
 const context={window,document:window.document,MutationObserver:window.MutationObserver,Intl,Date,AbortSignal,setInterval:()=>1,addEventListener:()=>{},fetch:async(url,config)=>{calls.push({url,config});return {ok:true,json:async()=>data}}};
 vm.runInNewContext(fs.readFileSync(new URL('../src/calendar-client.js',import.meta.url),'utf8'),context);
 window.AtmosCalendar.start({wallpaper,headers:()=>({'X-Atmos-Session':'test-nonce'})});
 return {window,state,calls,get marked(){return marked}};
}
test('calendar DOM renders untrusted event titles as text and replaces nonfunctional OAuth controls',async()=>{
 const x=setup();await new Promise(resolve=>setImmediate(resolve));const calendar=x.window.document.getElementById('calendar');
 assert.match(calendar.textContent,/私人行程/);assert.equal(calendar.querySelector('img'),null);assert.equal(x.window.document.getElementById('google'),null);
 assert.match(x.window.document.querySelector('.calendar-section').textContent,/匯入日曆/);assert.equal(x.calls[0].config.headers['X-Atmos-Session'],'test-nonce');
 x.state.calendarMode='off';x.window.AtmosCalendar.render();assert.match(calendar.textContent,/已關閉/);
});
test('wallpaper creates no account or import controls',async()=>{
 const x=setup(true);await new Promise(resolve=>setImmediate(resolve));assert.equal(x.window.document.querySelector('input[type=file]'),null);assert.equal(x.calls.length,1);
});
test('manager import posts the file and marks calendar display mode as an explicit draft',async()=>{
 const x=setup();await new Promise(resolve=>setImmediate(resolve));x.state.calendarMode='demo';
 const input=x.window.document.querySelector('input[type=file]');Object.defineProperty(input,'files',{value:[{name:'test.ics',size:100,text:async()=>'BEGIN:VCALENDAR'}]});
 const button=[...x.window.document.querySelectorAll('button')].find(e=>e.textContent==='匯入日曆');await button.onclick();
 assert.equal(x.calls.at(-1).url,'/api/calendar/import');assert.equal(JSON.parse(x.calls.at(-1).config.body).text,'BEGIN:VCALENDAR');assert.equal(x.state.calendarMode,'ics');assert.equal(x.marked,true);
});
