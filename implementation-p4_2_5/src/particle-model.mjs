// Analytic, bounded particles: no retained emitters, timers, history or remote assets.
export const particleEffects=['meteor','firefly','sakura','fireworks','galaxy'];
const unit=(seed)=>{let x=(seed+1)*2654435761>>>0;x^=x>>>16;x=Math.imul(x,2246822507);x^=x>>>13;return (x>>>0)/4294967296};
const fraction=x=>x-Math.floor(x);
export function particleFrame(effect,elapsedMs,intensity,width,height){
 if(!particleEffects.includes(effect)||![elapsedMs,intensity,width,height].every(Number.isFinite)||width<=0||height<=0||intensity<=0)return [];
 const power=Math.min(100,intensity)/100,t=Math.max(0,elapsedMs)/1000,scale=Math.max(.45,Math.min(1.4,Math.sqrt(width*height/2073600))),out=[];
 const add=p=>out.push(p),count=n=>Math.max(1,Math.round(n*power*scale));
 if(effect==='meteor')for(let i=0;i<count(18);i++){
  const phase=fraction(t/(2+unit(i+30)*4)+unit(i)),x=unit(i+10)*1.4-.2+phase*.45,y=phase*1.3-.2;
  add({kind:'meteor',x:x*width,y:y*height,r:1+unit(i+70),tail:45+unit(i+100)*100,angle:.9,alpha:Math.sin(phase*Math.PI)*.85,hue:180+unit(i+80)*70});
 }
 if(effect==='firefly')for(let i=0;i<count(110);i++){
  add({kind:'glow',x:(unit(i)+Math.sin(t*.13+unit(i+40)*8)*.06)*width,y:(unit(i+10)+Math.cos(t*.17+unit(i+20)*8)*.07)*height,r:1.4+unit(i+90)*2.5,alpha:.2+.65*(.5+.5*Math.sin(t*(.5+unit(i+50))+i)),hue:55+unit(i+30)*90});
 }
 if(effect==='sakura')for(let i=0;i<count(65);i++){
  const phase=fraction(t/(13+unit(i+20)*14)+unit(i));
  add({kind:'petal',x:(fraction(unit(i+10)+phase*.28+Math.sin(t*.5+i)*.04)*1.1-.05)*width,y:(phase*1.15-.1)*height,r:4+unit(i+60)*6,angle:t*(.3+unit(i))+i,alpha:.55+unit(i+80)*.35,hue:325+unit(i+30)*30});
 }
 if(effect==='fireworks')for(let i=0;i<count(3);i++){
  const cycle=Math.floor((t+i*1.37)/4.6),phase=(t+i*1.37)%4.6,seed=cycle*71+i*907,cx=(.15+unit(seed)*.7)*width,cy=(.16+unit(seed+10)*.35)*height,hue=unit(seed+20)*360;
  if(phase<.9){add({kind:'meteor',x:cx,y:height-(height-cy)*phase/.9,r:1.5,tail:38,angle:-Math.PI/2,alpha:.8,hue});continue}
  const age=phase-.9;if(age>2.8)continue;
  for(let j=0;j<Math.round(64*power);j++){
   const angle=j*Math.PI*2/Math.round(64*power)+unit(seed+j+50)*.08,speed=(45+unit(seed+j+40)*65)*Math.min(width,height)/720,travel=speed*(1-Math.exp(-age*.65))/.65;
   add({kind:'spark',x:cx+Math.cos(angle)*travel,y:cy+Math.sin(angle)*travel+age*age*17,r:1+unit(j)*1.4,angle:angle,tail:3+10*(1-age/2.8),alpha:Math.pow(1-age/2.8,1.5)*.85,hue:hue+unit(j+31)*25});
  }
 }
 if(effect==='galaxy')for(let i=0;i<count(230);i++){
  const radial=Math.sqrt(unit(i+3)),angle=radial*10+t*(.07+.05*(1-radial))+(i%3)*Math.PI*2/3+unit(i+20)*.5,radius=radial*Math.min(width,height)*.57;
  add({kind:'glow',x:width*.5+Math.cos(angle)*radius,y:height*.5+Math.sin(angle)*radius*.53,r:.8+unit(i+70)*2.2,alpha:.25+unit(i+30)*.65,hue:180+radial*110+Math.sin(t*.1)*15});
 }
 return out;
}
