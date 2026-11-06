import {createElement} from 'react';import {createRoot} from 'react-dom/client';import {flushSync} from 'react-dom';import {WeatherCard} from './weather-card.js';
const roots=new Map(),reduced=matchMedia('(prefers-reduced-motion: reduce)');
function sweep(){for(const [body,root]of roots)if(!body.isConnected){root.unmount();roots.delete(body)}}
window.AtmosWeather={motionPaused:()=>reduced.matches||document.hidden||!!window.AtmosAnimationSuspended?.(),sweep,
 render(body,data,locale,options={}){sweep();let root=roots.get(body);if(!root){root=createRoot(body);roots.set(body,root)}flushSync(()=>root.render(createElement(WeatherCard,{data,locale,staticIcon:options.staticIcon===true||window.AtmosWeather.motionPaused()})))},
 diagnostics:()=>({roots:roots.size}),dispose(){for(const root of roots.values())root.unmount();roots.clear()}};
