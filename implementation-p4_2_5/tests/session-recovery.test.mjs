import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
const context={window:{}};vm.runInNewContext(fs.readFileSync(new URL('../src/session-recovery.js',import.meta.url),'utf8'),context);
const Recovery=context.window.AtmosSessionRecovery;
function setup(wallpaper=true){const values=new Map(),jobs=new Map();let id=0,reloads=0;const recovery=new Recovery({wallpaper,key:'test',storage:{getItem:k=>values.get(k),setItem:(k,v)=>values.set(k,v),removeItem:k=>values.delete(k)},reload:()=>reloads++,schedule:(fn,delay)=>{jobs.set(++id,{fn,delay});return id},cancel:id=>jobs.delete(id)});return {recovery,values,jobs,get reloads(){return reloads}}}
test('expired wallpaper session schedules exactly one reload; healthy response cancels it',()=>{const x=setup();assert.equal(x.recovery.unauthorized(),true);x.recovery.unauthorized();assert.equal(x.jobs.size,1);assert.equal([...x.jobs.values()][0].delay,1000);x.recovery.healthy();assert.equal(x.jobs.size,0);assert.equal(x.values.size,0);assert.equal(x.reloads,0)});
test('manager never reloads and never discards drafts after an unauthorized response',()=>{const x=setup(false);assert.equal(x.recovery.unauthorized(),false);assert.equal(x.jobs.size,0);assert.equal(x.values.size,0)});
test('repeated rejected sessions back off across reloads and cap at one minute',()=>{const x=setup();for(let n=0;n<10;n++){x.recovery.unauthorized();const job=[...x.jobs.values()].at(-1);assert.equal(job.delay,Math.min(60000,1000*2**n));job.fn()}assert.equal(x.reloads,10)});
test('unavailable session storage still permits recovery',()=>{let delay;const x=new Recovery({wallpaper:true,key:'test',storage:{getItem(){throw Error('unavailable')},removeItem(){throw Error('unavailable')}},schedule:(_,ms)=>{delay=ms;return 1},cancel:()=>{}});assert.equal(x.unauthorized(),true);assert.equal(delay,1000);x.healthy()});
test('browser timers are called without the recovery object as their receiver',()=>{let called=false;context.setTimeout=function(){assert.equal(this instanceof Recovery,false);called=true;return 1};context.clearTimeout=function(){assert.equal(this instanceof Recovery,false)};const x=new Recovery({wallpaper:true,key:'test',storage:{getItem:()=>null,setItem:()=>{},removeItem:()=>{}}});x.unauthorized();assert.equal(called,true);x.healthy()});
