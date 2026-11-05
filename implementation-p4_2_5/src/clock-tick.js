let clockDateFormatter=null,clockDateLocale=null,clockDateOffset=null;
function updateClockText(element,value){const node=element.firstChild;if(node?.nodeType===3&&!node.nextSibling){if(node.data!==value)node.data=value}else if(element.textContent!==value)element.textContent=value}
function tick(){
 const d=new Date(),view=clockPresentation(d,{hourCycle:state.hourCycle,showSeconds:state.showSeconds,locale:state.locale}),text=view.digits,time=$('time');
 let digits=time.querySelector('.clock-digits');
 if(!digits){digits=document.createElement('span');digits.className='clock-digits';digits.setAttribute('aria-hidden','true');for(let i=0;i<8;i++)digits.append(document.createElement('span'));time.replaceChildren(digits)}
 const seconds=String(state.showSeconds);if(digits.dataset.seconds!==seconds)digits.dataset.seconds=seconds;
 for(let i=0;i<8;i++){const ch=text[i]||'';updateClockText(digits.children[i],ch);const hidden=i>=text.length;if(digits.children[i].hidden!==hidden)digits.children[i].hidden=hidden}
 let period=time.querySelector('.clock-period');
 if(!period){period=document.createElement('span');period.className='clock-period';period.setAttribute('aria-hidden','true');time.append(period)}
 updateClockText(period,view.period);if(time.getAttribute('aria-label')!==view.ariaLabel)time.setAttribute('aria-label',view.ariaLabel);
 const offset=d.getTimezoneOffset();if(!clockDateFormatter||clockDateLocale!==state.locale||clockDateOffset!==offset){clockDateFormatter=new Intl.DateTimeFormat(state.locale,{month:'short',day:'numeric',weekday:'short'});clockDateLocale=state.locale;clockDateOffset=offset}
 updateClockText($('date'),clockDateFormatter.format(d));
}
