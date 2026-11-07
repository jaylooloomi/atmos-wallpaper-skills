import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
const source=fs.readFileSync(new URL('../src/p422-ui.js',import.meta.url),'utf8');
const align=source.slice(source.indexOf('function align(){'),source.indexOf('function schedule(){'));
function style(){return {removeProperty(k){delete this[k];delete this[k.replace(/-([a-z])/g,(_,c)=>c.toUpperCase())]},setProperty(k,v){this[k]=v}}}
test('panel alignment clears stale inline sizing',()=>{
 const target={style:style()};
 const side={style:style()};
 target.style.minHeight='700px';
 side.style.height='1200px';
 vm.runInNewContext(align+';align()',{pending:true,target,side});
 assert.equal(target.style.minHeight,undefined);
 assert.equal(side.style.height,undefined);
});
