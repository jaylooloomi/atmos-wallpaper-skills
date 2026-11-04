# Pure fixtures: no PDH initialization, process inspection, settings or desktop changes.
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'probe-gpu.ps1')
$checks=[Collections.Generic.List[string]]::new()
function Assert($condition,[string]$message){if(-not $condition){throw $message}}
function Item([string]$counter,[string]$instance,[double]$value,[uint32]$status=0){[pscustomobject]@{Counter=$counter;Instance=$instance;Value=$value;Status=$status}}
function Frame($items,[bool]$warmup=$false,$errors=@()){[pscustomobject]@{Items=@($items);Warmup=$warmup;Errors=@($errors);CountersOpened=5}}
$prefix='luid_0x00000000_0x00000001_phys_0_eng_'
$items=@((Item engine "pid_101_${prefix}0_engtype_3d" 20),(Item engine "pid_102_${prefix}0_engtype_3d" 30),(Item engine "pid_101_${prefix}1_engtype_copy" 40),(Item engine "pid_1010_${prefix}0_engtype_3d" 80),(Item engine "pid_999_${prefix}0_engtype_3d" 500))
$r=ConvertTo-AtmosGpuReport -Frame (Frame $items) -OwnedPids @(101,102)
Assert ($r.engine.available -and $r.engine.busiestEnginePercent -eq 50 -and $r.engine.engineCount -eq 2) 'Must sum within physical engine, then take maximum; never sum engines or unrelated PIDs.';$checks.Add('physical-engine-max-and-exact-pid-attribution')
$r=ConvertTo-AtmosGpuReport -Frame (Frame @((Item engine "pid_101_${prefix}0_engtype_3d" 0))) -OwnedPids @(101)
Assert ($r.engine.available -and $r.engine.busiestEnginePercent -eq 0) 'A valid measured zero is distinguishable from missing.';$checks.Add('valid-zero-remains-measured-zero')
$r=ConvertTo-AtmosGpuReport -Frame (Frame @()) -OwnedPids @(101)
Assert (-not $r.engine.available -and $null -eq $r.engine.busiestEnginePercent) 'Missing samples must be null.';$checks.Add('missing-is-null-not-zero')
$r=ConvertTo-AtmosGpuReport -Frame (Frame $items $true) -OwnedPids @(101,102)
Assert ($r.warmup -and -not $r.engine.available -and $null -eq $r.engine.busiestEnginePercent) 'Warmup is not a measurement.';$checks.Add('warmup-rejected')
$r=ConvertTo-AtmosGpuReport -Frame (Frame @((Item engine "pid_101_${prefix}0_engtype_3d" 20),(Item engine "pid_102_${prefix}0_engtype_3d" 10 3221228486))) -OwnedPids @(101,102)
Assert (-not $r.engine.available -and $r.engine.invalidInstanceCount -eq 1) 'Invalid owned coverage must not silently undercount.';$checks.Add('invalid-owned-coverage-fails-closed')
$r=ConvertTo-AtmosGpuReport -Frame (Frame @((Item engine "pid_101_${prefix}0_engtype_3d" 70),(Item engine "pid_102_${prefix}0_engtype_3d" 60))) -OwnedPids @(101,102)
Assert (-not $r.engine.available -and $r.engine.outOfRangeCount -gt 0 -and $null -eq $r.engine.busiestEnginePercent) 'Impossible aggregate percentage must not be clamped into success.';$checks.Add('out-of-range-not-clamped')
$r=ConvertTo-AtmosGpuReport -Frame (Frame $items $false @('Read.engine:0xC0000BC6')) -OwnedPids @(101,102)
Assert (-not $r.engine.available) 'Query error must not look successful.';$checks.Add('query-error-prevents-success')
$memory=@((Item dedicated 'pid_101_luid_a_phys_0' 10),(Item dedicated 'pid_102_luid_a_phys_0' 20),(Item shared 'pid_101_luid_a_phys_0' 5),(Item dedicated 'pid_999_luid_a_phys_0' 900),(Item adapterDedicated 'luid_a_phys_0' 9999))
$r=ConvertTo-AtmosGpuReport -Frame (Frame $memory) -OwnedPids @(101,102)
Assert ($r.processMemory.accountedDedicatedBytes -eq 30 -and $r.processMemory.accountedSharedBytes -eq 5 -and $r.systemAdapterMemory.dedicatedBytes -eq 9999) 'Per-process accounting and system adapter totals must stay separate.';$checks.Add('process-memory-and-machine-total-separated')
$r=ConvertTo-AtmosGpuReport -Frame (Frame @((Item shared 'pid_101_luid_a_phys_0' 0))) -OwnedPids @(101)
Assert ($r.processMemory.available -and $null -eq $r.processMemory.accountedDedicatedBytes -and $r.processMemory.accountedSharedBytes -eq 0) 'Missing dedicated gauge must not become zero or hide valid shared zero.';$checks.Add('memory-gauge-missing-versus-valid-zero')
$r=ConvertTo-AtmosGpuReport -Frame (Frame $items) -OwnedPids @()
Assert ($r.requestedProcessCount -eq 0 -and -not $r.engine.available) 'Empty tree must not include unrelated processes.';$checks.Add('empty-tree-safe')
[pscustomobject]@{schemaVersion=1;testType='synthetic-GPU-counter-aggregation-fixtures-no-hardware-measurement';passed=$checks.Count;checks=@($checks)}|ConvertTo-Json -Depth 3 -Compress
