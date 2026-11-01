/** Align an outer panel border box to a measured target bottom; never infer target from sibling height. */
export function panelHeightForBottom({targetBottom,sideTop,boxSizing='border-box',paddingTop=0,paddingBottom=0,borderTop=0,borderBottom=0}){
 for(const v of [targetBottom,sideTop,paddingTop,paddingBottom,borderTop,borderBottom])if(!Number.isFinite(v))throw new TypeError('finite CSS pixel geometry required');
 const outer=Math.max(0,targetBottom-sideTop),extras=boxSizing==='border-box'?0:Math.max(0,paddingTop)+Math.max(0,paddingBottom)+Math.max(0,borderTop)+Math.max(0,borderBottom);
 return Math.max(0,outer-extras);
}
