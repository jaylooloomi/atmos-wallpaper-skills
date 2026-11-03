param(
 [ValidateRange(5,3600)][int]$RunSeconds=600,
 [string]$Node='C:\Program Files\nodejs\node.exe',
 [string]$Python='C:\Python314\python.exe',
 [switch]$SkipBuild
)
$ErrorActionPreference='Stop'
$hostRoot=Split-Path -Parent $PSScriptRoot
$repoRoot=Split-Path -Parent $hostRoot
$runtimeRoot=Join-Path $repoRoot 'implementation-p4_2_5'
$cargo=Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
foreach($file in @($Node,$Python)){if(!(Test-Path -LiteralPath $file -PathType Leaf)){throw "Executable missing: $file"}}
& $Python -c 'import PIL'
if($LASTEXITCODE -ne 0){throw 'Python requires Pillow'}
& $Node (Join-Path $runtimeRoot 'scripts\prepare.mjs') --assets-only
if($LASTEXITCODE -ne 0){throw 'Asset preparation failed; run npm ci --ignore-scripts in implementation-p4_2_5 first'}
if(!$SkipBuild){
 if(!(Test-Path -LiteralPath $cargo)){throw 'Install Rust and Microsoft C++ Build Tools first; see desktop-host/README.md'}
 & $cargo build --locked --manifest-path (Join-Path $hostRoot 'Cargo.toml')
 if($LASTEXITCODE -ne 0){throw 'Rust build failed'}
}
$exe=Join-Path $hostRoot 'target\debug\atmos-desktop-prototype.exe'
if(!(Test-Path -LiteralPath $exe)){throw 'Build the prototype first'}
$localRoot=Join-Path $env:LOCALAPPDATA 'AtmosWallpaper\native-prototype'
$status=Join-Path $localRoot ('runs\'+(Get-Date -Format 'yyyyMMdd-HHmmss')+'\status.json')
Write-Host "Applying Atmos to the primary monitor for up to $RunSeconds seconds. Tray: Exit and restore desktop. Console: type exit + Enter."
Write-Host "Private diagnostics: $status"
& $exe --runtime-root $runtimeRoot --node $Node --python $Python --store (Join-Path $localRoot 'store') --status $status --run-seconds $RunSeconds
if($LASTEXITCODE -ne 0){throw "Prototype exited with code $LASTEXITCODE; see $status"}
