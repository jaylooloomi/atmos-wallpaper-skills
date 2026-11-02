import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {parseCalendar} from '../src/calendar-parse.mjs';
import {publicAddress,subscriptionURL,fetchCalendar} from '../src/calendar-fetch.mjs';
import {createCalendarService,parseIsolated} from '../src/calendar-service.mjs';
import {Store} from '../src/portable-store.mjs';
import {ControlCore} from '../src/portable-core.mjs';
import {createBridge} from '../src/router.mjs';
import {EventEmitter} from 'node:events';
import {Readable} from 'node:stream';
const now=Date.parse('2026-11-03T00:00:00+08:00'),options={now,zone:'Asia/Taipei'};
const ics=events=>'BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Atmos Tests//EN\r\n'+events+'\r\nEND:VCALENDAR';
const event=(extra='',uid='one')=>`BEGIN:VEVENT\r\nUID:${uid}\r\nDTSTART:20261103T020000Z\r\nDTEND:20261103T030000Z\r\nSUMMARY:PRIVATE_TITLE\r\n${extra}\r\nEND:VEVENT`;
function temp(t){const dir=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-calendar-'));t.after(()=>fs.rmSync(dir,{recursive:true,force:true}));return dir}
test('ICS parses UTF-8, folded lines, all-day exclusive end and safe plain text titles',()=>{
 const text=ics(event().replace('PRIVATE_TITLE','<img src=x onerror=alert(1)>中文\r\n 測試')+'\r\nBEGIN:VEVENT\r\nUID:all\r\nDTSTART;VALUE=DATE:20261102\r\nDTEND;VALUE=DATE:20261104\r\nSUMMARY:全天\r\nEND:VEVENT');
 const result=parseCalendar(text,options);assert.equal(result.events.length,2);assert.equal(result.events[0].allDay,true);assert.equal(result.events[1].title,'<img src=x onerror=alert(1)>中文測試');assert.equal(Date.parse(result.events[1].start),Date.parse('2026-11-03T10:00:00+08:00'));
});
test('recurrences respect excluded dates and overridden occurrences',()=>{
 const master=event('RRULE:FREQ=DAILY;COUNT=4\r\nEXDATE:20261104T020000Z');
 const exception='BEGIN:VEVENT\r\nUID:one\r\nRECURRENCE-ID:20261105T020000Z\r\nDTSTART:20261105T040000Z\r\nDTEND:20261105T050000Z\r\nSUMMARY:改期\r\nEND:VEVENT';
 const events=parseCalendar(ics(master+'\r\n'+exception),options).events;assert.equal(events.length,3);assert.equal(events[1].title,'改期');assert.equal(events[1].start,'2026-11-05T04:00:00.000Z');
});
test('standalone recurrence instances remain visible when a feed omits their master',()=>{
 const text=ics(event('RECURRENCE-ID:20261103T020000Z'));assert.equal(parseCalendar(text,options).events.length,1);
});
test('IANA time zones without VTIMEZONE resolve through system time-zone data',()=>{
 const text=ics(event().replace('DTSTART:20261103T020000Z','DTSTART;TZID=America/New_York:20261103T100000').replace('DTEND:20261103T030000Z','DTEND;TZID=America/New_York:20261103T110000'));
 assert.equal(Date.parse(parseCalendar(text,options).events[0].start),Date.parse('2026-11-03T15:00:00Z'));
 assert.throws(()=>parseCalendar(text.replaceAll('America/New_York','Unknown/Zone'),options),e=>e.code==='ICS_TIMEZONE_UNSUPPORTED');
});
test('cancelled events and events ending before the window are absent',()=>{
 const result=parseCalendar(ics(event('STATUS:CANCELLED')+'\r\n'+event('', 'old').replaceAll('20261103','20261002')),options);assert.equal(result.events.length,0);
 assert.throws(()=>parseCalendar('<html>sign in</html>',options),e=>e.code==='ICS_INVALID');
 assert.throws(()=>parseCalendar('x'.repeat(1048577),options),e=>e.code==='ICS_TOO_LARGE');
 const midnight=event().replace('DTSTART:20261103T020000Z','DTSTART:20261102T150000Z').replace('DTEND:20261103T030000Z','DTEND:20261102T160000Z');assert.equal(parseCalendar(ics(midnight),options).events.length,0);
});
test('dense ancient recurrence is bounded and parsing runs outside the server event loop',async()=>{
 await assert.rejects(parseIsolated(ics(event('RRULE:FREQ=SECONDLY').replaceAll('20261103','20000101')),options),e=>['ICS_TOO_COMPLEX','ICS_INVALID'].includes(e.code));
 assert.equal((await parseIsolated(ics(event()),options)).events.length,1);
});
test('subscription URL validation blocks non-HTTPS, credentials, nonstandard ports and private networks',async()=>{
 for(const address of ['127.0.0.1','10.0.0.1','169.254.169.254','100.64.0.1','0.0.0.0','192.168.1.1','::1','::ffff:127.0.0.1','fc00::1','2001:db8::1','2002:7f00:1::'])assert.equal(publicAddress(address),false,address);
 assert.equal(publicAddress('8.8.8.8'),true);assert.equal(publicAddress('2606:4700:4700::1111'),true);
 for(const url of ['file:///secret','http://example.com/a.ics','https://u:p@example.com/a','https://example.com:444/a'])assert.throws(()=>subscriptionURL(url));
 assert.equal(subscriptionURL('webcal://example.com/a.ics').href,'https://example.com/a.ics');
 let called=false;await assert.rejects(fetchCalendar('https://example.com/private',{lookup:async()=>[{address:'127.0.0.1',family:4}],request:()=>{called=true}}),/ICS_URL_BLOCKED/);assert.equal(called,false);
});
test('HTTPS fetch pins the validated DNS address and checks redirect destinations again',async()=>{
 let pinned,requests=0;
 const request=(url,config,callback)=>{requests++;config.lookup(url.hostname,{},(_,address)=>pinned=address);const req=new EventEmitter();req.end=()=>{const response=Readable.from([Buffer.from(ics(event()))]);response.statusCode=200;response.headers={};callback(response);response.once('end',()=>req.emit('close'))};req.destroy=e=>{req.emit('error',e);req.emit('close')};return req};
 const text=await fetchCalendar('https://calendar.example/a.ics',{lookup:async()=>[{address:'8.8.8.8',family:4}],request});assert.match(text,/BEGIN:VCALENDAR/);assert.equal(pinned,'8.8.8.8');assert.equal(requests,1);
 const redirect=(url,config,callback)=>{const req=new EventEmitter();req.end=()=>{const res=Readable.from([]);res.statusCode=302;res.headers={location:'https://127.0.0.1/secret'};callback(res);res.once('end',()=>req.emit('close'))};req.destroy=e=>{req.emit('error',e);req.emit('close')};return req};
 await assert.rejects(fetchCalendar('https://calendar.example/a.ics',{lookup:async()=>[{address:'8.8.8.8',family:4}],request:redirect}),/ICS_URL_BLOCKED/);
});
test('import persists privately, restart reloads it, failed import retains good content, and disconnect clears it',async t=>{
 const store=new Store(temp(t));let service=createCalendarService({store,now:()=>now});
 const status=await service.change('import',{name:'測試',text:ics(event())});assert.equal(status.events.length,1);assert.equal(status.type,'file');assert.equal(Object.hasOwn(status,'url'),false);
 if(process.platform==='win32')assert.equal(fs.readFileSync(store.path('calendar/connection.bin')).includes(Buffer.from('PRIVATE_TITLE')),false);
 service=createCalendarService({store,now:()=>now});assert.equal((await service.events()).events[0].title,'PRIVATE_TITLE');
 await assert.rejects(service.change('import',{text:'invalid'}));assert.equal((await service.events()).events.length,1);
 await service.change('disconnect',{});assert.equal((await service.events()).configured,false);assert.equal((await createCalendarService({store,now:()=>now}).events()).configured,false);
});
test('subscription refresh hides the URL, keeps last good data offline and can be removed',async t=>{
 const store=new Store(temp(t));let offline=false,clock=now;
 const service=createCalendarService({store,now:()=>clock,fetcher:async()=>{if(offline)throw Error('network contains secret URL');return ics(event())}});
 const value=await service.change('subscribe',{name:'訂閱',url:'https://example.com/PRIVATE_FEED_TOKEN.ics'});assert.equal(JSON.stringify(value).includes('PRIVATE_FEED_TOKEN'),false);
 offline=true;clock+=16*60000;const result=await service.events();assert.equal(result.events.length,1);assert.equal(result.lastError,'ICS_FETCH_FAILED');
});
test('concurrent first reads after restart await the same parse instead of reporting an empty calendar',async t=>{
 const store=new Store(temp(t));await createCalendarService({store,now:()=>now}).change('import',{text:ics(event())});
 let finish,calls=0;const service=createCalendarService({store,now:()=>now,parser:()=>{calls++;return new Promise(resolve=>finish=resolve)}});
 const first=service.events(),second=service.events();assert.equal(calls,1);finish(parseCalendar(ics(event()),options));
 const values=await Promise.all([first,second]);assert(values.every(value=>value.events.length===1));
});
test('calendar HTTP routes enforce origin and manager writes; wallpaper can only read normalized events',async t=>{
 const dir=temp(t),root=fileURLToPath(new URL('../',import.meta.url)),origin='http://127.0.0.1:4173';new ControlCore(dir).init();
 const bridge=createBridge({store:dir,staticRoot:path.join(root,'dist'),prototypeRoot:path.resolve(root,'../prototype-v8.1-frozen'),origin});
 const call=(route,body,wallpaper=false,extra={})=>bridge.handle(new Request(origin+route,{method:body?'POST':'GET',headers:{Host:'127.0.0.1:4173',Origin:origin,'Sec-Fetch-Site':'same-origin','Content-Type':'application/json','X-Atmos-Session':wallpaper?bridge.wallpaperSessionToken:bridge.sessionToken,...extra},...(body?{body:JSON.stringify(body)}:{})}));
 assert.equal((await call('/api/calendar/import',{text:ics(event())},true)).status,403);
 assert.equal((await call('/api/calendar/import',{text:ics(event())},false,{Origin:'https://attacker.example'})).status,403);
 assert.equal((await call('/api/calendar/import',{text:ics(event())})).status,200);
 const data=await(await call('/api/calendar/events',null,true)).json();assert.equal(data.configured,true);assert.equal(Object.hasOwn(data,'text'),false);assert.equal(Object.hasOwn(data,'url'),false);
 assert.equal((await call('/api/calendar/disconnect',{})).status,200);
});
