import fs from 'node:fs';import assert from 'node:assert/strict';import {createHash} from 'node:crypto';import {validateRelativeName,validateLocalRoot} from '../src/portable-store.mjs';
const cases=[['relative','imports/COM¹.png'],['relative','imports/LPT².txt'],['root','C:\\data:stream'],['root','C:\\NUL\\data']];
const results=cases.map(([kind,value])=>{try{(kind==='root'?v=>validateLocalRoot(v,'win32'):validateRelativeName)(value);return {kind,value,rejected:false}}catch(e){return {kind,value,rejected:e.code==='PATH_REJECTED',code:e.code}}});
assert.equal(validateRelativeName('imports/山湖.png'),'imports/山湖.png');assert.equal(validateLocalRoot('C:\\Users\\Person\\Atmos','win32'),'C:\\Users\\Person\\Atmos');
const source=fs.readFileSync(new URL('../src/portable-store.mjs',import.meta.url));const report={kind:'Pure path-string validation; no Windows filesystem, listener or browser',sourceSha256:createHash('sha256').update(source).digest('hex'),results,allRejected:results.every(r=>r.rejected)};fs.writeFileSync(new URL('../evidence/independent-path-review.json',import.meta.url),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));

assert.equal(report.allRejected,true);
