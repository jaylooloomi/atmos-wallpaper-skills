// Read-only, source-only migration baseline. Build-time Node is intentional.
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';

const repo=fileURLToPath(new URL('../../',import.meta.url));
const product='implementation-p4_2_5/src/';
const entries=['windows-runtime.mjs','agent-cli.mjs','appearance-cli.mjs','library-cli.mjs','quick-actions.mjs','library-worker.mjs','calendar-worker.mjs'].map(p=>product+p);
const modules=new Map(),builtins=new Set(),packages=new Set();
function visit(relative){
 if(modules.has(relative))return;
 const source=fs.readFileSync(path.join(repo,relative),'utf8');
 const dependencies=[...new Set([
  ...[...source.matchAll(/\b(?:import|export)\s+(?:(?:[^'";\n]*?)\s+from\s*)?["']([^"']+)["']|\bimport\s*\(\s*["']([^"']+)["']/g)].map(m=>m[1]||m[2]),
  ...[...source.matchAll(/new URL\(\s*["']([^"']+\.mjs)["']\s*,\s*import\.meta\.url\s*\)/g)].map(m=>m[1]),
 ])].sort();
 modules.set(relative,{path:relative,bytes:Buffer.byteLength(source),sha256:createHash('sha256').update(source).digest('hex'),dependencies});
 for(const specifier of dependencies){
  if(specifier.startsWith('node:'))builtins.add(specifier);
  else if(specifier.startsWith('.')){
   const target=path.posix.normalize(path.posix.join(path.posix.dirname(relative),specifier));
   if(target.startsWith('../')||path.posix.isAbsolute(target))throw Error('Dependency escaped repository');
   visit(target);
  }else packages.add(specifier);
 }
}
for(const entry of entries)visit(entry);
const nativeLaunchSites=[];
for(const name of fs.readdirSync(path.join(repo,'desktop-host/src')).filter(n=>n.endsWith('.rs'))){
 const relative='desktop-host/src/'+name,lines=fs.readFileSync(path.join(repo,relative),'utf8').split('\n');
 for(const [i,line] of lines.entries())if(/Command::new\(.*(?:node\.exe|options\.node)/.test(line))nativeLaunchSites.push({path:relative,line:i+1});
}
const routes=[...new Set([...modules.values()].flatMap(m=>[...fs.readFileSync(path.join(repo,m.path),'utf8').matchAll(/["'](\/api\/[a-zA-Z0-9_/-]+)["']/g)].map(x=>x[1])))].sort();
const report={schemaVersion:1,method:'literal ES module imports and new URL worker edges; source bytes, not installed footprint or process memory',limitations:['Dynamic/computed imports, external package transitive dependencies and generated browser bundles are not traversed.','API literals include prefixes; this is an inventory, not proof of route or runtime parity.','No installed application, user store, backup or process was read.'],entries,nativeLaunchSites,summary:{sourceModules:modules.size,sourceBytes:[...modules.values()].reduce((n,m)=>n+m.bytes,0),nodeBuiltins:builtins.size,externalPackages:packages.size,apiLiterals:routes.length},builtins:[...builtins].sort(),packages:[...packages].sort(),apiLiterals:routes,modules:[...modules.values()].sort((a,b)=>a.path.localeCompare(b.path))};
process.stdout.write(JSON.stringify(report,null,2)+'\n');
