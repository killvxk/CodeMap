@echo off
:: codegraph.cmd — Windows CMD wrapper with multi-level binary discovery
:: 查找优先级: PATH > %USERPROFILE%\.codemap\bin\ > 插件目录 > 开发构建 > 自动下载

setlocal enabledelayedexpansion

set "SCRIPT_DIR=%~dp0"

:: ── 架构检测 ──────────────────────────────────────────────────────────────────

if /I "%PROCESSOR_ARCHITECTURE%"=="AMD64" (
    set "_ARCH=x86_64"
) else if /I "%PROCESSOR_ARCHITECTURE%"=="ARM64" (
    set "_ARCH=aarch64"
) else if /I "%PROCESSOR_ARCHITEW6432%"=="AMD64" (
    set "_ARCH=x86_64"
) else (
    echo [CodeMap] Unsupported architecture: %PROCESSOR_ARCHITECTURE% >&2
    exit /b 1
)

set "_BIN_NAME=codegraph-%_ARCH%-windows.exe"
set "CODEMAP_HOME=%USERPROFILE%\.codemap"
set "CODEMAP_BIN_DIR=%CODEMAP_HOME%\bin"
set "GITHUB_REPO=killvxk/CodeMap"
set "_BIN="

:: ── 1. PATH 中查找 ───────────────────────────────────────────────────────────

where %_BIN_NAME% >nul 2>&1
if %errorlevel% equ 0 (
    for /f "delims=" %%i in ('where %_BIN_NAME%') do (
        set "_BIN=%%i"
        goto :found
    )
)

:: ── 2. ~/.codemap/bin/ ───────────────────────────────────────────────────────

if exist "%CODEMAP_BIN_DIR%\%_BIN_NAME%" (
    set "_BIN=%CODEMAP_BIN_DIR%\%_BIN_NAME%"
    goto :found
)

:: ── 3. 插件目录 (向后兼容) ───────────────────────────────────────────────────

if exist "%SCRIPT_DIR%%_BIN_NAME%" (
    set "_BIN=%SCRIPT_DIR%%_BIN_NAME%"
    goto :found
)

:: ── 4. 开发构建 ──────────────────────────────────────────────────────────────

if exist "%SCRIPT_DIR%..\..\rust-cli\target\release\codegraph.exe" (
    set "_BIN=%SCRIPT_DIR%..\..\rust-cli\target\release\codegraph.exe"
    goto :found
)
if exist "%SCRIPT_DIR%..\..\rust-cli\target\debug\codegraph.exe" (
    set "_BIN=%SCRIPT_DIR%..\..\rust-cli\target\debug\codegraph.exe"
    goto :found
)
if exist "rust-cli\target\release\codegraph.exe" (
    set "_BIN=rust-cli\target\release\codegraph.exe"
    goto :found
)
if exist "rust-cli\target\debug\codegraph.exe" (
    set "_BIN=rust-cli\target\debug\codegraph.exe"
    goto :found
)

:: ── 5. 自动下载 ──────────────────────────────────────────────────────────────

echo [CodeMap] 未找到 codegraph 二进制 (%_BIN_NAME%)，正在从 GitHub Releases 下载... >&2

set "_DOWNLOAD_URL=https://github.com/%GITHUB_REPO%/releases/latest/download/%_BIN_NAME%"

if not exist "%CODEMAP_BIN_DIR%" mkdir "%CODEMAP_BIN_DIR%"
set "_TARGET=%CODEMAP_BIN_DIR%\%_BIN_NAME%"

:: 尝试 curl (Windows 10+ 内置)
where curl >nul 2>&1
if %errorlevel% equ 0 (
    curl -fSL --progress-bar -o "%_TARGET%" "%_DOWNLOAD_URL%"
    if %errorlevel% equ 0 (
        if exist "%_TARGET%" (
            echo [CodeMap] 已下载到 %_TARGET% >&2
            set "_BIN=%_TARGET%"
            goto :found
        )
    )
)

:: 尝试 PowerShell
where powershell >nul 2>&1
if %errorlevel% equ 0 (
    powershell -NoProfile -Command "Invoke-WebRequest -Uri '%_DOWNLOAD_URL%' -OutFile '%_TARGET%'" 2>nul
    if exist "%_TARGET%" (
        echo [CodeMap] 已下载到 %_TARGET% >&2
        set "_BIN=%_TARGET%"
        goto :found
    )
)

echo [CodeMap] 下载失败，请手动下载: %_DOWNLOAD_URL% >&2
echo [CodeMap] 放置到以下任一位置: >&2
echo [CodeMap]   1. %CODEMAP_BIN_DIR%\%_BIN_NAME% >&2
echo [CodeMap]   2. PATH 中的任意目录 >&2
exit /b 1

:found
"%_BIN%" %*
