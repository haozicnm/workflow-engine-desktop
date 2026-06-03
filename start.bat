@echo off
title Workflow Engine

set "BIND=127.0.0.1:19529"
set "STATIC_DIR=%~dp0dist"
set "URL=http://%BIND%"

echo.
echo   Workflow Engine v7.8.0
echo   Visual Workflow Automation
echo.
echo   Starting server on %BIND% ...

start "" "%~dp0workflow-engine.exe"

echo   Waiting for server...
REM Windows 10+ built-in curl; fallback to fixed 4s sleep
curl -s -o nul %URL%/api/health 2>nul
if %errorlevel% equ 0 goto ready
echo   Still waiting... (4s)
ping 127.0.0.1 -n 5 >nul

:ready
start %URL%

echo.
echo   Server ready: %URL%
echo   Tip: Click the address bar icon to install as PWA.
echo.
pause
