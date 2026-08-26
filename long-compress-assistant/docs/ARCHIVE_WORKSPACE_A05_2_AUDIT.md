# A-05.2 能力来源与组件边界收口审计

日期：2026-08-26

分支：`codex/archive-media-roadmap`

公开版本：`v1.1.13`（本步骤不升版）

## 1. 开发目标与需求边界

本步骤只收口压缩包浏览中心 2.0 的能力来源和代码边界：格式声明必须来自真实后端能力，浏览页的请求与导航职责需要独立测试。不得借重构新增媒体压缩、归档编辑或未经验证的格式声明；A-01 至 A-05.1 已有交互、安全命令和选择性解压事务必须保持不变。

## 2. 预期、初始实际与修正结果

| 审计项 | 预期 | 初始实际 | 修正后实际 |
| --- | --- | --- | --- |
| 格式真相源 | 浏览、嵌套和预览只由后端实际能力决定 | 浏览页分别硬编码归档、嵌套、图片预览、文本预览和预览格式列表 | 后端结构化返回五类工作区能力，前端只消费该响应 |
| zstd 能力 | 内置引擎报告的格式应进入统一能力 | 7-Zip 26.02 实际输出 `zstd zst tzst`，Rust 解析表遗漏 zstd；前端却自行把 `.zst` 视为归档，形成漂移 | 解析表补齐 zstd；真实 zstd 流装入真实 ZIP 后，右键嵌套入口由后端能力驱动并可用 |
| 请求边界 | 读取、取消、超时和错误分类可独立验证 | 生命周期与页面状态混在主视图 | `useArchiveBrowseRequest` 独立管理请求并有单元测试 |
| 导航边界 | 目录树、面包屑、前后/上级历史和刷新协调可独立验证 | 派生目录和导航状态混在主视图 | `useArchiveWorkspaceNavigation` 与 `ArchiveDirectoryPane` 独立，导航和目录展示职责清晰 |
| 验收稳定性 | 自动化等待用户可观察的最终结果 | 错误密码重试曾等待一闪而过的加载按钮；默认应用缓存文件可能早于 NTFS 安全标记出现 | 内层错误使用递增修订号确认本次结果；默认应用验收等待文件和 `Zone.Identifier` 同时落盘，仍严格验证内容与标记 |

`ArchiveBrowserView.vue` 从 1072 行降至 931 行。仍保留选择、预览、嵌套和发布动作的跨域编排；后续若继续拆分属于可维护性优化，不再阻塞 A-06，且不得改变已通过的桌面语义。

## 3. 真实测试证据

### 3.1 静态与单元回归

- `npm.cmd run type-check`：通过；
- `npm.cmd run test:unit -- --run ...`：40 个测试文件、234 项全部通过；
- `cargo test utils::archive_tools`：3/3 通过；
- `cargo clippy --all-targets --features custom-protocol,desktop-e2e -- -D warnings`：零告警；
- `VITE_DESKTOP_E2E=1 npm.cmd run build`：通过；
- `cargo build --release --features custom-protocol,desktop-e2e`：通过。

### 3.2 Windows Release WebView2 真实门禁

命令：`npm.cmd run test:e2e:desktop:archive-browser`，使用与本机 Edge 151 匹配的 EdgeDriver。

- 现场生成 180,000 条目的真实 TAR，点击取消后 94 ms 恢复；
- 用 Node 原生 zstd 压缩生成真实 zstd 流并装入真实 ZIP，后端报告能力真实驱动右键“进入压缩包”；
- 目录右键、双击、后退、前进、上一级和刷新保留有效选择全部通过；
- 中文八层长路径 ZIP 与加密 7Z 完成精确选择性解压和内容核对；固定加密 RAR 完成密码解锁和内容核对；
- TXT、PNG、PDF 通过 Windows 默认应用打开，缓存字节和 `Zone.Identifier` 均核对；
- 嵌套归档错误密码、正确密码、逐层密码隔离和外层状态恢复通过。

最终输出：`Real Windows Tauri archive-browser gate passed.`

## 4. 收口结论与下一步

A-05.2 已对齐最初目标：消除了格式第二真相源，修正真实 zstd 能力漂移，并为请求、能力和导航边界建立独立测试。没有引入媒体引擎、归档编辑或版本发布，开发范围未偏移。

下一步进入 A-06：必须在正式安装态执行普通/加密 ZIP、7Z、RAR、TAR/TAR.GZ、长路径、混合内容、三层嵌套、负向安全场景、资源管理器经典右键入口和选择性解压综合矩阵。A-06 与发布门禁通过后才允许提升 `1.1.14`、构建安装包并更新 Release。
