# Pure vendor parser fixtures: no native calls, hardware sampling or settings changes.
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'probe-power.ps1')
function Assert($condition,[string]$message){if(-not $condition){throw $message}}
$checks=[Collections.Generic.List[string]]::new()
$r=ConvertFrom-AtmosNvidiaPowerDraw -Raw '[N/A]'
Assert (-not $r.available -and $null -eq $r.readings[0].watts -and $r.readings[0].state -eq 'not-supported') 'Unsupported is null, never zero.';$checks.Add('unsupported-is-null')
$r=ConvertFrom-AtmosNvidiaPowerDraw -Raw '0.00'
Assert ($r.available -and $r.readings[0].watts -eq 0 -and $r.readings[0].state -eq 'driver-reported') 'Valid numeric zero remains distinguishable.';$checks.Add('valid-zero')
$r=ConvertFrom-AtmosNvidiaPowerDraw -Raw "18.25`r`n[N/A]`r`n"
Assert ($r.available -and -not $r.allBoardsAvailable -and $r.validReadingCount -eq 1 -and $r.readings.Count -eq 2 -and $null -eq $r.watts) 'Mixed support is explicit and never summed as total.';$checks.Add('mixed-support-no-total')
foreach($text in @('NaN','Infinity','-3','20000','21 W','GPU-UUID-PRIVATE','18.5, 100','')) {$r=ConvertFrom-AtmosNvidiaPowerDraw -Raw $text;Assert (-not $r.available) 'Invalid/unexpected columns must not become readings.'}
$checks.Add('invalid-or-unexpected-format-rejected')
$r=ConvertFrom-AtmosNvidiaPowerDraw -Raw ('x'*16385)
Assert (-not $r.available -and $r.invalidCount -gt 0) 'Response cap enforced.';$checks.Add('bounded-response')
$r=ConvertFrom-AtmosNvidiaPowerDraw -Raw ((1..33|ForEach-Object{'10.0'}) -join "`n")
Assert (-not $r.available) 'GPU row cap enforced.';$checks.Add('bounded-row-count')
$r=ConvertFrom-AtmosNvidiaPowerDraw -Raw 'GPU-UUID-PRIVATE'
Assert (-not (($r|ConvertTo-Json -Depth 6) -match 'GPU-UUID-PRIVATE')) 'Raw IDs/output are not returned.';$checks.Add('raw-identifiers-not-returned')
[pscustomobject]@{schemaVersion=1;testType='pure-vendor-power-parser-no-hardware-query';passed=$checks.Count;checks=@($checks)}|ConvertTo-Json -Depth 3 -Compress
