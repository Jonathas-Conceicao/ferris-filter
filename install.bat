@echo off
REM install.bat - Copies universal.filter into the PoE2 config folder so it takes effect in-game.
setlocal

set "SRC=%~dp0universal.filter"
set "DEST=%USERPROFILE%\OneDrive\Documentos\My Games\Path of Exile 2"

if not exist "%SRC%" (
    echo [ERROR] Source filter not found: "%SRC%"
    exit /b 1
)

if not exist "%DEST%" (
    echo [ERROR] Destination folder not found: "%DEST%"
    echo Make sure Path of Exile 2 has been run at least once.
    exit /b 1
)

echo Copying universal.filter to "%DEST%" ...
copy /Y "%SRC%" "%DEST%\universal.filter" >nul
if errorlevel 1 (
    echo [ERROR] Copy failed.
    exit /b 1
)

echo [OK] Installed universal.filter.
endlocal
