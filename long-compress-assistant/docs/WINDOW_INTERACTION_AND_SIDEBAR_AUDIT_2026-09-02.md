# v1.2.4 窗口交互与最小高度侧栏审计

日期：2026-09-02

分支：`codex/window-drag-resize-sidebar-fix`

基线：`master@98512a55efa8c7d21a2f845f2e222e136633b06d`（公开 `v1.2.4` 后续文档提交）

## 范围与路线对齐

本轮仅修复用户在公开 v1.2.4 壳层中复现的三个缺陷：标题栏不能移动窗口、松开鼠标后缩放会在再次移入时恢复、最小高度下左侧导航出现滚动条。没有改变“压缩中心只处理归档、图片/视频/PDF 位于特殊压缩”的既定产品路线，没有扩展媒体能力，也没有提升版本或制作发布资产。

## 实际根因与修复

1. 标题栏组件存在拖动意图，但 Rust `tauri` 依赖未启用 `window-start-dragging`，`tauri.conf.json` 也未允许 `window.startDragging`。现同时开启两项，并由空白标题栏的主鼠标键事件显式调用 `appWindow.startDragging()`；最小化、最大化和关闭按钮被排除。
2. 原缩放状态机在 `isMaximized`、DPI、位置、尺寸等异步读取完成后才建立完整会话，快速松键或在 WebView 外松键可能丢失。现先注册全局结束监听和 pointer capture，再读取原生指标；`pointerup`、`pointercancel`、`blur`、`lostpointercapture` 或任一 `buttons & 1 === 0` 都会幂等清理会话，已排队的动画帧也不能复活它。
3. `screenX/screenY` 是物理屏幕坐标，而 `LogicalSize/LogicalPosition` 接收逻辑像素。现用 `scaleFactor` 换算位移，避免高 DPI 下缩放幅度错误。
4. 真实 920×520 / 250% DPI 的标题栏、品牌、任务槽和版本区占位后，导航可用高度为 283 px。高度不超过 640 px 时，8 个入口改为每项 32 px、间距 2 px、上下内边距各 6 px，合计正好 283 px；内容完整显示，未用单纯隐藏滚动条掩盖溢出。

## 回归保护

- `WindowTitleBar.test.ts`：空白标题栏发起一次原生拖动；三个窗口按钮不发起拖动。
- `MainLayout.test.ts`：正常东南缩放；无左键回入立即结束；原生指标仍在加载时快速松键不能复活；200% DPI 物理位移正确换算。
- `app.spec.ts`：桌面 Chromium 920×520 下 8 个入口全部位于导航边界内，`scrollHeight <= clientHeight`。
- 发布身份门禁：同时要求 Cargo `window-start-dragging` 与 Tauri `window.startDragging` allowlist，避免以后只配置一侧。
- Windows 壳门禁：真实 Release WebView2 收到标题栏拖动请求；250% DPI 下缩放增量为 16×12 逻辑像素；松键回入后尺寸保持；侧栏 `clientHeight=scrollHeight=283` 且 8/8 可见。

## 验证结果

| 门禁 | 结果 |
| --- | --- |
| `npm.cmd run type-check` | 通过 |
| `npm.cmd run test:unit` | 49 文件，286/286 通过 |
| `npm.cmd run test:e2e` | 40 通过、20 条件跳过、0 失败 |
| `npm.cmd run build` | 通过 |
| `npm.cmd run test:release-identity` | v1.2.4 身份与能力约束通过 |
| `cargo check` | 通过 |
| `cargo clippy --all-targets -- -D warnings` | 通过 |
| `cargo test` | 主库 384/384（10 忽略）且全部集成测试通过 |
| `npm.cmd run test:e2e:desktop:shell-polish` | 真实隔离 Tauri/WebView2 expected/actual 差异 0 |

本机原始截图和 `test-results/desktop-e2e/shell-polish-result.json` 属于被忽略的运行产物，不提交 Git；可移植证据是本审计、测试代码和可在新电脑重跑的门禁。当前 EdgeDriver 与 WebView2 均为 `152.0.4191.53`。

## 证据边界与后续

Selenium/WebView 合成指针不会维持 Windows 的真实鼠标键状态，不能把它写成系统窗口已发生物理移动的证据。本轮已证明真实 Release WebView2 接收到标题栏事件、原生命令能力已编译并授权、组件只在正确区域调用；修复版候选发布前仍需用真实鼠标手工拖动一次，并记录结果。其余两项缺陷已有真实桌面几何和状态证据。

本分支推送后先等待 CI。若决定发布修复版，再独立提升版本，并完整执行 NSIS、安装/卸载恢复、真实鼠标体验、合并、标签、GitHub Release、资产回下载与公开更新验收；本审计不提前宣称新版本已经发布。
