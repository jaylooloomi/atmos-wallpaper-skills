(()=>{'use strict';const reduced=matchMedia('(prefers-reduced-motion: reduce)');
 window.AtmosParticles={draw(ctx,{width,height,effect,intensity,elapsedMs,paused=false,reducedMotion=false}){
  if(paused||reducedMotion||reduced.matches||document.hidden||window.AtmosAnimationSuspended?.())return;
  const particles=particleFrame(effect,elapsedMs,intensity,width,height);if(!particles.length)return;
  ctx.save();try{
   for(const p of particles){ctx.globalAlpha=p.alpha;ctx.fillStyle=`hsl(${p.hue} 95% 78%)`;ctx.strokeStyle=ctx.fillStyle;
    if(p.kind==='glow'){
     const r=p.r*5,g=ctx.createRadialGradient(p.x,p.y,0,p.x,p.y,r);g.addColorStop(0,`hsla(${p.hue},100%,85%,.9)`);g.addColorStop(.2,`hsla(${p.hue},95%,65%,.5)`);g.addColorStop(1,`hsla(${p.hue},90%,55%,0)`);ctx.fillStyle=g;ctx.fillRect(p.x-r,p.y-r,r*2,r*2);
    }else if(p.kind==='petal'){
     ctx.save();ctx.translate(p.x,p.y);ctx.rotate(p.angle);ctx.scale(.45+.55*Math.abs(Math.cos(p.angle*.7)),1);ctx.beginPath();ctx.moveTo(0,-p.r);ctx.bezierCurveTo(p.r*1.5,-p.r,p.r,p.r*.8,0,p.r);ctx.bezierCurveTo(-p.r,p.r*.6,-p.r*1.2,-p.r*.4,0,-p.r);ctx.fill();ctx.restore();
    }else{
     const dx=Math.cos(p.angle)*p.tail,dy=Math.sin(p.angle)*p.tail,g=ctx.createLinearGradient(p.x-dx,p.y-dy,p.x,p.y);g.addColorStop(0,`hsla(${p.hue},95%,70%,0)`);g.addColorStop(1,`hsla(${p.hue},100%,90%,.95)`);ctx.strokeStyle=g;ctx.lineWidth=p.r;ctx.lineCap='round';ctx.beginPath();ctx.moveTo(p.x-dx,p.y-dy);ctx.lineTo(p.x,p.y);ctx.stroke();
    }
   }
  }finally{ctx.restore()}
 }};
})();
