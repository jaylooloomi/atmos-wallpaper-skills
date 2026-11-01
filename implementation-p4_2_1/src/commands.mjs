import {fail,requestId,revision} from './contract.mjs';
function object(v){return v&&typeof v==='object'&&!Array.isArray(v)}
export function executeCommand(core,c){
 if(!object(c)||Object.keys(c).some(k=>!['schemaVersion','requestId','expectedRevision','type','changes'].includes(k))||c.schemaVersion!==1||c.type!=='settings.update')fail('COMMAND_INVALID','Only version 1 settings.update is supported');
 requestId(c.requestId);if(c.requestId.length>80)fail('INVALID_REQUEST_ID','UI requestId must be at most 80 characters');revision(c.expectedRevision);
 if(!object(c.changes)||!Object.keys(c.changes).length||Object.keys(c.changes).some(k=>!['visual','widgets'].includes(k))||Object.values(c.changes).some(v=>!object(v)||!Object.keys(v).length))fail('COMMAND_INVALID','Only nonempty visual/widgets changes are allowed');
 const patch=structuredClone(c.changes);if(patch.visual&&Object.hasOwn(patch.visual,'background'))patch.assetId=null;
 // Both operations use the exact frozen core receipt/CAS/validation machinery.
 // Stable derived IDs and deterministic patch preserve retry after lost responses.
 const preview=core.preview({requestId:'ui.'+c.requestId+'.preview',expectedRevision:c.expectedRevision,patch});
 const result=core.apply({requestId:'ui.'+c.requestId+'.apply',expectedRevision:c.expectedRevision,previewId:preview.previewId});
 return {committed:true,...result,requestId:c.requestId,rendererAcknowledged:false,hostVisibility:'unknown'};
}
