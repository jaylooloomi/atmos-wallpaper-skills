param([Parameter(Mandatory=$true)][string]$MeasurementDirectory)
$ErrorActionPreference='Stop'
$metadata=Get-Content -LiteralPath (Join-Path $MeasurementDirectory 'metadata.json') -Raw | ConvertFrom-Json
$start=([DateTimeOffset]::Parse($metadata.startedAt)).LocalDateTime
$events=@()
$availability=@{}
foreach($query in @(
 @{name='kernel-power';filter=@{LogName='System';ProviderName='Microsoft-Windows-Kernel-Power';Id=@(42,107);StartTime=$start}},
 @{name='power-troubleshooter';filter=@{LogName='System';ProviderName='Microsoft-Windows-Power-Troubleshooter';Id=1;StartTime=$start}},
 @{name='session-audit';filter=@{LogName='Security';Id=@(4800,4801);StartTime=$start}}
)){
 try{
  $records=@(Get-WinEvent -FilterHashtable $query.filter -ErrorAction Stop)
  $availability[$query.name]='records-found'
  foreach($record in $records){
   $value=switch($record.Id){42{'Suspend'}107{'Resume'}1{'Resume'}4800{'SessionLock'}4801{'SessionUnlock'}}
   $events+=@{at=$record.TimeCreated.ToUniversalTime().ToString('o');source=$record.ProviderName;eventId=$record.Id;value=$value}
  }
 }catch{if($_.FullyQualifiedErrorId -like 'NoMatchingEventsFound*'){$availability[$query.name]='no-matching-events'}else{$availability[$query.name]='unavailable';$availability[$query.name+'Error']=$_.Exception.Message}}
}
$result=@{capturedAt=[DateTimeOffset]::UtcNow.ToString('o');availability=$availability;events=$events}
[IO.File]::WriteAllText((Join-Path $MeasurementDirectory 'windows-lifecycle-events.json'),($result | ConvertTo-Json -Depth 6),[Text.UTF8Encoding]::new($false))
$result | ConvertTo-Json -Depth 6
