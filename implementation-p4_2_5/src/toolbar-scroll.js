// Presentation only: CSS keeps the toolbar in flow; mark it only while actually pinned.
(()=>{'use strict';let started=false;
window.AtmosToolbar={start({wallpaper=false}={}){
 if(started||wallpaper)return;const bar=document.querySelector('.bridge-commands');if(!bar)return;started=true;let queued=false,timer,frame;
 const paint=()=>{const inset=parseFloat(getComputedStyle(bar).top);const stuck=window.scrollY>0&&Number.isFinite(inset)&&bar.getBoundingClientRect().top<=inset+.5;const value=String(stuck);if(bar.dataset.stuck!==value)bar.dataset.stuck=value};
 const flush=()=>{if(!queued)return;queued=false;clearTimeout(timer);cancelAnimationFrame(frame);paint()};
 const schedule=()=>{if(!queued){queued=true;frame=requestAnimationFrame(flush);timer=setTimeout(flush,50)}};
 addEventListener('scroll',schedule,{passive:true});addEventListener('resize',schedule,{passive:true});
 if(typeof ResizeObserver==='function')new ResizeObserver(schedule).observe(bar);
 paint();
}};
})();
