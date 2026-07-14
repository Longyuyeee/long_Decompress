@echo off
title Long Decompress Build

cd /d "%~dp0"

echo ======================================
echo  Long Decompress - Build Release
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

echo Installing dependencies...
call npm install
if errorlevel 1 goto :install_failed

echo.
echo Building release...
echo This may take 10-20 minutes
echo Please wait...
echo.

call npm run tauri build

if errorlevel 1 goto :build_failed

echo.
echo ======================================
echo  Build Success!
echo ======================================
echo.
echo Installer location:
echo   src-tauri\target\release\bundle\
echo.

if exist "src-tauri\target\release\bundle\" (
    explorer "src-tauri\target\release\bundle"
)

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

:build_failed
echo ERROR: Build failed
echo Check the error message above
goto :end

:end
echo.
pause
