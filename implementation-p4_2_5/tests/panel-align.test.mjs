import test from 'node:test';import assert from 'node:assert/strict';import {panelHeightForBottom as height} from '../src/panel-align.mjs';
test('actual reproduced1672 target subtracts10px extra sibling flow',()=>{assert.equal(height({targetBottom:1282.15625,sideTop:364}),918.15625)});
test('different tops use measured edges not left sibling height',()=>{assert.equal(height({targetBottom:900.5,sideTop:310.25}),590.25)});
test('border box does not double subtract padding or borders',()=>{assert.equal(height({targetBottom:900,sideTop:300,paddingTop:20,paddingBottom:20,borderTop:1,borderBottom:1}),600)});
test('content box accounts for actual outer additions',()=>{assert.equal(height({targetBottom:900,sideTop:300,boxSizing:'content-box',paddingTop:20,paddingBottom:10,borderTop:1,borderBottom:1}),568)});
test('fractional sizes remain fractional and invalid measurements reject',()=>{assert.equal(height({targetBottom:200.125,sideTop:100.0625}),100.0625);assert.equal(height({targetBottom:10,sideTop:100}),0);assert.throws(()=>height({targetBottom:NaN,sideTop:1}),TypeError)});
