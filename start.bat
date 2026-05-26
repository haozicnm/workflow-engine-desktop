@echo off
chcp 65001 >nul
title Workflow Engine

set "PORT=19528"
set "BIND=127.0.0.1:%PORT%"
set "STATIC_DIR=%~dp0dist"

echo.
echo   ╔══════════════════════════════════════╗
echo   ║     Workflow Engine v7.1.0          ║
echo   ║     可视化工作流编排引擎              ║
echo   ╚══════════════════════════════════════╝
echo.

REM Check if port is already in use
netstat -ano | findstr ":%PORT% " | findstr "LISTENING" >nul 2>&1
if %errorlevel% equ 0 (
    echo   [ok] 服务已在运行
) else (
    echo   [..] 正在启动服务...
    start "" /B "%~dp0workflow-engine.exe"
    REM Wait for server to be ready
    set "retry=0"
    :wait
    timeout /t 1 /nobreak >nul
    set /a retry+=1
    curl -s --max-time 1 http://127.0.0.1:%PORT%/api/health >nul 2>&1
    if %errorlevel% neq 0 (
        if %retry% lss 10 goto :wait
        echo   [!!] 服务启动超时，请检查 workflow-engine.exe 是否存在
        pause
        exit /b 1
    )
    echo   [ok] 服务已启动
)

echo   [ok] 正在打开浏览器...
echo.
echo   首次使用请点击浏览器地址栏的「安装」按钮
echo   即可将 Workflow Engine 安装到桌面
echo.

start http://127.0.0.1:%PORT%
