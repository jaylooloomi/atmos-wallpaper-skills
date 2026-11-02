import https from 'node:https';
import dns from 'node:dns/promises';
import {BlockList,isIP} from 'node:net';
const blocked=new BlockList();
for(const [ip,bits] of [['0.0.0.0',8],['10.0.0.0',8],['100.64.0.0',10],['127.0.0.0',8],['169.254.0.0',16],['172.16.0.0',12],['192.0.0.0',24],['192.0.2.0',24],['192.168.0.0',16],['198.18.0.0',15],['198.51.100.0',24],['203.0.113.0',24],['224.0.0.0',4],['240.0.0.0',4]])blocked.addSubnet(ip,bits,'ipv4');
const ipv6=new BlockList();ipv6.addSubnet('2000::',3,'ipv6');
blocked.addSubnet('192.88.99.0',24,'ipv4');
const special6=new BlockList();for(const [ip,bits] of [['2001::',23],['2001:db8::',32],['2002::',16]])special6.addSubnet(ip,bits,'ipv6');
export const publicAddress=address=>isIP(address)===4?!blocked.check(address,'ipv4'):isIP(address)===6&&ipv6.check(address,'ipv6')&&!special6.check(address,'ipv6');
export function subscriptionURL(input){
 let url;try{url=new URL(String(input).replace(/^webcal:/i,'https:'))}catch{throw Error('ICS_URL_INVALID')}
 if(String(input).length>4096||url.protocol!=='https:'||url.username||url.password||url.hash||(url.port&&url.port!=='443')||url.hostname==='localhost')throw Error('ICS_URL_INVALID');
 return url;
}
export async function fetchCalendar(input,{lookup=dns.lookup,request=https.request}={}){
 const deadline=Date.now()+12000;
 async function get(input,redirects=0){
  const url=subscriptionURL(input),hostname=url.hostname.replace(/^\[|\]$/g,'');
  const records=isIP(hostname)?[{address:hostname,family:isIP(hostname)}]:await Promise.race([lookup(hostname,{all:true,verbatim:true}),new Promise((_,reject)=>{const timer=setTimeout(()=>reject(Error('ICS_FETCH_FAILED')),Math.max(1,deadline-Date.now()));timer.unref()})]);
  if(!records.length||records.some(r=>!publicAddress(r.address)))throw Error('ICS_URL_BLOCKED');
  const pin=records[0];
  return new Promise((resolve,reject)=>{
   const req=request(url,{family:pin.family,autoSelectFamily:false,agent:false,lookup:(_,options,callback)=>callback(null,pin.address,pin.family),headers:{Accept:'text/calendar','Accept-Encoding':'identity','User-Agent':'Atmos-Calendar/1.0'}},res=>{
    if([301,302,303,307,308].includes(res.statusCode)){res.resume();if(redirects>=3||!res.headers.location)return reject(Error('ICS_FETCH_FAILED'));get(new URL(res.headers.location,url).href,redirects+1).then(resolve,reject);return}
    if(res.statusCode!==200){res.resume();reject(Error('ICS_FETCH_FAILED'));return}
    let size=0;const chunks=[];
    res.on('data',chunk=>{size+=chunk.length;if(size>1048576){req.destroy(Error('ICS_TOO_LARGE'));return}chunks.push(chunk)});
    res.on('error',reject);res.on('end',()=>resolve(Buffer.concat(chunks).toString('utf8')));
   });
   const timeout=setTimeout(()=>req.destroy(Error('ICS_FETCH_FAILED')),Math.max(1,deadline-Date.now()));timeout.unref();
   req.on('close',()=>clearTimeout(timeout));req.on('error',reject);req.end();
  });
 }
 return get(input);
}
