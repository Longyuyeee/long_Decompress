# Windows 真实桌面 E2E

现有 Playwright 测试在 Chromium 中加载 Vite 前端，适合快速验证路由、布局和无障碍语义。
真实桌面 E2E 则通过 `tauri-driver`、Microsoft EdgeDriver 和 Selenium 启动 Release Tauri
二进制，实际覆盖 Rust 后端初始化、WebView2、自定义协议和桌面窗口。

## 当前覆盖

- Release Tauri 二进制成功启动并进入默认解压工作区。
- 主工作区标题可见，五个侧栏入口存在，解压中心默认选中。
- 在真实 WebView2 中进入设置中心。
- 会话失败时保存 `tauri-driver` 日志和桌面截图。

测试构建启用 `desktop-e2e` Cargo feature，使用独立的单实例名称、IPC socket 和
`LONG_DECOMPRESS_E2E_DATA_DIR` 数据目录。因此本机已运行的正式版不会阻止测试，
测试也不会读取或修改正式版密码库和设置。该 feature 不进入正式安装包构建。

## 本机运行

需要 Windows、Microsoft Edge、Node.js 20 以上和稳定版 Rust。

```powershell
npm ci
npm run build

Push-Location src-tauri
cargo build --release --features custom-protocol,desktop-e2e
Pop-Location

cargo install tauri-driver --version 2.0.6 --locked
$driver = powershell.exe -NoProfile -ExecutionPolicy Bypass `
  -File scripts/install-edge-driver.ps1 `
  -Destination (Join-Path $env:TEMP "long-compress-edge-driver")
$env:EDGE_DRIVER_PATH = $driver
npm run test:e2e:desktop
Remove-Item Env:EDGE_DRIVER_PATH
```

也可以通过 `TAURI_APP_BINARY`、`TAURI_DRIVER_PATH` 和
`LONG_DECOMPRESS_E2E_DATA_DIR` 覆盖默认路径。

## CI

GitHub Actions 的 `Windows desktop E2E` job 会：

1. 构建前端和启用隔离 feature 的 Release Tauri 二进制；
2. 安装固定版本的 `tauri-driver`；
3. 读取 runner 上的 Edge 完整版本并下载完全匹配的 EdgeDriver；
4. 使用隔离的 WebView2 用户数据目录，并在无交互桌面的 runner 上启用 headless WebView2；
5. 执行真实桌面冒烟；
6. 失败时上传日志和截图。

`Windows installer` 依赖该 job，因此真实桌面冒烟失败时不会继续生成可发布安装包。

## 后续覆盖

- 通过真实 Tauri 命令执行压缩、解压和取消。
- 验证第二实例右键任务转发。
- 验证关闭到托盘、活动任务退出确认和更新阻断。
- 为文件选择与拖放增加可重复的测试夹具。

参考：[Tauri v1 WebDriver 文档](https://v1.tauri.app/v1/guides/testing/webdriver/introduction/)。
