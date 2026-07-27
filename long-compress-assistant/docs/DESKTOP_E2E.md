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

## CI 与运行环境限制

GitHub Actions 的 `Windows desktop E2E build` job 会：

1. 构建前端和启用隔离 feature 的 Release Tauri 二进制；
2. 校验 Node 和 PowerShell 测试脚本语法；
3. 读取 runner 上的 WebView2 Runtime 完整版本，下载并验证完全匹配的 EdgeDriver。

GitHub 托管 Windows runner 运行在非交互会话中，Tauri/WebView2 无法创建
`DevToolsActivePort`，因此不能把真实 GUI 冒烟伪装成可靠的托管 CI 门禁。真实桌面 E2E
必须在本机或具有交互桌面的 self-hosted Windows runner 上执行。`Windows installer`
当前依赖桌面 E2E 构建 job；接入交互式 runner 后，再让安装器依赖真实桌面执行结果。

## 后续覆盖

- 通过真实 Tauri 命令执行压缩、解压和取消。
- 验证第二实例右键任务转发。
- 验证关闭到托盘、活动任务退出确认和更新阻断。
- 为文件选择与拖放增加可重复的测试夹具。

参考：[Tauri v1 WebDriver 文档](https://v1.tauri.app/v1/guides/testing/webdriver/introduction/)。
