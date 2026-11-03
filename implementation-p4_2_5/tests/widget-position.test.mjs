import test from 'node:test';import assert from 'node:assert/strict';import fs from 'node:fs';import os from 'node:os';import path from 'node:path';import vm from 'node:vm';import {parseHTML} from 'linkedom';
import {ControlCore} from '../src/portable-core.mjs';import {executeCommand} from '../src/commands.mjs';import {DraftState} from '../src/draft-state.mjs';
const code=()=>fs.readFileSync(new URL('../dist/widgets.js',import.meta.url),'utf8');
const contexts=new WeakMap();const plain=v=>JSON.parse(JSON.stringify(v));
function size(el){const c=contexts.get(el.ownerDocument);if(el.id==='scene')return {w:c.width,h:c.height};if(el.classList.contains('widget-mobile-host'))return {w:c.width,h:parseFloat(el.style.height)||350};return {w:parseFloat(el.style.width)||0,h:parseFloat(el.style.height)||0}}
function rect(el){const c=contexts.get(el.ownerDocument),s=size(el);let left=20,top=30;
 if(el.id==='clock')return {left:20+(c.tray?0:800)*c.scale,top:30+(c.tray?0:300)*c.scale,width:(c.tray?c.width:200)*c.scale,height:(c.tray?c.height:100)*c.scale,right:0,bottom:0};
 if(el.id==='calendar')return {left:20+1500*c.scale,top:40,width:250*c.scale,height:250*c.scale,right:0,bottom:0};
 if(el.classList.contains('wall-widget')){const p=rect(el.parentNode.parentNode);left=p.left+(parseFloat(el.style.left)||0)*c.scale;top=p.top+(parseFloat(el.style.top)||0)*c.scale;}
 return {left,top,width:s.w*c.scale,height:s.h*c.scale,right:left+s.w*c.scale,bottom:top+s.h*c.scale};
}
function fixture(snapshot,{mobile=false,tray=false,scale=.5}={}){
 const {window}=parseHTML('<html><body><div class="component-section"></div><section><div id="scene"><div id="clock"></div><div id="calendar"></div></div></section></body></html>');const document=window.document;
 contexts.set(document,{width:mobile?390:tray?700:1920,height:mobile?844:1080,tray,scale});
 Object.defineProperty(window.HTMLSelectElement.prototype,'value',{configurable:true,get(){return this.getAttribute('data-test-value')||''},set(v){this.setAttribute('data-test-value',v)}});const prototype=window.HTMLElement.prototype;
 Object.defineProperty(prototype,'clientWidth',{configurable:true,get(){return size(this).w}});Object.defineProperty(prototype,'clientHeight',{configurable:true,get(){return size(this).h}});
 prototype.getBoundingClientRect=function(){return rect(this)};prototype.setPointerCapture=()=>{};prototype.releasePointerCapture=()=>{};prototype.focus=()=>{};
 document.body.dataset.renderMode='manager';window.atmosIsNarrow=()=>mobile;window.atmosBridgeCurrent={snapshot};window.AtmosAppearance={apply:()=>{}};
 window.AtmosCanvas={resize:()=>{}};window.AtmosEdgeGuide={snap:()=>({snappedX:false,snappedY:false}),clear:()=>{}};
 const draft=new DraftState();const view=()=>({visual:snapshot.desired.visual,widgets:plain(window.atmosBridgeWidgets.getState())});
 window.atmosBridgeChanged=section=>{draft.mark(section);draft.capture(view())};
 vm.runInNewContext(code(),{window,document,location:{search:''},URLSearchParams,Date,console,addEventListener:()=>{},setInterval:()=>1,getComputedStyle:()=>({display:'block'})});
 draft.establish(snapshot,view());const api=window.atmosBridgeWidgets;api.setEditing(true);
 const element=id=>document.querySelector(`[data-widget="${id}"]`),position=id=>({x:parseFloat(element(id).style.left),y:parseFloat(element(id).style.top)});
 function event(id,type,values={}){const handle=element(id).querySelector('[data-widget-drag]');const e=new window.Event(type,{bubbles:true,cancelable:true});Object.assign(e,{pointerId:7,clientX:100,clientY:100,...values});handle.dispatchEvent(e)}
 return {window,api,draft,position,event,apply(next){snapshot=next;window.atmosBridgeCurrent={snapshot};api.apply(snapshot.desired.widgets);draft.establish(snapshot,view())}};
}
function setup(t,options={}){const dir=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-widget-position-'));t.after(()=>fs.rmSync(dir,{recursive:true,force:true}));const core=new ControlCore(dir);core.init();executeCommand(core,{schemaVersion:1,type:'settings.update',requestId:'show-weather',expectedRevision:0,changes:{widgets:{cards:{weather:{enabled:true},todo:{enabled:true}}}}});const snapshot=()=>{const s=core.status();return {revision:s.revision,desired:s.desired}};return {core,snapshot,page:fixture(snapshot(),options)}}
for(const [name,options]of [['scene',{}],['overflow tray',{tray:true}],['narrow screen',{mobile:true}]])test(`weather drag retains displayed position through JSON apply and reload: ${name}`,t=>{
 const {core,snapshot,page}=setup(t,options);const before=page.position('weather');
 page.event('weather','pointerdown');page.event('weather','pointermove',{clientX:105,clientY:105});page.event('weather','pointermove',{clientX:110,clientY:110});page.event('weather','pointerup');
 const moved=page.position('weather');assert.ok(Math.abs(moved.x-before.x-20)<.001,'Drag must start at the visible position and use total, scaled pointer distance');assert.ok(Math.abs(moved.y-before.y-20)<.001);
 const todo=options.mobile?null:page.position('todo');executeCommand(core,page.draft.command('drag-weather'));page.apply(snapshot());
 assert.deepEqual(page.position('weather'),moved,'Snapshot apply must not auto-place the moved card');if(!options.mobile)assert.deepEqual(page.position('todo'),todo,'Applying one move must not move the other visible card');
 const reload=fixture(snapshot(),options);assert.deepEqual(reload.position('weather'),moved,'A fresh renderer must use saved geometry');
 const w=snapshot().desired.widgets;if(options.mobile)assert.equal(w.desktopZone,null);else assert.equal(w.desktopZone,options.tray?'tray':'scene');
});
test('cancel and a plain drag-handle click preserve all geometry without creating a draft',t=>{
 const {page}=setup(t);const state=plain(page.api.getState()),position=page.position('weather');page.event('weather','pointerdown');page.event('weather','pointermove');page.event('weather','pointerup');assert.deepEqual(plain(page.api.getState()),state);assert.equal(page.draft.dirty,false);
 page.event('weather','pointerdown');page.event('weather','pointermove',{clientX:160,clientY:140});page.event('weather','pointercancel');assert.deepEqual(plain(page.api.getState()),state);assert.deepEqual(page.position('weather'),position);assert.equal(page.draft.dirty,false);
});
test('keyboard and inspector positions survive apply, while desktop edits preserve narrow coordinates',t=>{
 const {core,snapshot,page}=setup(t);const original=plain(page.api.getState()),before=page.position('weather');page.event('weather','keydown',{key:'ArrowRight'});assert.equal(page.position('weather').x,before.x+8);
 executeCommand(core,page.draft.command('keyboard-weather'));page.apply(snapshot());assert.equal(page.position('weather').x,before.x+8);
 page.api.updateCard('weather',{x:35,y:42,custom:true});const moved=page.position('weather');executeCommand(core,page.draft.command('inspector-weather'));page.apply(snapshot());assert.deepEqual(page.position('weather'),moved);
 for(const [id,c]of Object.entries(snapshot().desired.widgets.cards)){assert.equal(c.mx,original.cards[id].mx);assert.equal(c.my,original.cards[id].my);assert.equal(c.mobileCustom,original.cards[id].mobileCustom);}
});
test('legacy manual coordinates with an unset desktop zone survive automatic layout',t=>{
 const {core,snapshot}=setup(t);executeCommand(core,{schemaVersion:1,type:'settings.update',requestId:'legacy-position',expectedRevision:1,changes:{widgets:{cards:{weather:{custom:true,x:35,y:42}}}}});
 const page=fixture(snapshot());assert.deepEqual(page.position('weather'),{x:672,y:453.59999999999997});assert.equal(page.api.getState().desktopZone,null);
 page.api.setAllEnabled(false);page.api.setAllEnabled(true);assert.deepEqual(page.position('weather'),{x:672,y:453.59999999999997});
});
