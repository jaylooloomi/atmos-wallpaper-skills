import test from 'node:test';import assert from 'node:assert/strict';import fs from 'node:fs';import path from 'node:path';
import {createElement as h} from 'react';import {renderToStaticMarkup} from 'react-dom/server';
import {weatherCondition,weatherSlugs,thermometerLevel} from '../src/weather-model.mjs';import {WeatherCard} from '../src/weather-card.js';
const sample=(weatherCode=3,isDay=true)=>({enabled:true,available:true,city:'東京',data:{temperatureC:13.7,humidityPercent:99,weatherCode,isDay,modelTimeMs:1793932200000}});
test('WMO and day/night mapping selects real conditions rather than temperature or decoration',()=>{
 for(const [codes,slug]of [[[3],'overcast'],[[45,48],'fog'],[[51,53,55],'drizzle'],[[56,57,66,67],'sleet'],[[61,63,65,80,81,82],'rain'],[[71,73,75,77,85,86],'snow'],[[95,96,97,99],'thunderstorms-rain']])for(const code of codes)assert.equal(weatherCondition(code,false).slug,slug);
 assert.equal(weatherCondition(0,true).slug,'clear-day');assert.equal(weatherCondition(0,false).slug,'clear-night');assert.equal(weatherCondition(0,null).slug,'not-available');assert.equal(weatherCondition(2,true).slug,'partly-cloudy-day');assert.equal(weatherCondition(2,false).slug,'partly-cloudy-night');assert.equal(weatherCondition(999,true).slug,'not-available');assert.equal(weatherCondition(3,true).label,'陰天');
});
test('React card has measured values, attribution, static variants, stale and unavailable states without fake data',()=>{
 const animated=renderToStaticMarkup(h(WeatherCard,{data:sample()}));assert.match(animated,/data-user-content/);assert.match(animated,/13\.7/);assert.match(animated,/東京/);assert.match(animated,/陰天/);assert.match(animated,/99%/);assert.match(animated,/animated\/overcast\.svg/);assert.match(animated,/Open-Meteo/);assert.doesNotMatch(animated,/WMO|min cache/);
 const still=renderToStaticMarkup(h(WeatherCard,{data:{...sample(61),stale:true,data:{...sample(61).data,temperatureC:-5.2}},staticIcon:true}));assert.match(still,/static\/rain\.svg/);assert.match(still,/-5\.2/);assert.match(still,/離線快取/);assert.match(still,/data-weather-static="true"/);
 for(const data of [null,{enabled:false},{enabled:true,available:false}]){const html=renderToStaticMarkup(h(WeatherCard,{data}));assert.doesNotMatch(html,/13\.7|99%/);assert.match(html,/not-available\.svg/)}
 const html=renderToStaticMarkup(h(WeatherCard,{data:{...sample(),city:'<img src=x onerror=alert(1)>'}}));assert.match(html,/&lt;img/);assert.doesNotMatch(html,/<img src=x/);
 assert.equal(thermometerLevel(-100),0);assert.equal(thermometerLevel(100),100);assert.equal(thermometerLevel(10),50);assert.equal(thermometerLevel(NaN),0);
});
test('packaged Meteocons assets are local, exact upstream bytes, static variants contain no animations, and licenses ship',()=>{
 for(const kind of ['animated','static'])for(const slug of weatherSlugs){const file=new URL('../dist/assets/weather/'+kind+'/'+slug+'.svg',import.meta.url),svg=fs.readFileSync(file,'utf8');assert.match(svg,/<svg\b/);assert.doesNotMatch(svg,/<script\b|<foreignObject\b|(?:href|src)=["'](?:https?:|\/\/|data:)|onload=/i);const pkg=kind==='static'?'svg-static':'svg';assert.equal(svg,fs.readFileSync(new URL('../node_modules/@meteocons/'+pkg+'/fill/'+slug+'.svg',import.meta.url),'utf8'));if(kind==='static')assert.doesNotMatch(svg,/<(?:animate|animateTransform|animateMotion|set)\b|@keyframes|\banimation\s*:/i)}
 for(const name of ['react','react-dom','scheduler','@meteocons-svg','@meteocons-svg-static'])assert.match(fs.readFileSync(new URL('../dist/licenses/'+name+'-LICENSE.txt',import.meta.url),'utf8'),/Permission is hereby granted/);
 assert.ok(fs.statSync(new URL('../dist/weather-react.js',import.meta.url)).size<300000);
});
