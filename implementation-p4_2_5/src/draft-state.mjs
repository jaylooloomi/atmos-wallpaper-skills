export const copy=x=>JSON.parse(JSON.stringify(x));
export const same=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
export function difference(base,next){const out={};for(const k of Object.keys(next)){if(k==='undo')continue;const a=base?.[k],b=next[k];if(b&&typeof b==='object'&&!Array.isArray(b)&&a&&typeof a==='object'&&!Array.isArray(a)){const d=difference(a,b);if(Object.keys(d).length)out[k]=d}else if(!same(a,b))out[k]=copy(b)}return out}
export class DraftState{
 constructor(){this.baseline=null;this.view=null;this.draft=null;this.inflight=null;this.remote=null;this.error=null;this.committedRevision=null}
 establish(snapshot,view){this.baseline=copy(snapshot);this.view=copy(view);this.remote=null;this.draft=null;this.inflight=null;this.error=null;this.committedRevision=null}
 mark(section='visual'){const timer=section==='timer';if(timer)section='widgets';if(!['visual','widgets'].includes(section))return;if(!this.baseline||this.inflight)return;if(!this.draft)this.draft={baseRevision:this.baseline.revision,baseView:copy(this.view),changes:{},pending:true,touched:{},timerTouched:false,baseTimer:this.baseline.desired?.widgets?.timer?copy(this.baseline.desired.widgets.timer):null};if(timer)this.draft.timerTouched=true;this.draft.touched[section]=true;this.draft.pending=true}
 capture(view){if(!this.draft||this.inflight)return;const base={},next={};for(const section of Object.keys(this.draft.touched)){base[section]=copy(this.draft.baseView[section]);next[section]=copy(view[section]);if(section==='widgets'){if(this.draft.timerTouched&&this.draft.baseTimer)base.widgets.timer=copy(this.draft.baseTimer);else{delete base.widgets.timer;delete next.widgets.timer}}}this.draft.changes=difference(base,next);this.draft.pending=false;if(!Object.keys(this.draft.changes).length)this.draft=null}
 incoming(snapshot){if(this.draft||this.inflight){if(snapshot.revision!==this.baseline.revision)this.remote=copy(snapshot);return false}return true}
 command(id){if(this.inflight)return copy(this.inflight);if(!this.draft||this.draft.pending||!Object.keys(this.draft.changes).length)throw Error('NO_SETTLED_DRAFT');if(this.remote&&this.remote.revision!==this.draft.baseRevision)throw Error('CONFLICT');if(this.inflight)return copy(this.inflight);this.inflight={schemaVersion:1,type:'settings.update',requestId:id,expectedRevision:this.draft.baseRevision,changes:copy(this.draft.changes)};this.error=null;return copy(this.inflight)}
 unknown(message){this.error=message||'OUTCOME_UNKNOWN'}
 rejected(code){this.error=code;this.inflight=null}
 committed(revision){this.committedRevision=revision;this.draft=null;this.inflight=null;this.error=null;this.remote=null}
 rebase(){if(!this.remote||!this.draft||this.inflight)throw Error('REBASE_UNAVAILABLE');this.draft.baseRevision=this.remote.revision;this.baseline=copy(this.remote);this.remote=null;this.error=null}
 discard(){if(this.inflight)throw Error('OUTCOME_UNKNOWN_RETRY_FIRST');this.draft=null;this.remote=null;this.error=null}
 get dirty(){return !!(this.draft||this.inflight)}
}
