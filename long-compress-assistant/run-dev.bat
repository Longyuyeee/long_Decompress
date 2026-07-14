@echo off
chcp 65001 >nul
title 胧解压 - 开发模式

echo ========================================
echo   胧解压·方便助手 - 开发环境启动
echo ========================================
echo.

echo [1/3] 检查 Node.js 环境...
where node >nul 2>&1
if %errorlevel% neq 0 (
    echo [错误] 未找到 Node.js，请先安装 Node.js
    echo 下载地址: https://nodejs.org/
    pause
    exit /b 1
)
node --version
echo.

echo [2/3] 检查 Rust 环境...
where cargo >nul 2>&1
if %errorlevel% neq 0 (
    echo [错误] 未找到 Rust，请先安装 Rust
    echo 下载地址: https://www.rust-lang.org/tools/install
    pause
    exit /b 1
)
rustc --version
echo.

echo [3/3] 启动开发服务器...
echo.
echo ----------------------------------------
echo   提示：
echo   - 首次启动需要编译 Rust 代码，可能需要 5-10 分钟
echo   - 窗口会自动打开，请勿关闭此命令行窗口
echo   - 按 Ctrl+C 可停止开发服务器
echo ----------------------------------------
echo.

npm run tauri dev

if %errorlevel% neq 0 (
    echo.
    echo [错误] 开发服务器启动失败
    echo 请检查错误信息并修复后重试
    pause
    exit /b 1
)

pause
