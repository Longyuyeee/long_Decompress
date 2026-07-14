@echo off
title Long Decompress Dev

cd /d "%~dp0"

echo ======================================
echo  Long Decompress - Dev Server
echo ======================================
echo.
echo Directory: %CD%
echo.

if not exist "package.json" goto :no_package

where node >nul 2>&1
if errorlevel 1 goto :no_node

where npm >nul 2>&1
if errorlevel 1 goto :no_npm

where cargo >nul 2>&1
if errorlevel 1 goto :no_cargo

if not exist "node_modules" (
    echo Installing dependencies...
    call npm install
    if errorlevel 1 goto :install_failed
)

echo.
echo Starting dev server...
echo First build may take 5-10 minutes
echo Press Ctrl+C to stop
echo.

call npm run tauri dev

if errorlevel 1 goto :dev_failed

echo.
echo Server stopped
goto :end

:no_package
echo ERROR: package.json not found
echo Run this script in project root
goto :end

:no_node
echo ERROR: Node.js not found
echo Download from https://nodejs.org/
goto :end

:no_npm
echo ERROR: npm not found
goto :end

:no_cargo
echo ERROR: Rust not found
echo Download from https://www.rust-lang.org/
goto :end

:install_failed
echo ERROR: npm install failed
goto :end

:dev_failed
echo ERROR: Dev server failed
echo Check the error message above
goto :end

:end
echo.
pause
