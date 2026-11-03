// Initial-page gate only. Preserve layout measurements while concealing staged UI.
(()=>{
 const root=document.documentElement,loading=()=>root.hasAttribute('data-atmos-loading');
 const frame=()=>new Promise(resolve=>requestAnimationFrame(resolve));
 const notice=message=>{if(!loading())return;const text=document.getElementById('atmosBootMessage');if(text)text.textContent=message};
 window.AtmosBootVisibility={
  async settle(){
   if(!loading())return;
   let timer;
   try{await Promise.race([(async()=>{
    for(const link of document.querySelectorAll('link[rel="stylesheet"]'))if(!link.sheet||!link.sheet.cssRules.length)throw Error('Required stylesheet failed to load');
    await document.fonts?.ready;
    await Promise.allSettled([...document.querySelectorAll('.scene-picker-top img')].map(image=>image.decode()));
    await document.fonts?.ready;
    window.AtmosCanvas?.resize();window.atmosBridgeApp?.syncGeometry();window.atmosBridgeWidgets?.syncGeometry();window.atmosSchedulePanelAlign?.();
    await frame();await frame();
   })(),new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Initial layout readiness timed out')),10000)})])}finally{clearTimeout(timer)}
  },
  reveal(){root.removeAttribute('data-atmos-loading');document.getElementById('atmosBootNotice')?.remove()},
  fail(){notice('Atmos 尚未完成載入。請檢查連線後重新載入。')}
 };
})();
