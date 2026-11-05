export function clockText(date){return [date.getHours(),date.getMinutes(),date.getSeconds()].map(v=>String(v).padStart(2,'0')).join(':')}
export function snapClock(x,y,width,height,{snapPx=8,guidePx=14}={}){if(![x,y,width,height].every(Number.isFinite)||width<=0||height<=0)return {x,y,guideX:false,guideY:false,snappedX:false,snappedY:false};const dx=Math.abs(x-50)*width/100,dy=Math.abs(y-50)*height/100;return {x:dx<=snapPx+1e-7?50:x,y:dy<=snapPx+1e-7?50:y,guideX:dx<=guidePx+1e-7,guideY:dy<=guidePx+1e-7,snappedX:dx<=snapPx+1e-7,snappedY:dy<=snapPx+1e-7}}
export function centeredClock(visual,mobile,both=false){const next={...visual};next[mobile?'mobileClockX':'clockX']=50;if(both)next[mobile?'mobileClockY':'clockY']=50;return next}

let clockFormatterKey='',clockFormatter=null;
export function clockPresentation(date,{hourCycle=24,showSeconds=true,locale='zh-TW'}={}){
 if(!(date instanceof Date)||!Number.isFinite(date.getTime()))throw new TypeError('valid Date required');
 if(![12,24].includes(hourCycle)||typeof showSeconds!=='boolean')throw new RangeError('hourCycle must be12 or24 and showSeconds boolean');
 const hour=hourCycle===12?(date.getHours()%12||12):date.getHours(),values=[hour,date.getMinutes()];if(showSeconds)values.push(date.getSeconds());
 const key=JSON.stringify([locale,hourCycle,showSeconds,date.getTimezoneOffset()]);if(!clockFormatter||clockFormatterKey!==key){clockFormatter=new Intl.DateTimeFormat(locale,{hour:'2-digit',minute:'2-digit',...(showSeconds?{second:'2-digit'}:{}),hourCycle:hourCycle===12?'h12':'h23'});clockFormatterKey=key}const formatter=clockFormatter,parts=hourCycle===12?formatter.formatToParts(date):[];
 return {digits:values.map(x=>String(x).padStart(2,'0')).join(':'),period:hourCycle===12?(parts.find(p=>p.type==='dayPeriod')?.value||''):'',ariaLabel:formatter.format(date)};
}
