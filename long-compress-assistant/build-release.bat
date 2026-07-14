@echo off
chcp 936 >nul 2>&1
title Long Decompress - Build Release

cd /d "%~dp0"

echo ========================================
echo   Long Decompress - Production Build
echo ========================================
echo.
echo Current Dir: %CD%
echo.

if not exist "package.json" (
    echo [ERROR] package.json not found
    echo Please run this script in project root
    echo.
    pause
    exit /b 1
)

echo [1/5] Checking Node.js...
where node >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Node.js not found
    echo Download: https://nodejs.org/
    echo.
    pause
    exit /b 1
)
node --version
echo Node.js OK
echo.

echo [2/5] Checking npm...
where npm >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] npm not found
    echo.
    pause
    exit /b 1
)
npm --version
echo npm OK
echo.

echo [3/5] Checking Rust...
where cargo >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Rust not found
    echo Download: https://www.rust-lang.org/tools/install
    echo.
    pause
    exit /b 1
)
rustc --version
cargo --version
echo Rust OK
echo.

echo [4/5] Installing dependencies...
call npm install
if %errorlevel% neq 0 (
    echo [ERROR] npm install failed
    echo.
    pause
    exit /b 1
)
echo Dependencies installed
echo.

echo [5/5] Building...
echo.
echo ========================================
echo   IMPORTANT:
echo   - Build may take 10-20 minutes
echo   - First build downloads Rust deps
echo   - Installer will be in bundle folder
echo   - Please wait, do NOT close window
echo ========================================
echo.
echo Building...
echo.

call npm run tauri build

if %errorlevel% neq 0 (
    echo.
    echo ========================================
    echo [ERROR] Build failed (code: %errorlevel%)
    echo ========================================
    echo.
    echo Common issues:
    echo   1. Rust compile error - check src-tauri/src
    echo   2. Disk space low - need ~2GB free
    echo   3. Dependency conflict - delete node_modules and target
    echo.
    pause
    exit /b 1
)

echo.
echo ========================================
echo   Build Success!
echo ========================================
echo.
echo Installer location:
echo   src-tauri\target\release\bundle\msi\
echo   src-tauri\target\release\bundle\nsis\
echo.
echo Opening bundle folder...

if exist "src-tauri\target\release\bundle\" (
    explorer "src-tauri\target\release\bundle"
) else (
    echo [WARN] Bundle folder not found, but build completed
)

echo.
pause
