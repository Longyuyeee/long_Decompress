@echo off
chcp 65001 >nul
title 胧解压 - 生产打包

:: 切换到脚本所在目录
cd /d "%~dp0"

echo ========================================
echo   胧解压·方便助手 - 生产环境打包
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
echo [1/5] 检查 Node.js 环境...
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
echo [2/5] 检查 npm 环境...
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
echo [3/5] 检查 Rust 环境...
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

:: 安装依赖
echo [4/5] 安装/更新依赖...
call npm install
if %errorlevel% neq 0 (
    echo [错误] 依赖安装失败
    echo.
    pause
    exit /b 1
)
echo 依赖安装完成
echo.

:: 开始打包
echo [5/5] 开始打包...
echo.
echo ========================================
echo   重要提示：
echo   - 打包过程约 10-20 分钟
echo   - 首次打包会下载 Rust 依赖
echo   - 完成后会自动打开安装包目录
echo   - 请耐心等待，不要关闭窗口
echo ========================================
echo.
echo 正在打包...
echo.

call npm run tauri build

if %errorlevel% neq 0 (
    echo.
    echo ========================================
    echo [错误] 打包失败 (错误码: %errorlevel%)
    echo ========================================
    echo.
    echo 请检查上方的错误信息
    echo 常见问题：
    echo   1. Rust 编译错误 - 检查 src-tauri/src 中的代码
    echo   2. 磁盘空间不足 - 打包需要约 2GB 空间
    echo   3. 依赖版本冲突 - 尝试删除 node_modules 和 target 后重试
    echo.
    pause
    exit /b 1
)

echo.
echo ========================================
echo   打包成功！
echo ========================================
echo.
echo 安装包位置：
echo   src-tauri\target\release\bundle\msi\
echo   src-tauri\target\release\bundle\nsis\
echo.
echo 正在打开安装包目录...

if exist "src-tauri\target\release\bundle\" (
    explorer "src-tauri\target\release\bundle"
) else (
    echo [警告] 未找到打包目录，但构建已完成
)

echo.
pause
