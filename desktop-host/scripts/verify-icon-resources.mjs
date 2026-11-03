// Read icon resources from the installed binaries. No desktop/UI automation.
import fs from 'node:fs';import path from 'node:path';import os from 'node:os';import {fileURLToPath} from 'node:url';import {spawnSync} from 'node:child_process';import {createHash} from 'node:crypto';
if(process.platform!=='win32')throw Error('Windows resource extraction only');
const root=fileURLToPath(new URL('../',import.meta.url)),config=JSON.parse(fs.readFileSync(path.join(root,'tauri.conf.json'))),install=path.join(process.env.LOCALAPPDATA,'Atmos Wallpaper'),temp=fs.mkdtempSync(path.join(os.tmpdir(),'atmos-icon-resources-'));
const binaries=[['application',path.join(install,'atmos-wallpaper.exe')],['installer',path.join(root,'target/release/bundle/nsis',config.productName+'_'+config.version+'_x64-setup.exe')],['uninstaller',path.join(install,'uninstall.exe')]];
function run(exe,args){const r=spawnSync(exe,args,{encoding:'utf8',windowsHide:true,timeout:45000,maxBuffer:2*1024*1024});if(r.status!==0)throw Error((r.stderr||'')+String(r.error||''));return r.stdout}
const literal=v=>"'"+v.replaceAll("'","''")+"'";
for(const [name,file]of binaries){if(!fs.existsSync(file))throw Error('Missing '+name);run('C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe',['-NoProfile','-NonInteractive','-Command',`Add-Type -AssemblyName System.Drawing; $icon=[Drawing.Icon]::ExtractAssociatedIcon(${literal(file)}); if(!$icon){throw 'No icon resource'}; $bitmap=$icon.ToBitmap(); try{$bitmap.Save(${literal(path.join(temp,name+'.png'))},[Drawing.Imaging.ImageFormat]::Png)}finally{$bitmap.Dispose();$icon.Dispose()}`])}
const result=JSON.parse(run(process.env.ATMOS_PYTHON||'C:/Python314/python.exe',['-c',String.raw`
from PIL import Image
from pathlib import Path
import sys,json
source=Image.open(sys.argv[1]); folder=Path(sys.argv[2]); checks=[]
for name in ['application','installer','uninstaller']:
 actual=Image.open(folder/(name+'.png')).convert('RGBA'); expected=source.ico.getimage(actual.size).convert('RGBA')
 assert actual.getpixel((0,0))[3]==0, name+' corner is not transparent'
 errors=[]
 for a,b in zip(actual.getdata(),expected.getdata()):
  if b[3]>=250: errors.extend(abs(a[c]-b[c]) for c in range(3))
 mean=sum(errors)/len(errors)
 assert mean<=5, name+' does not match the new source icon: '+str(mean)
 checks.append({'binary':name,'size':list(actual.size),'transparentCorner':True,'opaquePixelMeanError':mean,'matchesSource':True})
print(json.dumps({'checks':checks,'passed':True}))
`,path.join(root,'icons/icon.ico'),temp]));
const evidence={schemaVersion:1,date:'2026-11-04',version:config.version,method:'System.Drawing resource extraction from installed application, NSIS installer and uninstaller; no UI automation',sourceIconSha256:createHash('sha256').update(fs.readFileSync(path.join(root,'icons/icon.ico'))).digest('hex'),...result,limitations:['Resource verification is not manual observation of Windows icon caches or notification-area rendering.']};
fs.writeFileSync(path.join(root,'evidence/app-icon.json'),JSON.stringify(evidence,null,2)+'\n');console.log(JSON.stringify(result));
