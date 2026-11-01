/* Plain-text external-AI handoff. No network, execution, account data or persistence. */
(()=>{'use strict';
const translate=(source,params={})=>window.atmosT?window.atmosT(source,params):source.replace(/\{(\w+)\}/g,(all,key)=>Object.hasOwn(params,key)?String(params[key]):all);
const safeNumber=value=>Number.isFinite(value)?String(value):'—';
window.AtmosBuildPrompt=function(request,state,positionKeys,names,themeCatalog){
 const t=translate,v=state||{},keys=typeof positionKeys==='function'?positionKeys():positionKeys||{x:'clockX',y:'clockY',calX:'calendarX',cal:'calendarY'};
 const raw=String(request??'');
 const safe=raw.replace(/\b(?:sk-[A-Za-z0-9_-]{10,}|ghp_[A-Za-z0-9]{10,})\b/g,t('[已遮蔽可能憑證]')).replace(/((?:access|refresh|id)[_-]?token|api[_-]?key|client[_-]?secret)\s*[:=]\s*[^\s]+/gi,(_,key)=>key+'='+t('[已遮蔽]'));
 const named=(table,key)=>t(table?.[key]?.name||table?.[key]||t('未提供'));
 const fontName=value=>t(({theme:'跟隨主題',system:'系統預設',sans:'無襯線',serif:'襯線',mono:'等寬',rounded:'圓體'})[value]||'跟隨主題');
 const styleValue=value=>value===null||value===undefined||value==='theme'?t('跟隨主題'):String(value);
 const line=(name,visible,x,y)=>t('元件：{name}；顯示：{visible}；位置：{x}% / {y}%',{name:t(name),visible:t(visible?'是':'否'),x:safeNumber(x),y:safeNumber(y)});
 return [
 t('請協助動態桌布的背景生成與場景調整。以下是文字交接，不代表你已連接此頁。'),
 t('【使用者需求】'),safe,
 t('【本次產生指令時的狀態】'),
 t('目前布局固定為已選桌布布局；切換風格不重排元件。'),
 t('場景：{scene}；程式天氣：{weather}',{scene:named(names,v.background),weather:named(names,v.effect)}),
 t('風格：{style}（獨立場景素材與介面樣式）',{style:named(themeCatalog,v.theme)}),
 t('場景鎖定：{locked}',{locked:t(v.lockScene?'是':'否')}),
 line('時鐘',v.clockVisible,v[keys.x],v[keys.y]),
 line('日曆',v.calendarVisible,v[keys.calX||'calendarX'],v[keys.cal||'calendarY']),
 t('時鐘尺寸：{clockSize}%；日曆基準寬度：{calendarWidth}%。',{clockSize:safeNumber(v.clockSize),calendarWidth:safeNumber(v.calendarWidth)}),
 t('文字樣式：{name}；字體 {font}；字級 {size}；顏色 {color}',{name:t('時鐘'),font:fontName(v.clockFont),size:styleValue(v.clockFontSize),color:styleValue(v.clockColor)}),
 t('文字樣式：{name}；字體 {font}；字級 {size}；顏色 {color}',{name:t('日曆'),font:fontName(v.calendarFont),size:styleValue(v.calendarFontSize),color:styleValue(v.calendarColor)}),
 t('天氣強度：{intensity}%；動畫播放固定開啟。',{intensity:safeNumber(v.intensity)}),
 t('低動態固定關閉、柔和雷光允許；保留內部暫停與閃光限頻。'),
 t('以上只含非帳號介面設定；未附私人日曆事件、帳號、token、截圖或憑證。'),
 t('【能力與權限邊界】'),
 '1. '+t('先核實你是否具有圖片生成工具。若沒有，明確回報缺少的工具與可行下一步，不得偽造圖片、檔案路徑或成功結果。'),
 '2. '+t('背景與程式天氣、日期時間及日曆分層；背景不要烙入文字、時鐘、卡片或天氣特效，保留元件可讀區。'),
 '3. '+t('外部 AI 不會自動連線；請先核實可用工具與授權，只使用已確認的受限介面，不臆造命令。將「已生成」和「已套用」分開回報。'),
 '4. '+t('本需求不授權安裝工具、改 OS 桌布或設定、連 OAuth／AI 帳號、讀帳號秘密或私人行程、購買或公開發布。不要將日曆或 token 送入生成模型。'),
 '5. '+t('保留目前布局、四種風格、元件設定及程式天氣；必要變更先說明，不因風格或生圖而靜默刪除功能。'),
 t('【請回覆】'),
 t('實際可用工具、已完成結果與證據、未完成或受阻內容，以及能否安全導入本頁；沒有測通就標未驗證。')
 ].join('\n');
};
})();
