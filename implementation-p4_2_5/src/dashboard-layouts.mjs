export const dashboardLayouts=Object.freeze({
 left:{name:'左側資訊欄',github:{x:3,y:3},yahoo:{x:3,y:58},rss:{x:52,y:3}},
 split:{name:'左右資訊',github:{x:3,y:58},yahoo:{x:52,y:58},rss:{x:3,y:3}},
 top:{name:'上方資訊',github:{x:3,y:3},yahoo:{x:52,y:3},rss:{x:3,y:58}}
});
export function applyDashboardLayout(settings,id){const preset=dashboardLayouts[id];if(!preset)throw Error('DASHBOARD_LAYOUT_UNKNOWN');const result=structuredClone(settings);for(const key of ['github','yahoo'])Object.assign(result.cards[key],preset[key]);if(result.rss)Object.assign(result.rss,preset.rss);return result}
