@echo off
setlocal enabledelayedexpansion
title Workflow Engine

set "BIND=127.0.0.1:19529"
set "URL=http://%BIND%"
set "EXE=%~dp0workflow-engine.exe"
set "GUI_EXE=%~dp0workflow-engine-gui.exe"

:: ── colored banner (PowerShell) ──
powershell -NoProfile -Command ^
  "Write-Host ''; ^
   Write-Host '  ═══════════════════════════════════════════' -ForegroundColor Cyan; ^
   Write-Host '    ⚡  Workflow Engine                        ' -ForegroundColor White; ^
   Write-Host '       Visual Workflow Automation            ' -ForegroundColor Gray; ^
   Write-Host '  ═══════════════════════════════════════════' -ForegroundColor Cyan; ^
   Write-Host ''"

:: ── choose mode: GUI (desktop window) or Server (browser) ──
if exist "%GUI_EXE%" (
    echo   [GUI] 启动桌面应用模式...
    start "" "%GUI_EXE%"
    echo   Workflow Engine 已启动（桌面窗口）
    echo   浏览器访问: %URL%
    exit /b 0
)

:: ── fallback: server mode (browser) ──

:: ── check exe ──
if not exist "%EXE%" (
    echo   [ERROR] workflow-engine.exe not found at: %EXE%
    echo.
    pause
    exit /b 1
)

:: ── step 1: start server in background ──
echo   [1/3] Starting server process...
start /MIN "" "%EXE%" >nul 2>&1

:: ── step 2: wait for health endpoint ──
echo   [2/3] Waiting for server to be ready...
set "READY=0"
for /l %%i in (1,1,30) do (
    curl -s -o nul "%URL%/api/health" 2>nul
    if !errorlevel! equ 0 (
        set "READY=1"
        goto :server_ready
    )
    <nul set /p "=."
    ping 127.0.0.1 -n 2 >nul 2>&1
)
:server_ready

if !READY! neq 1 (
    echo.
    echo   [ERROR] Server failed to start within 30 seconds.
    echo   Check if another instance is running or port %BIND% is in use.
    echo.
    pause
    exit /b 1
)

echo  OK

:: ── step 3: query APIs & show summary panel ──
echo   [3/3] Fetching server info...
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$ProgressPreference='SilentlyContinue'; ^
   try { ^
     $health  = Invoke-RestMethod -Uri 'http://127.0.0.1:19529/api/health' -TimeoutSec 3; ^
     $nodes   = Invoke-RestMethod -Uri 'http://127.0.0.1:19529/api/nodes/types' -TimeoutSec 3; ^
     $wfs     = Invoke-RestMethod -Uri 'http://127.0.0.1:19529/api/workflows' -TimeoutSec 3; ^
     $logInfo = Invoke-RestMethod -Uri 'http://127.0.0.1:19529/api/system/log-path' -TimeoutSec 3; ^
     ^
     $dataDir = $logInfo.path -replace '[/\\\\]logs$',''; ^
     ^
     Write-Host ''; ^
     Write-Host '  ┌──────────────────────────────────────────────┐' -ForegroundColor Cyan; ^
     Write-Host ('  │  Version:       ' + $health.version.PadRight(31) + '│') -ForegroundColor White; ^
     Write-Host ('  │  Nodes:         ' + ($nodes.Count.ToString() + ' types').PadRight(31) + '│') -ForegroundColor White; ^
     Write-Host ('  │  Workflows:     ' + ($wfs.Count.ToString() + ' saved').PadRight(31) + '│') -ForegroundColor White; ^
     Write-Host ('  │  Data Dir:      ' + $dataDir.Substring(0,[Math]::Min(31,$dataDir.Length)).PadRight(31) + '│') -ForegroundColor Gray; ^
     Write-Host '  ├──────────────────────────────────────────────┤' -ForegroundColor Cyan; ^
     Write-Host '  │  🌐  http://127.0.0.1:19529                   │' -ForegroundColor Green; ^
     Write-Host '  └──────────────────────────────────────────────┘' -ForegroundColor Cyan; ^
     Write-Host '' ^
   } catch { ^
     Write-Host '  (skipped - server info unavailable)' -ForegroundColor Yellow; ^
     Write-Host '' ^
   }"

:: ── open browser ──
start %URL%

echo   ─────────────────────────────────────────────
echo   Press any key to stop the server...
pause >nul

:: ── kill server on exit ──
taskkill /f /im workflow-engine.exe >nul 2>&1
echo   Server stopped.
