# 当前开发状态与跨设备接续审计（2026-08-28）

审计日期：2026-08-28（Asia/Shanghai）

GitHub：`https://github.com/Longyuyeee/long_Decompress.git`

开发分支：`codex/archive-media-roadmap`

原始审计锚点：`5e396c67e0b8aa4f4bd3cfc822a63c2f1b24f1e3`（`feat: complete B-03 image backend execution`）

最新接续状态：B-04.1 已完成；下一接续点为 B-04.2 阶段事件。完成证据见 [B04_1_IMAGE_FACT_CONTRACT_AUDIT.md](B04_1_IMAGE_FACT_CONTRACT_AUDIT.md)。

公开版本：`v1.1.14`，标签仍固定在 `cfc58ec9a14dc8ccb3f0e026986786af5693b6cc`；当前开发分支不升版、不更新 Release。

## 1. 审计结论

当前开发分支已完成大节点 A、媒体前置 B-00、图片依赖基线 B-01、图片前端工作区 B-02 和图片后端执行/发布事务 B-03。分支相对 `origin/master` 为领先 23 个提交、落后 0 个提交，当前本地与 GitHub 同名远端分支一致。

B-04 已开始，B-04.1 输入/输出双侧事实模型已经完成。下一接续点严格为 **B-04.2 阶段事件**。不能直接把图片“开始压缩”按钮改为可用：实际代码仍缺图片阶段事件、批量编排、前端命令调用和终态历史闭环。B-05 安装版真实矩阵及版本发布仍未开始。

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
| 前端文案滞后 | `ImageCompressionWorkspace.vue` 仍显示“B-02 前端”“B-03 接入后开放”“B-03 实际编码后显示” | B-03 已完成，文案已过期；但在 B-04 事实闭环前不能只改文案并点亮按钮 |
| 前端没有调用后端 | `src` 中没有 `compress_image_file` 调用，按钮仍为静态 `disabled` | 必须新增请求/响应类型、配置映射、目标路径与批量任务编排 |
| 后端结果事实不足 | **B-04.1 已解决**：`ImageCompressionOutcome` 返回真实 `input/output`，仅更小策略返回 `input/candidate` | 后续编排可直接消费后端事实，不得退回浏览器预览尺寸 |
| 历史指标结构不足 | **B-04.1 已解决**：前后端 `MediaMetricsV1` 向后兼容增加可选 `image.input/output`，Rust 严格字段同步 | B-04.4 可写入真实图片历史；旧历史继续无损读取 |
| 图片阶段事件缺失 | 图片服务/命令没有发出 `task-log` 或 `task-progress`；现有事件只由归档 `CompressionService` 发出 | 必须新增解码、缩放、编码、验证、发布阶段事实；不可伪造连续字节进度 |
| 冲突策略尚未映射 | 前端有 rename/skip/replace-if-smaller，后端请求只有 `onlyIfSmaller`，目标已存在时失败关闭 | B-04 编排层必须安全解析 rename/skip；不得绕过发布事务直接覆盖 |
| 批量和终态未闭环 | 图片草稿状态只有 inspecting/ready/rejected，没有 running/completed/failed/cancelled 的实际映射 | 必须接统一 task store，完成、失败、取消三种终态均写入历史并跨重启读取 |
| 回收源文件未开放 | 图片配置没有删除源文件选项，后端始终只读源文件 | 不得在 B-04 顺手增加默认删除；若未来开放，只能在发布成功后调用共享回收站 |

## 4. 换机后的唯一推荐开发顺序

1. **B-04.1 事实契约（已完成）**：Rust 结果、前端类型和历史指标已覆盖输入/输出格式、可见尺寸、编码矩阵、方向、帧数、Alpha；旧历史兼容与严格字段门禁已通过。
2. **B-04.2 阶段事件（下一步）**：为图片命令建立解码、可选缩放、编码、验证、发布阶段日志。进度优先采用批量文件完成数加离散阶段，编码器没有可信字节信息时不得制造平滑百分比。
3. **B-04.3 安全编排**：将前端同源设置映射为后端请求；生成确定目标路径；在编排层实现 rename/skip，replace-if-smaller 继续交给已审计服务；每张图使用唯一 task id 并支持统一取消。
4. **B-04.4 队列与历史**：复用 task store，写入 `taskType=compression`、`workloadKind=image` 和后端真实指标；验证完成/失败/取消跨重启保留。
5. **B-04.5 UI 开放**：真实结果文件、预览、节省字节和历史均可用后，才更新滞后 B-02/B-03 文案并启用按钮；不能用原图或预计值填充结果区。
6. **B-04 审计后进入 B-05**：再做每格式三样本、100 张混合批量、超大像素、中文长路径、冲突、磁盘不足、取消、回收源文件和安装版完整流程。

## 5. 本次审计证据

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
```

Windows 11 AppX 上下文菜单部署测试需要管理员 PowerShell 和 LocalMachine 证书信任；普通终端拒绝启动不应被误判为图片功能失败。B-01 至 B-05 全部通过前，不提升版本、不生成公开 Release。

## 7. 工作区与提交边界

本机长期显示 `src-tauri/src/commands/file.rs` 与 `src/testing/desktopE2EBridge.ts` 已修改，但 `git diff` 没有实际内容；它们是 CRLF/索引状态噪声，不属于 B-04，也不应在换机审计或后续功能提交中被批量暂存。提交前始终使用明确文件列表，并执行 `git diff --cached --check`。

完整上游证据：

- [ARCHIVE_WORKSPACE_AND_MEDIA_COMPRESSION_PLAN.md](ARCHIVE_WORKSPACE_AND_MEDIA_COMPRESSION_PLAN.md)
- [B03_IMAGE_ENCODING_TRANSACTION_AUDIT.md](B03_IMAGE_ENCODING_TRANSACTION_AUDIT.md)
- [B03_2_IMAGE_TRANSFORM_EXECUTION_AUDIT.md](B03_2_IMAGE_TRANSFORM_EXECUTION_AUDIT.md)
- [DEVELOPMENT_HANDOFF.md](DEVELOPMENT_HANDOFF.md)
