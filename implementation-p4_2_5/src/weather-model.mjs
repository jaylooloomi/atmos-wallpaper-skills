export const weatherSlugs=Object.freeze(['clear-day','clear-night','partly-cloudy-day','partly-cloudy-night','cloudy','overcast','fog','drizzle','rain','sleet','snow','thunderstorms-rain','not-available']);
const labels={clear:['晴朗','晴朗','Clear','晴れ','맑음'],mostly:['晴時多雲','晴时多云','Mostly clear','おおむね晴れ','대체로 맑음'],clouds:['局部多雲','局部多云','Partly cloudy','晴れ時々曇り','구름 조금'],overcast:['陰天','阴天','Overcast','曇り','흐림'],fog:['霧','雾','Fog','霧','안개'],drizzle:['毛毛雨','毛毛雨','Drizzle','霧雨','이슬비'],rain:['下雨','下雨','Rain','雨','비'],sleet:['凍雨','冻雨','Freezing rain','着氷性の雨','어는 비'],snow:['下雪','下雪','Snow','雪','눈'],storm:['雷雨','雷雨','Thunderstorm','雷雨','뇌우'],unknown:['天氣未分類','天气未分类','Unknown weather','天気不明','날씨 불명']};
const locales=['zh-TW','zh-CN','en','ja','ko'];
export function weatherCondition(code,isDay,locale='zh-TW'){
 const day=isDay===true?'day':isDay===false?'night':null;
 let kind='unknown',slug='not-available';
 if(code===0){kind='clear';slug=day?'clear-'+day:'not-available'}
 else if(code===1||code===2){kind=code===1?'mostly':'clouds';slug=day?'partly-cloudy-'+day:'cloudy'}
 else if(code===3){kind='overcast';slug='overcast'}
 else if([45,48].includes(code)){kind='fog';slug='fog'}
 else if([51,53,55].includes(code)){kind='drizzle';slug='drizzle'}
 else if([56,57,66,67].includes(code)){kind='sleet';slug='sleet'}
 else if([61,63,65,80,81,82].includes(code)){kind='rain';slug='rain'}
 else if([71,73,75,77,85,86].includes(code)){kind='snow';slug='snow'}
 else if([95,96,97,99].includes(code)){kind='storm';slug='thunderstorms-rain'}
 return {kind,slug,label:labels[kind][Math.max(0,locales.indexOf(locale))]};
}
export function thermometerLevel(temp){return Number.isFinite(temp)?Math.max(0,Math.min(100,(temp+30)/80*100)):0}
export function weatherCopy(locale){return ({'zh-TW':['線上天氣未啟用','正在取得天氣…','暫無天氣資料','離線快取','模型天氣','濕度','資料時間','溫度計，示意刻度 −30 至 50°C'],'zh-CN':['在线天气未启用','正在获取天气…','暂无天气数据','离线缓存','模型天气','湿度','数据时间','温度计，示意刻度 −30 至 50°C'],en:['Online weather disabled','Loading weather…','Weather unavailable','Cached weather','Model weather','Humidity','Data time','Thermometer; visual scale −30 to 50°C'],ja:['オンライン天気は無効','天気を取得中…','天気データなし','キャッシュ','モデル天気','湿度','データ時刻','温度計（目盛り −30～50°C）'],ko:['온라인 날씨 꺼짐','날씨 불러오는 중…','날씨 데이터 없음','캐시된 날씨','모델 날씨','습도','자료 시각','온도계 눈금 −30~50°C']})[locale]||weatherCopy('en')}
