# Windows 真实桌面 E2E

现有 Playwright 测试在 Chromium 中加载 Vite 前端，适合快速验证路由、布局和无障碍语义。
真实桌面 E2E 则通过 `tauri-driver`、Microsoft EdgeDriver 和 Selenium 启动 Release Tauri
二进制，实际覆盖 Rust 后端初始化、WebView2、自定义协议和桌面窗口。

## 当前覆盖

- Release Tauri 二进制成功启动并进入默认解压工作区。
- 主工作区标题可见，七个侧栏入口存在，解压中心默认选中。
- 真实 ZIP 压缩/解压的终态写入统一历史，重启隔离测试应用后仍可读取；历史页面在 760×520 最小窗口不产生横向溢出。
- 通过第二实例参数发送“一键打包”，执行真实 ZIP 压缩并验证输出非空。
- 通过第二实例参数发送“一键解压”，执行真实解压并逐字节校验源文件与输出文件。
- 真实一键打包/解压任务保留本机卷容量、文件系统、介质和体积估算，详情卡片可见且无横向溢出；可靠不足经正式 IPC 返回 blocked，目标目录不会创建。
- 两项真实 7Z 压缩按显式并发 2 同时执行；写入同一输出目录的两项解压即使全局并发为 2 也保持串行。
- 全局进度面板在实际传输期间显示后端字节计算的速度和剩余时间，不用模拟值通过门禁。
- 普通 ZIP 与 AES ZIP 使用 64 MiB 随机非空载荷验证真实中间/最终字节、可见速度与 ETA、AES 错误密码拒绝；另用 24 MiB + 40 MiB 双文件验证累计单调与精确总量，全部经独立 7-Zip 完整性和解出文件 SHA-256 复核。可用聚焦门禁避开无关高负载场景。
- TAR、TAR.GZ、TAR.BZ2、TAR.XZ、TAR.ZST 各使用 64 MiB 随机非空载荷验证中间/最终真实字节、速率与 ETA，并逐格式执行独立 7-Zip 完整性测试、应用解压和 SHA-256 回环。
- AES 密码双文件 7Z 使用单固实块创建，独立 7-Zip 元数据与完整解出共同确认；非原生密码格式拒绝转换时不创建任务，确认后才生成并验证 `.7z`。
- 在真实 WebView2 中进入设置中心。
- 启动确定性长任务，通过实际取消注册表停止任务，并验证未完成输出被清理。
- 验证活动任务退出判断、三操作确认框，以及更新安装在任务运行时保持禁用。
- 通过 Tauri 窗口状态验证隐藏到托盘，并通过第二实例 IPC 验证窗口恢复。
- 会话失败时保存 `tauri-driver` 日志和桌面截图。

测试构建同时设置 `VITE_DESKTOP_E2E=1` 并启用 `desktop-e2e` Cargo feature，使用独立的单实例名称、IPC socket 和
`LONG_DECOMPRESS_E2E_DATA_DIR` 数据目录。因此本机已运行的正式版不会阻止测试，
测试也不会读取或修改正式版密码库和设置。测试桥在普通生产前端构建中会被移除，
对应后端命令在未启用 feature 时会拒绝执行；该 feature 不进入正式安装包构建。
归档闭环使用系统临时目录中的唯一夹具，成功后自动清理；失败时保留夹具以便排查。

## 本机运行

需要 Windows、Microsoft Edge、Node.js 20 以上和稳定版 Rust。

```powershell
npm ci
$env:VITE_DESKTOP_E2E = "1"
npm run build

Push-Location src-tauri
cargo build --release --features custom-protocol,desktop-e2e
Pop-Location
Remove-Item Env:VITE_DESKTOP_E2E

cargo install tauri-driver --version 2.0.6 --locked
$driver = powershell.exe -NoProfile -ExecutionPolicy Bypass `
  -File scripts/install-edge-driver.ps1 `
  -Destination (Join-Path $env:TEMP "long-compress-edge-driver")
$env:EDGE_DRIVER_PATH = $driver

# 可选：启用虚拟磁盘、FAT 和 NTFS 真实载荷矩阵
npm.cmd run test:tools:qemu-img
npm.cmd run test:tools:wsl-fs

npm run test:e2e:desktop
# 只复验资源预检的真实卷、可见卡片与阻断状态
npm.cmd run test:e2e:desktop:resource-preflight
# 只复验归档并发、同目录串行、遥测、加密固实 7Z 和格式回退确认
npm.cmd run test:e2e:desktop:archive-flow
# 只复验普通/AES ZIP 的真实字节、可见遥测、密码与内容一致性
npm.cmd run test:e2e:desktop:zip-telemetry
# 只复验五种 TAR 系列的真实字节、可见遥测、独立校验与内容回环
npm.cmd run test:e2e:desktop:tar-telemetry
# 只复验真实 ZIP 往返、历史持久化、重启恢复与最小窗口适配
npm.cmd run test:e2e:desktop:history
# 只复验真实保险箱密码自动解压命中，并同步到本地当天使用趋势
npm.cmd run test:e2e:desktop:vault-usage
# 只复验固定官方加密 RAR 的错误密码拒绝、正确密码解压和逐文件 SHA-256
npm.cmd run test:e2e:desktop:encrypted-rar
# 发布前全格式验收会强制检查所有生成器，不允许静默跳过
npm.cmd run test:prepare:full-format
npm.cmd run test:e2e:desktop:full-format
Remove-Item Env:EDGE_DRIVER_PATH
```

正式安装态工作区矩阵必须先准备 EdgeDriver 与固定外部样本；`test-installed-release.ps1`
会在备份、覆盖安装或修改菜单之前检查这些条件，缺失时无损失败：

```powershell
$driver = powershell.exe -NoProfile -ExecutionPolicy Bypass `
  -File scripts/install-edge-driver.ps1 `
  -Destination (Join-Path $env:TEMP "long-compress-edge-driver")
$env:EDGE_DRIVER_PATH = $driver
npm.cmd run test:fixtures:archives

npm.cmd run test:installed-release -- `
  -PreviousInstaller "C:\path\to\previous.exe" `
  -CandidateInstaller "C:\path\to\candidate.exe" `
  -PreviousVersion 1.1.14 `
  -CandidateVersion 1.1.14 `
  -CandidateExecutable "C:\path\to\candidate-app.exe" `
  -AllowExistingInstall `
  -RunArchiveWorkspaceMatrix
```

也可以通过 `TAURI_APP_BINARY`、`TAURI_DRIVER_PATH` 和
`LONG_DECOMPRESS_E2E_DATA_DIR` 覆盖默认路径。

## CI 与运行环境限制

GitHub Actions 的 `Windows desktop E2E build` job 会：

1. 使用隔离前端开关构建前端，并启用隔离 feature 的 Release Tauri 二进制；
2. 校验 Node 和 PowerShell 测试脚本语法；
3. 读取 runner 上的 WebView2 Runtime 完整版本，下载并验证完全匹配的 EdgeDriver。

GitHub 托管 Windows runner 运行在非交互会话中，Tauri/WebView2 无法创建
`DevToolsActivePort`，因此不能把真实 GUI 冒烟伪装成可靠的托管 CI 门禁。真实桌面 E2E
必须在本机或具有交互桌面的 self-hosted Windows runner 上执行。`Windows installer`
当前依赖桌面 E2E 构建 job；接入交互式 runner 后，再让安装器依赖真实桌面执行结果。

## 后续覆盖

- 为文件选择与拖放增加可重复的测试夹具。
- 将完整套件接入具有交互桌面的 self-hosted Windows runner。

参考：[Tauri v1 WebDriver 文档](https://v1.tauri.app/v1/guides/testing/webdriver/introduction/)。
