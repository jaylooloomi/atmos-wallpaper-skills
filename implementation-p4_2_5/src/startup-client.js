(()=>{'use strict';
 let started=false,busy=false,config,state=null,root,checkbox,status;
 const copy={
  'zh-TW':['Windows 啟動','登入 Windows 後自動套用桌布','立即生效，不需要按「套用變更」。Windows 的啟動應用程式設定仍有優先權。','已登錄登入啟動','登入啟動已關閉','僅 Windows 安裝版可使用','無法讀取或儲存啟動設定，請重試。','處理中…'],
  'zh-CN':['Windows 启动','登录 Windows 后自动应用壁纸','立即生效，无需点击“应用更改”。Windows 启动应用设置仍有优先权。','已注册登录启动','登录启动已关闭','仅 Windows 安装版可用','无法读取或保存启动设置，请重试。','处理中…'],
  en:['Windows startup','Apply wallpaper automatically when I sign in','Takes effect immediately. Windows startup-app settings can override this registration.','Registered to start at sign-in','Sign-in startup is off','Available in the installed Windows app only','Could not read or save startup settings. Please retry.','Working…'],
  ja:['Windows 起動','Windows サインイン後に壁紙を自動適用','即時に反映されます。Windows のスタートアップ設定が優先されます。','サインイン起動を登録済み','サインイン起動はオフ','Windows インストール版のみ対応','起動設定を読み込み／保存できません。再試行してください。','処理中…'],
  ko:['Windows 시작','Windows 로그인 후 배경 화면 자동 적용','즉시 적용됩니다. Windows 시작 앱 설정이 우선합니다.','로그인 시 시작 등록됨','로그인 시 시작 꺼짐','Windows 설치 버전에서만 지원','시작 설정을 읽거나 저장하지 못했습니다. 다시 시도하세요.','처리 중…']
 };
 let error=false;
 function words(){return copy[window.atmosBridgeApp?.getState().locale]||copy['zh-TW']}
 function render(){if(!root)return;const t=words();root.querySelector('h3').textContent=t[0];root.querySelector('[data-startup-label]').textContent=t[1];root.querySelector('small').textContent=t[2];checkbox.checked=state?.enabled===true;checkbox.disabled=busy||!state?.available;status.textContent=busy?t[7]:error?t[6]:!state?.available?t[5]:state.enabled?t[3]:t[4];}
 async function request(enabled){const writing=typeof enabled==='boolean';const response=await fetch('/api/host/startup',{method:writing?'POST':'GET',headers:{...config.headers(),...(writing?{'Content-Type':'application/json'}:{})},...(writing?{body:JSON.stringify({enabled})}:{}),cache:'no-store',signal:AbortSignal.timeout(20000)});const result=await response.json();if(!response.ok)throw Error(result.error?.code||'STARTUP_FAILED');return result;}
 async function refresh(){if(busy)return;try{state=await request();error=false}catch{error=true}render()}
 async function change(){if(busy)return;const enabled=checkbox.checked;busy=true;error=false;render();try{state=await request(enabled)}catch{error=true;try{state=await request()}catch{}}finally{busy=false;render()}}
 window.AtmosStartup={start(options){if(started||options.wallpaper)return;started=true;config=options;
  root=document.createElement('section');root.className='startup-settings';root.setAttribute('aria-label','Windows startup');
  root.innerHTML='<h3></h3><label><input type="checkbox" id="windowsStartup"><span data-startup-label></span></label><p role="status" aria-live="polite"></p><small></small>';
  checkbox=root.querySelector('input');status=root.querySelector('[role=status]');checkbox.disabled=true;checkbox.onchange=change;
  const panel=document.querySelector('.side .panel-scroll')||document.querySelector('.side')||document.querySelector('main');panel?.prepend(root);
  render();refresh();addEventListener('focus',refresh);document.addEventListener('visibilitychange',()=>{if(!document.hidden)refresh()});setInterval(()=>{render();if(!document.hidden)refresh()},15000);
 },refresh};
})();
