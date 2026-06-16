@echo off
REM aphrodite - minimal one-command local install (Windows)
REM Install from a local clone. Expects:
REM   target\release\aphrodite.exe
REM   plugins\aphrodite\
REM   profiles\*\

setlocal enabledelayedexpansion

set "REPO=%~dp0"
set "REPO=%REPO:~0,-1%"
if "%HERMES%"=="" set "HERMES=%USERPROFILE%\.hermes"

set "BINARY=%HERMES%\aphrodite\aphrodite.exe"
set "PLUGIN_SRC=%REPO%\plugins\aphrodite"
set "SKILLS_SRC=%PLUGIN_SRC%\skills"

echo === aphrodite install (Windows) ===
echo   repo:   %REPO%
echo   hermes: %HERMES%

REM --- 1. Binary ---
if not exist "%REPO%\target\release\aphrodite.exe" (
    echo   binary: SKIP - no release build found
    echo           Run: cargo build --release -p aphrodite
) else (
    if not exist "%HERMES%\aphrodite" mkdir "%HERMES%\aphrodite"
    copy /Y "%REPO%\target\release\aphrodite.exe" "%BINARY%" >nul
    echo   binary: %BINARY%
)

REM --- 2. Plugin symlink ---
if not exist "%HERMES%\plugins" mkdir "%HERMES%\plugins"
if exist "%HERMES%\plugins\aphrodite" (
    rmdir /S /Q "%HERMES%\plugins\aphrodite" 2>nul
    del "%HERMES%\plugins\aphrodite" 2>nul
)
mklink /J "%HERMES%\plugins\aphrodite" "%PLUGIN_SRC%" >nul 2>&1 || (
    echo   plugin: WARN - symlink failed, copying instead
    xcopy /E /I "%PLUGIN_SRC%" "%HERMES%\plugins\aphrodite" >nul
)
echo   plugin: %HERMES%\plugins\aphrodite ^<-> %PLUGIN_SRC%

REM --- 3. Skills ---
if exist "%HERMES%\skills\hermes" (
    rmdir /S /Q "%HERMES%\skills\hermes" 2>nul
    del "%HERMES%\skills\hermes" 2>nul
)
mklink /J "%HERMES%\skills\hermes" "%SKILLS_SRC%" >nul 2>&1 || (
    echo   skills: WARN - symlink failed, copying instead
    xcopy /E /I "%SKILLS_SRC%" "%HERMES%\skills\hermes" >nul
)
echo   skills: %HERMES%\skills\hermes ^<-> %SKILLS_SRC%

REM --- 4. Profiles ---
set PROFILE_NAMES=barebone proxy-cache proxy-token compress-off compress-light compress-medium compress-aggressive

for %%p in (%PROFILE_NAMES%) do (
    set "profile=aphrodite-%%p"
    set "src=%REPO%\profiles\!profile!"
    set "dst=%HERMES%\profiles\!profile!"

    if exist "!src!" (
        if exist "!dst!" (
            rmdir /S /Q "!dst!" 2>nul
            del "!dst!" 2>nul
        )
        mklink /J "!dst!" "!src!" >nul 2>&1 || (
            echo   profile: !profile! - WARN: symlink failed
        )
        echo   profile: !profile! ✓
    ) else (
        echo   profile: !profile! - SKIP (no directory)
    )
)

REM --- 5. Enable plugin ---
hermes plugins enable aphrodite 2>nul || echo   plugin: could not auto-enable, run: hermes plugins enable aphrodite

echo.
echo === done ===
echo   Launch: hermes --profile aphrodite-compress-aggressive
echo   Proxy:  hermes --profile aphrodite-proxy-token
