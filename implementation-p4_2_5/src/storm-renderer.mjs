// Called by prepare; the frozen baseline remains unchanged.
export function upgradeStormRenderer(app){
 const old="if(!state.reduced&&!state.paused&&state.lightning&&['thunder','typhoon'].includes(f)){const phase=t%9000;$('flash').style.opacity=phase<250?String(.1*Math.sin(phase/250*Math.PI)):'0'}else $('flash').style.opacity='0'";
 if(!app.includes(old))throw Error('Storm baseline hook missing');
 if(!app.includes("function draw(t){"))throw Error("Weather clock hook missing");
 return app.replace("function draw(t){","function draw(t){t=Date.now()-(window.AtmosAnimationEpoch||0);").replace(old,"window.AtmosLightning?.draw(ctx,{width:w,height:h,effect:f,intensity:state.intensity,paused:state.paused,reducedMotion:state.reduced,elapsedMs:Date.now()-(window.AtmosAnimationEpoch||0)});");
}
