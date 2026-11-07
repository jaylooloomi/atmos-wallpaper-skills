// Presentation-only navigation. Existing nodes and their draft/native handlers are retained.
(()=>{'use strict';let started=false,active='desktop',root,manager,panels={},tabs={},checks=[],save,selectedWidget=null,openInspector;
const ids=['clock','calendar','weather','todo','timer','system','media'];
const copy={
 "zh-TW": [
  "桌面元件",
  "沉浸體驗",
  "系統設定",
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
  "沉浸体验",
  "系统设置",
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
  "Immersive experience",
  "System settings",
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
  "没入体験",
  "システム設定",
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
  "몰입 경험",
  "시스템 설정",
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
function selectWidget(id){const rows=[...root.querySelectorAll('[data-settings-widget-card]')];if(id!==null&&!rows.some(e=>e.dataset.settingsWidgetCard===id))throw Error('SETTINGS_WIDGET_UNKNOWN');selectedWidget=id;if(id!==null)for(const details of panels.desktop.querySelectorAll(':scope>.settings-section-accordion'))details.open=false;if(ids.includes(id))openInspector?.(id);for(const row of rows){const selected=row.dataset.settingsWidgetCard===id;row.dataset.selected=String(selected);row.querySelector('.settings-widget-select')?.setAttribute('aria-pressed',String(selected));row.querySelector('.settings-widget-select')?.setAttribute('aria-expanded',String(selected));row.querySelector('.settings-widget-content').hidden=!selected}for(const feed of root.querySelectorAll('.daily-feed-settings'))feed.dataset.active=String(['github','yahoo','rss'].includes(id));window.atmosSchedulePanelAlign?.()}
function registerWidget(row){const id=row.dataset.settingsWidgetCard,header=row.querySelector('.settings-widget-summary'),name=header.querySelector('span'),control=header.querySelector('.settings-widget-toggle'),button=node('button','settings-widget-select'),content=row.querySelector('.settings-widget-content');button.type='button';button.id='settings-widget-select-'+id;button.setAttribute('aria-controls','settings-widget-content-'+id);button.append(name);button.onclick=()=>selectWidget(selectedWidget===id?null:id);header.onclick=e=>{if(e.target===header)selectWidget(selectedWidget===id?null:id)};button.onkeydown=e=>{if(!['ArrowRight','ArrowLeft','Home','End'].includes(e.key))return;const buttons=[...root.querySelectorAll('.settings-widget-select')];let index=buttons.indexOf(button);index=e.key==='Home'?0:e.key==='End'?buttons.length-1:(index+(e.key==='ArrowRight'?1:buttons.length-1))%buttons.length;e.preventDefault();buttons[index].focus();buttons[index].click()};header.replaceChildren(button,control);const title=node('h3','settings-widget-detail-title');title.append(name.cloneNode(true));content.prepend(title);content.id='settings-widget-content-'+id;content.setAttribute('role','region');content.setAttribute('aria-labelledby',button.id);selectWidget(selectedWidget)}
// Wrap, rather than rebuild, native sections: their selectors and save handlers stay intact.
function accordionSections(panel){for(const section of [...panel.children]){
 if(panel===panels.desktop&&!section.classList.contains('weather-section'))continue;
 if(section.id==='componentInspector'||section.hidden||!['SECTION','DIV'].includes(section.tagName))continue;
 const heading=section.querySelector('h2,h3,h4,legend'),fallback=section.classList.contains('audio-settings')?'桌布音訊':section.classList.contains('weather-section')?'天氣特效':null;
 if(!heading&&!fallback)continue;
 const details=node('details','settings-section-accordion'),summary=node('summary'),title=node('span'),icon=node('span','settings-section-chevron');icon.setAttribute('aria-hidden','true');const svg=document.createElementNS('http://www.w3.org/2000/svg','svg'),path=document.createElementNS('http://www.w3.org/2000/svg','path');svg.setAttribute('viewBox','0 0 20 20');svg.setAttribute('width','18');svg.setAttribute('height','18');svg.setAttribute('focusable','false');path.setAttribute('d','M5 7.5 10 12.5 15 7.5');path.setAttribute('fill','none');path.setAttribute('stroke','currentColor');path.setAttribute('stroke-width','1.75');path.setAttribute('stroke-linecap','round');path.setAttribute('stroke-linejoin','round');svg.append(path);icon.append(svg);title.textContent=heading?.textContent||fallback;details.dataset.settingsAccordion='';details.open=false;
 summary.append(title,icon);section.before(details);details.append(summary,section);section.classList.add('settings-accordion-body');if(heading)heading.dataset.accordionHeading='';
 const open=value=>{if(value&&panel===panels.desktop)selectWidget(null);if(value)for(const other of panel.querySelectorAll(':scope>.settings-section-accordion'))if(other!==details)other.open=false;details.open=value;window.atmosSchedulePanelAlign?.()};
 summary.onclick=e=>{e.preventDefault();open(!details.open)};details.ontoggle=()=>{if(details.open)open(true);window.atmosSchedulePanelAlign?.()};
 details._atmosAccordionTitle={title,heading};
}}
function refreshAccordionTitles(){for(const details of root.querySelectorAll('.settings-section-accordion')){const {title,heading}=details._atmosAccordionTitle;if(heading&&title.textContent!==heading.textContent)title.textContent=heading.textContent}}
function label(e,index){e.dataset.settingsCopy=String(index);return e}
function toggle(id){const wrap=node('label','settings-widget-toggle'),input=node('input');input.type='checkbox';input.dataset.settingsWidget=id;input.onclick=e=>e.stopPropagation();input.onchange=e=>{e.stopPropagation();change(id,input.checked)};const track=node('span','settings-toggle-track');track.setAttribute('aria-hidden','true');wrap.append(input,track);checks.push({id,input});return wrap}
function refresh(){if(!root)return;refreshAccordionTitles();const t=words();for(const e of root.querySelectorAll('[data-settings-copy]')){const value=t[Number(e.dataset.settingsCopy)];if(e.textContent!==value)e.textContent=value}for(const {id,input}of checks){input.checked=!!enabled(id);input.setAttribute('aria-label',t[6+ids.indexOf(id)])}save.disabled=!!document.getElementById('applyDraft')?.disabled}
function start({wallpaper=false}={}){
 if(started||wallpaper||document.body.dataset.renderMode==='wallpaper')return;
 const scroll=document.querySelector('.side .panel-scroll'),section=document.querySelector('.component-section');manager=document.getElementById('widgetManager');const inspector=document.getElementById('componentInspector');if(!scroll||!section||!manager||!inspector)return;
 started=true;openInspector=window.AtmosInspector.open.bind(window.AtmosInspector);root=section;root.classList.add('settings-tabs');const originalSections=[...scroll.children].filter(e=>e!==section);
 const bar=node('div','settings-tablist');bar.setAttribute('role','tablist');bar.setAttribute('aria-label','Control panel');
 for(const [id,i]of [['desktop',0],['system',2],['appearance',1]]){
  const button=label(node('button','settings-tab'),i);button.type='button';button.id='settings-tab-'+id;button.setAttribute('role','tab');button.setAttribute('aria-controls','settings-panel-'+id);button.onclick=()=>select(id);
  button.onkeydown=e=>{const keys=['desktop','system','appearance'];let index=keys.indexOf(active);if(e.key==='ArrowRight')index=(index+1)%keys.length;else if(e.key==='ArrowLeft')index=(index+keys.length-1)%keys.length;else if(e.key==='Home')index=0;else if(e.key==='End')index=keys.length-1;else return;e.preventDefault();select(keys[index],{focus:true})};
  tabs[id]=button;bar.append(button);const panel=node('section','settings-tabpanel');panel.id='settings-panel-'+id;panel.dataset.settingsPanel=id;panel.setAttribute('role','tabpanel');panel.setAttribute('aria-labelledby',button.id);panels[id]=panel;
 }
 const retired=node('div','retired-manager-chrome');retired.hidden=true;retired.inert=true;document.body.append(retired);for(const old of manager.querySelectorAll(':scope>.manager-top,:scope>p'))retired.append(old);
 // Keep all original widget operations under their delegated #widgetManager ancestor.
 for(const child of [...manager.children])if(child!==inspector)panels.appearance.append(child);
 panels.appearance.append(inspector);manager.append(...Object.values(panels));section.prepend(bar);
 const tools=inspector.querySelector('.component-tools');if(tools)panels.desktop.append(tools);
 const rows=node('div','settings-widget-list');panels.desktop.append(rows);
 for(const [i,id]of ids.entries()){
  const row=node('section','settings-widget-card');row.dataset.settingsWidgetCard=id;const summary=node('div','settings-widget-summary'),name=label(node('span'),6+i);summary.append(name,toggle(id));row.append(summary);
  const content=node('div','settings-widget-content'),props=document.getElementById('props-'+id);content.classList.add('atmos-inspector');
  for(const e of [...props.querySelectorAll(':scope>.inspect-extra')])props.prepend(e);
  props.removeAttribute('aria-labelledby');props.removeAttribute('role');content.append(props);row.append(content);rows.append(row);registerWidget(row);

 }
 // Native controls live in their own tab; preserve their nodes and save handlers.
 for(const e of originalSections){e.classList.add('settings-native-controls');e.addEventListener('change',event=>event.stopPropagation());if(e.classList.contains('screensaver-settings'))e.remove();else if(e.classList.contains('provider-settings'))rows.querySelector('[data-settings-widget-card="weather"] .settings-widget-detail-title').after(e);else if(e.classList.contains('weather-section'))panels.desktop.prepend(e);else if(e.classList.contains('preset-settings'))e.remove();else panels.system.append(e)}
 const theme=document.getElementById('themeSettings');if(theme){theme.addEventListener('change',event=>event.stopPropagation());const scenes=document.querySelector('.scene-section');if(scenes)scenes.before(theme)}
 for(const empty of document.querySelectorAll('.below-preview,.settings-bottom'))if(!empty.children.length)empty.remove();
 const footer=node('div','settings-tab-footer');save=label(node('button','settings-apply'),4);save.type='button';save.onclick=()=>document.getElementById('applyDraft')?.click();footer.append(save);section.append(footer);
 const priority=['.display-settings','.startup-settings[aria-label="Windows startup"]'].map(selector=>panels.system.querySelector(':scope>'+selector)).filter(Boolean);panels.system.prepend(...priority);
 for(const id of ['desktop','system','appearance'])accordionSections(panels[id]);
 for(const child of [...panels.appearance.children])retired.append(child);
 inspector.hidden=true;const status=inspector.querySelector('.inspect-status');if(status)panels.desktop.append(status);retired.append(inspector);window.AtmosInspector.open=id=>{select('desktop');selectWidget(id)};
 window.AtmosSettingsTabs={start,select,selectWidget,registerWidget,get selectedWidget(){return selectedWidget},get active(){return active},refresh};select('desktop');refresh();setInterval(refresh,500);
}
window.AtmosSettingsTabs={start,select,selectWidget,registerWidget,get selectedWidget(){return selectedWidget},get active(){return active},refresh};
})();
