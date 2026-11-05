// Presentation-only navigation. Existing nodes and their draft/native handlers are retained.
(()=>{'use strict';let started=false,active='desktop',root,manager,panels={},tabs={},checks=[],save;
const ids=['clock','calendar','weather','todo','timer','system','media'];
const copy={
 "zh-TW": [
  "桌面元件",
  "外觀字型",
  "桌布與系統設定",
  "字型／位置",
  "套用變更",
  "外觀與元件先成為草稿；系統設定依各區說明保存。",
  "日期與時間",
  "日曆",
  "天氣",
  "待辦事項",
  "番茄鐘",
  "系統資訊",
  "播放資訊"
 ],
 "zh-CN": [
  "桌面组件",
  "外观字体",
  "桌布与系统设置",
  "字体／位置",
  "应用更改",
  "外观与组件先成为草稿；系统设置按各区说明保存。",
  "日期与时间",
  "日历",
  "天气",
  "待办事项",
  "番茄钟",
  "系统信息",
  "播放信息"
 ],
 "en": [
  "Desktop widgets",
  "Appearance & fonts",
  "Wallpaper & system",
  "Font / position",
  "Apply changes",
  "Appearance changes are drafts. System controls save as described in each section.",
  "Date & time",
  "Calendar",
  "Weather",
  "To-do",
  "Timer",
  "System",
  "Media"
 ],
 "ja": [
  "デスクトップ",
  "外観・フォント",
  "壁紙とシステム",
  "文字・位置",
  "変更を適用",
  "外観は下書きです。システム設定は各欄の説明に従って保存します。",
  "日時",
  "カレンダー",
  "天気",
  "タスク",
  "タイマー",
  "システム",
  "メディア"
 ],
 "ko": [
  "바탕화면 위젯",
  "모양 및 글꼴",
  "배경화면 및 시스템",
  "글꼴 / 위치",
  "변경 적용",
  "모양 변경은 초안입니다. 시스템 설정은 각 영역의 안내에 따라 저장됩니다.",
  "날짜 및 시간",
  "달력",
  "날씨",
  "할 일",
  "타이머",
  "시스템",
  "미디어"
 ]
};
const words=()=>copy[window.atmosBridgeApp?.getState?.().locale]||copy.en;
const node=(tag,cls)=>{const e=document.createElement(tag);if(cls)e.className=cls;return e};
function enabled(id){return id==='clock'?window.atmosBridgeApp.getState().clockVisible:id==='calendar'?window.atmosBridgeApp.getState().calendarVisible:window.atmosBridgeWidgets.getState().cards[id].enabled}
function change(id,value){if(id==='clock'||id==='calendar')window.atmosBridgeApp.update({[id+'Visible']:value});else window.atmosBridgeWidgets.updateCard(id,{enabled:value});window.AtmosInspector.refresh();refresh()}
function select(id,{focus=false}={}){if(!panels[id])throw Error('SETTINGS_TAB_UNKNOWN');active=id;for(const key of Object.keys(panels)){panels[key].hidden=key!==id;tabs[key].setAttribute('aria-selected',String(key===id));tabs[key].tabIndex=key===id?0:-1}if(focus)tabs[id].focus();window.atmosSchedulePanelAlign?.();return panels[id]}
function label(e,index){e.dataset.settingsCopy=String(index);return e}
function toggle(id){const wrap=node('label','settings-widget-toggle'),input=node('input');input.type='checkbox';input.dataset.settingsWidget=id;input.onclick=e=>e.stopPropagation();input.onchange=e=>{e.stopPropagation();change(id,input.checked)};const track=node('span','settings-toggle-track');track.setAttribute('aria-hidden','true');wrap.append(input,track);checks.push({id,input});return wrap}
function refresh(){if(!root)return;const t=words();for(const e of root.querySelectorAll('[data-settings-copy]')){const value=t[Number(e.dataset.settingsCopy)];if(e.textContent!==value)e.textContent=value}for(const {id,input}of checks){input.checked=!!enabled(id);input.setAttribute('aria-label',t[6+ids.indexOf(id)])}save.disabled=!!document.getElementById('applyDraft')?.disabled}
function start({wallpaper=false}={}){
 if(started||wallpaper||document.body.dataset.renderMode==='wallpaper')return;
 const scroll=document.querySelector('.side .panel-scroll'),section=document.querySelector('.component-section');manager=document.getElementById('widgetManager');const inspector=document.getElementById('componentInspector');if(!scroll||!section||!manager||!inspector)return;
 started=true;root=section;root.classList.add('settings-tabs');const originalSections=[...scroll.children].filter(e=>e!==section);
 const bar=node('div','settings-tablist');bar.setAttribute('role','tablist');bar.setAttribute('aria-label','Control panel');
 for(const [i,id]of ['desktop','appearance'].entries()){
  const button=label(node('button','settings-tab'),i);button.type='button';button.id='settings-tab-'+id;button.setAttribute('role','tab');button.setAttribute('aria-controls','settings-panel-'+id);button.onclick=()=>select(id);
  button.onkeydown=e=>{const keys=['desktop','appearance'];let index=keys.indexOf(active);if(e.key==='ArrowRight')index=(index+1)%2;else if(e.key==='ArrowLeft')index=(index+1)%2;else if(e.key==='Home')index=0;else if(e.key==='End')index=keys.length-1;else return;e.preventDefault();select(keys[index],{focus:true})};
  tabs[id]=button;bar.append(button);const panel=node('section','settings-tabpanel');panel.id='settings-panel-'+id;panel.dataset.settingsPanel=id;panel.setAttribute('role','tabpanel');panel.setAttribute('aria-labelledby',button.id);panels[id]=panel;
 }
 // Keep all original widget operations under their delegated #widgetManager ancestor.
 for(const child of [...manager.children])if(child!==inspector)panels.appearance.append(child);
 panels.appearance.append(inspector);manager.append(...Object.values(panels));section.prepend(bar);
 const tools=inspector.querySelector('.component-tools');if(tools)panels.desktop.append(tools);
 const rows=node('div','settings-widget-list');panels.desktop.append(rows);
 for(const [i,id]of ids.entries()){
  const row=node('details','settings-widget-card');row.dataset.settingsWidgetCard=id;const summary=node('summary','settings-widget-summary'),name=label(node('span'),6+i),chevron=node('span','settings-widget-chevron');chevron.setAttribute('aria-hidden','true');const svg=document.createElementNS('http://www.w3.org/2000/svg','svg');svg.setAttribute('viewBox','0 0 24 24');svg.setAttribute('width','20');svg.setAttribute('height','20');const path=document.createElementNS('http://www.w3.org/2000/svg','path');path.setAttribute('d','m6 9 6 6 6-6');path.setAttribute('fill','none');path.setAttribute('stroke','currentColor');path.setAttribute('stroke-width','2');path.setAttribute('stroke-linecap','round');path.setAttribute('stroke-linejoin','round');svg.append(path);chevron.append(svg);summary.append(name,chevron,toggle(id));row.append(summary);
  const content=node('div','settings-widget-content'),props=document.getElementById('props-'+id);for(const e of [...props.children])if(e.classList.contains('inspect-operations')||e.classList.contains('inspect-extra')||e.classList.contains('calendar-section'))content.append(e);
  const edit=label(node('button','settings-edit-appearance'),3);edit.type='button';edit.onclick=()=>{select('appearance');window.AtmosInspector.open(id);inspector.querySelector('#inspect-title-'+id)?.focus()};content.append(edit);row.append(content);rows.append(row);
 }
 const advanced=node('details','settings-advanced');advanced.append(label(node('summary'),2));panels.desktop.append(advanced);
 for(const e of originalSections){e.classList.add('settings-native-controls');e.addEventListener('change',event=>event.stopPropagation());if(e.classList.contains('screensaver-settings'))e.remove();else if(e.classList.contains('provider-settings'))rows.querySelector('[data-settings-widget-card="weather"] .settings-widget-content').prepend(e);else if(e.classList.contains('weather-section')||e.classList.contains('preset-settings'))panels.appearance.prepend(e);else advanced.append(e)}
 const theme=document.getElementById('themeSettings');if(theme){theme.addEventListener('change',event=>event.stopPropagation());panels.desktop.prepend(theme)}
 for(const empty of document.querySelectorAll('.below-preview,.settings-bottom'))if(!empty.children.length)empty.remove();
 const footer=node('div','settings-tab-footer');save=label(node('button','settings-apply'),4);save.type='button';save.onclick=()=>document.getElementById('applyDraft')?.click();footer.append(label(node('small'),5),save);section.append(footer);
 const oldOpen=window.AtmosInspector.open.bind(window.AtmosInspector);window.AtmosInspector.open=id=>{select('appearance');oldOpen(id)};
 window.AtmosSettingsTabs={start,select,get active(){return active},refresh};select('desktop');refresh();setInterval(refresh,500);
}
window.AtmosSettingsTabs={start,select,get active(){return active},refresh};
})();
