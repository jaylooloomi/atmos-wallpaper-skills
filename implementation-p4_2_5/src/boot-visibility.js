// Initial-page gate only. Preserve layout measurements while concealing staged UI.
(()=>{
 const root=document.documentElement,loading=()=>root.hasAttribute('data-atmos-loading');
 const frame=()=>new Promise(resolve=>requestAnimationFrame(resolve));
 const text=(id,message)=>{const node=document.getElementById(id);if(node&&node.textContent!==message)node.textContent=message};
 const stage=phase=>{if(!loading())return;const node=document.getElementById('atmosBootNotice');if(!node||node.dataset.state==='error')return;const labels={connect:'正在連接本機設定',scene:'正在同步場景與圖片',layout:'正在整理字型與版面'};if(!labels[phase])return;node.dataset.stage=phase;text('atmosBootStage',labels[phase])};
 window.AtmosBootVisibility={
  stage,
  async settle(){
   if(!loading())return;
   stage('layout');
   let timer;
   try{await Promise.race([(async()=>{
    for(const link of document.querySelectorAll('link[rel="stylesheet"]'))if(!link.sheet||!link.sheet.cssRules.length)throw Error('Required stylesheet failed to load');
    await document.fonts?.ready;
    const node=document.getElementById('atmosBootNotice');if(node?.dataset.state==='error'){node.dataset.state='loading';node.setAttribute('aria-busy','true');text('atmosBootMessage','正在載入 Atmos，請稍候…');stage('layout')}
    await Promise.allSettled([...document.querySelectorAll('.scene-picker-top img')].map(image=>image.decode()));
    await document.fonts?.ready;
    window.AtmosCanvas?.resize();window.atmosBridgeApp?.syncGeometry();window.atmosBridgeWidgets?.syncGeometry();window.atmosSchedulePanelAlign?.();
    await frame();await frame();
   })(),new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Initial layout readiness timed out')),10000)})])}finally{clearTimeout(timer)}
  },
  reveal(){root.removeAttribute('data-atmos-loading');document.getElementById('atmosBootNotice')?.remove()},
  fail(){if(!loading())return;const node=document.getElementById('atmosBootNotice');if(node){node.dataset.state='error';node.setAttribute('aria-busy','false')}text('atmosBootMessage','Atmos 尚未完成載入。請檢查連線後重新載入。');text('atmosBootStage','確認連線後，可以重新嘗試。')}
 };
})();
