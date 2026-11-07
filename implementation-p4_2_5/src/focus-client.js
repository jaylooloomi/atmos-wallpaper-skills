// Synchronize the existing host display policy without manager mode controls.
(()=>{'use strict';if(document.body.dataset.renderMode==='wallpaper')document.body.dataset.atmosFocus='focus';let started=false;
window.AtmosFocus={start({headers}){if(started)return;started=true;async function refresh(){try{const response=await fetch('/api/host/focus',{method:'GET',headers:headers(),cache:'no-store',signal:AbortSignal.timeout(8000)});if(!response.ok)return;const state=await response.json();document.body.dataset.atmosFocus=state?.mode||'focus'}catch{}}refresh();setInterval(refresh,2000);addEventListener('focus',refresh)}};
})();
