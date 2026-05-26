@echo off
title Workflow Engine

set "PORT=19528"
set "URL=http://127.0.0.1:%PORT%"

echo.
echo   Workflow Engine v7.1.1
echo   Visual Workflow Automation
echo.
echo   Starting server...

start "Workflow Engine Server" "%~dp0workflow-engine.exe"

ping 127.0.0.1 -n 4 >nul
start %URL%

echo.
echo   Tip: Click the address bar icon to install as PWA.
echo.

pause
