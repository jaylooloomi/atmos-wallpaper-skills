(()=>{'use strict';
 let options,data=null,busy=false,started=false,latestError='',timer;
 const messages={ICS_INVALID:'無法辨識這份 ICS 日曆。',ICS_TOO_LARGE:'日曆檔案不能超過 1 MiB。',ICS_TOO_COMPLEX:'日曆或重複規則太複雜，請匯出較小的日曆。',ICS_TIMEZONE_UNSUPPORTED:'無法辨識日曆時區，請重新匯出包含時區的 ICS。',ICS_URL_BLOCKED:'無法訂閱本機或內網網址，請使用公開 HTTPS 日曆網址。',ICS_URL_INVALID:'請使用 HTTPS 或 webcal 日曆網址。',ICS_FETCH_FAILED:'無法更新訂閱，請檢查連結或網路。',ICS_STORAGE_FAILED:'本機日曆資料無法讀取或儲存。',ICS_NOT_CONFIGURED:'請先匯入或訂閱日曆。',ICS_BUSY:'另一個日曆操作仍在執行，請稍後重試。'};
 const make=(tag,text)=>{const e=document.createElement(tag);if(text)e.textContent=translate(text);return e};
 const translate=s=>window.atmosT?.(s)||s;
 let statusNode,noticeNode,controls=[];
 function render(){
  const calendar=document.getElementById('calendar'),mode=window.atmosBridgeApp?.getState().calendarMode;
  if(!calendar||!['ics','off'].includes(mode))return;
  const locale=window.atmosBridgeApp.getState().locale||'zh-TW';
  const key=JSON.stringify([mode,locale,data,latestError]);
  if(calendar.firstElementChild?.dataset.realCalendar===key)return;
  const wrap=make('div');wrap.dataset.realCalendar=key;wrap.dataset.userContent='';
  wrap.append(make('h3',translate('日曆')));
  if(mode==='off'){wrap.append(make('p','日曆資料已關閉'));calendar.replaceChildren(wrap);return}
  wrap.append(make('small','ICS · 未來 7 天'));
  if(!data?.configured)wrap.append(make('p','請在日曆設定匯入檔案或貼上訂閱網址。'));
  else if(!data.events.length)wrap.append(make('p','未來 7 天沒有行程。'));
  else for(const event of data.events){
   const row=make('div');row.className='event';row.dataset.userContent='';
   const start=new Date(event.start);const date=new Intl.DateTimeFormat(locale,{month:'numeric',day:'numeric'}).format(start);
   const time=event.allDay?'全天':new Intl.DateTimeFormat(locale,{hour:'2-digit',minute:'2-digit',hourCycle:'h23'}).format(start);
   row.append(make('span',date+' '+time),make('div',event.title));wrap.append(row);
  }
  if(data?.truncated)wrap.append(make('small','行程過多，只顯示前 200 筆。'));
  const error=latestError||data?.lastError;if(error)wrap.append(make('p',(messages[error]||'日曆連線失敗。')+(data?.configured?' 保留上次成功內容。':'')));
  calendar.replaceChildren(wrap);
 }
 function describe(){
  if(!statusNode)return;
  statusNode.textContent=busy?translate('日曆處理中…'):latestError?messages[latestError]||'日曆操作失敗。':data?.configured?(data.name+' · '+(data.type==='subscription'?'每 15 分鐘更新':'已匯入')+' · '+(data.events?.length||0)+' 筆行程'):translate('尚未匯入日曆');
  for(const control of controls)control.disabled=busy;
 }
 async function request(action,body){
  const response=await fetch('/api/calendar/'+action,{method:body?'POST':'GET',headers:{...options.headers(),...(body?{'Content-Type':'application/json'}:{})},...(body?{body:JSON.stringify(body)}:{}),cache:'no-store',signal:AbortSignal.timeout(25000)});
  const result=await response.json();if(!response.ok)throw Error(result.error?.code||'ICS_FETCH_FAILED');return result;
 }
 async function refresh(){if(busy)return;try{data=await request('events');latestError=''}catch(e){latestError=messages[e.message]?e.message:'ICS_FETCH_FAILED'}describe();render()}
 async function act(action,body){if(busy)return;busy=true;latestError='';describe();try{
  data=await request(action,body);if(action==='import'||action==='subscribe'){window.atmosBridgeApp.update({calendarMode:'ics'});window.atmosBridgeChanged?.('visual');noticeNode.textContent=translate('日曆內容已儲存；請按上方「套用變更」保存顯示模式。')}
 }catch(e){latestError=messages[e.message]?e.message:'ICS_FETCH_FAILED'}finally{busy=false;describe();render()}}
 function manager(){
  const root=document.querySelector('.calendar-section');if(!root)return;root.replaceChildren();
  root.append(make('h3','ICS 日曆'));
  const description=make('p','不用登入。匯入 .ics 檔，或訂閱 Google／Outlook 提供的 ICS 連結。');root.append(description);
  const input=make('input');input.type='file';input.accept='.ics,text/calendar';input.setAttribute('aria-label','選擇 ICS 日曆檔案');
  const upload=make('button','匯入日曆');upload.type='button';upload.onclick=async()=>{const file=input.files?.[0];if(!file){latestError='ICS_NOT_CONFIGURED';describe();return}if(file.size>1048576){latestError='ICS_TOO_LARGE';describe();return}try{await act('import',{name:file.name.replace(/\.ics$/i,''),text:await file.text()})}catch{latestError='ICS_INVALID';describe()}};
  const name=make('input');name.placeholder=translate('日曆名稱（選填）');name.setAttribute('aria-label','日曆名稱');name.maxLength=80;
  const url=make('input');url.type='password';url.placeholder='https://… 或 webcal://…';url.setAttribute('aria-label','ICS 訂閱網址');url.autocomplete='off';
  const subscribe=make('button','訂閱日曆');subscribe.type='button';subscribe.onclick=async()=>{await act('subscribe',{name:name.value||'我的日曆',url:url.value});if(!latestError)url.value=''};
  const update=make('button','立即更新');update.type='button';update.onclick=()=>act('refresh',{});
  const remove=make('button','移除本機日曆');remove.type='button';remove.onclick=()=>act('disconnect',{});
  statusNode=make('p');statusNode.setAttribute('role','status');noticeNode=make('p');
  root.append(input,upload,name,url,subscribe,update,remove,statusNode,noticeNode,make('small','匯入、訂閱及移除立即生效。私密訂閱連結如同密碼，不會顯示在桌布或送給 AI。Windows 使用目前帳號加密保存在本機；移除只清除本機資料，不影響原日曆。'));
  root.style.cssText+=';display:grid;gap:10px';controls=[input,upload,name,url,subscribe,update,remove];
 }
 window.AtmosCalendar={render,start(config){if(started)return;started=true;options=config;if(!options.wallpaper)manager();const calendar=document.getElementById('calendar');if(calendar)new MutationObserver(render).observe(calendar,{childList:true,subtree:true});refresh();timer=setInterval(refresh,30000);addEventListener('online',refresh);document.addEventListener('visibilitychange',()=>{if(!document.hidden)refresh()})}};
})();
