# Read-only persistent English PDH GPU query. No UI control, repairs or background collector.
[CmdletBinding()]
param([int[]]$OwnedProcessIds=@(),[ValidateRange(0,10000)][int]$WarmupMilliseconds=1000)
if(-not (Get-Variable -Name AtmosGpuQuery -Scope Script -ErrorAction SilentlyContinue)){$script:AtmosGpuQuery=$null}
function Initialize-AtmosGpu {
 if (-not ('AtmosGpuPdh.Query' -as [type])) {
  Add-Type -TypeDefinition @'
using System;using System.Collections.Generic;using System.Runtime.InteropServices;
namespace AtmosGpuPdh {
 public sealed class Item {public string Counter;public string Instance;public uint Status;public double Value;}
 public sealed class Frame {public bool Warmup;public Item[] Items;public string[] Errors;public int CountersOpened;}
 public sealed class Query:IDisposable {
  [StructLayout(LayoutKind.Sequential)]struct CounterValue{public uint Status;public double Value;}
  [StructLayout(LayoutKind.Sequential)]struct ArrayItem{public IntPtr Name;public CounterValue Value;}
  [DllImport("pdh.dll",CharSet=CharSet.Unicode,EntryPoint="PdhOpenQueryW")]static extern uint Open(string source,IntPtr user,out IntPtr query);
  [DllImport("pdh.dll",CharSet=CharSet.Unicode,EntryPoint="PdhAddEnglishCounterW")]static extern uint Add(IntPtr query,string path,IntPtr user,out IntPtr counter);
  [DllImport("pdh.dll")]static extern uint PdhCollectQueryData(IntPtr query);
  [DllImport("pdh.dll",CharSet=CharSet.Unicode,EntryPoint="PdhGetFormattedCounterArrayW")]static extern uint ReadArray(IntPtr counter,uint format,ref uint bytes,ref uint count,IntPtr data);
  [DllImport("pdh.dll")]static extern uint PdhCloseQuery(IntPtr query);
  readonly Dictionary<string,IntPtr> counters=new Dictionary<string,IntPtr>();readonly List<string> initialErrors=new List<string>();IntPtr query;bool first=true;
  string Error(string phase,uint value){return phase+":0x"+value.ToString("X8");}
  public Query(){uint code=Open(null,IntPtr.Zero,out query);if(code!=0){initialErrors.Add(Error("OpenQuery",code));return;}AddOne("engine",@"\GPU Engine(*)\Utilization Percentage");AddOne("dedicated",@"\GPU Process Memory(*)\Dedicated Usage");AddOne("shared",@"\GPU Process Memory(*)\Shared Usage");AddOne("adapterDedicated",@"\GPU Adapter Memory(*)\Dedicated Usage");AddOne("adapterShared",@"\GPU Adapter Memory(*)\Shared Usage");code=PdhCollectQueryData(query);if(code!=0)initialErrors.Add(Error("WarmupCollect",code));}
  void AddOne(string name,string path){IntPtr counter;uint code=Add(query,path,IntPtr.Zero,out counter);if(code==0)counters[name]=counter;else initialErrors.Add(Error("Add."+name,code));}
  public Frame Sample(){var items=new List<Item>();var errors=new List<string>(initialErrors);bool warmup=first;first=false;if(query==IntPtr.Zero)return new Frame{Warmup=warmup,Items=items.ToArray(),Errors=errors.ToArray(),CountersOpened=0};uint collected=PdhCollectQueryData(query);if(collected!=0)errors.Add(Error("Collect",collected));foreach(var pair in counters){uint bytes=0,count=0;uint code=ReadArray(pair.Value,0x00008200,ref bytes,ref count,IntPtr.Zero);if(code!=0x800007D2&&code!=0){errors.Add(Error("ReadSize."+pair.Key,code));continue;}if(bytes==0||count==0)continue;if(bytes>32*1024*1024){errors.Add("ReadSize."+pair.Key+":TOO_LARGE");continue;}IntPtr buffer=Marshal.AllocHGlobal((int)bytes);try{code=ReadArray(pair.Value,0x00008200,ref bytes,ref count,buffer);if(code!=0){errors.Add(Error("Read."+pair.Key,code));continue;}int size=Marshal.SizeOf(typeof(ArrayItem));if((long)size*count>bytes){errors.Add("Read."+pair.Key+":INVALID_ARRAY");continue;}for(int i=0;i<count;i++){ArrayItem value=(ArrayItem)Marshal.PtrToStructure(IntPtr.Add(buffer,i*size),typeof(ArrayItem));items.Add(new Item{Counter=pair.Key,Instance=Marshal.PtrToStringUni(value.Name),Status=value.Value.Status,Value=value.Value.Value});}}finally{Marshal.FreeHGlobal(buffer);}}return new Frame{Warmup=warmup,Items=items.ToArray(),Errors=errors.ToArray(),CountersOpened=counters.Count};}
  public void Dispose(){if(query!=IntPtr.Zero){PdhCloseQuery(query);query=IntPtr.Zero;}GC.SuppressFinalize(this);}~Query(){Dispose();}
 }
}
'@ -ErrorAction Stop | Out-Null
 }
 if ($script:AtmosGpuQuery) { $script:AtmosGpuQuery.Dispose() }
 $script:AtmosGpuQuery=[AtmosGpuPdh.Query]::new()
}
function Read-AtmosGpu {
 param([Parameter(Mandatory=$true)][AllowEmptyCollection()][int[]]$OwnedPids)
 if (-not $script:AtmosGpuQuery) { throw 'Initialize-AtmosGpu must be called once before Read-AtmosGpu' }
 $frame=$script:AtmosGpuQuery.Sample()
 ConvertTo-AtmosGpuReport -Frame $frame -OwnedPids $OwnedPids
}
function ConvertTo-AtmosGpuReport {
 param([Parameter(Mandatory=$true)]$Frame,[Parameter(Mandatory=$true)][AllowEmptyCollection()][int[]]$OwnedPids)
 $frame=$Frame
 $owned=[Collections.Generic.HashSet[int]]::new();foreach($processId in $OwnedPids){if($processId -gt 0){[void]$owned.Add($processId)}}
 $engines=@{};$matchedEngines=0;$invalidEngine=0;$outOfRange=0;$memoryCount=@{dedicated=0;shared=0};$memoryTotals=@{dedicated=0.0;shared=0.0};$memoryInvalid=0;$adapterTotals=@{adapterDedicated=0.0;adapterShared=0.0};$adapterCount=0
 foreach($item in $frame.Items){
  $valid=($item.Status -eq 0 -or $item.Status -eq 1)-and -not [double]::IsNaN($item.Value)-and -not [double]::IsInfinity($item.Value)-and $item.Value -ge 0
  if($item.Counter -eq 'adapterDedicated' -or $item.Counter -eq 'adapterShared'){if($valid){$adapterTotals[$item.Counter]+=$item.Value;$adapterCount++};continue}
  if($item.Instance -notmatch '^pid_(\d+)_' -or -not $owned.Contains([int]$matches[1])){continue}
  if($item.Counter -eq 'engine'){
   if(-not $valid){$invalidEngine++;continue}
   if($item.Instance -notmatch '^pid_\d+_(luid_.+?_phys_\d+_eng_\d+)_engtype_'){ $invalidEngine++;continue }
   $key=$matches[1];if(-not $engines.ContainsKey($key)){$engines[$key]=0.0};$engines[$key]+=$item.Value;$matchedEngines++;if($item.Value -gt 100.0001){$outOfRange++}
  }elseif($item.Counter -eq 'dedicated' -or $item.Counter -eq 'shared'){
   if(-not $valid){$memoryInvalid++;continue};$memoryCount[$item.Counter]++;$memoryTotals[$item.Counter]+=$item.Value
  }
 }
 $engineError=@($frame.Errors|Where-Object {$_ -match '^(Add\.engine|ReadSize\.engine|Read\.engine|Collect|OpenQuery):'}).Count -gt 0
 $maximum=$null;if($engines.Count -gt 0){$maximum=[double](($engines.Values|Measure-Object -Maximum).Maximum);if($maximum -gt 100.0001){$outOfRange++}}
 [pscustomobject]@{
  schemaVersion=1;capturedAtUtc=[DateTime]::UtcNow.ToString('o');source='Windows PDH English GPU Engine / GPU Process Memory';warmup=$frame.Warmup;countersOpened=$frame.CountersOpened;requestedProcessCount=$owned.Count
  engine=[pscustomobject]@{available=(-not $frame.Warmup -and $matchedEngines -gt 0 -and $outOfRange -eq 0 -and $invalidEngine -eq 0 -and -not $engineError);busiestEnginePercent=if(-not $frame.Warmup -and $matchedEngines -gt 0 -and $outOfRange -eq 0 -and $invalidEngine -eq 0 -and -not $engineError){$maximum}else{$null};engineCount=$engines.Count;validInstanceCount=$matchedEngines;invalidInstanceCount=$invalidEngine;outOfRangeCount=$outOfRange;aggregation='Sum requested PIDs within the same physical engine, then maximum across engines; never sum all engine percentages.'}
  processMemory=[pscustomobject]@{available=($memoryCount.dedicated -gt 0 -or $memoryCount.shared -gt 0);accountedDedicatedBytes=if($memoryCount.dedicated -gt 0){$memoryTotals.dedicated}else{$null};accountedSharedBytes=if($memoryCount.shared -gt 0){$memoryTotals.shared}else{$null};validDedicatedInstances=$memoryCount.dedicated;validSharedInstances=$memoryCount.shared;invalidInstanceCount=$memoryInvalid;aggregation='Per-process accounting sum for requested PIDs; cross-process shared allocations can be double counted. Not unique physical VRAM and not leak proof.'}
  systemAdapterMemory=[pscustomobject]@{available=($adapterCount -gt 0);dedicatedBytes=if($adapterCount -gt 0){$adapterTotals.adapterDedicated}else{$null};sharedBytes=if($adapterCount -gt 0){$adapterTotals.adapterShared}else{$null};scope='All adapter instances across this machine; not attributed to Atmos.'}
  errors=@($frame.Errors);limitations=@('Counters require supported GPU/driver exposure; missing or invalid data is null, never zero.','PID ownership is supplied by the caller; include the native host and its actual WebView2 descendants, not every WebView2 process.','GPU use does not measure electrical energy, GPU temperature, frame presentation or human-visible smoothness.','Windows GPU process-memory counters have documented limitations, including shared allocation double counting and known Windows 10 incorrect accounting cases.')
 }
}
function Close-AtmosGpu {if($script:AtmosGpuQuery){$script:AtmosGpuQuery.Dispose();$script:AtmosGpuQuery=$null}}
if($MyInvocation.InvocationName -ne '.'){
 if($OwnedProcessIds.Count -eq 0){throw 'Pass explicit -OwnedProcessIds including the app host and its actual descendants.'}
 try{Initialize-AtmosGpu;Read-AtmosGpu -OwnedPids $OwnedProcessIds|Out-Null;if($WarmupMilliseconds -gt 0){Start-Sleep -Milliseconds $WarmupMilliseconds};Read-AtmosGpu -OwnedPids $OwnedProcessIds|ConvertTo-Json -Depth 6 -Compress}finally{Close-AtmosGpu}
}
