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
