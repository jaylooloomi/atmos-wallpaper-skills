import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import {parseHTML} from 'linkedom';
function setup(date,mode='demo',wallpaper=false){
 const {window}=parseHTML('<html><body><div id="calendar"></div><section class="calendar-section"><button>Google Microsoft</button></section></body></html>');
 let current=date,interval,requests=0;
 class Clock extends Date{constructor(...args){super(...(args.length?args:[current]))}}
 window.atmosBridgeApp={getState:()=>({locale:'zh-TW',calendarMode:mode})};
 vm.runInNewContext(fs.readFileSync(new URL('../src/calendar-client.js',import.meta.url),'utf8'),{window,document:window.document,Date:Clock,Intl,setInterval(fn){interval=fn},addEventListener(){},fetch(){requests++;throw Error('No calendar requests allowed')}});
 window.AtmosCalendar.start({wallpaper,headers:()=>({})});
 return {window,calendar:window.document.getElementById('calendar'),advance(date){current=date;interval()},get requests(){return requests}};
}
test('plain calendar renders actual leap month and today without accounts, events or network calls',()=>{
 const x=setup(new Date(2024,1,29));assert.equal(x.calendar.querySelectorAll('[data-day]').length,29);assert.equal(x.calendar.querySelectorAll('.month-weekday').length,7);assert.equal(x.calendar.querySelector('[aria-current=date]').dataset.day,'29');assert.equal(x.calendar.querySelector('.month-day').getAttribute('aria-hidden'),'true');assert.equal(x.window.document.querySelector('.calendar-section'),null);assert.doesNotMatch(x.calendar.textContent,/Google|Microsoft|ICS|14:00|專注時間/);assert.equal(x.requests,0);
 const first=x.calendar.firstElementChild;x.window.AtmosCalendar.render();assert.equal(x.calendar.firstElementChild,first);x.advance(new Date(2024,2,1));assert.equal(x.calendar.querySelectorAll('[data-day]').length,31);assert.equal(x.calendar.querySelector('[aria-current=date]').dataset.day,'1');assert.equal(x.requests,0);
});
test('all legacy saved calendar modes show the same month without modifying settings',()=>{
 for(const mode of ['demo','ics','off'])for(const wallpaper of [false,true]){const x=setup(new Date(2026,10,6),mode,wallpaper);assert.equal(x.calendar.querySelectorAll('[data-day]').length,30);assert.equal(x.calendar.querySelector('[aria-current=date]').dataset.day,'6');assert.equal(x.requests,0)}
});
