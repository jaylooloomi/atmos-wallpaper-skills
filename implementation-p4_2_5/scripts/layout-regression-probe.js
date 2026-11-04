// Runs only inside the isolated layout fixture, never in the installed app.
window.AtmosLayoutRegression={measure({injectOverlap=false}={}){
 const failures=[],states=[],body=document.body,originalTheme=body.dataset.theme,originalCard=window.AtmosInspector.activeId;
 const rect=e=>e.getBoundingClientRect(),shown=e=>!e.hidden&&getComputedStyle(e).display!=='none'&&rect(e).width>0&&rect(e).height>0;
 const fail=(code,theme,card)=>failures.push({code,theme,card});
 const injected=document.createElement('style');if(injectOverlap){injected.textContent='body[data-render-mode=manager] #componentInspector .inspect-props input,body[data-render-mode=manager] #componentInspector .inspect-props select,body[data-render-mode=manager] #componentInspector .inspect-props button{position:absolute!important;left:0!important;top:0!important}';document.head.append(injected)}
 try{for(const theme of ['natural','neon','warm','minimal']){
  body.dataset.theme=theme;
  for(const card of ['clock','calendar','weather','todo','timer','system','media']){
   window.AtmosInspector.open(card);window.atmosSchedulePanelAlign?.();
   const props=document.getElementById('props-'+card),stress=document.createElement('p');
   stress.className='inspect-helper';stress.textContent='排版回歸測試：長文字也必須完整容納。 Layout regression: long content must wrap without overlapping controls. '.repeat(4);props.append(stress);
   try{
    for(const selector of ['.side','.component-section','#widgetManager','#componentInspector','.inspect-card.is-open']){
     const parent=document.querySelector(selector),p=rect(parent);
     if([...parent.children].filter(shown).some(e=>rect(e).bottom>p.bottom+2||rect(e).top<p.top-2))fail('CHILD_OUTSIDE_CONTAINER',theme,card);
    }
    const siblings=[...document.querySelector('.panel-scroll').children].filter(shown);
    if(siblings.slice(1).some((e,i)=>rect(e).top<rect(siblings[i]).bottom-1))fail('SETTINGS_SECTIONS_OVERLAP',theme,card);
    if(props.scrollHeight>props.clientHeight+2)fail('NESTED_COMPONENT_SCROLL',theme,card);
    const controls=[...props.querySelectorAll('input,select,button,textarea')].filter(shown),bounds=rect(props);
    if(controls.some(e=>rect(e).left<bounds.left-1||rect(e).right>bounds.right+1||rect(e).top<bounds.top-1||rect(e).bottom>bounds.bottom+1))fail('CONTROL_OUTSIDE_FIELDS',theme,card);
    let overlap=false;for(let i=0;i<controls.length;i++)for(let j=i+1;j<controls.length;j++){const a=controls[i],b=controls[j];if(a.contains(b)||b.contains(a))continue;const x=rect(a),y=rect(b);if(Math.min(x.right,y.right)-Math.max(x.left,y.left)>1&&Math.min(x.bottom,y.bottom)-Math.max(x.top,y.top)>1)overlap=true}if(overlap)fail('CONTROLS_OVERLAP',theme,card);
    const side=rect(document.querySelector('.side'));
    if([...document.querySelectorAll('.side input,.side select,.side button')].filter(shown).some(e=>rect(e).left<side.left-1||rect(e).right>side.right+1))fail('CONTROL_OUTSIDE_PANEL',theme,card);
    const a=rect(document.querySelector('[data-sc-install]')),b=rect(document.querySelector('[data-sc-settings]'));
    const gap=Math.max(b.top-a.bottom,a.top-b.bottom,b.left-a.right,a.left-b.right);
    if(gap<8||a.height<44||b.height<44)fail('SCREENSAVER_ACTION_SPACING',theme,card);
    if(document.documentElement.scrollWidth>document.documentElement.clientWidth+1)fail('PAGE_HORIZONTAL_OVERFLOW',theme,card);
    states.push({theme,card,propsHeight:rect(props).height,sideHeight:side.height,buttonGap:gap});
   }finally{stress.remove()}
  }
 }}finally{injected.remove();body.dataset.theme=originalTheme;if(originalCard)window.AtmosInspector.open(originalCard);window.atmosSchedulePanelAlign?.()}
 return {states,failures,passed:states.length===28&&failures.length===0};
}};
