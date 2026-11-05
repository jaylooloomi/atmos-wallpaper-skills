// Transform generated output only; the historical prototype remains frozen.
export function removeManagerPanels(html,app){
 const marker='<div class="promptbox" id="promptBox">',start=html.indexOf(marker);
 if(start<0||html.indexOf(marker,start+1)>=0)throw Error('Expected one legacy prompt panel');
 const tags=/<\/?div\b[^>]*>/g;tags.lastIndex=start;let depth=0,end=-1,match;
 while((match=tags.exec(html))){depth+=match[0].startsWith('</')?-1:1;if(depth===0){end=tags.lastIndex;break}}
 if(end<0)throw Error('Unclosed legacy prompt panel');
 html=html.slice(0,start)+html.slice(end);
 html=html.replace(/<button\b[^>]*\bid="openPrompt"[^>]*>[\s\S]*?<\/button>/,'');
 const from=app.indexOf('function buildExternalPrompt(request)'),to=app.indexOf("$('apply').onclick",from);
 if(from<0||to<from)throw Error('Expected bounded legacy prompt handlers');
 app=app.slice(0,from)+app.slice(to);
 const reset="$('prompt').value='';$('generatedPrompt').value='';$('promptResult').hidden=true;";
 if(!app.includes(reset))throw Error('Expected legacy reset bindings');
 app=app.replace(reset,'').replace(/^\$\('openPrompt'\)\.onclick=.*\r?\n/m,'');
 app='const atmosLegacyMessage={textContent:""};\n'+app.replaceAll("$('status')",'atmosLegacyMessage');
 return {html,app};
}
