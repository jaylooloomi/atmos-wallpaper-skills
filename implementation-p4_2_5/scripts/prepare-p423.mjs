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
write('widgets.js',w);
