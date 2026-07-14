@echo off
chcp 65001 >nul
title 胧解压 - 生产打包

cd /d "%~dp0"

echo ========================================
echo   胧解压·方便助手 - 生产环境打包
echo ========================================
echo.

echo [1/4] 检查 Node.js 环境...
where node >nul 2>&1
if %errorlevel% neq 0 (
    echo [错误] 未找到 Node.js，请先安装 Node.js
    echo 下载地址: https://nodejs.org/
    pause
    exit /b 1
)
node --version
echo.

echo [2/4] 检查 Rust 环境...
where cargo >nul 2>&1
if %errorlevel% neq 0 (
    echo [错误] 未找到 Rust，请先安装 Rust
    echo 下载地址: https://www.rust-lang.org/tools/install
    pause
    exit /b 1
)
rustc --version
echo.

echo [3/4] 安装依赖...
call npm install
if %errorlevel% neq 0 (
    echo [错误] 依赖安装失败
    pause
    exit /b 1
)
echo.

echo [4/4] 开始打包...
echo.
echo ----------------------------------------
echo   提示：
echo   - 打包过程可能需要 10-20 分钟
echo   - 完成后安装包位于 src-tauri/target/release/bundle/
echo   - 请耐心等待，不要关闭此窗口
echo ----------------------------------------
echo.

npm run tauri build

if %errorlevel% neq 0 (
    echo.
    echo [错误] 打包失败
    echo 请检查错误信息并修复后重试
    pause
    exit /b 1
)

echo.
echo ========================================
echo   打包完成！
echo ========================================
echo.
echo 安装包位置：
echo   src-tauri\target\release\bundle\msi\
echo   src-tauri\target\release\bundle\nsis\
echo.

explorer src-tauri\target\release\bundle

pause
