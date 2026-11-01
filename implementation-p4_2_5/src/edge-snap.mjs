/** Pure left-edge alignment in screen CSS pixels. Caller applies its own bounds.
 * No sticky state, Y changes, center snapping, visibility inference or rearrangement.
 * A guide may appear within guideThreshold without applying any delta outside threshold.
 */
const validRect=r=>r&&['left','top','width','height'].every(k=>typeof r[k]==='number'&&Number.isFinite(r[k]))&&r.width>0&&r.height>0;
export function snapLeftEdge(moving,targets,{movingId=moving?.id,threshold=6,guideThreshold=10}={}){
 if(!Number.isFinite(threshold)||threshold<0||!Number.isFinite(guideThreshold)||guideThreshold<threshold)throw new RangeError('Thresholds require 0 <= threshold <= guideThreshold in CSS pixels');
 const empty={deltaX:0,snapped:false,targetId:null,guide:null};
 if(!validRect(moving)||!Array.isArray(targets))return empty;
 let best=null;
 for(const target of targets){
  if(!validRect(target)||target.visible!==true||typeof target.id!=='string'||!target.id||target.id===movingId)continue;
  const delta=target.left-moving.left,distance=Math.abs(delta);
  if(!Number.isFinite(delta)||distance>guideThreshold)continue;
  if(!best||distance<best.distance||(distance===best.distance&&(target.id<best.id||(target.id===best.id&&target.left<best.left))))best={id:target.id,left:target.left,delta,distance};
 }
 if(!best)return empty;
 const snapped=best.distance<=threshold;
 return {deltaX:snapped?(best.delta===0?0:best.delta):0,snapped,targetId:snapped?best.id:null,guide:{targetId:best.id,left:best.left}};
}

/** Same-edge alignment: left/right compete on X, top is independent on Y.
 * Coordinates and thresholds are screen CSS pixels (after canvas projection).
 * Every call evaluates raw pointer-start + total-delta, never previous snapped output.
 */
export function snapComponentEdges(moving,targets,{movingId=moving?.id,threshold=6,guideThreshold=10}={}){
 if(!Number.isFinite(threshold)||threshold<0||!Number.isFinite(guideThreshold)||guideThreshold<threshold)throw new RangeError('Thresholds require 0 <= threshold <= guideThreshold in CSS pixels');
 const empty={deltaX:0,deltaY:0,snappedX:false,snappedY:false,guides:[]};
 if(!validRect(moving)||!Array.isArray(targets))return empty;
 const candidates={x:[],y:[]};
 for(const t of targets){
  if(!validRect(t)||t.visible!==true||typeof t.id!=='string'||!t.id||t.id===movingId)continue;
  for(const [edge,axis,from,to,rank] of [['left','x',moving.left,t.left,0],['right','x',moving.left+moving.width,t.left+t.width,1],['top','y',moving.top,t.top,0]]){
   const delta=to-from,distance=Math.abs(delta);if(!Number.isFinite(delta)||distance>guideThreshold)continue;
   candidates[axis].push({edge,axis,coordinate:to,targetId:t.id,delta,distance,rank});
  }
 }
 const result={...empty,guides:[]};
 for(const axis of ['x','y']){
  candidates[axis].sort((a,b)=>a.distance-b.distance||a.rank-b.rank||(a.targetId<b.targetId?-1:a.targetId>b.targetId?1:0)||a.coordinate-b.coordinate);
  const best=candidates[axis][0];if(!best)continue;
  const snapped=best.distance<=threshold;
  result[axis==='x'?'snappedX':'snappedY']=snapped;
  result[axis==='x'?'deltaX':'deltaY']=snapped?(best.delta||0):0;
  result.guides.push({axis,edge:best.edge,coordinate:best.coordinate,targetId:best.targetId,snapped});
 }
 return result;
}
