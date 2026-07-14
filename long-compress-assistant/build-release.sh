#!/bin/bash

echo "========================================"
echo "  胧解压·方便助手 - 生产环境打包"
echo "========================================"
echo

echo "[1/4] 检查 Node.js 环境..."
if ! command -v node &> /dev/null; then
    echo "[错误] 未找到 Node.js，请先安装 Node.js"
    echo "下载地址: https://nodejs.org/"
    exit 1
fi
node --version
echo

echo "[2/4] 检查 Rust 环境..."
if ! command -v cargo &> /dev/null; then
    echo "[错误] 未找到 Rust，请先安装 Rust"
    echo "下载地址: https://www.rust-lang.org/tools/install"
    exit 1
fi
rustc --version
echo

echo "[3/4] 安装依赖..."
npm install
if [ $? -ne 0 ]; then
    echo "[错误] 依赖安装失败"
    exit 1
fi
echo

echo "[4/4] 开始打包..."
echo
echo "----------------------------------------"
echo "  提示："
echo "  - 打包过程可能需要 10-20 分钟"
echo "  - 完成后安装包位于 src-tauri/target/release/bundle/"
echo "  - 请耐心等待，不要中断进程"
echo "----------------------------------------"
echo

npm run tauri build

if [ $? -ne 0 ]; then
    echo
    echo "[错误] 打包失败"
    echo "请检查错误信息并修复后重试"
    exit 1
fi

echo
echo "========================================"
echo "  打包完成！"
echo "========================================"
echo
echo "安装包位置："
echo "  src-tauri/target/release/bundle/"
echo

if command -v xdg-open &> /dev/null; then
    xdg-open src-tauri/target/release/bundle
elif command -v open &> /dev/null; then
    open src-tauri/target/release/bundle
fi
