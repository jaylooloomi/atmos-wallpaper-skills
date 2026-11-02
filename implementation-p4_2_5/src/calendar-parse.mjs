import ICAL from 'ical.js';
import {DateTime} from 'luxon';
const fail=code=>{throw Object.assign(new Error(code),{code})};
export function parseCalendar(text,{now=Date.now(),zone=Intl.DateTimeFormat().resolvedOptions().timeZone}={}){
 if(typeof text!=='string'||Buffer.byteLength(text)>1048576)fail('ICS_TOO_LARGE');
 let root;try{root=new ICAL.Component(ICAL.parse(text.replace(/^\uFEFF/,'')))}catch{fail('ICS_INVALID')}
 if(root.name!=='vcalendar'||!root.hasProperty('version'))fail('ICS_INVALID');
 const from=DateTime.fromMillis(now,{zone}).startOf('day'),until=from.plus({days:7});
 const components=root.getAllSubcomponents('vevent');if(components.length>3000)fail('ICS_TOO_COMPLEX');
 const masters=new Set(),exceptions=new Map();
 for(const component of components){const uid=component.getFirstPropertyValue('uid');if(component.hasProperty('recurrence-id')){if(!exceptions.has(uid))exceptions.set(uid,[]);exceptions.get(uid).push(component)}else masters.add(uid)}
 const result=[],seen=new Set();let iterations=0;
 function date(time,item,property){
  if(!time)fail('ICS_INVALID');
  const tz=item.component.getFirstProperty(property)?.getParameter('tzid')||item.component.getFirstProperty('dtstart')?.getParameter('tzid');
  if(!time.isDate&&time.zone?.tzid&&!['floating','UTC'].includes(time.zone.tzid))return DateTime.fromJSDate(time.toJSDate(),{zone});
  const fields={year:time.year,month:time.month,day:time.day,hour:time.isDate?0:time.hour,minute:time.isDate?0:time.minute,second:time.isDate?0:time.second};
  const value=DateTime.fromObject(fields,{zone:time.zone?.tzid==='UTC'?'UTC':tz||zone});
  if(!value.isValid)fail('ICS_TIMEZONE_UNSUPPORTED');return value;
 }
 function add(item,start,end){
  if(String(item.component.getFirstPropertyValue('status')).toUpperCase()==='CANCELLED')return;
  const a=date(start,item,'dtstart'),b=date(end,item,'dtend');
  if(a>=until||b<from||(b.toMillis()===from.toMillis()&&b>a))return;
  const id=(item.uid||'event')+':'+a.toISO();if(seen.has(id))return;seen.add(id);
  result.push({id,title:String(item.summary||'（未命名行程）').slice(0,300),start:a.toISO(),end:b.toISO(),allDay:!!start.isDate});
 }
 try{for(const component of components){
  const event=new ICAL.Event(component,{exceptions:component.hasProperty('recurrence-id')?[]:exceptions.get(component.getFirstPropertyValue('uid'))||[]});
  if(event.isRecurrenceException()&&masters.has(event.uid))continue;
  if(String(component.getFirstPropertyValue('status')).toUpperCase()==='CANCELLED')continue;
  if(!event.startDate)fail('ICS_INVALID');
  if(!event.isRecurring()){add(event,event.startDate,event.endDate);continue}
  const iterator=event.iterator();let occurrence;
  while((occurrence=iterator.next())){
   if(++iterations>50000)fail('ICS_TOO_COMPLEX');
   const item=event.getOccurrenceDetails(occurrence);add(item.item,item.startDate,item.endDate);
   if(date(occurrence,event,'dtstart')>=until)break;
  }
 }}catch(e){if(e.code)throw e;fail('ICS_INVALID')}
 result.sort((a,b)=>Date.parse(a.start)-Date.parse(b.start)||a.title.localeCompare(b.title));
 return {events:result.slice(0,200),truncated:result.length>200,from:from.toISO(),until:until.toISO(),zone};
}
