param(
 [ValidateRange(15,86400)][int]$DurationSeconds=7200,
 [ValidateRange(1,60)][int]$SampleSeconds=15,
 [string]$OutputDirectory,
 [switch]$Run
)
$ErrorActionPreference='Stop'
if(-not $Run){throw 'Explicit -Run is required. Read-only installed-host collection; no wallpaper settings change.'}
if(-not $OutputDirectory -or -not [IO.Path]::IsPathRooted($OutputDirectory)){throw 'An explicit absolute private output directory is required'}
if(Test-Path -LiteralPath $OutputDirectory){throw 'Use a new output directory; existing evidence is never overwritten'}
$scriptRoot=$PSScriptRoot
. (Join-Path $scriptRoot 'windows-owned-processes.ps1')
. (Join-Path $scriptRoot 'probe-gpu.ps1')
. (Join-Path $scriptRoot 'probe-power.ps1')
$exe=Join-Path $env:LOCALAPPDATA 'Atmos Wallpaper\atmos-wallpaper.exe'
$store=Join-Path $env:LOCALAPPDATA 'io.atmos.wallpaper\store'
$initial=& $exe --host-command get | ConvertFrom-Json
if($LASTEXITCODE -ne 0 -or -not $initial.runtimeFresh -or -not $initial.runtime.nativeAttached){throw 'A fresh attached installed Atmos host is required'}
$rootPid=[int]$initial.runtime.pid
$hostSession=[string]$initial.runtime.hostSessionId
if($hostSession -notmatch '^\d+-\d+$'){throw 'Host session identity is missing'}
$sampler=[AtmosLongTest.OwnedSampler]::new($rootPid,$exe)
$first=$sampler.Sample()
if(-not $first.rootAlive -or @($first.processes | Where-Object group -eq 'webview').Count -lt 1){throw 'Owned WebView2 tree could not be proven'}
$runs=Join-Path $env:LOCALAPPDATA 'io.atmos.wallpaper\runs'
$runDir=@(Get-ChildItem -LiteralPath $runs -Directory | Where-Object {$_.Name.EndsWith("-$rootPid")}) | Select-Object -First 1
if(-not $runDir){throw 'Matching native runtime journal missing'}
$journal=Get-Content -LiteralPath (Join-Path $runDir.FullName 'status.json') -Raw | ConvertFrom-Json
$port=[int](@($journal.events | Where-Object kind -eq 'runtime-ready')[-1].detail.port)
if($port -lt 1 -or $port -gt 65535){throw 'Runtime port missing'}
$origin="http://127.0.0.1:$port"
New-Item -ItemType Directory -Path $OutputDirectory | Out-Null
$utf8=[Text.UTF8Encoding]::new($false)
function Save-Json($name,$value){[IO.File]::WriteAllText((Join-Path $OutputDirectory $name),($value|ConvertTo-Json -Depth 16),$utf8)}
function Append-Sample($name,$value){[IO.File]::AppendAllText((Join-Path $OutputDirectory $name),($value|ConvertTo-Json -Depth 16 -Compress)+[Environment]::NewLine,$utf8)}
function Read-Record([string]$file,[long]$max=131072){$item=Get-Item -LiteralPath $file;if($item.Length -gt $max){throw 'RECORD_TOO_LARGE'};Get-Content -LiteralPath $file -Raw | ConvertFrom-Json}
function Load-Workload($native){
 $profiles=@{}
 foreach($target in @($native.targets)){
  $scope=[string]$target.scopeId
  if($scope -eq 'shared'){$dir=$store}
  elseif($scope -match '^display-[a-f0-9]{64}$'){$dir=Join-Path $store "targets\$scope"}
  else{continue}
  if($profiles.ContainsKey($scope)){continue}
  try{$live=Read-Record (Join-Path $dir 'live.json') 16777216;$v=$live.desired.visual;$profiles[$scope]=@{revision=$live.revision;scene=$v.background;effect=$v.effect;intensity=$v.intensity;customArtwork=($null -ne $live.desired.assetId);widgetCount=(@($live.desired.widgets.cards.PSObject.Properties|Where-Object {$_.Value.enabled}).Count+[int]$v.clockVisible+[int]$v.calendarVisible)}}catch{$profiles[$scope]=@{error='WORKLOAD_UNAVAILABLE'}}
 }
 $profiles
}
$power=Invoke-AtmosPowerProbe
$gpuInit=Initialize-AtmosGpu
Add-Type -AssemblyName System.Net.Http
$client=[Net.Http.HttpClient]::new()
$client.Timeout=[TimeSpan]::FromSeconds(4)
$sessionToken=$null
function Get-Health {
 if(-not $script:sessionToken){
  $request=[Net.Http.HttpRequestMessage]::new([Net.Http.HttpMethod]::Get,"$origin/manager")
  $request.Headers.Add('Sec-Fetch-Mode','navigate');$request.Headers.Add('Sec-Fetch-Dest','document');$request.Headers.Add('Sec-Fetch-Site','none')
  try{$response=$client.SendAsync($request).GetAwaiter().GetResult();try{$response.EnsureSuccessStatusCode()|Out-Null;$html=$response.Content.ReadAsStringAsync().GetAwaiter().GetResult();$match=[regex]::Match($html,'window.atmosBridgeSession=("[^"]+")');if(-not $match.Success){throw 'BOOTSTRAP_MISSING'};$script:sessionToken=$match.Groups[1].Value|ConvertFrom-Json}finally{$response.Dispose()}}finally{$request.Dispose()}
 }
 $request=[Net.Http.HttpRequestMessage]::new([Net.Http.HttpMethod]::Get,"$origin/api/health")
 $request.Headers.Add('X-Atmos-Session',$script:sessionToken);$request.Headers.Add('Origin',$origin);$request.Headers.Add('Sec-Fetch-Site','same-origin')
 try{$response=$client.SendAsync($request).GetAwaiter().GetResult();try{if([int]$response.StatusCode -eq 401){$script:sessionToken=$null};$response.EnsureSuccessStatusCode()|Out-Null;($response.Content.ReadAsStringAsync().GetAwaiter().GetResult()|ConvertFrom-Json)}finally{$response.Dispose()}}finally{$request.Dispose()}
}
$eventSources=@()
$eventsAvailable=@{power=$false;session=$false}
try{Add-Type -AssemblyName Microsoft.Win32.SystemEvents -ErrorAction Stop}catch{}
foreach($entry in @(@('PowerModeChanged','AtmosSoakPower','power'),@('SessionSwitch','AtmosSoakSession','session'))){
 try{Register-ObjectEvent -InputObject ([Microsoft.Win32.SystemEvents]) -EventName $entry[0] -SourceIdentifier $entry[1] | Out-Null;$eventSources+=$entry[1];$eventsAvailable[$entry[2]]=$true}catch{}
}
$started=[DateTimeOffset]::UtcNow
$timer=[Diagnostics.Stopwatch]::StartNew()
$metadata=@{schemaVersion=1;startedAt=$started.ToString('o');startedAtTaipei=$started.ToOffset([TimeSpan]::FromHours(8)).ToString('o');plannedDurationSeconds=$DurationSeconds;sampleIntervalSeconds=$SampleSeconds;logicalProcessors=[Environment]::ProcessorCount;collectorPid=$PID;root=@{pid=$rootPid;startedAt=$sampler.RootStartedAt};hostSessionId=$hostSession;expectedTargetIds=@($initial.runtime.targets.targetId);origin=$origin;version=[Diagnostics.FileVersionInfo]::GetVersionInfo($exe).ProductVersion;workload=Load-Workload $initial.runtime;power=$power;gpuInitialization=$gpuInit;eventsAvailable=$eventsAvailable;method='Read-only pinned installed native host + owned Toolhelp descendants + persistent PDH + genuine renderer reports';ownershipLimitations=@('Children born and exited entirely between samples can be missed; CPU churn intervals are marked partial.','Previously proven surviving children retain ownership by PID and creation time after reparenting.','Other applications and collectors are not included as Atmos processes.')}
Save-Json 'metadata.json' $metadata
$status='collecting';$count=0;$lastProgress=-60;$lastElapsed=-1;$rootLost=$false;$firstElapsed=$null
try{
 while($true){
  $iteration=[Diagnostics.Stopwatch]::StartNew()
  $elapsed=$timer.Elapsed.TotalSeconds
  if($null -eq $firstElapsed){$firstElapsed=$elapsed}
  $sample=@{monotonicSeconds=$elapsed;rootAlive=$null;processes=$null;processErrors=@();gpu=$null;power=@{available=$false;watts=$null;joules=$null;reason='No validated energy readings; see startup capability probe'}}
  foreach($event in @(Get-Event -ErrorAction SilentlyContinue|Where-Object {$_.SourceIdentifier -in $eventSources})){
   $value=if($event.SourceIdentifier -eq 'AtmosSoakPower'){$event.SourceEventArgs.Mode.ToString()}else{$event.SourceEventArgs.Reason.ToString()}
   Append-Sample 'events.jsonl' @{at=$event.TimeGenerated.ToUniversalTime().ToString('o');source=$event.SourceIdentifier;value=$value}
   Remove-Event -EventIdentifier $event.EventIdentifier
  }
  try{$owned=$sampler.Sample();$sample.rootAlive=$owned.rootAlive;$sample.processes=@($owned.processes);$sample.processErrors=@($owned.processErrors);$sample.discoveredDescendantCount=$owned.discoveredDescendantCount;if(-not $owned.rootAlive){$rootLost=$true}}
  catch{$sample.processErrors+=('OWNERSHIP_COUNTER_FAILURE:'+ $_.Exception.GetType().Name);try{$sample.rootAlive=((Get-Process -Id $rootPid).StartTime.ToUniversalTime().ToString('o') -eq $sampler.RootStartedAt)}catch{$sample.rootAlive=$false;$rootLost=$true}}
  try{$sample.gpu=Read-AtmosGpu -OwnedPids ([int[]]@($sample.processes|ForEach-Object {$_.pid}))}catch{$sample.gpu=@{available=$false;reason='GPU_READ_FAILED';errorType=$_.Exception.GetType().Name}}
  try{$native=Read-Record (Join-Path $store 'host-status.json');$native|Add-Member -NotePropertyName observedAtMs -NotePropertyValue ([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()) -Force;$sample.native=$native;$sample.workload=Load-Workload $native}catch{$sample.native=$null;$sample.nativeError='NATIVE_STATUS_UNAVAILABLE'}
  try{$h=Get-Health;$sample.health=@{ackRecent=$h.rendererRecentlyAcknowledged;allTargetsAcknowledged=$h.allTargetsAcknowledged;perTarget=@($h.perTarget|Select-Object targetId,scopeId,revision,desiredHash,rendererAcknowledged,nativeAttached,lastAckAt);renderers=@($h.renderers|Select-Object role,revision,recent,capturedAtMs)}}catch{$sample.health=$null;$sample.httpError=$_.Exception.GetType().Name;$script:sessionToken=$null}
  $sample.at=[DateTimeOffset]::UtcNow.ToString('o')
  $sample.collectionDurationMs=$iteration.Elapsed.TotalMilliseconds
  Append-Sample 'samples.jsonl' $sample
  $count++;$lastElapsed=$elapsed
  $progress=@{status='collecting';samples=$count;elapsedSeconds=[Math]::Round($elapsed,1);plannedSeconds=$DurationSeconds;rootAlive=$sample.rootAlive;ownedProcessCount=@($sample.processes).Count;webviewCount=@($sample.processes|Where-Object group -eq 'webview').Count;ackRecent=$sample.health.ackRecent;gpuAvailable=$sample.gpu.engine.available;gpuBusiestEnginePercent=$sample.gpu.engine.busiestEnginePercent}
  Save-Json 'progress.json' $progress
  if($elapsed-$lastProgress -ge 60){$progress|ConvertTo-Json -Compress;$lastProgress=$elapsed}
  if($rootLost){$status='root-lost';break}
  if(Test-Path -LiteralPath (Join-Path $OutputDirectory 'stop.request')){$status='cancelled-by-request';break}
  if($elapsed-$firstElapsed -ge $DurationSeconds){$status='collected';break}
  $slot=[Math]::Floor(($timer.Elapsed.TotalSeconds-$firstElapsed)/$SampleSeconds)+1
  $next=[Math]::Min($firstElapsed+$DurationSeconds,$firstElapsed+$slot*$SampleSeconds)
  $remaining=$next-$timer.Elapsed.TotalSeconds
  if($remaining -gt 0){Start-Sleep -Milliseconds ([int]($remaining*1000))}
 }
}finally{
 foreach($id in $eventSources){Unregister-Event -SourceIdentifier $id -ErrorAction SilentlyContinue;Get-Event -SourceIdentifier $id -ErrorAction SilentlyContinue|Remove-Event -ErrorAction SilentlyContinue}
 Close-AtmosGpu
 $client.Dispose()
 Save-Json 'complete.json' @{schemaVersion=1;status=$status;completedAt=[DateTimeOffset]::UtcNow.ToString('o');elapsedSeconds=$timer.Elapsed.TotalSeconds;lastSampleMonotonicSeconds=$lastElapsed;samples=$count;plannedDurationSeconds=$DurationSeconds;userSettingsChanged=$false}
}
if($status -ne 'collected'){throw "Soak ended without the planned duration: $status"}
