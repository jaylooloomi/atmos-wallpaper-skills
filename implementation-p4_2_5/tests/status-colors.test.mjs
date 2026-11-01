import test from 'node:test';import assert from 'node:assert/strict';import fs from 'node:fs';
const css=fs.readFileSync(new URL('../src/p423-ui.css',import.meta.url),'utf8');
const luminance=hex=>{const rgb=hex.match(/\w\w/g).map(x=>parseInt(x,16)/255).map(x=>x<=.04045?x/12.92:((x+.055)/1.055)**2.4);return rgb[0]*.2126+rgb[1]*.7152+rgb[2]*.0722};
for(const [skin,ink,paper]of [['natural','304437','f8faf5'],['neon','d9f4ff','19334a'],['warm','5b3e26','fff3df'],['minimal','26332d','f5f7f3']])test('validation feedback '+skin+' fixed colors exceed4.5 contrast',()=>{assert.ok(css.includes('color:#'+ink+';background:#'+paper));const a=luminance(ink),b=luminance(paper),ratio=(Math.max(a,b)+.05)/(Math.min(a,b)+.05);assert.ok(ratio>=4.5,skin+' contrast '+ratio)});
