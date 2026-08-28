# 当前开发状态与跨设备接续审计（2026-08-28）

审计日期：2026-08-28（Asia/Shanghai）

GitHub：`https://github.com/Longyuyeee/long_Decompress.git`

开发分支：`codex/archive-media-roadmap`

原始审计锚点：`5e396c67e0b8aa4f4bd3cfc822a63c2f1b24f1e3`（`feat: complete B-03 image backend execution`）

最新接续状态：B-04 整体、B-05.1 三格式矩阵和 B-05.2 百图/故障边界已完成；下一接续点为 B-05.3 正式安装版全流程。完成证据见 [B05_1_IMAGE_FORMAT_MATRIX_AUDIT.md](B05_1_IMAGE_FORMAT_MATRIX_AUDIT.md)、[B05_2_1_IMAGE_BATCH_AUDIT.md](B05_2_1_IMAGE_BATCH_AUDIT.md) 与 [B05_2_2_IMAGE_FAILURE_BOUNDARIES_AUDIT.md](B05_2_2_IMAGE_FAILURE_BOUNDARIES_AUDIT.md)。

公开版本：`v1.1.14`，标签仍固定在 `cfc58ec9a14dc8ccb3f0e026986786af5693b6cc`；当前开发分支不升版、不更新 Release。

## 1. 审计结论

当前开发分支已完成大节点 A、媒体前置 B-00、图片依赖基线 B-01、图片前端工作区 B-02、图片后端执行/发布事务 B-03 和图片任务事实/UI B-04。B-04.5 开发前分支相对 `origin/master` 为领先 28 个提交、落后 0 个提交，当前工作从 GitHub 同名远端分支顶端继续。

B-04.1 至 B-04.5 已全部完成。B-05.1 已完成 9 样本真实生产矩阵；B-05.2.1 已完成 Windows Release/WebView2 百图批量；B-05.2.2 已完成像素上下界、340 UTF-16 中文长路径、目标冲突、StorageFull 和编码期取消。下一接续点严格为 **B-05.3 正式安装版全流程**；B-05 整体、版本提升和公开发布仍未完成。

## 2. 已完成的实际代码

### 2.1 已发布基线与归档主线

- A-01 至 A-06 已随公开 `v1.1.14` 收口；归档浏览、嵌套、预览、解压/压缩事务和安装态门禁不因图片开发改写。
- B-00 已提供媒体共用任务/历史类型、容量预检、唯一暂存、原子发布和系统回收站边界。
- B-01 固定图片依赖版本、feature、制品大小与 SHA-256；FFmpeg、qpdf 和 Ghostscript 仍在各自冻结边界。

### 2.2 图片前端 B-02

- 图片草稿复用现有 compression store，没有新建旁路媒体 store。
- 支持 JPEG/PNG/WebP 输入检查、GIF/非图片明确拒绝、应用方向后的预览尺寸、全局/单项同源配置和受限本地资产授权。
- 预计大小只标为参考区间，未冒充真实输出。

### 2.3 图片后端 B-03

- `compress_image_file` 已注册为 Tauri 命令，并复用 compression 容量预检、统一取消注册表、输出占用守卫和 `spawn_blocking`。
- `compress_single_image` 支持 JPEG/PNG/WebP 同格式压缩、格式转换和按可见尺寸等比例缩放。
- 输入由魔数和真实解码共同确认；动画拒绝；完整解码上限 1 亿像素，编码文件上限 512 MiB。
- 输出只写目标旁唯一暂存；发布前重新解码并验证真实格式、编码矩阵、方向、可见尺寸、帧数、Alpha，以及配置承诺的 EXIF/ICC。
- 默认“仅在更小时替换”；取消、失败、空间不足、目标存在和发布竞态均不发布半成品。
- 最终同版本 NSIS 对照已固化：基线 7,736,450 B，当前 8,613,866 B，净增 877,416 B（11.3413%）。

## 3. 本次审计发现的真实偏移与阻断项

| 项目 | 实际代码 | 对 B-04 的影响 |
| --- | --- | --- |
| 前端文案滞后 | **B-04.5 已解决**：工作区改为真实执行边界，架构门禁拒绝 B-02/B-03 结果占位回归 | B-05 只验证安装态，不重新加入开发节点文案 |
| 前端执行桥缺失 | **B-04.3 至 B-04.5 已解决**：强类型批量执行器、统一 task store/历史和真实结果 UI 已接通，按钮按 ready/运行状态开放或取消 | B-05 复用该入口做安装态矩阵，不复制编排与历史 |
| 后端结果事实不足 | **B-04.1 已解决**：`ImageCompressionOutcome` 返回真实 `input/output`，仅更小策略返回 `input/candidate` | 后续编排可直接消费后端事实，不得退回浏览器预览尺寸 |
| 历史指标结构不足 | **B-04.1/B-04.4 已解决**：图片双侧事实已由唯一 task store 写入口持久化；严格剥离运行时 `encodedBytes` 嵌套字段 | B-04.5 只能读取这些真实指标，不用预计值覆盖 |
| 图片阶段事件缺失 | **B-04.2 已解决**：图片服务产生真实阶段，命令映射统一 `task-log`；架构门禁禁止图片命令发送合成 `task-progress` | B-04.3 已按批量完成数计算进度；B-04.4/5 不得退回平滑伪百分比 |
| 冲突策略尚未映射 | **B-04.3 已解决**：后端权威处理 rename/skip；replace-if-smaller 保留候选大小策略，既有目标仍安全失败 | B-04.4/5 不得绕过目标规划器或发布事务直接覆盖 |
| 批量和终态未闭环 | **B-04.3 至 B-04.5 已解决**：逐图 task id、文件数进度、统一取消、三终态历史与结果 UI 已完成；真实 SQLite 关闭/重开和 Windows WebView2 三图片执行通过 | B-05 扩展样本量与安装版边界 |
| 回收源文件未开放 | 图片配置没有删除源文件选项，后端始终只读源文件 | 不得在 B-04 顺手增加默认删除；若未来开放，只能在发布成功后调用共享回收站 |

## 4. 换机后的唯一推荐开发顺序

1. **B-04.1 事实契约（已完成）**：Rust 结果、前端类型和历史指标已覆盖输入/输出格式、可见尺寸、编码矩阵、方向、帧数、Alpha；旧历史兼容与严格字段门禁已通过。
2. **B-04.2 阶段事件（已完成）**：图片命令已建立解码、条件缩放、编码、验证、发布日志；编码器没有可信字节信息时不发送 `task-progress`。
3. **B-04.3 安全编排（已完成）**：前端同源设置已映射为后端请求；后端生成确定目标并处理 rename/skip；每张图使用唯一 task id，批量进度按文件终态计算且支持统一取消。
4. **B-04.4 队列与历史（已完成）**：复用 task store，写入 `taskType=compression`、`workloadKind=image` 和后端真实指标；完成/失败/取消已跨 SQLite 关闭/重开保留。
5. **B-04.5 UI 开放（已完成）**：已接入审计批量 composable；真实结果文件、预览、节省字节和历史进入工作区，过期 B-02/B-03 文案已移除，按钮按真实可执行状态开放。
6. **B-05.1 三格式矩阵（已完成）**：JPEG、PNG、WebP 各 3 个冻结真实样本直接调用生产压缩服务，9 个输出重新解码差异为 0。
7. **B-05.2.1 百图批量（已完成）**：100 个真实磁盘输入经可见桌面入口执行，100/100 完成、100 个唯一输出、100 条历史、源哈希变化 0。
8. **B-05.2.2 资源与故障边界（已完成）**：96 MP/100.01 MP 上下界、340 UTF-16 中文长路径、冲突/竞态、StorageFull 注入和编码期取消全部通过；现有产品未提供删除源文件选项，不得把该项冒充已可测功能。
9. **B-05.3（下一步）**：执行正式安装版拖入、配置、对比、执行、历史查看和重新打开输出完整流程。

## 5. 本次审计证据

- B-05.2.2 专项生产门禁最终耗时 2,432 ms、差异 0：96 MP 解码成功，100.01 MP 有效 PNG 明确拒绝；340 UTF-16 中文长路径发布成功；冲突、StorageFull 和编码期取消均未留下错误输出或事务暂存。首轮多余 `mut` 警告和 Pillow 预期大图日志已修正。完整证据见 [B05_2_2_IMAGE_FAILURE_BOUNDARIES_AUDIT.md](B05_2_2_IMAGE_FAILURE_BOUNDARIES_AUDIT.md)。
- B-05.2.1 真实 Windows Release Tauri/WebView2 百图门禁已通过：输入/ready/completed/唯一输出/历史均为 100，JPEG/PNG/WebP 为 34/33/33，源 SHA-256 变化 0，实际耗时 17,059 ms；首次仅因 Selenium 可见文本读取遗漏滚动区摘要而失败，改为收起真实设置面板后从头复跑通过。完整差异见 [B05_2_1_IMAGE_BATCH_AUDIT.md](B05_2_1_IMAGE_BATCH_AUDIT.md)。
- B-05.1 真实生产矩阵已通过：JPEG/PNG/WebP 各 3 个、9 个发布输出、三帧 GIF 明确拒绝且无输出，独立重新解码差异 0；前端 276/276、Rust 318 通过/4 忽略、Clippy 与生产构建通过。完整预期—实际修正见 [B05_1_IMAGE_FORMAT_MATRIX_AUDIT.md](B05_1_IMAGE_FORMAT_MATRIX_AUDIT.md)。
- B-04.5 前端全量 47 个文件、276 项通过；Rust 317 项通过、4 项显式忽略、0 失败；Clippy、生产构建与全部媒体门禁通过。
- 真实 Windows WebView2 中实际点击执行固定 JPEG/WebP/PNG，三项磁盘输出、metadata 字节、后端尺寸/格式、结果预览和三条统一历史均通过；GIF/PDF 拒绝与归档队列隔离继续通过。
- 第一轮真实桌面测试使默认 PNG 请求实际失败，修正为 PNG 按格式能力使用无损优化后同场景通过；完整差异证据见 [B04_5_IMAGE_RESULT_UI_AUDIT.md](B04_5_IMAGE_RESULT_UI_AUDIT.md)。
- 以下 `5e396c6`、23 个提交和旧测试数量是本文件原始跨设备审计快照，用于保留历史基线，不代表当前分支顶端。
- `git fetch` 后本地与 `origin/codex/archive-media-roadmap` 同为 `5e396c67e0b8aa4f4bd3cfc822a63c2f1b24f1e3`；相对 `origin/master` 为领先 23、落后 0。
- `npm ci` 按锁文件恢复 358 个包，审计报告 0 个 npm 漏洞；前端类型检查通过。
- 图片 store 与 CompressionView 聚焦测试 2 个文件、23 项通过。
- Rust 图片服务 9 项通过；任务历史聚焦测试 6 项通过。
- 媒体依赖静态门禁通过（6 个锁定依赖）；图片基线门禁通过（5 个固定输入、877,416 B 最终 NSIS 增量）；媒体架构门禁通过（5 个生产文件）。
- B-03 收口提交已有全量证据：前端 45 个文件、265 项通过；Rust 310 项通过、4 项显式忽略、0 失败；Clippy 零警告；真实依赖、真实图片和同版本 NSIS 对照通过。详见 [B03_2_IMAGE_TRANSFORM_EXECUTION_AUDIT.md](B03_2_IMAGE_TRANSFORM_EXECUTION_AUDIT.md)。

## 6. 跨设备恢复步骤

新电脑首次获取：

```powershell
git clone https://github.com/Longyuyeee/long_Decompress.git
Set-Location long_Decompress\long-compress-assistant
git fetch origin
git switch -c codex/archive-media-roadmap --track origin/codex/archive-media-roadmap
git status --short
git log -3 --oneline
```

已有仓库：

```powershell
Set-Location <仓库根目录>
git fetch origin
git switch codex/archive-media-roadmap
git pull --ff-only origin codex/archive-media-roadmap
Set-Location .\long-compress-assistant
```

应确认分支名为 `codex/archive-media-roadmap`，并从 GitHub 分支顶端继续；不要从 `master`、公开标签 `v1.1.14` 或 B-03 之前的提交另开实现。

基础工具约束：`package.json` 要求 Node `^20.19.0 || >=22.12.0`；本次审计机实际为 Node 25.2.1、npm 11.6.2、Rust/Cargo 1.93.1、Git 2.50.0.windows.1。安装依赖后先运行：

```powershell
npm ci
npm run type-check
npm exec vitest run
npm run test:media-dependencies
npm run test:image-baseline
npm run test:media-architecture
Push-Location src-tauri
cargo test --lib
cargo clippy --all-targets --all-features -- -D warnings
Pop-Location
```

真实图片/依赖门禁需要网络或已缓存制品：

```powershell
npm run test:media-dependencies:real
npm run test:image-baseline:real
npm run test:image-matrix:real
npm run test:image-boundaries:real
```

Windows 11 AppX 上下文菜单部署测试需要管理员 PowerShell 和 LocalMachine 证书信任；普通终端拒绝启动不应被误判为图片功能失败。B-01 至 B-05 全部通过前，不提升版本、不生成公开 Release。

## 7. 工作区与提交边界

本机长期显示 `src-tauri/src/commands/file.rs` 与 `src/testing/desktopE2EBridge.ts` 已修改，但 `git diff` 没有实际内容；它们是 CRLF/索引状态噪声，不属于 B-04，也不应在换机审计或后续功能提交中被批量暂存。提交前始终使用明确文件列表，并执行 `git diff --cached --check`。

完整上游证据：

- [ARCHIVE_WORKSPACE_AND_MEDIA_COMPRESSION_PLAN.md](ARCHIVE_WORKSPACE_AND_MEDIA_COMPRESSION_PLAN.md)
- [B03_IMAGE_ENCODING_TRANSACTION_AUDIT.md](B03_IMAGE_ENCODING_TRANSACTION_AUDIT.md)
- [B03_2_IMAGE_TRANSFORM_EXECUTION_AUDIT.md](B03_2_IMAGE_TRANSFORM_EXECUTION_AUDIT.md)
- [DEVELOPMENT_HANDOFF.md](DEVELOPMENT_HANDOFF.md)
