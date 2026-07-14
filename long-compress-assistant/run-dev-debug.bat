@echo off
echo Starting script...
pause

cd /d "%~dp0"
echo Changed to: %CD%
pause

if not exist "package.json" (
    echo ERROR: package.json not found
    pause
    exit /b 1
)
echo Found package.json
pause

where node >nul 2>&1
if %errorlevel% neq 0 (
    echo ERROR: Node.js not found
    pause
    exit /b 1
)
echo Node.js found
node --version
pause

where npm >nul 2>&1
if %errorlevel% neq 0 (
    echo ERROR: npm not found
    pause
    exit /b 1
)
echo npm found
npm --version
pause

where cargo >nul 2>&1
if %errorlevel% neq 0 (
    echo ERROR: Rust not found
    pause
    exit /b 1
)
echo Rust found
rustc --version
pause

echo All checks passed
echo Starting npm run tauri dev...
pause

call npm run tauri dev

echo Command finished with code: %errorlevel%
pause
