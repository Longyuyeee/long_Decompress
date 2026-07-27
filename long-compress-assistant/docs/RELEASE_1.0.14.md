# Long解压 1.0.14

本版本完成 AES 流式加密容器、真实 Windows 桌面生命周期回归和发布治理收口，重点提升大文件加密的内存稳定性，以及任务取消、托盘、退出和应用内更新路径的可验证性。

## 主要更新

- 新增 `AESENC02/TARAES02` 分块加密格式，大文件加密和解密保持有界内存；旧 `AESENC01/TARAES01` 在资源上限内继续只读兼容。
- 加密流程覆盖错误密码、篡改、截断、空文件、多分块、任务取消和磁盘写满清理，失败不会提交不完整输出。
- 真实 Windows Tauri E2E 覆盖启动导航、第二实例右键任务转发，以及真实 ZIP 压缩、解压和逐字节一致性闭环。
- 生命周期门禁覆盖确定性长任务取消、残留输出清理、活动任务退出确认、更新安装阻断、托盘隐藏和第二实例恢复。
- 修复后台运行所需的 Tauri 窗口显示/隐藏权限，并为任务取消按钮补充可访问名称。
- 前端覆盖率达到 75.93% 行、72.68% 分支和 56.07% 函数；关键 Tauri 桥接错误具备直接回归。
- `master` 启用强制 PR、必需 CI、对话解决、禁止强推和禁止删除保护，并建立版本化 Release 验收清单与 Issue 表单。
- 更新 PostCSS、Nanoid 和 Brace Expansion 的非破坏性安全补丁，生产依赖审计无已知漏洞。

## 验证项目

- 前端类型检查、150 项覆盖率测试、生产构建和 Chromium Playwright 测试通过。
- Rust 全目标测试、Clippy、Shell Extension 构建及真实归档矩阵通过。
- Windows 桌面 E2E 隔离二进制构建通过；完整生命周期套件已在交互式 Windows 环境执行通过。
- Windows NSIS 安装包、updater ZIP、签名文件和 `latest.json` 由 Release 工作流生成并校验。
- 普通生产前端包不包含桌面 E2E 测试桥；后端测试夹具仅在 `desktop-e2e` feature 下启用。

## 已知限制

- GitHub 托管 Windows runner 无法创建 WebView2 调试端口，因此 CI 构建并校验桌面 E2E 二进制和脚本，完整 GUI 套件等待交互式 self-hosted Windows runner。
- 当前没有商业代码签名证书，Windows SmartScreen 可能显示警告；请只从本项目 GitHub Releases 下载。
- Windows 11 顶层右键菜单身份包继续暂缓，传统“显示更多选项”右键菜单和一键解压/打包保持可用。
