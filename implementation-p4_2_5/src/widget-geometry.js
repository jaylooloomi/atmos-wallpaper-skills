// Commit the displayed projection only when the user explicitly edits a position.
function geometrySnapshot(){return {desktopZone:data.desktopZone,cards:Object.fromEntries(Object.entries(data.cards).map(([id,c])=>[id,Object.fromEntries(['x','y','mx','my','custom','mobileCustom'].map(k=>[k,c[k]]))]))}}
function commitRenderedGeometry(id){
 const mobile=window.atmosIsNarrow(),[kx,ky]=keys();
 const anchor=key=>{const el=layer.querySelector(`[data-widget="${key}"]`);const p=el?{x:parseFloat(el.style.left),y:parseFloat(el.style.top)}:clampPoint(autoPoint(key).x,autoPoint(key).y,dimensions(key).w,dimensions(key).h,stage.clientWidth,stage.clientHeight);const c=data.cards[key];c[kx]=p.x/stage.clientWidth*100;c[ky]=p.y/stage.clientHeight*100;c[mobile?'mobileCustom':'custom']=true};
 if(!mobile&&data.desktopZone===null){data.desktopZone=stage===scene?'scene':'tray';for(const key of Object.keys(data.cards))if(!data.cards[key].custom)anchor(key)}
 anchor(id);
}
function endDrag(cancel=false){
 window.AtmosEdgeGuide?.clear();if(!active)return;const a=active,restore=cancel||!a.moved;
 if(restore){data.desktopZone=a.before.desktopZone;for(const [id,c]of Object.entries(a.before.cards))Object.assign(data.cards[id],c)}
 try{a.handle.releasePointerCapture(a.pointer)}catch{}active=null;layer.classList.remove('align-x','align-y');
 if(restore){renderCards(true);if(a.moved)window.atmosBridgeChanged?.('widgets')}else save();
}
