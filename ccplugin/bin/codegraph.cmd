@echo off
:: codegraph.cmd — Windows CMD wrapper for codegraph
:: 检测 CPU 架构并执行对应的预编译二进制

setlocal

set "SCRIPT_DIR=%~dp0"

:: 检测架构（PROCESSOR_ARCHITECTURE: AMD64 / ARM64 / x86）
if /I "%PROCESSOR_ARCHITECTURE%"=="AMD64" (
    set "_ARCH=x86_64"
) else if /I "%PROCESSOR_ARCHITECTURE%"=="ARM64" (
    set "_ARCH=aarch64"
) else if /I "%PROCESSOR_ARCHITEW6432%"=="AMD64" (
    :: 32-bit process on 64-bit Windows
    set "_ARCH=x86_64"
) else (
    echo [CodeMap] Unsupported architecture: %PROCESSOR_ARCHITECTURE% >&2
    exit /b 1
)

set "_BIN=%SCRIPT_DIR%codegraph-%_ARCH%-windows.exe"

if not exist "%_BIN%" (
    echo [CodeMap] Binary not found: %_BIN% >&2
    echo [CodeMap] Please download the release binary from: >&2
    echo [CodeMap]   https://github.com/killvxk/CodeMap/releases >&2
    exit /b 1
)

"%_BIN%" %*
