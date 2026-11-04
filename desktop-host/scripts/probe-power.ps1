# One-off readonly capability probe, not a power/energy sampler.
# Dot-source this file, then call Invoke-AtmosPowerProbe once before collection.
function Invoke-AtmosPowerProbe {
    [CmdletBinding()]
    param()
    if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
        return [pscustomobject]@{schemaVersion=1;available=$false;kind='capability-only';reason='Windows native interfaces unavailable';watts=$null;joules=$null}
    }
    try {
        if (-not ('AtmosPowerProbeNative' -as [type])) {
            Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class AtmosPowerProbeNative {
 [StructLayout(LayoutKind.Sequential)] public struct DeviceInterface { public int Size; public Guid Guid; public int Flags; public IntPtr Reserved; }
 [StructLayout(LayoutKind.Sequential)] public struct PowerStatus { public byte AC; public byte BatteryFlag; public byte Percent; public byte Saver; public uint Lifetime; public uint FullLifetime; }
 [DllImport("setupapi.dll",SetLastError=true)] static extern IntPtr SetupDiGetClassDevs(ref Guid g,IntPtr e,IntPtr w,uint f);
 [DllImport("setupapi.dll",SetLastError=true)] static extern bool SetupDiEnumDeviceInterfaces(IntPtr h,IntPtr info,ref Guid g,uint i,ref DeviceInterface data);
 [DllImport("setupapi.dll")] static extern bool SetupDiDestroyDeviceInfoList(IntPtr h);
 [DllImport("kernel32.dll",SetLastError=true)] public static extern bool GetSystemPowerStatus(out PowerStatus s);
 public static int EnergyInterfaces() {
  var g=new Guid("45BD8344-7ED6-49cf-A440-C276C933B053");
  var h=SetupDiGetClassDevs(ref g,IntPtr.Zero,IntPtr.Zero,0x12);
  if(h==new IntPtr(-1))return -Marshal.GetLastWin32Error();
  try { int n=0;for(uint i=0;i<64;i++) {
   var d=new DeviceInterface();d.Size=Marshal.SizeOf(typeof(DeviceInterface));
   if(!SetupDiEnumDeviceInterfaces(h,IntPtr.Zero,ref g,i,ref d)) {
    int e=Marshal.GetLastWin32Error();return e==259?n:-e;
   } n++;
  } return n; } finally { SetupDiDestroyDeviceInfoList(h); }
 }
}
'@ | Out-Null
        }
        $taskStatus=New-Object AtmosPowerProbeNative+PowerStatus
        $taskOk=[AtmosPowerProbeNative]::GetSystemPowerStatus([ref]$taskStatus)
        $taskEmi=[AtmosPowerProbeNative]::EnergyInterfaces()
        $taskBatteryPresent=if(-not $taskOk -or $taskStatus.BatteryFlag -eq 255){$null}else{($taskStatus.BatteryFlag -band 128) -eq 0}
        return [pscustomobject]@{
            schemaVersion=1
            capturedAtUtc=[DateTime]::UtcNow.ToString('o')
            kind='capability-only'
            available=$false
            watts=$null
            joules=$null
            reason='No watt/joule measurement performed or validated. CPU/GPU activity and Windows Power Usage categories are not physical power readings.'
            systemPowerStatus=[pscustomobject]@{
                success=$taskOk
                acLineStatus=if($taskOk){[int]$taskStatus.AC}else{$null}
                batteryPresent=$taskBatteryPresent
                batteryFlag=if($taskOk){[int]$taskStatus.BatteryFlag}else{$null}
                batteryPercent=if($taskOk -and $taskStatus.Percent -le 100){[int]$taskStatus.Percent}else{$null}
                batteryLifetimeSeconds=if($taskOk -and $taskStatus.Lifetime -ne [uint32]::MaxValue){[uint32]$taskStatus.Lifetime}else{$null}
                batteryRateMilliwatts=$null
                batteryRateMeasured=$false
            }
            energyMeterInterface=[pscustomobject]@{
                discoverySucceeded=$taskEmi -ge 0
                presentCount=if($taskEmi -ge 0){$taskEmi}else{$null}
                discoveryTruncated=$taskEmi -eq 64
                nativeError=if($taskEmi -lt 0){-$taskEmi}else{$null}
                metadataAndMeasurementValidated=$false
                note='Interface presence is only a candidate capability; units, hardware scope and two interval measurements must be validated separately.'
            }
            restrictions=[pscustomobject]@{settingsChanged=$false;elevationRequested=$false;driversInstalled=$false;devicePathsReturned=$false;wmiPolled=$false;energyDiagnosticRun=$false}
        }
    } catch {
        return [pscustomobject]@{schemaVersion=1;capturedAtUtc=[DateTime]::UtcNow.ToString('o');kind='capability-only';available=$false;watts=$null;joules=$null;reason='Readonly power capability probe failed';errorType=$_.Exception.GetType().Name}
    }
}

# Pure parser for the fixed one-column `--query-gpu=power.draw` output.
# Never interprets [N/A] as zero and never returns UUID, serial, device path or raw output.
function ConvertFrom-AtmosNvidiaPowerDraw {
    [CmdletBinding()]
    param([AllowNull()][AllowEmptyString()][string]$Raw)
    $taskReadings=[Collections.Generic.List[object]]::new()
    $taskInvalid=0
    if ($null -eq $Raw -or [Text.Encoding]::UTF8.GetByteCount($Raw) -gt 16384) {
        return [pscustomobject]@{schemaVersion=1;available=$false;source='nvidia-smi power.draw';readings=@();invalidCount=1;reason='Missing or oversized vendor response';watts=$null;joules=$null}
    }
    $taskLines=@($Raw -split '\r?\n' | Where-Object { $_.Trim().Length -gt 0 })
    if ($taskLines.Count -eq 0 -or $taskLines.Count -gt 32) {
        return [pscustomobject]@{schemaVersion=1;available=$false;source='nvidia-smi power.draw';readings=@();invalidCount=1;reason='Missing or oversized vendor response';watts=$null;joules=$null}
    }
    foreach($taskLine in $taskLines) {
        $taskValue=$taskLine.Trim()
        $taskWatts=$null
        $taskState='invalid'
        if ($taskValue -match '^(\[N/A\]|N/A|Not Supported|\[Not Supported\])$') { $taskState='not-supported' }
        elseif ($taskValue -match '^\d+(?:\.\d+)?$') {
            $taskNumber=0.0
            if ([double]::TryParse($taskValue,[Globalization.NumberStyles]::AllowDecimalPoint,[Globalization.CultureInfo]::InvariantCulture,[ref]$taskNumber) -and $taskNumber -ge 0 -and $taskNumber -le 10000) { $taskWatts=$taskNumber;$taskState='driver-reported' }
        }
        if ($taskState -eq 'invalid') { $taskInvalid++ }
        $taskReadings.Add([pscustomobject]@{gpuIndex=$taskReadings.Count;available=$null -ne $taskWatts;watts=$taskWatts;state=$taskState})
    }
    $taskValid=@($taskReadings | Where-Object {$_.available}).Count
    return [pscustomobject]@{
        schemaVersion=1;source='nvidia-smi power.draw';available=$taskValid -gt 0;validReadingCount=$taskValid;invalidCount=$taskInvalid
        allBoardsAvailable=$taskValid -eq $taskReadings.Count;readings=@($taskReadings);watts=$null;joules=$null
        scope='Individual driver-reported GPU board readings only; no whole-PC total, Atmos attribution, external-meter calibration or energy integration.'
        reason=if($taskValid -eq 0){'No supported numeric GPU board power reading'}else{'Driver readings available only at the stated board scope'}
    }
}

# Explicit one-off optional check. Not called by Invoke-AtmosPowerProbe or any sample loop.
# Query only power.draw: no power-limit, temperature, clock, driver or GPU configuration queries.
function Invoke-AtmosVendorPowerProbe {
    [CmdletBinding()]
    param()
    $taskExe=Join-Path $env:SystemRoot 'System32\nvidia-smi.exe'
    if (-not (Test-Path -LiteralPath $taskExe -PathType Leaf)) {
        return [pscustomobject]@{schemaVersion=1;available=$false;source='nvidia-smi power.draw';reason='Existing system vendor utility unavailable';watts=$null;joules=$null}
    }
    $taskProcess=[Diagnostics.Process]::new()
    try {
        $taskProcess.StartInfo.FileName=$taskExe
        $taskProcess.StartInfo.Arguments='--query-gpu=power.draw --format=csv,noheader,nounits'
        $taskProcess.StartInfo.UseShellExecute=$false
        $taskProcess.StartInfo.CreateNoWindow=$true
        $taskProcess.StartInfo.RedirectStandardOutput=$true
        $taskProcess.StartInfo.RedirectStandardError=$true
        if(-not $taskProcess.Start()){throw 'Vendor probe start failed'}
        $taskOutput=$taskProcess.StandardOutput.ReadToEndAsync()
        $taskErrorOutput=$taskProcess.StandardError.ReadToEndAsync()
        if(-not $taskProcess.WaitForExit(5000)){$taskProcess.Kill();throw 'Vendor probe timeout'}
        if($taskProcess.ExitCode -ne 0){throw 'Vendor probe failed'}
        $taskRaw=$taskOutput.GetAwaiter().GetResult()
        return ConvertFrom-AtmosNvidiaPowerDraw -Raw $taskRaw
    } catch {
        return [pscustomobject]@{schemaVersion=1;available=$false;source='nvidia-smi power.draw';reason='Readonly vendor probe failed or timed out';watts=$null;joules=$null;errorType=$_.Exception.GetType().Name}
    } finally { $taskProcess.Dispose() }
}
