@echo off
chcp 65001 >nul
title 胧解压 - 开发模式

:: 切换到脚本所在目录
cd /d "%~dp0"

echo ========================================
echo   胧解压·方便助手 - 开发环境启动
echo ========================================
echo.
echo 当前目录: %CD%
echo.

:: 检查 package.json 是否存在
if not exist "package.json" (
    echo [错误] 未找到 package.json 文件
    echo 请确保在项目根目录运行此脚本
    echo.
    pause
    exit /b 1
)

:: 检查 Node.js
echo [1/4] 检查 Node.js 环境...
where node >nul 2>&1
if %errorlevel% neq 0 (
    echo [错误] 未找到 Node.js
    echo 请先安装 Node.js: https://nodejs.org/
    echo.
    pause
    exit /b 1
)
node --version
echo Node.js 检查通过
echo.

:: 检查 npm
echo [2/4] 检查 npm 环境...
where npm >nul 2>&1
if %errorlevel% neq 0 (
    echo [错误] 未找到 npm
    echo npm 应该随 Node.js 一起安装
    echo.
    pause
    exit /b 1
)
npm --version
echo npm 检查通过
echo.

:: 检查 Rust
echo [3/4] 检查 Rust 环境...
where cargo >nul 2>&1
if %errorlevel% neq 0 (
    echo [错误] 未找到 Rust
    echo 请先安装 Rust: https://www.rust-lang.org/tools/install
    echo.
    pause
    exit /b 1
)
rustc --version
cargo --version
echo Rust 检查通过
echo.

:: 检查 node_modules
if not exist "node_modules" (
    echo [警告] node_modules 目录不存在
    echo 正在安装依赖...
    echo.
    call npm install
    if %errorlevel% neq 0 (
        echo [错误] 依赖安装失败
        echo.
        pause
        exit /b 1
    )
    echo.
)

:: 启动开发服务器
echo [4/4] 启动开发服务器...
echo.
echo ========================================
echo   重要提示：
echo   - 首次启动需要编译 Rust，约 5-10 分钟
echo   - 编译完成后窗口会自动打开
echo   - 请勿关闭此命令行窗口
echo   - 按 Ctrl+C 可停止服务器
echo ========================================
echo.
echo 正在启动...
echo.

call npm run tauri dev

if %errorlevel% neq 0 (
    echo.
    echo ========================================
    echo [错误] 开发服务器启动失败 (错误码: %errorlevel%)
    echo ========================================
    echo.
    echo 请检查上方的错误信息
    echo 常见问题：
    echo   1. Rust 编译错误 - 检查 Rust 代码语法
    echo   2. 端口占用 - 检查 1420 端口是否被占用
    echo   3. 依赖缺失 - 尝试删除 node_modules 后重新运行
    echo.
    pause
    exit /b 1
)

echo.
echo 开发服务器已停止
pause
