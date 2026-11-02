param(
 [int]$DurationSeconds=7200,
 [int]$SampleSeconds=10,
 [string]$Origin='http://127.0.0.1:41873',
 [string]$OutputDirectory=(Join-Path $env:LOCALAPPDATA 'AtmosWallpaper\host-validation-2026-11-03-evidence')
)
$ErrorActionPreference='Stop'
if($DurationSeconds -lt 1 -or $SampleSeconds -lt 1){throw 'Positive duration and sample interval required'}
$uri=[uri]$Origin
if($uri.Scheme -ne 'http' -or $uri.Host -ne '127.0.0.1' -or $uri.AbsolutePath -ne '/'){throw 'Explicit loopback origin required'}
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$samples=Join-Path $OutputDirectory 'samples.jsonl'
$eventsFile=Join-Path $OutputDirectory 'events.jsonl'
if(Test-Path -LiteralPath $samples){throw 'Output already contains samples; use a new directory'}
function Write-JsonLine($file,$value){[IO.File]::AppendAllText($file,(ConvertTo-Json -InputObject $value -Depth 8 -Compress)+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))}
$started=[DateTimeOffset]::UtcNow
$metadata=@{startedAt=$started.ToString('o');durationSeconds=$DurationSeconds;sampleSeconds=$SampleSeconds;logicalProcessors=[Environment]::ProcessorCount;origin=$Origin;collector='Windows process counters; no desktop capture or UI automation';powerEvents=$false;sessionEvents=$false}
try{Register-ObjectEvent -InputObject ([Microsoft.Win32.SystemEvents]) -EventName PowerModeChanged -SourceIdentifier AtmosPower | Out-Null;$metadata.powerEvents=$true}catch{$metadata.powerEventError=$_.Exception.Message}
try{Register-ObjectEvent -InputObject ([Microsoft.Win32.SystemEvents]) -EventName SessionSwitch -SourceIdentifier AtmosSession | Out-Null;$metadata.sessionEvents=$true}catch{$metadata.sessionEventError=$_.Exception.Message}
[IO.File]::WriteAllText((Join-Path $OutputDirectory 'metadata.json'),($metadata | ConvertTo-Json -Depth 5),[Text.UTF8Encoding]::new($false))
$headers=$null
try{
 do{
  foreach($event in @(Get-Event | Where-Object {$_.SourceIdentifier -in @('AtmosPower','AtmosSession')})){
   $value=if($event.SourceIdentifier -eq 'AtmosPower'){$event.SourceEventArgs.Mode.ToString()}else{$event.SourceEventArgs.Reason.ToString()}
   Write-JsonLine $eventsFile @{at=$event.TimeGenerated.ToUniversalTime().ToString('o');source=$event.SourceIdentifier;value=$value}
   Remove-Event -EventIdentifier $event.EventIdentifier
  }
  $sample=@{at=[DateTimeOffset]::UtcNow.ToString('o');elapsedSeconds=([DateTimeOffset]::UtcNow-$started).TotalSeconds}
  try{
   if(-not $headers){$html=(Invoke-WebRequest ($Origin+'/manager') -UseBasicParsing -Headers @{'Sec-Fetch-Mode'='navigate'} -TimeoutSec 5).Content;$match=[regex]::Match($html,'window.atmosBridgeSession=("[^"]+")');if(-not $match.Success){throw 'Session bootstrap missing'};$headers=@{'X-Atmos-Session'=($match.Groups[1].Value | ConvertFrom-Json);Origin=$Origin;'Sec-Fetch-Site'='same-origin'}}
   $health=Invoke-RestMethod ($Origin+'/api/health') -Headers $headers -TimeoutSec 5
   $sample.health=@{ackRecent=$health.rendererRecentlyAcknowledged;lastAckAt=$health.lastAckAt;renderers=@($health.renderers | Select-Object rendererId,role,revision,recent,capturedAtMs)}
  }catch{$sample.httpError=$_.Exception.Message;$headers=$null}
  try{
   $all=@(Get-CimInstance Win32_Process)
   $players=@($all | Where-Object {$_.Name -eq 'Lively.Player.WebView2.exe' -and $_.CommandLine -like ('*'+$Origin+'/wallpaper*')})
   $ids=@($players.ProcessId)
   do{$before=$ids.Count;$ids+=@($all | Where-Object {$_.ParentProcessId -in $ids -and $_.ProcessId -notin $ids}).ProcessId;$ids=@($ids | Where-Object {$null -ne $_} | Select-Object -Unique)}while($ids.Count -gt $before)
   $serverIds=@(Get-NetTCPConnection -LocalAddress 127.0.0.1 -LocalPort $uri.Port -State Listen -ErrorAction SilentlyContinue).OwningProcess
   $counters=@()
   foreach($procId in @($ids+$serverIds | Select-Object -Unique)){
    try{$p=Get-Process -Id $procId -ErrorAction Stop;$counters+=@{id=$p.Id;name=$p.ProcessName;group=$(if($procId -in $serverIds){'server'}else{'wallpaper'});startedAt=$p.StartTime.ToUniversalTime().ToString('o');cpuSeconds=$p.TotalProcessorTime.TotalSeconds;workingSetBytes=$p.WorkingSet64;privateBytes=$p.PrivateMemorySize64;handles=$p.HandleCount;threads=$p.Threads.Count}}catch{}
   }
   $sample.processes=$counters
   $sample.playerCount=$players.Count
  }catch{$sample.processError=$_.Exception.Message}
  Write-JsonLine $samples $sample
  $remaining=($started.AddSeconds($DurationSeconds)-[DateTimeOffset]::UtcNow).TotalSeconds
  if($remaining -gt 0){Start-Sleep -Milliseconds ([int]([Math]::Min($SampleSeconds,$remaining)*1000))}
 }while([DateTimeOffset]::UtcNow -lt $started.AddSeconds($DurationSeconds))
 [IO.File]::WriteAllText((Join-Path $OutputDirectory 'complete.json'),(@{completedAt=[DateTimeOffset]::UtcNow.ToString('o');elapsedSeconds=([DateTimeOffset]::UtcNow-$started).TotalSeconds;status='collected'} | ConvertTo-Json),[Text.UTF8Encoding]::new($false))
}finally{foreach($id in @('AtmosPower','AtmosSession')){Unregister-Event -SourceIdentifier $id -ErrorAction SilentlyContinue}}
