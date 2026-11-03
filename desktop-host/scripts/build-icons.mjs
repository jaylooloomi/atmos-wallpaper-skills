// Original code-native SVG artwork -> deterministic raster/ICO exports.
// Uses an isolated Chromium CLI render, never the user's browser or desktop UI.
import fs from 'node:fs';import os from 'node:os';import path from 'node:path';import {fileURLToPath,pathToFileURL} from 'node:url';import {spawnSync} from 'node:child_process';
const root=fileURLToPath(new URL('../',import.meta.url)),icons=path.join(root,'icons'),temp=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-icons-'));
const chrome=process.env.ATMOS_TEST_CHROME||'C:/Program Files/Google/Chrome/Application/chrome.exe',python=process.env.ATMOS_PYTHON||'C:/Python314/python.exe';
function run(exe,args){const r=spawnSync(exe,args,{encoding:'utf8',windowsHide:true,timeout:45000,maxBuffer:2*1024*1024});if(r.status!==0)throw Error((r.stderr||'')+String(r.error||''))}
const html=path.join(temp,'icon.html'),master=path.join(temp,'master.png');
fs.writeFileSync(html,'<!doctype html><meta charset="utf-8"><style>html,body{margin:0;width:1024px;height:1024px;background:transparent;overflow:hidden}svg{display:block;width:1024px;height:1024px}</style>'+fs.readFileSync(path.join(icons,'atmos.svg'),'utf8'));
run(chrome,['--headless=new','--no-first-run','--no-default-browser-check','--disable-extensions','--disable-background-networking','--hide-scrollbars','--force-device-scale-factor=1','--default-background-color=00000000','--window-size=1024,1024','--user-data-dir='+path.join(temp,'profile'),'--virtual-time-budget=1000','--screenshot='+master,pathToFileURL(html).href]);
run(python,['-c',String.raw`
from PIL import Image,ImageDraw,ImageFont
from pathlib import Path
import sys,json
source=Image.open(sys.argv[1]).convert('RGBA'); dest=Path(sys.argv[2])
assert source.size==(1024,1024)
assert source.getpixel((0,0))[3]==0, 'Transparent corners required'
master=source.resize((512,512),Image.Resampling.LANCZOS)
master.save(dest/'atmos.png')
sizes=[16,20,24,32,40,48,64,128,256]
frames=[]
for s in sizes:
 frame=master.resize((s,s),Image.Resampling.LANCZOS)
 frame.putalpha(frame.getchannel('A').point(lambda a: 0 if a<4 else a))
 for xy in [(0,0),(s-1,0),(0,s-1),(s-1,s-1)]:
  r,g,b,a=frame.getpixel(xy);frame.putpixel(xy,(r,g,b,0))
 frames.append(frame)
master.save(dest/'icon.ico',format='ICO',sizes=[(s,s) for s in sizes],append_images=frames)
check=Image.open(dest/'icon.ico')
assert check.ico.sizes()==set((s,s) for s in sizes)
for s in sizes:
 frame=check.ico.getimage((s,s)).convert('RGBA')
 assert frame.getpixel((0,0))[3]==0
 assert len(set(frame.getdata()))>32, 'A solid placeholder is not a usable icon'
sheet=Image.new('RGB',(1040,570),'#EAF0EF'); draw=ImageDraw.Draw(sheet)
font=ImageFont.truetype('C:/Windows/Fonts/segoeui.ttf',18)
draw.text((28,20),'ATMOS / app icon',(26,49,62),font=font)
sheet.paste(master,(18,48),master)
for row,(bg,ink) in enumerate([('#F7FAF8','#243747'),('#101B29','#E7F5EF')]):
 y=78+row*224;draw.rounded_rectangle((554,y,1012,y+196),radius=18,fill=bg)
 draw.text((574,y+16),'16 / 20 / 24 / 32 / 48 / 64 px',fill=ink,font=font)
 x=579
 for s in [16,20,24,32,48,64]:
  image=check.ico.getimage((s,s)).convert('RGBA');sheet.paste(image,(x,y+78+(64-s)//2),image)
  draw.text((x,y+154),str(s),fill=ink,font=font);x+=s+20
sheet.save(dest/'preview.png')
print(json.dumps({'source':'original SVG','png':[512,512],'icoSizes':sizes,'transparentCorners':True}))
`,master,icons]);
console.log(JSON.stringify({iconsBuilt:true,source:'icons/atmos.svg',outputs:['icons/atmos.png','icons/icon.ico','icons/preview.png']}));
