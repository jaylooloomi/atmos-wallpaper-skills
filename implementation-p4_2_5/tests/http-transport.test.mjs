import test from 'node:test';
import assert from 'node:assert/strict';
import http from 'node:http';
import {once} from 'node:events';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHostBridge} from '../src/host-router.mjs';
import {createLocalServer} from '../src/http-transport.mjs';

async function fixture(t,handle){
 let origin;const server=createLocalServer({bridge:{handle:(...args)=>handle(...args)},origin:'http://127.0.0.1:12345'});
 server.listen({host:'127.0.0.1',port:0});await once(server,'listening');
 origin='http://127.0.0.1:'+server.address().port;
 t.after(()=>new Promise(resolve=>{server.close(resolve);server.closeAllConnections();}));
 return (method,path,body,headers={})=>new Promise((resolve,reject)=>{
  const req=http.request(origin+path,{method,headers:{...(body?{'Content-Length':body.length}:{}),...headers}},res=>{
   const chunks=[];res.on('data',chunk=>chunks.push(chunk));res.on('end',()=>resolve({status:res.statusCode,headers:res.headers,body:Buffer.concat(chunks)}));
  });req.on('error',reject);req.end(body);
 });
}

test('wire boundary preserves raw path, role/scope headers, bytes and bridge response',async t=>{
 let captured;
 const send=await fixture(t,async(req,context)=>{
  captured={url:req.url,method:req.method,context,headers:req.headers,body:Buffer.from(await req.arrayBuffer())};
  return new Response(new Uint8Array([0,128,255]),{status:409,headers:{'Content-Type':'application/octet-stream','Cache-Control':'no-store','X-Content-Type-Options':'nosniff'}});
 });
 const body=Buffer.from('{"expectedRevision":7}');
 const result=await send('POST','/api/state/%2f',body,{'X-Atmos-Session':'fixture-token','X-Atmos-Target':'fixture-target','X-Atmos-Scope':'shared','Sec-Fetch-Site':'same-origin','Origin':'http://127.0.0.1:12345'});
 assert.equal(captured.url,'http://127.0.0.1:12345/api/state/%2f');
 assert.equal(captured.context.rawPath,'/api/state/%2f');assert.equal(captured.method,'POST');
 assert.equal(captured.headers.get('X-Atmos-Session'),'fixture-token');
 assert.equal(captured.headers.get('X-Atmos-Target'),'fixture-target');assert.equal(captured.headers.get('X-Atmos-Scope'),'shared');
 assert.equal(captured.headers.get('Sec-Fetch-Site'),'same-origin');assert.deepEqual(captured.body,body);
 assert.equal(result.status,409);assert.equal(result.headers['cache-control'],'no-store');assert.equal(result.headers['x-content-type-options'],'nosniff');
 assert.deepEqual(result.body,Buffer.from([0,128,255]));
});

test('body ceilings are byte based; only exact calendar import gets the larger ceiling',async t=>{
 let calls=0;const send=await fixture(t,async req=>{calls++;return Response.json({bytes:(await req.arrayBuffer()).byteLength});});
 for(const [path,size,status] of [['/api/state',32768,200],['/api/state',32769,413],['/api/calendar/import',2*1048576,200],['/api/calendar/import',2*1048576+1,413],['/api/calendar/import?x=1',32769,413]]){
  const result=await send('POST',path,Buffer.alloc(size,0xc3));assert.equal(result.status,status,path+' '+size);
  if(status===413)assert.deepEqual(JSON.parse(result.body),{error:{code:'BODY_TOO_LARGE'}});
 }
 assert.equal(calls,2);
});

test('GET/HEAD bodies never reach the bridge; empty requests still do',async t=>{
 let calls=0;const send=await fixture(t,()=>{calls++;return new Response('ok');});
 assert.equal((await send('GET','/',Buffer.from('x'))).status,400);
 assert.equal((await send('HEAD','/',Buffer.from('x'))).status,400);
 assert.equal((await send('GET','/')).status,200);
 assert.equal((await send('HEAD','/')).status,200);assert.equal(calls,2);
});

test('bridge failures disclose only the established transport error',async t=>{
 const send=await fixture(t,()=>{throw Error('private store path and token');});
 const result=await send('POST','/api/state',Buffer.from('{}'));
 assert.equal(result.status,500);assert.deepEqual(JSON.parse(result.body),{error:{code:'LOCAL_OPERATION_FAILED'}});
});

test('real bridge over HTTP retains authentication, role isolation, CAS and receipts',async t=>{
 const store=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-http-contract-'));
 t.after(()=>fs.rmSync(store,{recursive:true,force:true}));
 const app=fileURLToPath(new URL('../',import.meta.url)),origin='http://127.0.0.1:12345';
 const bridge=createHostBridge({store,origin,staticRoot:path.join(app,'dist'),prototypeRoot:path.resolve(app,'../prototype-v8.1-frozen'),hostSessionId:'fixture-only'});
 const send=await fixture(t,(...args)=>bridge.handle(...args));
 const headers={Host:'127.0.0.1:12345',Origin:origin,'Sec-Fetch-Site':'same-origin','Content-Type':'application/json','X-Atmos-Session':bridge.sessionToken};
 const command={schemaVersion:1,type:'settings.update',requestId:'http.fixture.1',expectedRevision:0,changes:{visual:{effect:'rain'}}};
 const body=Buffer.from(JSON.stringify(command));
 assert.equal((await send('GET','/api/snapshot',undefined,{...headers,'X-Atmos-Session':'invalid'})).status,401);
 assert.equal((await send('POST','/api/commands',body,{...headers,'X-Atmos-Session':bridge.wallpaperSessionToken})).status,403);
 assert.equal((await send('POST','/api/commands',body,{...headers,Origin:'https://example.invalid'})).status,403);
 assert.equal(bridge.registry.owner.status().revision,0);
 const first=await send('POST','/api/commands',body,headers);assert.equal(first.status,200);
 assert.equal((await send('POST','/api/commands',body,headers)).status,200);
 assert.equal(bridge.registry.owner.status().revision,1);
 const conflict=await send('POST','/api/commands',Buffer.from(JSON.stringify({...command,requestId:'http.fixture.2'})),headers);
 assert.equal(conflict.status,409);assert.equal(bridge.registry.owner.status().revision,1);
 assert.equal(bridge.registry.owner.status().desired.visual.effect,'rain');
 assert.equal(bridge.rendererReports().length,0);
});
