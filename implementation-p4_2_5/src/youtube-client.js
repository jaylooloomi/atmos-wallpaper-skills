(()=>{'use strict';let started=false,root,dock,slot,message,frame=null,input,source=null;
const make=(tag,text)=>{const e=document.createElement(tag);if(text)e.textContent=text;return e};
function say(text){if(message.textContent!==text)message.textContent=text;message.hidden=!text}
function stop(text=''){if(frame){frame.remove();frame=null}dock.hidden=true;say(text)}
function fits(){const height=Math.max(200,Math.min(270,(innerWidth-32)*.5625)),bar=document.querySelector('.bridge-commands'),bottom=bar?.getBoundingClientRect().bottom||0;return (document.documentElement.clientWidth||innerWidth)>=240&&innerHeight>=Math.max(360,height+64+Math.max(0,bottom)+32)}
function play(){
 let next,embed;try{next=window.AtmosYouTubeLinks.parseYouTubeLink(input.value);embed=window.AtmosYouTubeLinks.youtubeEmbedUrl(input.value,location.origin)}catch{say('請貼上有效的 YouTube 影片或播放清單連結。');return}
 if(document.hidden){say('請回到管理頁後再播放。');return}
 if(!fits()){say('請放大視窗後再播放，播放器至少需要 200 × 200 像素。');return}
 stop();source=next;dock.hidden=false;frame=make('iframe');frame.title='YouTube 音樂播放器';frame.className='youtube-frame';
 frame.setAttribute('allow','autoplay; encrypted-media; picture-in-picture; fullscreen');frame.setAttribute('allowfullscreen','');
 frame.setAttribute('referrerpolicy','strict-origin-when-cross-origin');frame.setAttribute('sandbox','allow-scripts allow-same-origin allow-presentation allow-popups');
 const current=frame;frame.onload=()=>{if(frame===current)say('若未開始播放，請按 YouTube 播放鍵；部分影片限制嵌入播放。')};
 frame.onerror=()=>{if(frame===current)stop('播放器載入失敗，請檢查網路或更換連結。')};
 frame.src=embed;
 slot.append(frame);say('正在載入 YouTube…');
}
function start({wallpaper=false}={}){
 if(started||wallpaper||document.body.dataset.renderMode==='wallpaper')return;
 root=document.querySelector('[data-settings-widget-card=media] .settings-widget-content');if(!root)return;started=true;
 // Preserve legacy bindings for syncManager; the obsolete demo UI is no longer shown.
 const legacy=root.querySelector('.inspect-operations');if(legacy){let bindings=document.getElementById('legacy-control-bindings');if(!bindings){bindings=make('div');bindings.id='legacy-control-bindings';bindings.hidden=true;bindings.inert=true;document.body.append(bindings)}bindings.append(legacy)}
 const form=make('form');form.className='youtube-controls';const label=make('label','YouTube 影片／播放清單');input=make('input');input.type='text';input.inputMode='url';input.id='youtubeUrl';input.setAttribute('aria-label','YouTube 影片或播放清單連結');input.placeholder='https://www.youtube.com/watch?v=…';input.maxLength=2048;input.autocomplete='off';label.append(input);
 const actions=make('div');actions.className='youtube-actions';const load=make('button','播放'),close=make('button','停止');load.type='submit';close.type='button';close.onclick=()=>stop();actions.append(load,close);
 message=make('p');message.className='youtube-status';message.setAttribute('role','status');message.setAttribute('aria-live','polite');message.hidden=true;
 form.append(label,actions,message);form.onsubmit=event=>{event.preventDefault();play()};form.addEventListener('change',event=>event.stopPropagation());
 const edit=root.querySelector('.settings-edit-appearance');if(edit)root.insertBefore(form,edit);else root.append(form);
 dock=make('section');dock.className='youtube-dock';dock.id='youtubeDock';dock.hidden=true;dock.setAttribute('aria-label','YouTube 播放器');
 const heading=make('div');heading.className='youtube-dock-heading';const title=make('strong','YouTube'),exit=make('button','停止並關閉');exit.type='button';exit.onclick=()=>stop();heading.append(title,exit);slot=make('div');slot.className='youtube-frame-slot';dock.append(heading,slot);document.body.append(dock);
 document.addEventListener('visibilitychange',()=>{if(document.hidden&&frame)stop('頁面已隱藏，播放已停止；按播放可重新開始。')});
 addEventListener('pagehide',()=>stop());addEventListener('resize',()=>{if(frame&&!fits())stop('視窗過小，播放已停止；放大後可重新播放。')});
 window.AtmosYouTube={start,play,stop,get active(){return !!frame},get source(){return source?{...source}:null}};
}
window.AtmosYouTube={start};
})();
