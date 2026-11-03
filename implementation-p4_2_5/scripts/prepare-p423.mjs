import fs from 'node:fs';import path from 'node:path';import {fileURLToPath} from 'node:url';const root=fileURLToPath(new URL('../',import.meta.url)),dist=path.join(root,'dist'),read=n=>fs.readFileSync(path.join(dist,n),'utf8'),src=n=>fs.readFileSync(path.join(root,'src',n),'utf8'),write=(n,s)=>fs.writeFileSync(path.join(dist,n),s);
let app=read('app.js').replace('const defaults={previewMode:', 'const defaults={hourCycle:24,showSeconds:true,previewMode:');const start=app.indexOf('function tick(){'),end=app.indexOf('function scheduleClock()',start);if(start<0||end<start)throw Error('Clock schedule hook missing');app=app.slice(0,start)+src('clock-tick.js')+app.slice(end);write('app.js',app);
for(const n of ['p423-copy.js','p423-ui.js'])write(n,src(n));write('bridge-client.js',read('bridge-client.js').replace("await load('p422-copy.js');","await load('p422-copy.js');await load('p423-copy.js');").replace("await load('p422-ui.js');","await load('p422-ui.js');await load('p423-ui.js');"));write('hifi.css',read('hifi.css')+'\n'+src('p423-ui.css'));

// Visibility-only bulk operations preserve stored geometry; rendering may project/clamp without persisting it.
let w=read('widgets.js');
w=w.replace('function renderCards(){','function renderCards(preserve=true){const original=preserve?JSON.parse(JSON.stringify(data)):null;');
w=w.replace('place(el.dataset.widget,el);updateTimer()}','place(el.dataset.widget,el);updateTimer();if(original)data=original}');
w=w.replace('syncGeometry:()=>renderCards(),','syncGeometry:()=>renderCards(true),setAllEnabled:value=>{for(const c of Object.values(data.cards))c.enabled=!!value;renderCards(true);syncManager();save()},');
w=w.replace('syncManager();renderCards();bridgeAppliedDesired=', 'syncManager();renderCards(true);bridgeAppliedDesired=');
w=w.replace("if(active)endDrag(true);renderCards()});document.addEventListener", "if(active)endDrag(true);renderCards(true)});document.addEventListener");
w=w.replace("Object.assign(data.cards[id],patch);renderCards();syncManager();save()", "Object.assign(data.cards[id],patch);renderCards(!Object.keys(patch).some(k=>['x','y','mx','my','size','font','fontSize'].includes(k)));syncManager();save()");
w=w.replace("c.custom=false;c.mobileCustom=false}renderCards();save();syncManager()", "c.custom=false;c.mobileCustom=false}renderCards(false);save();syncManager()");
function geometryReplace(from,to){if(!w.includes(from))throw Error('Widget geometry hook missing: '+from);w=w.replace(from,to)}
geometryReplace('for(const [id,p] of Object.entries(autoPlan)){data.cards[id].x=', 'for(const [id,p] of Object.entries(autoPlan)){if(data.cards[id].custom)continue;data.cards[id].x=');
geometryReplace('for(const id of Object.keys(specs)){const p=gridPoint(id);', 'for(const id of Object.keys(specs)){if(data.cards[id].custom)continue;const p=gridPoint(id);');
geometryReplace('function move(id,dx,dy){const c=', 'function move(id,dx,dy){if(!active)commitRenderedGeometry(id);const c=');
geometryReplace('const id=h.dataset.widgetDrag;active={id,', 'const id=h.dataset.widgetDrag,before=geometrySnapshot();commitRenderedGeometry(id);active={before,moved:false,id,');
geometryReplace('const [rawX,rawY]=keys();', 'if(e.clientX!==active.x||e.clientY!==active.y)active.moved=true;if(!active.moved)return;const [rawX,rawY]=keys();');
const dragStart=w.indexOf('function endDrag(cancel=false){'),dragEnd=w.indexOf("layer.addEventListener('pointerup'",dragStart);
if(dragStart<0||dragEnd<0)throw Error('Widget end-drag hook missing');w=w.slice(0,dragStart)+src('widget-geometry.js')+'\n'+w.slice(dragEnd);
geometryReplace('updateCard:(id,patch)=>{Object.assign', "updateCard:(id,patch)=>{if(Object.keys(patch).some(k=>['x','y','mx','my'].includes(k)))commitRenderedGeometry(id);Object.assign");
geometryReplace('center:(id,both)=>{const c=', 'center:(id,both)=>{commitRenderedGeometry(id);const c=');
write('widgets.js',w);
