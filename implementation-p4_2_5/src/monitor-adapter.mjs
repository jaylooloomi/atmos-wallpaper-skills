// Read-only host metadata. Never use a browser viewport as evidence of a target monitor.
import {spawnSync} from 'node:child_process';
const fallback=()=>({width:1920,height:1080,source:'default-aspect',detected:false});
const idOK=id=>typeof id==='string'&&/^\\\\\.\\DISPLAY[1-9][0-9]{0,3}$/i.test(id);
const dimension=n=>Number.isInteger(n)&&n>=1&&n<=65536;
export function normalizeMonitorMetadata(raw,{targetId=null,source='fixture',capturedAt=new Date().toISOString()}={}){
 const unavailable=reason=>({schemaVersion:1,status:'unavailable',source,reason,capturedAt,target:null,selection:'unresolved',monitors:[],previewFallback:fallback()});
 if(targetId!==null&&!idOK(targetId))return unavailable('INVALID_TARGET_ID');
 if(!raw||raw.schemaVersion!==1||!Array.isArray(raw.monitors)||raw.monitors.length>64)return unavailable('INVALID_MONITOR_METADATA');
 const monitors=[],ids=new Set();
 for(const m of raw.monitors){
  if(!m||!idOK(m.id)||ids.has(m.id.toUpperCase())||![true,false,null].includes(m.primary))return unavailable('INVALID_MONITOR_METADATA');
  if((m.width!==null&&!dimension(m.width))||(m.height!==null&&!dimension(m.height))||(m.scalePercent!==null&&(!Number.isInteger(m.scalePercent)||m.scalePercent<100||m.scalePercent>500)))return unavailable('INVALID_MONITOR_METADATA');
  ids.add(m.id.toUpperCase());monitors.push({id:m.id,idKind:'windows-gdi-device-name',primary:m.primary,width:m.width,height:m.height,scalePercent:m.scalePercent,resolutionUnit:'physical-pixels',scaleSource:m.scalePercent===null?'unknown':'GetScaleFactorForMonitor'});
 }
 const primary=monitors.filter(m=>m.primary===true),target=targetId?monitors.find(m=>m.id.toUpperCase()===targetId.toUpperCase()):primary.length===1?primary[0]:null;
 if(!target)return {...unavailable(targetId?'TARGET_NOT_FOUND':monitors.length?'PRIMARY_UNRESOLVED':'NO_MONITORS'),monitors};
 const complete=target.width!==null&&target.height!==null&&target.scalePercent!==null;
 return {schemaVersion:1,status:complete?'detected':'partial',source,reason:complete?null:'FIELDS_UNAVAILABLE',capturedAt,target,selection:targetId?'explicit':'primary',monitors,previewFallback:target.width&&target.height?null:fallback()};
}
// Constant script only: IDs/requests are selected in JS, never interpolated into PowerShell.
export const WINDOWS_MONITOR_SCRIPT=String.raw`
$ErrorActionPreference='Stop'
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class AtmosReadOnlyMonitors {
 [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left,Top,Right,Bottom; }
 [StructLayout(LayoutKind.Sequential,CharSet=CharSet.Unicode)] public struct MONITORINFOEX { public uint cbSize; public RECT rcMonitor,rcWork; public uint dwFlags; [MarshalAs(UnmanagedType.ByValTStr,SizeConst=32)] public string szDevice; }
 [StructLayout(LayoutKind.Sequential,CharSet=CharSet.Unicode)] public struct DEVMODE {
  [MarshalAs(UnmanagedType.ByValTStr,SizeConst=32)] public string dmDeviceName;
  public ushort dmSpecVersion,dmDriverVersion,dmSize,dmDriverExtra; public uint dmFields;
  public int dmPositionX,dmPositionY; public uint dmDisplayOrientation,dmDisplayFixedOutput;
  public short dmColor,dmDuplex,dmYResolution,dmTTOption,dmCollate;
  [MarshalAs(UnmanagedType.ByValTStr,SizeConst=32)] public string dmFormName;
  public ushort dmLogPixels; public uint dmBitsPerPel,dmPelsWidth,dmPelsHeight,dmDisplayFlags,dmDisplayFrequency,dmICMMethod,dmICMIntent,dmMediaType,dmDitherType,dmReserved1,dmReserved2,dmPanningWidth,dmPanningHeight;
 }
 public delegate bool MonitorProc(IntPtr h,IntPtr dc,ref RECT r,IntPtr data);
 [DllImport("user32.dll")] [return:MarshalAs(UnmanagedType.Bool)] public static extern bool EnumDisplayMonitors(IntPtr dc,IntPtr clip,MonitorProc callback,IntPtr data);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] [return:MarshalAs(UnmanagedType.Bool)] public static extern bool GetMonitorInfoW(IntPtr h,ref MONITORINFOEX info);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] [return:MarshalAs(UnmanagedType.Bool)] public static extern bool EnumDisplaySettingsW(string name,int mode,ref DEVMODE dev);
 [DllImport("shcore.dll")] public static extern int GetScaleFactorForMonitor(IntPtr h,out int scale);
 public sealed class Item { public string id; public bool primary; public uint? width,height; public int? scalePercent; }
 public static Item[] Read() {
  var list=new List<Item>();
  MonitorProc cb=delegate(IntPtr h,IntPtr dc,ref RECT rect,IntPtr data) {
   var info=new MONITORINFOEX();info.cbSize=(uint)Marshal.SizeOf(typeof(MONITORINFOEX));
   if(!GetMonitorInfoW(h,ref info)) return true;
   var item=new Item();item.id=info.szDevice;item.primary=(info.dwFlags&1)!=0;
   var mode=new DEVMODE();mode.dmSize=(ushort)Marshal.SizeOf(typeof(DEVMODE));
   if(EnumDisplaySettingsW(info.szDevice,-1,ref mode)){item.width=mode.dmPelsWidth;item.height=mode.dmPelsHeight;}
   try {int scale;if(GetScaleFactorForMonitor(h,out scale)==0)item.scalePercent=scale;} catch(DllNotFoundException){} catch(EntryPointNotFoundException){}
   list.Add(item);return true;
  };
  if(!EnumDisplayMonitors(IntPtr.Zero,IntPtr.Zero,cb,IntPtr.Zero))throw new InvalidOperationException("Monitor enumeration failed");
  return list.ToArray();
 }
}
'@
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
@{schemaVersion=1;monitors=@([AtmosReadOnlyMonitors]::Read())} | ConvertTo-Json -Depth 5 -Compress
`;
export function readMonitorMetadata({targetId=null,runner,fixture,now=()=>new Date().toISOString(),enabled=false}={}){
 const capturedAt=now();
 if(fixture!==undefined)return normalizeMonitorMetadata(fixture,{targetId,source:'fixture',capturedAt});
 const unavailable=reason=>({...normalizeMonitorMetadata({schemaVersion:1,monitors:[]},{targetId,source:'unavailable',capturedAt}),reason});
 if(!enabled)return unavailable('ADAPTER_NOT_ENABLED');
 // An injected runner is a pure test seam and is always labelled fixture, never host evidence.
 if(!runner&&process.platform!=='win32')return unavailable('WINDOWS_ADAPTER_UNAVAILABLE');
 try{
  const run=runner||spawnSync;
  const result=run('powershell.exe',['-NoLogo','-NoProfile','-NonInteractive','-EncodedCommand',Buffer.from(WINDOWS_MONITOR_SCRIPT,'utf16le').toString('base64')],{encoding:'utf8',windowsHide:true,timeout:8000,maxBuffer:128*1024,shell:false});
  if(result.error||result.status!==0)return unavailable('MONITOR_QUERY_FAILED');
  if(typeof result.stdout!=='string'||Buffer.byteLength(result.stdout)>128*1024)return unavailable('MONITOR_RESPONSE_INVALID');
  const raw=JSON.parse(result.stdout.replace(/^\uFEFF/,''));
  return normalizeMonitorMetadata(raw,{targetId,source:runner?'fixture':'windows-native-readonly',capturedAt});
 }catch{return unavailable('MONITOR_QUERY_FAILED')}
}
