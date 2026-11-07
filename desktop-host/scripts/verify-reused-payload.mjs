// Diagnostic frontend comparison only; never authorizes release or payload reuse.
import fs from 'node:fs';import path from 'node:path';import {createHash} from 'node:crypto';
export function verifyReusedFrontend(prepared,cached){
 const fail=()=>{throw Error('REUSED_PAYLOAD_STALE: rebuild without --reuse-payload')};
 const walk=(dir,prefix='')=>{if(!fs.existsSync(dir)||fs.lstatSync(dir).isSymbolicLink())fail();return fs.readdirSync(dir,{withFileTypes:true}).flatMap(e=>{if(e.isSymbolicLink())fail();const name=prefix+e.name;return e.isDirectory()?walk(path.join(dir,e.name),name+'/'):e.isFile()?[name]:fail()}).sort()};
 const expected=walk(prepared),actual=walk(cached);if(JSON.stringify(expected)!==JSON.stringify(actual))fail();
 for(const name of expected){const a=path.join(prepared,name),b=path.join(cached,name);if(fs.statSync(a).size!==fs.statSync(b).size)fail();const hash=p=>createHash('sha256').update(fs.readFileSync(p)).digest('hex');if(hash(a)!==hash(b))fail()}
 return {matched:true,files:expected.length};
}
