<#
.SYNOPSIS
Read Windows fault/reset events for a private Atmos resource-measurement run.
.DESCRIPTION
Requires an explicit local absolute RunDirectory below LOCALAPPDATA\AtmosWallpaper.
Metadata and samples are mandatory; a collected completion marker is required unless
AllowPartial is supplied. Repository paths, installed application/store paths, and
reparse points (including ancestor directories and input files) are rejected.

No application or OS policy is changed. Raw XML/messages remain in a newly created,
unique event-log-check-* child of the private run. Standard output contains only
the anonymous JSON summary or a fixed error code. No raw error is printed.
.EXAMPLE
& ./desktop-host/scripts/read-soak-events.ps1 -RunDirectory $privateAbsoluteRunPath
.EXAMPLE
& ./desktop-host/scripts/read-soak-events.ps1 -RunDirectory $privateAbsoluteRunPath -AllowPartial
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$RunDirectory,
    [switch]$AllowPartial
)

$ErrorActionPreference = 'Stop'
$utf8 = [Text.UTF8Encoding]::new($false)

function Convert-ExactDateValue($Value) {
    # ConvertFrom-Json versions differ in whether ISO values remain strings.
    # Preserve all seven fractional digits; never round-trip DateTime through text.
    if ($Value -is [DateTimeOffset]) { return $Value }
    if ($Value -is [DateTime]) {
        if ($Value.Kind -eq [DateTimeKind]::Unspecified) { throw 'DATE_ZONE_REQUIRED' }
        return [DateTimeOffset]$Value
    }
    $text = [string]$Value
    if ($text -notmatch '^\d{4}-\d\d-\d\dT.+(?:Z|[+-]\d\d:\d\d)$') { throw 'DATE_ZONE_REQUIRED' }
    return [DateTimeOffset]::Parse($text, [Globalization.CultureInfo]::InvariantCulture)
}

function Convert-FaultPid([string]$Text) {
    try {
        $value = if ($Text -match '^0x[0-9a-fA-F]+$') {
            [Convert]::ToInt32($Text.Substring(2), 16)
        } elseif ($Text -match '^\d+$') {
            [int]::Parse($Text, [Globalization.CultureInfo]::InvariantCulture)
        } else { return $null }
        if ($value -gt 0) { return $value }
    } catch { }
    return $null
}

function Convert-CreationFileTime([string]$Text) {
    try {
        $value = if ($Text -match '^0x[0-9a-fA-F]+$') {
            [Convert]::ToInt64($Text.Substring(2), 16)
        } elseif ($Text -match '^[0-9a-fA-F]+$' -and $Text -match '[a-fA-F]') {
            [Convert]::ToInt64($Text, 16)
        } elseif ($Text -match '^\d+$') {
            [long]::Parse($Text, [Globalization.CultureInfo]::InvariantCulture)
        } elseif ($Text -match '^\d{4}-\d\d-\d\dT') {
            (Convert-ExactDateValue $Text).UtcDateTime.ToFileTimeUtc()
        } else { return $null }
        if ($value -gt 0) { return $value }
    } catch { }
    return $null
}

function Get-OwnedIdentityKey([int]$FaultPid, [long]$CreationTime) {
    if ($FaultPid -le 0 -or $CreationTime -le 0) { throw 'IDENTITY_INVALID' }
    return $FaultPid.ToString([Globalization.CultureInfo]::InvariantCulture) + ':' +
        $CreationTime.ToString([Globalization.CultureInfo]::InvariantCulture)
}

function Get-FirstDataValue($Map, [string[]]$Names) {
    foreach ($name in $Names) {
        if ($Map.ContainsKey($name) -and -not [string]::IsNullOrWhiteSpace([string]$Map[$name])) {
            return [string]$Map[$name]
        }
    }
    return $null
}

function Test-SameOrBelow([string]$Path, [string]$Base) {
    return $Path.Equals($Base, [StringComparison]::OrdinalIgnoreCase) -or
        $Path.StartsWith($Base + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)
}

function Assert-NoReparseAncestors([string]$Path) {
    $current = Get-Item -LiteralPath $Path -Force
    while ($null -ne $current) {
        if (($current.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'LINK_REJECTED' }
        $current = if ($current -is [IO.DirectoryInfo]) { $current.Parent } else { $current.Directory }
    }
}

function Get-InputRecord([string]$Directory, [string]$Name, [long]$MaxBytes) {
    $path = Join-Path $Directory $Name
    $item = Get-Item -LiteralPath $path -Force
    if ($item -isnot [IO.FileInfo] -or $item.Length -gt $MaxBytes) { throw 'INPUT_RECORD_INVALID' }
    Assert-NoReparseAncestors $path
    return $item
}

function Read-JsonRecord([IO.FileInfo]$Item) {
    $text = [IO.File]::ReadAllText($Item.FullName)
    # Newer PowerShell can retain dates as strings explicitly. Older versions are
    # covered by Convert-ExactDateValue's DateTime/DateTimeOffset handling.
    if ((Get-Command ConvertFrom-Json).Parameters.ContainsKey('DateKind')) {
        return ConvertFrom-Json -InputObject $text -DateKind String
    }
    return ConvertFrom-Json -InputObject $text
}

try {
    # Local drive paths only: no relative, drive-relative, UNC, device namespace,
    # alternate-data-stream, or provider paths. Never resolve a supplied link.
    if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT -or
        $RunDirectory -notmatch '^[A-Za-z]:[\\/]' -or $RunDirectory.Substring(2).Contains(':')) {
        throw 'EXPLICIT_LOCAL_ABSOLUTE_PATH_REQUIRED'
    }
    $run = [IO.Path]::GetFullPath($RunDirectory).TrimEnd([char[]]'\/')
    $privateBase = [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'AtmosWallpaper')).TrimEnd([char[]]'\/')
    if ($run.Equals($privateBase, [StringComparison]::OrdinalIgnoreCase) -or
        -not (Test-SameOrBelow $run $privateBase)) { throw 'PRIVATE_RUN_DIRECTORY_REQUIRED' }
    $repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..')).TrimEnd([char[]]'\/')
    if ((Test-SameOrBelow $run $repo) -or (Test-SameOrBelow $repo $run)) { throw 'REPOSITORY_REJECTED' }
    foreach ($relative in @('Atmos Wallpaper', 'io.atmos.wallpaper')) {
        $installed = [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA $relative)).TrimEnd([char[]]'\/')
        if ((Test-SameOrBelow $run $installed) -or (Test-SameOrBelow $installed $run)) { throw 'INSTALLED_DIRECTORY_REJECTED' }
    }
    if ((Get-Item -LiteralPath $run -Force) -isnot [IO.DirectoryInfo]) { throw 'RUN_DIRECTORY_INVALID' }
    Assert-NoReparseAncestors $run
    $metadataFile = Get-InputRecord $run 'metadata.json' 2097152
    $samplesFile = Get-InputRecord $run 'samples.jsonl' 536870912
    $metadata = Read-JsonRecord $metadataFile
    $start = Convert-ExactDateValue $metadata.startedAt
    $completionPath = Join-Path $run 'complete.json'
    $hasCompletion = Test-Path -LiteralPath $completionPath
    $final = $false
    if ($hasCompletion) {
        $complete = Read-JsonRecord (Get-InputRecord $run 'complete.json' 1048576)
        $end = Convert-ExactDateValue $complete.completedAt
        $final = $complete.status -eq 'collected'
        if (-not $final -and -not $AllowPartial) { throw 'PARTIAL_RUN_REQUIRES_OPT_IN' }
    } else {
        if (-not $AllowPartial) { throw 'COMPLETION_REQUIRED' }
        $end = [DateTimeOffset]::UtcNow
    }
    if ($end -lt $start) { throw 'TIME_SCOPE_INVALID' }

    $owned = @{}
    $sampleCount = 0
    foreach ($line in [IO.File]::ReadLines($samplesFile.FullName)) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        if ($line.Length -gt 8388608 -or ++$sampleCount -gt 100000) { throw 'SAMPLE_LIMIT_EXCEEDED' }
        $sample = ConvertFrom-Json -InputObject $line
        foreach ($process in $sample.processes) {
            $created = (Convert-ExactDateValue $process.startedAt).UtcDateTime.ToFileTimeUtc()
            $key = Get-OwnedIdentityKey ([int]$process.pid) $created
            # Only collector-known generic groups can enter anonymous output.
            $group = if ([string]$process.group -in @('host', 'runtime', 'webview', 'other')) { [string]$process.group } else { 'unknown' }
            $owned[$key] = $group
            if ($owned.Count -gt 4096) { throw 'IDENTITY_LIMIT_EXCEEDED' }
        }
    }
    if ($sampleCount -eq 0 -or $owned.Count -eq 0) { throw 'OWNED_IDENTITIES_REQUIRED' }

    # Each invocation gets a new private directory; previous evidence is never overwritten.
    Assert-NoReparseAncestors $run
    $stamp = [DateTimeOffset]::UtcNow.ToString('yyyyMMddTHHmmssfff')
    $rawDir = Join-Path $run ('event-log-check-' + $stamp + '-' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $rawDir | Out-Null
    Assert-NoReparseAncestors $rawDir
    $queries = @(
        @{ tag = 'system-display-4101'; log = 'System'; provider = 'Display'; id = 4101; background = $true },
        @{ tag = 'application-error-1000'; log = 'Application'; provider = 'Application Error'; id = 1000; background = $false },
        @{ tag = 'application-hang-1002'; log = 'Application'; provider = 'Application Hang'; id = 1002; background = $false },
        @{ tag = 'wer-1001'; log = 'Application'; provider = 'Windows Error Reporting'; id = 1001; background = $false }
    )
    $public = @()
    $raw = @()
    foreach ($query in $queries) {
        $available = $true
        $unavailableCode = $null
        $events = @()
        try {
            $events = @(Get-WinEvent -FilterHashtable @{
                LogName = $query.log; ProviderName = $query.provider; Id = $query.id
                StartTime = $start.UtcDateTime; EndTime = $end.UtcDateTime
            } -MaxEvents 10000 -ErrorAction Stop)
        } catch {
            if ($_.FullyQualifiedErrorId -match '^NoMatchingEventsFound(?:,|$)') { $events = @() }
            else {
                $available = $false
                $unavailableCode = 'EVENT_LOG_UNAVAILABLE'
                [IO.File]::WriteAllText((Join-Path $rawDir ($query.tag + '-query-error.txt')), $_.ToString(), $utf8)
            }
        }
        $counts = @{ owned = 0; unmatchedIdentity = 0; identityUnavailable = 0; background = 0 }
        $ownedGroups = @{}
        $timeline = @()
        $missingPid = 0
        $missingCreation = 0
        foreach ($event in $events) {
            $xml = $event.ToXml()
            [xml]$document = $xml
            $data = @{}
            foreach ($node in $document.Event.EventData.Data) {
                $name = [string]$node.Name
                if ($name) { $data[$name] = [string]$node.'#text' }
            }
            $pidText = Get-FirstDataValue $data @('ProcessId', 'ProcessID', 'FaultingProcessId', 'FaultingProcessID', 'AppProcessId', 'ApplicationProcessId')
            $creationText = Get-FirstDataValue $data @('ProcessCreationTime', 'ProcessStartTime', 'FaultingProcessCreationTime', 'AppStartTime', 'StartTime')
            $faultPid = Convert-FaultPid $pidText
            $creation = Convert-CreationFileTime $creationText
            $attribution = 'unknown-identity-unavailable'
            $group = $null
            if ($query.background) {
                $attribution = 'background-unattributed'
                $counts.background++
            } elseif ($null -eq $faultPid -or $null -eq $creation) {
                $counts.identityUnavailable++
                if ($null -eq $faultPid) { $missingPid++ }
                if ($null -eq $creation) { $missingCreation++ }
            } else {
                $key = Get-OwnedIdentityKey $faultPid $creation
                if ($owned.ContainsKey($key)) {
                    $attribution = 'matched-recorded-owned-lifetime'
                    $counts.owned++
                    $group = $owned[$key]
                    if (-not $ownedGroups.ContainsKey($group)) { $ownedGroups[$group] = 0 }
                    $ownedGroups[$group]++
                } else {
                    $attribution = 'unknown-not-matched-to-recorded-owned-lifetime'
                    $counts.unmatchedIdentity++
                }
            }
            $relative = ($event.TimeCreated.ToUniversalTime() - $start.UtcDateTime).TotalSeconds
            $timeline += @{ eventCode = $query.tag; relativeSeconds = [Math]::Round($relative, 3); attribution = $attribution; ownedGroup = $group }
            $message = $null
            try { $message = $event.Message } catch { $message = 'MESSAGE_RENDER_UNAVAILABLE' }
            $raw += @{ query = $query.tag; recordId = $event.RecordId; timeCreatedUtc = $event.TimeCreated.ToUniversalTime().ToString('o'); xml = $xml; message = $message; attribution = $attribution }
            [IO.File]::WriteAllText((Join-Path $rawDir ($query.tag + '-' + $event.RecordId + '.xml')), $xml, $utf8)
        }
        $public += @{
            eventCode = $query.tag; available = $available; unavailableCode = $unavailableCode
            queriedCount = if ($available) { $events.Count } else { $null }
            potentiallyTruncated = ($events.Count -ge 10000)
            ownedMatchedCount = if ($available) { $counts.owned } else { $null }
            ownedMatchedGroups = $ownedGroups
            backgroundUnattributedCount = if ($available) { $counts.background } else { $null }
            unknownNotMatchedRecordedIdentityCount = if ($available) { $counts.unmatchedIdentity } else { $null }
            unknownIdentityUnavailableCount = if ($available) { $counts.identityUnavailable } else { $null }
            missingFaultPidCount = if ($available) { $missingPid } else { $null }
            missingCreationTimeCount = if ($available) { $missingCreation } else { $null }
            timeline = $timeline
        }
    }
    $rawMetadata = @{
        schemaVersion = 1; startedAt = $start.ToString('o'); queriedThrough = $end.ToString('o')
        hasCompletionMarker = $hasCompletion; collectedCompletion = $final; events = $raw
    }
    [IO.File]::WriteAllText((Join-Path $rawDir 'raw-events-and-messages.json'), ($rawMetadata | ConvertTo-Json -Depth 12), $utf8)
    $report = @{
        schemaVersion = 1; reportType = 'readonly-event-log-scope-check'; isFinalScope = $final
        scopeElapsedSeconds = [Math]::Round(($end - $start).TotalSeconds, 3); queries = $public
        attributionRule = 'Only faulting EventData PID plus process-creation time matching a recorded owned lifetime is attributed. Event System.Execution PID and app names alone are never used.'
        limitations = @(
            'System Display4101 is machine-level driver context, never attributed to Atmos.',
            'Unmatched identity does not prove unrelated; an unrecorded short-lived child could be absent from samples.',
            'Readable log with no matching events is distinct from an unavailable query.',
            'Missing PID/creation time, including many WER records, remains unknown.',
            'No driver, tracing, service, permission, application or collector changes.'
        )
    }
    [IO.File]::WriteAllText((Join-Path $rawDir 'safe-summary.json'), ($report | ConvertTo-Json -Depth 12), $utf8)
    $report | ConvertTo-Json -Depth 12 -Compress
} catch {
    Write-Output '{"error":"EVENT_SCOPE_CHECK_FAILED"}'
    exit 1
}
