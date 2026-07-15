@echo off
title Long Decompress Dev Server
setlocal enabledelayedexpansion

cd /d "%~dp0"

echo ======================================
echo  Long Decompress - Dev Server
echo ======================================
echo.
echo Directory: %CD%
echo Time: %date% %time%
echo.

REM === Environment Checks ===
if not exist "package.json" (
    echo [ERROR] package.json not found
    echo Run this script in project root directory
    goto :end
)

where node >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Node.js not found
    echo Download from https://nodejs.org/
    goto :end
)

where npm >nul 2>&1
if errorlevel 1 (
    echo [ERROR] npm not found
    echo Install Node.js from https://nodejs.org/
    goto :end
)

where cargo >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Rust not found
    echo Install from https://www.rust-lang.org/
    goto :end
)

REM === Clean up existing processes ===
echo [INFO] Checking for existing processes...

tasklist /FI "IMAGENAME eq node.exe" 2>nul | find /I /N "node.exe">nul
if not errorlevel 1 (
    echo [WARN] Found running node.exe processes
    echo Attempting to stop them...
    taskkill /F /IM node.exe >nul 2>&1
    timeout /t 2 /nobreak >nul
)

REM === Port cleanup ===
echo [INFO] Checking port 1420...
for /f "tokens=5" %%a in ('netstat -ano ^| findstr ":1420" 2^>nul') do (
    set "pid=%%a"
    if not "!pid!"=="" if not "!pid!"=="0" (
        echo [WARN] Port 1420 occupied by PID !pid!
        echo Terminating process...
        taskkill /F /PID !pid! >nul 2>&1
        if not errorlevel 1 (
            echo [OK] Process !pid! terminated
        ) else (
            echo [WARN] Failed to kill PID !pid!
        )
    )
)

timeout /t 2 /nobreak >nul

netstat -ano | findstr ":1420" >nul 2>&1
if not errorlevel 1 (
    echo [ERROR] Port 1420 still in use after cleanup
    echo Please manually close the application or restart system
    goto :end
)

echo [OK] Port 1420 is ready

REM === Dependencies check ===
if not exist "node_modules" (
    echo [INFO] node_modules not found
    echo Installing dependencies...
    call npm install
    if errorlevel 1 (
        echo [ERROR] npm install failed
        echo Try running: npm cache clean --force
        goto :end
    )
    echo [OK] Dependencies installed
) else (
    echo [OK] Dependencies found
)

REM === Start dev server ===
echo.
echo ======================================
echo  Starting Development Server
echo ======================================
echo.
echo [INFO] First build may take 5-10 minutes
echo [INFO] Press Ctrl+C to stop the server
echo.
echo Logs:
echo.

call npm run tauri dev

if errorlevel 1 (
    echo.
    echo ======================================
    echo [ERROR] Dev server failed
    echo ======================================
    echo.
    echo Common issues:
    echo 1. Port still occupied - run this script again
    echo 2. Rust compilation error - check Rust toolchain
    echo 3. Dependencies issue - delete node_modules and retry
    echo 4. Tauri config error - check src-tauri/tauri.conf.json
    echo.
    echo For detailed logs, check console output above
    goto :end
)

echo.
echo [INFO] Server stopped cleanly
goto :end

:end
echo.
echo ======================================
echo Press any key to exit...
pause >nul
endlocal
