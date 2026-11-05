(()=>{'use strict';
 let started=false;
 function render(){
  const calendar=document.getElementById('calendar');if(!calendar)return;
  const locale=window.atmosBridgeApp?.getState?.().locale||'zh-TW',now=new Date(),year=now.getFullYear(),month=now.getMonth();
  const key=[locale,year,month,now.getDate()].join(':');if(calendar.firstElementChild?.dataset.monthCalendar===key)return;
  const wrap=document.createElement('div');wrap.className='month-calendar';wrap.dataset.monthCalendar=key;wrap.dataset.userContent='';
  const title=document.createElement('h3');title.textContent=new Intl.DateTimeFormat(locale,{year:'numeric',month:'long'}).format(now);wrap.append(title);
  const grid=document.createElement('div');grid.className='month-calendar-grid';
  for(let day=0;day<7;day++){const heading=document.createElement('span');heading.className='month-weekday';heading.textContent=new Intl.DateTimeFormat(locale,{weekday:'narrow'}).format(new Date(2023,0,1+day));grid.append(heading)}
  const first=new Date(year,month,1).getDay(),days=new Date(year,month+1,0).getDate();
  for(let i=0;i<Math.ceil((first+days)/7)*7;i++){
   const cell=document.createElement('span'),day=i-first+1;cell.className='month-day';
   if(day>0&&day<=days){cell.textContent=new Intl.NumberFormat(locale,{useGrouping:false}).format(day);cell.dataset.day=String(day);cell.setAttribute('aria-label',new Intl.DateTimeFormat(locale,{dateStyle:'full'}).format(new Date(year,month,day)));if(day===now.getDate()){cell.classList.add('is-today');cell.setAttribute('aria-current','date')}}else cell.setAttribute('aria-hidden','true');grid.append(cell);
  }
  wrap.append(grid);calendar.replaceChildren(wrap);
 }
 window.AtmosCalendar={render,start(){if(started)return;started=true;for(const section of document.querySelectorAll('.calendar-section'))section.remove();render();setInterval(render,60000);addEventListener('pageshow',render);document.addEventListener('visibilitychange',()=>{if(!document.hidden)render()})}};
})();
