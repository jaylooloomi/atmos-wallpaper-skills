import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import net from 'node:net';
import http from 'node:http';
import {spawn,spawnSync} from 'node:child_process';
import {once} from 'node:events';
import {fileURLToPath} from 'node:url';

// Node fetch supplies its own Sec-Fetch-Mode; use HTTP to exercise navigation headers.
function request(url,options={}){return new Promise((resolve,reject)=>{
 const req=http.request(url,options,res=>{const chunks=[];res.on('data',c=>chunks.push(c));res.on('end',()=>resolve(new Response(Buffer.concat(chunks),{status:res.statusCode,headers:res.headers})));res.on('error',reject)});
 req.on('error',reject);req.end(options.body);
})}

test('Windows HTTP runtime serves both roles and enforces authenticated writes', {skip:process.platform!=='win32',timeout:20000}, async t=>{
 const root=fileURLToPath(new URL('../',import.meta.url));
 const python=process.env.ATMOS_PYTHON||spawnSync('python',['-c','import sys; print(sys.executable)'],{encoding:'utf8',windowsHide:true}).stdout?.trim();
 assert.ok(python&&path.isAbsolute(python),'Set ATMOS_PYTHON to an existing Python executable or add Python to PATH');
 const store=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-http-test-'));
 const reservation=net.createServer();
 await new Promise(resolve=>reservation.listen(0,'127.0.0.1',resolve));
 const port=reservation.address().port;
 await new Promise(resolve=>reservation.close(resolve));
 const origin=`http://127.0.0.1:${port}`;
 const child=spawn(process.execPath,[path.join(root,'src/windows-runtime.mjs'),'--port',String(port),'--strictPort','--start-local-service'],{env:{...process.env,PORT:String(port),ATMOS_STORE:store,ATMOS_PYTHON:python},windowsHide:true,stdio:['ignore','pipe','pipe']});
 t.after(async()=>{if(child.exitCode===null){const done=once(child,'exit');child.kill();await done}fs.rmSync(store,{recursive:true,force:true})});
 await new Promise((resolve,reject)=>{let output='';child.stdout.on('data',chunk=>{output+=chunk;if(output.includes('runtime ready'))resolve()});child.once('error',reject);child.once('exit',code=>reject(Error(`Runtime exited ${code}: ${output}`)));child.stderr.on('data',chunk=>{output+=chunk})});
 const tokens={};
 for(const [role,route] of [['manager','/manager'],['wallpaper','/wallpaper']]){
  const response=await request(origin+route,{headers:{'Sec-Fetch-Mode':'navigate'}});
  assert.equal(response.status,200);
  assert.match(response.headers.get('content-security-policy'),/object-src 'none'/);
  const html=await response.text();
  tokens[role]=JSON.parse(html.match(/window\.atmosBridgeSession=("[^"]+")/)[1]);
 }
 assert.notEqual(tokens.manager,tokens.wallpaper);
 const headers=role=>({'Origin':origin,'Sec-Fetch-Site':'same-origin','X-Atmos-Session':tokens[role],'Content-Type':'application/json'});
 assert.equal((await fetch(origin+'/api/snapshot')).status,403);
 const before=await (await fetch(origin+'/api/snapshot',{headers:headers('manager')})).json();
 assert.equal(before.revision,0);
 const command={schemaVersion:1,type:'settings.update',requestId:'http-smoke',expectedRevision:0,changes:{visual:{effect:'snow'}}};
 const write=role=>fetch(origin+'/api/commands',{method:'POST',headers:headers(role),body:JSON.stringify(command)});
 assert.equal((await write('wallpaper')).status,403);
 assert.equal((await write('manager')).status,200);
 assert.equal((await write('manager')).status,200);
 const after=await (await fetch(origin+'/api/snapshot',{headers:headers('wallpaper')})).json();
 assert.equal(after.revision,1);
 assert.equal(after.desired.visual.effect,'snow');
 assert.equal(JSON.parse(fs.readFileSync(path.join(store,'live.json'))).revision,1);
 assert.equal((await fetch(origin+'/api/commands',{method:'POST',headers:headers('manager'),body:'x'.repeat(32769)})).status,413);
 for(const route of ['/app.js','/widgets.js','/bridge-client.js'])assert.equal((await fetch(origin+route)).status,200);
});
