import test from 'node:test';import assert from 'node:assert/strict';import fs from 'node:fs';import vm from 'node:vm';
const source=fs.readFileSync(new URL('../src/bridge-client.js',import.meta.url),'utf8');
const marker=source.slice(source.indexOf('const wallpaperMode='),source.indexOf('const token='));
test('native marker hides branding only for the native manager, never wallpaper or browser',()=>{
 for(const [mode,native,expected]of [['manager',true,'true'],['manager',false,undefined],['manager','true',undefined],['wallpaper',true,undefined]]){
  const document={body:{dataset:{}}},window={atmosBridgeMode:mode,__ATMOS_NATIVE_MANAGER__:native};vm.runInNewContext(marker,{window,document});assert.equal(document.body.dataset.nativeManager,expected);assert.equal(document.body.dataset.renderMode,mode);
 }
});
test('header remains in the DOM for existing UI hooks; CSS only collapses native manager branding',()=>{
 const css=fs.readFileSync(new URL('../src/native-manager.css',import.meta.url),'utf8');assert.match(css,/body\[data-render-mode="manager"\]\[data-native-manager="true"\] > header/);assert.match(css,/display: none !important/);
 const html=fs.readFileSync(new URL('../dist/index.html',import.meta.url),'utf8');assert.match(html,/<header>/);assert.match(html,/id="styleTitle"/);
 const rust=fs.readFileSync(new URL('../../desktop-host/src/app.rs',import.meta.url),'utf8');assert.match(rust,/\.initialization_script\("window\.__ATMOS_NATIVE_MANAGER__ = true;"\)/);
});
