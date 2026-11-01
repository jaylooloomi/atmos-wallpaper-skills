@echo off
setlocal
REM Only run after explicit authorization to start the local loopback service.
REM No installation, automatic startup, firewall change or wallpaper change.
cd /d "%~dp0"
where node >nul 2>nul
if errorlevel 1 (echo Node.js is required. Nothing was installed. & exit /b 1)
set "ATMOS_STORE=%LOCALAPPDATA%\AtmosWallpaper\data"
set "ATMOS_PYTHON=C:\Python314\python.exe"
if not exist "%ATMOS_PYTHON%" (echo Configured Python was not found. Stop and update the verified path. & exit /b 1)
if not exist "dist\index.html" (
  node "scripts\prepare.mjs" --assets-only
  if errorlevel 1 exit /b 1
)
echo Starting only this foreground service at http://127.0.0.1:4173/manager
echo Wallpaper display URL: http://127.0.0.1:4173/wallpaper
echo Press Ctrl+C to stop. Lively and the current wallpaper are not changed by this script.
node "src\windows-runtime.mjs" --host 127.0.0.1 --port 4173 --strictPort --start-local-service
exit /b %errorlevel%
