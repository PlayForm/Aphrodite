@echo off
REM aphrodite - minimal one-command local install (Windows, cmd.exe shim)
REM
REM The real install logic lives in install.ps1 (PowerShell) - this file just
REM delegates to it, so there's one source of truth for the Windows install
REM flow instead of two scripts to keep in sync. Prefer calling install.ps1
REM directly if you're already in PowerShell.

setlocal

set "SCRIPT_DIR=%~dp0"

where pwsh >nul 2>nul
if %ERRORLEVEL% EQU 0 (
    pwsh -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%install.ps1" %*
    exit /b %ERRORLEVEL%
)

where powershell >nul 2>nul
if %ERRORLEVEL% EQU 0 (
    powershell -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%install.ps1" %*
    exit /b %ERRORLEVEL%
)

echo ERROR: neither pwsh nor powershell found on PATH.
echo        Windows ships PowerShell by default - if this is missing, install
echo        PowerShell from https://aka.ms/powershell and re-run this script.
exit /b 1
