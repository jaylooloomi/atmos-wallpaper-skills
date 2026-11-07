// Deterministic build-time oracle from the original JS canonical() contract.
import fs from 'node:fs';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {canonical} from '../../implementation-p4_2_5/src/state.mjs';
const inputs=['null','true','false','-0','1e999','-1e999','9007199254740993','1000000000000000128','[1e-7,1e-6,1e20,1e21,5e-324,1.7976931348623157e308]','{"z":1,"a":2,"z":3}',JSON.stringify({'😀':1,'\ue000':2,'\ud800':3,'a':4,'\udfff':5}),JSON.stringify({text:'\ud800x\udfff',paired:'😀',controls:'\b\f\n\r\t\u0000\u001f',literal:'\\ud800',separators:'\u2028\u2029',city:'台北',quote:'"\\/'}),JSON.stringify({nested:[{todos:[{id:'🙂',text:'\ud800',done:false}],value:0.10000000000000002}],empty:{}})];
let seed=0x9e3779b97f4a7c15n;const mask=(1n<<64n)-1n,bytes=Buffer.alloc(8);
for(let group=0;group<16;group++){
 const numbers=[];
 while(numbers.length<256){seed^=seed<<13n;seed&=mask;seed^=seed>>7n;seed^=seed<<17n;seed&=mask;bytes.writeBigUInt64LE(seed);const n=bytes.readDoubleLE();if(Number.isFinite(n))numbers.push(n.toPrecision(17));}
 inputs.push('['+numbers.join(',')+']');
}
const values=inputs.map(input=>{const value=canonical(JSON.parse(input));return{input,canonical:value,sha256:createHash('sha256').update(value).digest('hex')};});
const file=fileURLToPath(new URL('../tests/fixtures/js-json-contract.json',import.meta.url)),text=JSON.stringify(values,null,2)+'\n';
if(process.argv.includes('--check')){if(fs.readFileSync(file,'utf8')!==text)throw Error('JS_JSON_FIXTURES_STALE');}
else{fs.mkdirSync(fileURLToPath(new URL('../tests/fixtures/',import.meta.url)),{recursive:true});fs.writeFileSync(file,text);}
console.log(JSON.stringify({cases:values.length,randomNumbers:4096,checked:process.argv.includes('--check')}));
