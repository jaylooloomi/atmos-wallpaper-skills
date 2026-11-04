$ErrorActionPreference='Stop'
if([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT){throw 'Windows required'}
. (Join-Path $PSScriptRoot 'windows-owned-processes.ps1')
$root=Get-Process -Id $PID
$sampler=[AtmosLongTest.OwnedSampler]::new($PID,$root.Path)
$first=$sampler.Sample()
if(-not $first.rootAlive -or -not @($first.processes|Where-Object {$_.pid -eq $PID -and $_.group -eq 'host'}).Count){throw 'Pinned root identity missing'}
$rejected=$false
try{[AtmosLongTest.OwnedSampler]::new($PID,(Join-Path $env:TEMP 'not-the-current-executable.exe'))|Out-Null}catch{$rejected=$true}
if(-not $rejected){throw 'Wrong root executable was accepted'}
$node=(Get-Command node.exe -ErrorAction SilentlyContinue).Source
if(-not $node){$node='C:\Program Files\nodejs\node.exe'}
if(-not (Test-Path -LiteralPath $node)){throw 'Existing Node executable required; no installation performed'}
$start=[Diagnostics.ProcessStartInfo]::new()
$start.FileName=$node
$start.Arguments='-e "setTimeout(()=>{},10000)"'
$start.UseShellExecute=$false;$start.CreateNoWindow=$true
$child=[Diagnostics.Process]::Start($start)
try{
 Start-Sleep -Milliseconds 150
 $active=$sampler.Sample()
 $record=@($active.processes|Where-Object {$_.pid -eq $child.Id})
 if($record.Count -ne 1 -or $record[0].parentPid -ne $PID -or $record[0].group -ne 'runtime'){throw 'Actual owned child missing or wrong parent/group'}
 if([DateTime]::Parse($record[0].startedAt).ToUniversalTime() -lt [DateTime]::Parse($sampler.RootStartedAt).ToUniversalTime()){throw 'Child creation time precedes root'}
 $child.Kill();$child.WaitForExit()
 $after=$sampler.Sample()
 if(@($after.processes|Where-Object {$_.pid -eq $child.Id}).Count){throw 'Exited child retained as a live process'}
 [pscustomobject]@{passed=$true;checks=4;method='Own current native console process plus one short-lived no-window Node child; root identity, ancestry, creation time and exit checked';noUserAppChanges=$true}|ConvertTo-Json -Compress
}finally{if(-not $child.HasExited){$child.Kill();$child.WaitForExit()};$child.Dispose()}
