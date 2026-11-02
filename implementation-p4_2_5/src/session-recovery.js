// Only the read-only wallpaper may reload itself: manager drafts must survive outages.
window.AtmosSessionRecovery=class {
 constructor({wallpaper,storage,reload=()=>location.reload(),schedule=(fn,ms)=>setTimeout(fn,ms),cancel=id=>clearTimeout(id),key='atmos-session-recovery:'+location.origin+location.pathname}={}){
  if(!storage){try{storage=window.sessionStorage}catch{storage={}}}
  Object.assign(this,{wallpaper,storage,reload,schedule,cancel,key});this.pending=null;
 }
 unauthorized(){
  if(!this.wallpaper)return false;
  if(this.pending!==null)return true;
  let attempts=0;
  try{const value=Number(this.storage.getItem(this.key));if(Number.isInteger(value)&&value>=0)attempts=Math.min(value,6);this.storage.setItem(this.key,String(attempts+1))}catch{}
  this.pending=this.schedule(()=>{this.pending=null;this.reload()},Math.min(60000,1000*2**attempts));
  return true;
 }
 healthy(){
  if(this.pending!==null)this.cancel(this.pending);
  this.pending=null;
  try{this.storage.removeItem(this.key)}catch{}
 }
};
