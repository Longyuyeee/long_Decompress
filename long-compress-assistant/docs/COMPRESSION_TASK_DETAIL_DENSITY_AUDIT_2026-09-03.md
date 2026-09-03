# 压缩任务详情密度审计（2026-09-03）

## 目标与结论

本节点接续 [工作区密度与双栏审计](WORKSPACE_DENSITY_AND_DUAL_PANE_AUDIT_2026-09-03.md) 中唯一未关闭的界面项：压缩任务展开详情在 760×520 窗口下纵向过密。修复保持配置栏与执行栏左右并列，不删除压缩参数，不改变任务、历史、资源预检或输出事务。

审计确认根因是已有的 `CompressionSettingsPanel.compact` 与 `ResourcePreflightCard.compact` 没有在压缩任务抽屉启用，同时页面在 760 px 断点把四项核心参数从两列强制改成单列。结果虽无横向滚动，但设置与预检必须经过过长的纵向滚动才能读取。

现已关闭：组任务与单文件任务都使用紧凑设置、紧凑分析说明和紧凑存储预检；760 px 下格式、强度、文件名、密码仍保持两列；配置栏内边距、控件间距和预检间距同步收紧。全部业务入口仍存在，高级配置继续按需展开。

## 实现与边界审计

- `CompressionAnalysisCard` 新增显式 `compact` 展示：保留“抽样估算、不自动修改设置”的决策信息，完整 2 MiB 说明保留在原生标题中；分析结果、重新分析、取消和采用建议逻辑未改变。
- `CompressionSettingsPanel` 复用既有 `compact` 契约；页面级 760 px 规则不再覆盖为单列。没有隐藏格式、等级、文件名、密码、输出路径、删除源文件、完成校验、分卷或固实压缩选项。
- `ResourcePreflightCard` 复用既有紧凑契约，仍显示状态、目标介质、剩余可用、预计占用、文件系统和预留容量；组任务与单文件任务保持一致。
- 配置栏仍独立纵向滚动，执行栏仍由日志区独立滚动；详情高度继续受 `clamp(22rem, 54vh, 32rem)` 约束，左右列顶边与高度一致。
- 浏览器门禁新增 760×520 上限与可见性断言：紧凑配置总高度不得超过视口加 240 px，存储预检至少 70% 位于窗口内。真实桌面门禁新增紧凑卡语义断言，并继续检查双栏最小宽度、有界高度、日志滚动及各层横向溢出。

## 验证结果

- `npx.cmd vitest run src/components/compression/__tests__/CompressionAnalysisCard.test.ts src/components/tasks/__tests__/ResourcePreflightCard.test.ts`：2 文件、10/10 通过。
- `npm.cmd run type-check`：通过。
- `npx.cmd playwright test e2e/app.spec.ts --project=chromium --grep "keeps compression and decompression details"`：1/1 通过；760×520 初始态与预检滚入态截图均生成。
- 设置 `$env:VITE_DESKTOP_E2E = '1'` 后运行 `npm.cmd run build`：生产前端构建通过。
- `cargo build --release --features custom-protocol,desktop-e2e --manifest-path src-tauri\\Cargo.toml`：隔离 Release 构建通过。
- `npm.cmd run test:e2e:desktop:responsive-layout`：真实 Windows Tauri/WebView2 在 920×620、760×520 下对压缩与解压双栏详情全部通过。

本机当前默认 Node 22.12.0 与 `scripts/test-tauri-desktop.mjs` 顶层使用的 `zstdCompressSync` 不兼容，门禁在启动产品前明确失败；改用仓库既有发布审计采用的 Node 24.14.0 后执行成功。EdgeDriver 同步为与 WebView2 `152.0.4191.53` 精确匹配的版本。这个环境契约偏差不记为产品失败，也不把首次未执行的门禁写成通过；后续应单独修正桌面脚本对 package engine 所声明 Node 版本的兼容性。

可见运行证据位于被忽略的 `test-results/` 与 `test-results/desktop-e2e/`，不提交机器路径或二进制；可移植证据为本审计、自动门禁和测试代码。

## 接续点

本节点完成后，下一步先修正 Node 22.12 桌面门禁启动兼容性并单独审计、提交、推送。随后再从最初路线检查仍未关闭的窄窗/高 DPI 活动态与终态任务视觉边界；没有新的发布授权时不升版、不创建标签或 Release。
