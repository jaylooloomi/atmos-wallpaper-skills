import {createElement as h} from 'react';
import {weatherCondition,thermometerLevel,weatherCopy} from './weather-model.mjs';
export function WeatherCard({data,locale='zh-TW',staticIcon=false}){
 const t=weatherCopy(locale),ready=!!data?.enabled&&!!data?.available&&Number.isFinite(data.data?.temperatureC),sample=ready?data.data:null,condition=sample?weatherCondition(sample.weatherCode,sample.isDay,locale):{slug:'not-available',label:''};
 const state=!data?t[1]:!data.enabled?t[0]:!ready?t[2]:data.stale?t[3]:t[4];
 const temp=sample?.temperatureC;
 return h('section',{className:'weather-card-view','data-user-content':'','data-weather-static':String(staticIcon),'data-weather-kind':condition.slug,'data-weather-status':ready?'available':'unavailable'},
  h('div',{className:'weather-overview'},
   h('img',{className:'weather-condition-icon',src:'assets/weather/'+(staticIcon?'static':'animated')+'/'+condition.slug+'.svg',alt:condition.label||state,width:80,height:80,decoding:'async'}),
   h('div',{className:'weather-reading'},ready?h('div',{className:'weather-temperature'},temp.toFixed(1),h('span',{className:'weather-unit'},' °C')):h('div',{className:'weather-temperature'},'—'),h('div',{className:'weather-condition',title:ready?data.city+' · '+condition.label:state},ready&&h('span',{className:'weather-city'},data.city+' · '),condition.label||state)),
   ready&&h('div',{className:'weather-thermometer',role:'img','aria-label':t[7]+' · '+temp.toFixed(1)+'°C','data-temperature':String(temp),style:{'--weather-level':thermometerLevel(temp)+'%','--weather-heat':temp>=28?'#f29b64':temp<10?'#6cb9ee':'#80c8b3'}},h('span',{className:'weather-thermometer-tube'},h('i')),h('span',{className:'weather-thermometer-bulb'}))),
  ready&&h('div',{className:'weather-details'},h('span',null,t[5]+' '+sample.humidityPercent+'%'),data.stale&&h('span',{className:'weather-data-state'},state)),
  !ready&&h('small',{className:'weather-empty-state'},state));
}
