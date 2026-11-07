import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {initialDesired as v1} from '../../implementation-p1/src/state.mjs';
import {initialDesired as v2} from '../../implementation-p4_2_5/src/state-v2.mjs';
import {initialDesired as v3} from '../../implementation-p4_2_5/src/state-v3.mjs';
import {initialDesired as v4,capabilities,canonical} from '../../implementation-p4_2_5/src/state.mjs';
import {validateSupportedDesired,normalizeLegacyDesired} from '../../implementation-p4_2_5/src/legacy-config.mjs';
import {Store} from '../../implementation-p4_2_5/src/portable-store.mjs';
const defaults={'1':v1(),'2':v2(),'3':v3(),'4':v4(),capabilities};
const temporary=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-state-contract-'));
try{
 const fake={path:name=>path.join(temporary,name),atomic(){}};
 defaults.migrationTexts={};
 for(const [name,effect] of [['other','clear'],['tornado','tornado']])defaults.migrationTexts[name]=Store.prototype.recordLegacyFallback.call(fake,'live',{desired:{visual:{effect}}}).text;
}finally{fs.rmSync(temporary,{recursive:true,force:true});}
const cases=[];
function add(value){const input=JSON.stringify(value);try{validateSupportedDesired(value);cases.push({input,normalized:canonical(normalizeLegacyDesired(value))});}catch(e){cases.push({input,error:e.code});}}
for(const initial of [v1,v2,v3,v4]){
 const d=initial();add(d);
 for(const mode of ['clear','tornado','meteor','invalid']){const v=structuredClone(d);v.visual.effect=mode;add(v);}
 for(const key of Object.keys(d.visual)){const missing=structuredClone(d);delete missing.visual[key];add(missing);const invalid=structuredClone(d);invalid.visual[key]={unexpected:true};add(invalid);}
 for(const key of Object.keys(d.widgets.cards.weather)){const v=structuredClone(d);delete v.widgets.cards.weather[key];add(v);}
 for(const value of [null,[],true,{},[{id:'a',text:'x',done:false},{id:'a',text:'y',done:false}],[{id:'\ud800',text:'\udfff🙂',done:false}],[{id:'emoji',text:'🙂'.repeat(81),done:false}]]){const v=structuredClone(d);v.widgets.todos=value;add(v);}
 for(const key of ['__proto__','prototype','constructor','unexpected']){const v=structuredClone(d);Object.defineProperty(v.visual,key,{value:1,enumerable:true});add(v);}
}
// Optional disk metrics added on main: keep old format-4 bytes and reject malformed additions.
for(const initial of [v4]){const old=initial();delete old.widgets.metrics.diskC;delete old.widgets.metrics.diskD;add(old);for(const c of [false,true,null,0]){const value=structuredClone(old);value.widgets.metrics.diskC=c;add(value)}const disksOnly=initial();Object.assign(disksOnly.widgets.metrics,{cpu:false,ram:false,up:false,down:false,diskC:true,diskD:false});add(disksOnly);const none=structuredClone(disksOnly);none.widgets.metrics.diskC=false;add(none)}
function save(relative,value){const file=fileURLToPath(new URL('../'+relative,import.meta.url)),text=JSON.stringify(value,null,2)+'\n';if(process.argv.includes('--check')){if(fs.readFileSync(file,'utf8')!==text)throw Error('STATE_CONTRACT_STALE: '+relative);}else{fs.mkdirSync(fileURLToPath(new URL('../'+relative.slice(0,relative.lastIndexOf('/')+1),import.meta.url)),{recursive:true});fs.writeFileSync(file,text);}}
save('src/cli-catalogs/state-defaults.json',defaults);save('tests/fixtures/desired-state-contract.json',cases);
console.log(JSON.stringify({versions:4,cases:cases.length,checked:process.argv.includes('--check')}));
