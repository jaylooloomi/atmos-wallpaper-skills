import test from 'node:test';import assert from 'node:assert/strict';import fs from 'node:fs';import vm from 'node:vm';import {panelHeightForBottom} from '../src/panel-align.mjs';
const source=fs.readFileSync(new URL('../src/p422-ui.js',import.meta.url),'utf8');
const align=source.slice(source.indexOf('function align(){'),source.indexOf('function schedule(){'));
function style(){return {removeProperty(k){delete this[k];delete this[k.replace(/-([a-z])/g,(_,c)=>c.toUpperCase())]},setProperty(k,v){this[k]=v}}}
test('a tall right panel does not inflate the explanatory card',()=>{
 for(const required of [200,1200]){const targetStyle=style(),sideStyle=style(),target={style:targetStyle,getBoundingClientRect:()=>({height:100,bottom:500})},side={style:sideStyle,getBoundingClientRect:()=>({top:100})};
 const context={pending:true,innerWidth:1280,target,side,window:{AtmosInlineLayout:{requiredHeight:()=>required},atmosPanelHeight:panelHeightForBottom},getComputedStyle:()=>({boxSizing:'border-box'})};vm.runInNewContext(align+';align()',context);
 assert.equal(targetStyle.minHeight,undefined,'Panel minimum must not be assigned to explanatory copy');assert.equal(sideStyle.height,Math.max(400,required)+'px');
 }
});
test('narrow layout clears obsolete explicit heights',()=>{const targetStyle=style(),sideStyle=style();targetStyle.minHeight='700px';sideStyle.height='1200px';vm.runInNewContext(align+';align()',{pending:true,innerWidth:900,target:{style:targetStyle},side:{style:sideStyle}});assert.equal(targetStyle.minHeight,undefined);assert.equal(sideStyle.height,undefined)});
