@echo off
chcp 936 >nul 2>&1
title Long解压 - Dev Mode

cd /d "%~dp0"

echo ========================================
echo   Long解压 - Development Server
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

echo [1/4] Checking Node.js...
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

echo [2/4] Checking npm...
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

echo [3/4] Checking Rust...
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

if not exist "node_modules" (
    echo [WARN] node_modules not found
    echo Installing dependencies...
    echo.
    call npm install
    if %errorlevel% neq 0 (
        echo [ERROR] npm install failed
        echo.
        pause
        exit /b 1
    )
    echo.
)

echo [4/4] Starting dev server...
echo.
echo ========================================
echo   IMPORTANT:
echo   - First build may take 5-10 minutes
echo   - Window will open when ready
echo   - DO NOT close this terminal
echo   - Press Ctrl+C to stop server
echo ========================================
echo.
echo Starting...
echo.

call npm run tauri dev

if %errorlevel% neq 0 (
    echo.
    echo ========================================
    echo [ERROR] Dev server failed (code: %errorlevel%)
    echo ========================================
    echo.
    echo Common issues:
    echo   1. Rust compile error - check syntax
    echo   2. Port 1420 occupied - check other apps
    echo   3. Missing deps - delete node_modules and retry
    echo.
    pause
    exit /b 1
)

echo.
echo Dev server stopped
pause
