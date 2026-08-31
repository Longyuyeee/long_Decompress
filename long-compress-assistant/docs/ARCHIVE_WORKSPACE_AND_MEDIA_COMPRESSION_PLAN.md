# Long解压：归档工作区与媒体压缩开发计划

公开基线：`master` / Long解压 `1.1.16`（tag `v1.1.16` 固定在 `a597422`）；当前审计代码基线为 `e3282dd`，PDF D-01 已关闭。历史 B-00 起始审计基线：`codex/archive-media-roadmap` / `19bb4b6`（S-00 总验收通过，比当时 `origin/master` 领先 6 个提交）

编制日期：2026-08-26

适用范围：压缩包浏览中心 2.0、图片压缩、视频压缩、PDF 压缩，以及这些能力与现有任务、历史、安全事务和发布流程的融合。

## 1. 文档目的

本计划把产品设想拆成可以独立开发、独立审计、独立真实验收和独立发布的大节点。每个大节点只有在功能、回归、桌面闭环和发布证据全部完成后，才允许把补丁版本提升一位，例如 `1.1.13 -> 1.1.14`。

本文中的“完成”不等于页面已出现或单元测试通过。一个工作项完成必须同时满足：

1. 用户可见流程完整，没有需要通过开发者工具或手工调用命令才能完成的步骤；
2. 真实文件经过真实引擎处理，输出内容、大小、哈希或媒体属性与预期一致；
3. 失败、取消、磁盘不足、目标冲突和应用重启等异常路径不会发布半成品；
4. 任务进度、日志、历史记录和最终状态互相一致；
5. 安装版 Windows 桌面流程通过，不用浏览器模拟结果代替；
6. 需求对齐表、测试证据、README 和 Release 文档均已更新。

## 2. 当前代码基线与可复用能力

### 2.1 压缩包浏览中心

当前 `src/views/ArchiveBrowserView.vue` 已具备：

- 选择或拖入压缩包；
- 目录树、当前目录文件列表、搜索和类型筛选；
- 聚焦与多选分离、面包屑、后退/前进/上一级、双击/键盘目录导航；
- 条目多选、归档内右键菜单和精确选择性解压；
- ZIP、7Z、RAR、TAR 系列及通用 7-Zip 元数据读取；
- 留空密码时复用密码保险箱；
- ZIP/TAR 系列内 PNG、JPEG、GIF、WebP、BMP 的受限图片预览；
- 输出目录选择和现有解压事务接入。

当前 A-01 至 A-06 已随 `v1.1.14` 发布，B-00.1 至 B-00.6 和图片 B-01 至 B-05.3 已完成并发布为 `v1.1.15`；视频 C-01 至 C-05 已在获批的非 N Windows x64 支持范围内完成并发布为 `v1.1.16`。安全单命令、统一任务/取消、真实进度心跳、完整验证、原子发布、最终指标和历史均已接通；5 种格式、4 个分辨率层级、三预设、10 分钟及 109.52 MiB 大输入的真实产品管线矩阵差异为 0，真实编码取消、无残留、跨完整重启历史和默认应用播放也已通过。C-05.4.1 正式安装生命周期 50/50、安装态视频工作区 20/20；公开 Release、资产回下载和 `v1.1.15 → v1.1.16` 应用内更新也已关闭。Windows N 实机证据仍未取得且 `windowsNRealMachinePassed=false`，产品负责人已明确将 Windows N 排除出 `v1.1.16` 支持范围，因此不阻塞该版本。PDF D-01 已完成运行时准入，D-02.1 已完成产品只读输入分析和十类真实 PDF 对账；下一唯一功能接续点为 D-02.2 两模式配置与风险界面。证据见 [RELEASE_AUDIT_1.1.16.md](RELEASE_AUDIT_1.1.16.md)、[D01_2_2_INSTALLED_QPDF_AND_SIGNED_DELTA_AUDIT.md](D01_2_2_INSTALLED_QPDF_AND_SIGNED_DELTA_AUDIT.md) 与 [D02_1_PDF_READONLY_ANALYSIS_AUDIT.md](D02_1_PDF_READONLY_ANALYSIS_AUDIT.md)。仍需注意：7Z 固实块和 RAR 不伪装成有界内部预览；归档内增删改继续不在大节点 A 范围内。

### 2.2 任务和历史模型

当前 `src/stores/task.ts` 的顶层任务类型只有 `compression` 与 `decompression`；任务已经具有状态、阶段、进度、当前文件、速度、已处理字节、输出字节、预计剩余时间、日志、取消和历史落库能力。

当前数据库 `task_operation_history` 也只允许上述两种顶层类型。这是合理的兼容边界，不应为了三个新 Tab 再复制三套任务系统。

融合策略：

- 归档、图片、视频和 PDF 处理都继续属于 `compression`；
- 增加可选 `workloadKind`：`archive | image | video | pdf`，旧任务缺省为 `archive`；
- 增加可选、版本化的 `metrics`，记录输入大小、输出大小、节省比例，以及媒体特有信息；
- 数据库使用新增可空列或版本化 JSON，不修改旧 `task_type` 检查约束；
- 历史中心按顶层类型继续兼容旧筛选，并增加工作负载子筛选；
- 所有日志继续执行密码与敏感参数脱敏，媒体元数据不得记录用户文件内容。

### 2.3 可复用的安全与发布能力

以下能力必须复用，禁止在媒体模块中重新实现弱化版本：

- 唯一临时输出、最终校验和原子发布；
- 目标存在时的冲突策略；
- 存储容量预检和运行时磁盘不足分类；
- 取消令牌、子进程终止和半成品清理；
- Mark-of-the-Web 传播；
- 任务日志、全局进度、历史任务和托盘状态；
- “完成后移入系统回收站”只能在最终输出校验通过后发生；
- 版本身份脚本、NSIS 安装包、updater ZIP、签名和 `latest.json` 发布门禁。

### 2.4 现有代码落点与改造责任

| 现有文件/模块 | 当前责任 | 本路线中的改造方式 |
| --- | --- | --- |
| `src/views/ArchiveBrowserView.vue` | 浏览页面、目录树、筛选、选择、图片预览 | 节点 A 拆出导航状态、条目动作、右键菜单和面包屑组件，避免继续堆积单文件 |
| `src/composables/useTauriCommands.ts` | Tauri 命令绑定与前端类型 | 增加结构化条目打开/缓存命令；媒体命令迁入独立 `useMediaCommands.ts` |
| `src-tauri/src/services/archive_browser.rs` | ZIP/7Z/RAR/TAR 与通用元数据 | 保持元数据门面，增加嵌套来源描述，不在这里承担临时文件生命周期 |
| `src-tauri/src/services/archive_preview.rs` | 有界 ZIP/TAR 图片读取 | 继续承担可信内部预览；新增文本读取时复用同一预算与魔数原则 |
| `src-tauri/src/services/extraction_transaction.rs` | 解压暂存、冲突、安全路径与提交 | 默认应用打开和嵌套归档只复用安全条目提取边界，不允许写入最终用户目录 |
| `src/views/CompressionView.vue` 与 `src/components/compression/` | 归档压缩队列与详情 | 增加模式容器，归档模式继续使用现有组件；媒体配置和详情使用独立子组件 |
| `src/stores/task.ts` | 统一活动任务、进度、日志、取消和落历史 | 保留两种顶层任务类型，增加可选工作负载与媒体指标 |
| `src/types/taskHistory.ts`、`src/stores/history.ts` | 历史模型、过滤和统计 | 对旧记录向后兼容，增加工作负载子筛选和真实节省指标 |
| `src-tauri/src/commands/task_history.rs` | 历史脱敏、限制和数据库读写 | 扩展可选字段并继续限制文本、来源数量和日志数量 |
| `src-tauri/src/database/migrations.rs` | SQLite 迁移 | 只新增兼容迁移，不修改或伪造旧任务 |
| `scripts/test-tauri-desktop.mjs` | 真实 Tauri/WebView2 桌面门禁 | 为 A/B/C/D 分别增加可单独运行的聚焦模式和固定样本证据 |
| `.github/workflows/ci.yml`、`release.yml` | CI、安装器和 Release | 增加第三方二进制身份/许可检查；继续由标签触发正式 Release |

## 3. 总体产品结构

### 3.0 B-00 媒体压缩前置门禁

B-00 在任何媒体编码页面和引擎之前执行，并以独立审计文档收口。本节点不产生图片、视频或 PDF 压缩能力。

工作项：

1. **B-00.1（2026-08-27 已完成）**：为活动任务和历史记录设计向后兼容的可选 `workloadKind` 与版本化 `metrics`；旧记录缺省解释为 `archive`，数据库迁移可重复执行且不重写旧任务；完整证据见 [B00_TASK_HISTORY_MODEL_AUDIT.md](B00_TASK_HISTORY_MODEL_AUDIT.md)；
2. **B-00.2（2026-08-27 已完成）**：定义并落地媒体任务必须复用的暂存、容量预检、取消、子进程终止、冲突处理、最终校验、原子发布、Mark-of-the-Web、历史脱敏及系统回收站边界；现有非分卷归档压缩已接入公共单文件发布事务，真实文件、Windows 系统回收站和 Release/WebView2 归档闭环通过，证据见 [B00_SHARED_TRANSACTION_AUDIT.md](B00_SHARED_TRANSACTION_AUDIT.md)；
3. **B-00.3（2026-08-27 已完成）**：固定图片、视频、PDF 候选的项目主页、精确版本、许可证、构建来源、SHA-256、支持平台、链接/进程方式、安装体积测量阶段和安全更新责任；静态门禁已进入 CI/Release，真实下载、FFmpeg PGP 和 qpdf 可执行身份已验证；所有运行时仍保持阻断，完整证据见 [B00_MEDIA_DEPENDENCY_AUDIT.md](B00_MEDIA_DEPENDENCY_AUDIT.md)；
4. **B-00.4（2026-08-27 已完成）**：建立不含隐私内容、可再生成且带精确预期属性的真实样本；图片覆盖透明度、EXIF、动图和 9600 万像素，视频覆盖 H.264/H.265、VFR、AAC、旋转矩阵和字幕，PDF 覆盖文本、扫描、透明、AcroForm、有效自签 CMS 和 AES-256 拒绝边界；生成、解析、签名验证和 Poppler 视觉复核证据见 [B00_MEDIA_FIXTURE_BASELINE_AUDIT.md](B00_MEDIA_FIXTURE_BASELINE_AUDIT.md)；
5. **B-00.5（2026-08-27 已完成）**：以可执行契约固定每类任务的真实指标来源。图片只按已完成条目数显示批量进度；视频只消费 FFmpeg progress pipe 的时间戳、临时大小和速度，ETA 至少等待两个有效样本；PDF 只有可验证阶段，不显示伪百分比或 ETA；最终输入/输出字节只取处理前源文件和校验发布后文件的文件系统元数据，估算值不得进入历史；完整证据见 [B00_MEDIA_METRIC_SOURCE_AUDIT.md](B00_MEDIA_METRIC_SOURCE_AUDIT.md)；
6. **B-00.6（2026-08-27 已完成）**：以可执行契约明确 B/C/D 节点的安装态桌面门禁、真实格式矩阵、失败回滚、版本提升和 Release 证据模板；当前 `1.1.14` 正式 NSIS 已完成覆盖安装、生产启动、数据保持、卸载和原版本恢复 44/44 项真实检查，完整证据见 [B00_MEDIA_RELEASE_GATE_AUDIT.md](B00_MEDIA_RELEASE_GATE_AUDIT.md)。

验收目标：

- 迁移测试证明旧任务、旧历史和现有筛选结果不变；
- 架构测试证明媒体任务没有复制第二套队列、历史、事务或删除源文件逻辑；
- 依赖清单在缺少许可证、哈希或来源不一致时会阻断构建/发布；
- 固定样本记录预期属性、首次实际结果、修正和最终实际结果；
- 完成独立 B-00 审计，确认没有媒体引擎、占位成功状态或无法验证的进度进入产品代码。

已解除的前置阻断：[DEVELOPMENT_ALIGNMENT_AUDIT_2026-08-26.md](DEVELOPMENT_ALIGNMENT_AUDIT_2026-08-26.md) 中的 S-00 密码存储语义、模型、文档与仓库卫生已经完成，并通过跨步骤总验收。

剩余阻断条件：

- 任一依赖的许可证、再分发权、固定二进制来源或哈希无法确认；
- 任务/历史迁移需要破坏旧数据，或者媒体任务绕过现有事务式发布和取消机制。

### 3.1 导航结构

左侧主导航保持现状，不新增三个并列主入口，避免功能碎片化。压缩中心内部增加四个工作模式：

`归档压缩 | 图片压缩 | 视频压缩 | PDF 压缩`

四种模式共享页面标题、任务队列、全局进度、历史记录、输出目录、冲突策略和清理策略；配置面板和任务详情按工作负载切换。

### 3.2 后端模块边界

建议增加：

```text
src-tauri/src/
  commands/media.rs
  models/media.rs
  services/media/
    mod.rs
    publish_transaction.rs
    image.rs
    video.rs
    video_probe.rs
    pdf.rs
```

`publish_transaction.rs` 只抽取现有压缩发布流程中可复用的“临时输出 -> 校验 -> 冲突决策 -> 原子发布 -> 可选回收源文件”边界，不移动与归档格式强绑定的逻辑。

前端建议增加：

```text
src/components/media/
  CompressionModeTabs.vue
  MediaTaskTable.vue
  MediaTaskDetails.vue
  ImageCompressionOptions.vue
  VideoCompressionOptions.vue
  PdfCompressionOptions.vue
src/composables/useMediaCommands.ts
src/types/media.ts
```

不得让三个页面各自监听一套进度事件。后端继续发出 `task-log` 和 `task-progress`，按可选媒体字段扩展事件载荷。

## 4. 大节点 A：压缩包浏览中心 2.0

建议目标版本：`1.1.14`。版本号只在本节点全部通过后写入。

### 开发目标

把当前“带目录的选择性解压器”升级为只读、安全、符合 Windows 习惯的归档文件工作区。用户可以像在文件管理器中一样进入目录、双击文件、使用右键菜单、打开嵌套归档，并仍然受到现有解压事务与资源限制保护。

### A-01 交互状态模型（2026-08-26 已完成）

工作内容：

- 把 `focusedEntry`、`selectedEntries`、`activeDirectory` 分离；
- 单击文件行只聚焦；复选框和 Ctrl/Shift 操作负责多选；
- 双击目录进入目录，Backspace/Alt+Left 返回，Enter 执行默认动作；
- 增加面包屑、后退、前进、上一级和刷新；
- 搜索结果保留归档内完整路径，退出搜索后恢复原目录；
- 避免窄窗口产生横向滚动，目录树与内容区只允许竖向滚动。

验收目标：

- 单击文件不会意外取消其他已选文件；
- 双击目录和键盘进入得到相同目录结果；
- 后退/前进栈在搜索、刷新和重新打开压缩包后状态正确；
- 1024×720、1366×768、150% 缩放下无水平滚动条和不可点击区域；
- Vue 单元测试覆盖鼠标、键盘、搜索、选择状态和响应式布局。

### A-02 归档内右键菜单（2026-08-26 已完成交互框架与当前可执行动作）

首期菜单：

- 打开；
- 内部查看器打开；
- 解压到当前输出目录；
- 解压到指定目录；
- 复制文件名；
- 复制归档内路径；
- 显示详细信息；
- 对嵌套归档显示“进入压缩包”。

依赖边界：文件的 Windows 默认应用“打开”只有 A-03 会话缓存通过后才显示；“进入压缩包”只有 A-05 的嵌套预算和归档链通过后才显示。A-02 不提前放置无效菜单项，但已经固定动作发现、右键多选语义和键盘入口，后续节点只向同一动作模型注册已验证能力。

明确不做：新建文件夹、重命名、删除、剪切、粘贴、直接添加到当前归档。不可用动作不展示，不保留无效灰色占位。

验收目标：

- 空白区、单文件、多文件、目录和嵌套归档分别出现正确菜单；
- 右键不会改变原有多选，右键未选项时只聚焦该项；
- 每个菜单动作都有键盘等价操作与自动化测试；
- 右键“解压到”仍进入现有选择性解压事务，不直接向最终目录写文件。

完成证据：见 [ARCHIVE_WORKSPACE_A02_AUDIT.md](ARCHIVE_WORKSPACE_A02_AUDIT.md)。

### A-03 默认应用打开与会话缓存（2026-08-26 已完成）

工作内容：

- 新增 `open_archive_entry` 命令；
- 后端只接受已规范化的归档内相对路径，拒绝绝对路径、`..`、设备路径、重解析点和目录穿越；
- 单条目解压到应用会话缓存，例如 `%LOCALAPPDATA%/LongDecompress/preview-cache/<session>/<id>/`；
- 保留扩展名但使用随机目录隔离同名文件；
- 输出完成后校验文件存在、大小不超过预算，再通过 Windows 默认关联程序打开；
- 互联网来源归档的临时文件继续携带 Mark-of-the-Web；
- EXE、MSI、BAT、CMD、PS1、JS、VBS、LNK、SCR 等可执行内容必须二次确认，默认按钮为取消；
- 应用退出时尽力清理；被外部程序占用的文件记录为延迟清理，下次启动按 TTL 清理；
- 缓存总大小、单文件大小、条目数量和存活时间均有硬上限。

验收目标：

- 真实 TXT、PNG、PDF 分别由系统默认应用打开；
- 中文、空格、长文件名和同名条目不会冲突；
- 含 `../`、绝对路径和设备路径的恶意 ZIP 被拒绝且缓存为空；
- 可执行内容没有确认时绝不启动；
- 退出后可删除缓存；模拟占用时下次启动完成回收；
- 密码不出现在命令行、日志、临时文件名或历史记录中。

完成证据：见 [ARCHIVE_WORKSPACE_A03_AUDIT.md](ARCHIVE_WORKSPACE_A03_AUDIT.md)。真实 Release WebView2 已验证 TXT、PNG、PDF 的默认应用打开、缓存字节一致、Mark-of-the-Web 一致，以及危险 CMD 默认取消时不落盘、不执行。

### A-04 内部查看器分层（2026-08-26 已完成）

工作内容：

- 保留现有 ZIP/TAR 图片有界预览；
- 增加受限纯文本预览：只读取前 N MiB，检测 BOM/UTF-8/常用本地编码，二进制内容不按文本渲染；
- PDF、音视频首期不在 WebView 内直接解析，交由默认应用打开；
- 7Z 固实块和当前 RAR 读取无法证明有界时继续禁用内部预览，但允许用户确认后提取单项并默认应用打开；
- UI 必须解释“内部预览不可用”和“可以提取后打开”的区别。

验收目标：

- 超限图片、超大像素、伪装扩展名、SVG 主动内容、截断图片继续拒绝；
- 文本预览不加载完整超大文件，二进制文件不造成页面冻结；
- 7Z/RAR 不绕过既有安全边界；
- 预览关闭后不改变已选条目和当前目录。

完成证据：见 [ARCHIVE_WORKSPACE_A04_AUDIT.md](ARCHIVE_WORKSPACE_A04_AUDIT.md)。真实 Release WebView2 已验证 ZIP UTF-8、超大日志截断、伪装二进制拒绝、UTF-16LE TAR 解码、7Z 禁用边界和内部预览不落盘。

### A-05 嵌套归档（2026-08-26 已完成）

工作内容：

- 嵌套归档条目先经 A-03 安全提取，再作为只读子工作区打开；
- 顶部显示归档链，例如 `outer.zip > assets.7z`；
- 默认最大嵌套深度 3，累计展开预算和缓存预算共享；
- 关闭内层归档返回外层原位置；
- 密码保险箱仍按当前待打开归档匹配，不继承错误密码；
- 禁止循环或重复打开相同内容造成无限栈。

验收目标：

- 真实 ZIP→7Z→ZIP 三层样本能够进入、返回并选择性解压最内层文件；
- 第四层被明确拒绝并解释原因；
- 伪装归档、损坏内层、加密内层和错误密码均给出稳定分类；
- 取消内层读取后外层浏览状态保持完整。

完成证据：见 [ARCHIVE_WORKSPACE_A05_AUDIT.md](ARCHIVE_WORKSPACE_A05_AUDIT.md)。真实 Release WebView2 已验证 ZIP→加密 7Z→ZIP、错误/正确内层密码、密码逐层隔离、第四层阻断、最内层精确解压、损坏内层返回和外层 2/2 选择状态恢复；服务端深度推导、共享缓存预算和祖先内容防循环另有 Rust 回归。

### A-05.1 归档结构读取可取消化（2026-08-26 已完成）

工作内容：

- 每次浏览或刷新使用唯一请求 ID，前端取消会传到后端，不再只丢弃迟到结果；
- ZIP、RAR、TAR 元数据遍历逐项检查取消；通用 7-Zip CLI 在取消时终止等待并配置子进程随 Future 丢弃；
- 所有结构读取统一设置 30 秒等待上限，超时后停止当前用户等待并触发取消标记；
- 处理“用户极快点击取消、后端尚未登记请求”的竞态；
- 用户界面展示明确的取消按钮、取消结果和中文错误分类，不直接显示底层英文异常。

验收目标：

- 真实大型归档读取期间可以从界面取消，5 秒内恢复到可继续选择文件的空闲状态；
- 取消或返回内层不会污染外层状态，迟到结果无权写回；
- ZIP/7Z/RAR、选择性解压、默认应用打开、预览和三层嵌套既有桌面路径不回归；
- 取消、超时、密码、损坏和不支持格式使用稳定且可理解的错误分类。

完成证据：见 [ARCHIVE_WORKSPACE_A05_1_AUDIT.md](ARCHIVE_WORKSPACE_A05_1_AUDIT.md)。真实 Windows Release WebView2 对现场生成的 18 万目录项 TAR 完成取消，点击后 55 ms 恢复；随后完整归档浏览门禁通过。

### A-05.2 能力来源与组件边界收口（已完成）

工作内容：

- 删除浏览页自行维护的归档扩展名真相源，改为消费后端实际引擎能力或共享的结构化能力描述；
- 把当前过大的 `ArchiveBrowserView.vue` 拆为导航/目录树、条目表、读取状态、右键菜单与详情/预览等边界明确的组件或组合式函数；
- 保持 A-01 至 A-05.1 的可见交互、测试选择器和安全命令不变，不借重构扩张产品范围。

验收目标：

- 一个格式是否可浏览、可预览、可作为嵌套归档只由后端能力与受限预览策略决定，前端不再出现第二份可漂移清单；
- 主视图只负责页面编排和跨组件状态，关键动作有独立单元测试；
- 完整前端、Clippy、真实归档浏览桌面门禁全部无回归。

完成结果：

- `get_archive_engine_capabilities` 现在同时返回可浏览、可嵌套、可有界预览的格式与图片/文本扩展策略；前端归档筛选、嵌套入口和预览入口只消费这份结构化能力；
- 修正 7-Zip 26.02 已真实报告 `zstd zst tzst`、后端解析表却漏掉 zstd 的漂移；真实 zstd 流被装入现场生成 ZIP 后，桌面右键“进入压缩包”可用；
- 请求生命周期、能力映射、目录/历史导航拆为三个组合式函数，目录树拆为独立组件，并分别补齐单元测试；主视图保留跨域编排、选择、预览、嵌套与发布动作，不改变 A-01 至 A-05.1 的可见语义；
- 全量前端 234/234、归档能力 Rust 3/3、Clippy 零告警、生产构建、Release 桌面构建及真实 Windows Tauri/WebView2 归档门禁通过。完整预期—实际—修正记录见 [ARCHIVE_WORKSPACE_A05_2_AUDIT.md](ARCHIVE_WORKSPACE_A05_2_AUDIT.md)。

### A-06 真实验收矩阵

至少包含：

- 普通/加密 ZIP、7Z、RAR；
- TAR、TAR.GZ；
- 中文八层长路径和同名条目；
- 含文本、图片、PDF、视频和可执行文件的混合归档；
- 三层嵌套归档；
- 路径穿越、超大预览、损坏归档、错误密码、磁盘不足和取消；
- 从资源管理器经典右键“浏览压缩包内容”进入；
- 安装版 WebView2 中完成双击、右键、默认应用打开、嵌套进入和选择性解压。

节点发布门禁：见第 9 节。A-01 至 A-06 任一未通过，不提升版本。

## 5. 大节点 B：图片压缩

建议目标版本：`1.1.15`。

### 开发目标

在压缩中心增加可批量、可比较、可取消、可验证的图片压缩模式。首期公开写入能力收紧为 JPEG、WebP 与无损 PNG。GIF 首期只识别并保持原文件或明确拒绝，直到动画编码链的许可与帧语义独立通过；TIFF、HEIC 在取得稳定解码、编码、元数据和许可方案前不公开声明。

候选引擎：JPEG/WebP 只评估关闭默认功能并显式启用 `jpg,webp` 的 Rust `libcaesium`；无损 PNG 独立评估 MIT `oxipng`。禁止启用会引入 AGPL `gifski` 或 GPL `imagequant` 的 libcaesium 默认/GIF/PNG 路径。

### B-01 依赖与基线实验（2026-08-27 已完成）

- 固定 libcaesium 与 oxipng 版本和 feature 集，只启用首期已审计格式；
- 在不违反 B-00 产品运行时冻结的前提下，记录隔离候选载荷的原始/压缩增量、冷进程全流程、峰值内存和四进程并发基线；最终 NSIS 增量在 B-03 接入正式运行时后测量；
- 建立照片、透明 PNG、截图、动画 GIF 拒绝边界、WebP 和元数据样本；TIFF 在进入公开范围时再补齐；
- 从 B-00 属性夹具中分离 B-01 固定基准输入：对每个可处理样本提交输入哈希清单、尺寸、帧数/动画属性、ICC/EXIF 预期和允许变化。B-00 可再生成夹具未承诺字节相同，不得直接用于性能结论。

验收目标：依赖许可证与 NOTICE 完整；没有未审计二进制下载；每种公开格式至少一个非空真实样本可重复处理。

完成结果：精确 feature 的 libcaesium 0.21.0 与 oxipng 10.2.0 已在 Windows x64/Rust 1.93.1 真实构建；五个固定输入连续两次生成 SHA-256 一致，JPEG/WebP/无损 PNG 输出可重新解码，GIF 明确拒绝且不落盘。隔离候选压缩增量为 1,077,127 B，单进程峰值工作集约 10.2 MiB，四进程并发完成。完整预期—实际—修正见 [B01_IMAGE_DEPENDENCY_BASELINE_AUDIT.md](B01_IMAGE_DEPENDENCY_BASELINE_AUDIT.md)。

### B-02 前端工作区

状态：**已完成 / 2026-08-28 收口**。前端工作区、压缩 store 内隔离草稿、同源配置、格式拒绝和安全预览授权已实现；收口审计纠正了首稿新建图片 store、Windows 资产 CSP、方向尺寸语义和系统选择后 0 B 四项偏移。Windows Release/WebView2 双尺寸矩阵与真实系统“选择图片文件”路径均已通过，证据见 [B02_IMAGE_WORKSPACE_PAUSE_AUDIT.md](B02_IMAGE_WORKSPACE_PAUSE_AUDIT.md) 与 [B02_NATIVE_PICKER_PATH_AUDIT.md](B02_NATIVE_PICKER_PATH_AUDIT.md)。允许进入 B-03，仍不升版、不开放伪执行。

- 压缩中心新增四模式切换组件，保留归档压缩为默认；
- 图片任务列表显示文件名、应用方向后的可见尺寸、输入大小、输出格式、状态、进度和节省比例；编码像素矩阵与方向信息保留为后端验证事实；
- 配置包含：无损/有损、质量、保持尺寸/最大宽高、输出格式、保留元数据、输出目录、冲突策略；
- 提供原图/结果图对比和预计结果说明，但预计值不得冒充实际值；
- 批量全局设置与单项覆盖使用同一模型。

验收目标：拖入非图片时明确拒绝；多选任务不挤压工具栏；窄窗口无横向滚动；切换 Tab 不污染归档压缩队列。

### B-03 后端执行与发布事务

状态：**已完成 / 2026-08-28 收口**。产品运行时严格准入 `libcaesium 0.21.0`（仅 `jpg,webp`）、`oxipng 10.2.0`（仅 `parallel,zopfli`）、`image 0.25.10`（仅 `jpeg,png,webp`）和 `img-parts 0.4.0`（仅 `std`）。服务支持同格式压缩、格式转换和按可见尺寸等比例缩放；编码后再次完整解码并核对格式、矩阵、方向、可见尺寸、帧数、Alpha 与配置承诺的 EXIF/ICC，再按用户大小策略调用共享原子发布事务。命令已复用统一取消注册表、容量预检和受控阻塞线程。最终 NSIS 相对 `f4ea25b` 同版本基线净增 877,416 B。详见 [B03_IMAGE_ENCODING_TRANSACTION_AUDIT.md](B03_IMAGE_ENCODING_TRANSACTION_AUDIT.md) 与 [B03_2_IMAGE_TRANSFORM_EXECUTION_AUDIT.md](B03_2_IMAGE_TRANSFORM_EXECUTION_AUDIT.md)。

边界纠偏：现有图片配置没有“删除源文件”选项，因此当前实现始终只读源文件，不把并不存在的回收功能冒充完成；未来若开放该选项，只能在共享发布成功后调用系统回收站。B-04 已完成统一事实链和真实结果 UI 开放；B-05 将验证安装版完整流程，不得借此绕过共享发布边界。

- 魔数和解码器共同确认输入，不只看扩展名；
- 输出写入唯一临时文件；
- 完成后重新解码，分别验证编码像素矩阵、方向信息、应用方向后的可见尺寸、帧数和格式；
- 只有输出有效且满足用户策略时才发布；
- “仅在更小时替换”作为默认行为，结果更大时保留源文件并明确说明；
- 取消、编码失败、磁盘不足和发布竞态全部清理临时输出；
- 源文件移入系统回收站只能在最终发布成功后执行。

验收目标：真实文件解码复核成功；透明通道、动画、方向信息和元数据按配置保持或明确移除；源文件不会被半成品替换。

### B-04 进度、日志和历史

状态：**已完成 / 2026-08-28 收口**。B-04.1 至 B-04.4 已完成输入/输出事实、真实阶段、安全批量编排、统一队列与跨重启历史；B-04.5 已将该链路接入图片工作区，按真实 ready 状态开放执行/取消，并展示后端验证的结果预览、路径、尺寸、格式和实际字节差。固定真实 JPEG/WebP/PNG 已在 Windows WebView2 中完成点击执行、磁盘 metadata、历史和双尺寸布局复核。证据见 [B04_1_IMAGE_FACT_CONTRACT_AUDIT.md](B04_1_IMAGE_FACT_CONTRACT_AUDIT.md)、[B04_2_IMAGE_STAGE_EVENT_AUDIT.md](B04_2_IMAGE_STAGE_EVENT_AUDIT.md)、[B04_3_IMAGE_SAFE_ORCHESTRATION_AUDIT.md](B04_3_IMAGE_SAFE_ORCHESTRATION_AUDIT.md)、[B04_4_IMAGE_QUEUE_HISTORY_AUDIT.md](B04_4_IMAGE_QUEUE_HISTORY_AUDIT.md) 与 [B04_5_IMAGE_RESULT_UI_AUDIT.md](B04_5_IMAGE_RESULT_UI_AUDIT.md)。下一接续点为 B-05。

- 日志展示解码、缩放、编码、验证、发布阶段；
- 进度按批量文件数与可用字节信息组合，不得长时间静止在虚假百分比；
- 历史记录 `taskType=compression`、`workloadKind=image`；
- 指标至少包含输入大小、输出大小、节省字节、节省比例、输入/输出可见尺寸与格式；图片可见尺寸统一为应用方向后的结果，编码矩阵与方向作为校验事实单独记录。

验收目标：完成、失败、取消三种历史跨重启保留；统计使用真实输出，不用估算覆盖实际结果。

### B-05 真实验收矩阵

分段状态（2026-08-28）：**B-05.1 至 B-05.3 已完成**。B-05.1 的 JPEG、PNG、WebP 各 3 个冻结真实输入通过生产服务并独立重新解码；B-05.2.1 在真实 Windows Release Tauri/WebView2 中完成 100/100 图片、唯一输出和统一历史；B-05.2.2 完成 96 MP/100.01 MP 上下界、340 UTF-16 中文长路径、冲突/竞态、StorageFull 和编码期取消；B-05.3 以无测试桥正式 NSIS 完成三张真实图片的输入、可见配置、执行前后对比、发布、历史、完整重启和输出重新打开，安装生命周期 50/50、图片链 17/17。证据见 [B05_1_IMAGE_FORMAT_MATRIX_AUDIT.md](B05_1_IMAGE_FORMAT_MATRIX_AUDIT.md)、[B05_2_1_IMAGE_BATCH_AUDIT.md](B05_2_1_IMAGE_BATCH_AUDIT.md)、[B05_2_2_IMAGE_FAILURE_BOUNDARIES_AUDIT.md](B05_2_2_IMAGE_FAILURE_BOUNDARIES_AUDIT.md) 与 [B05_3_INSTALLED_IMAGE_FULL_FLOW_AUDIT.md](B05_3_INSTALLED_IMAGE_FULL_FLOW_AUDIT.md)。下一接续点为 **v1.1.15 发布审计与公开更新闭环**。

- 每个公开格式至少 3 个真实样本，覆盖小图、大图、透明、动画和元数据；
- 100 张混合批量；
- 1 张超大像素图的资源上限；
- 中文长路径、目标冲突、磁盘不足、取消和回收源文件；
- 安装版完成拖入、配置、对比、执行、历史查看和重新打开输出。

节点发布门禁：B-01 至 B-05 全部通过后才允许提升补丁版本。

## 6. 大节点 C：视频压缩

建议目标版本：`1.1.16`。首个正式节点只承诺软件编码；硬件编码作为后续独立节点，不阻塞基础视频压缩发布。

### 开发目标

提供稳定、可预测、可验证的本地视频压缩。首期输入可覆盖 FFmpeg 探测到的常见格式，公开输出固定为 MP4/H.264/AAC，避免首版把编码器组合扩张为不可验证矩阵。

### C-01 FFmpeg 构建与合规

进度（2026-08-28）：C-01.1 已完成可复现候选；C-01.2.1 已关闭产品运行时准入；C-01.2.2 已完成生产 Media Foundation 稳定分类、正式应用内部预检、正式 NSIS 覆盖安装目录的真实软件转码、隔离缺失/替换拒绝、卸载/上一版本恢复，以及同提交 Tauri-updater-signed NSIS/updater 精确增量（均为 `6,821,970 B`）。上述结果满足最初 C-01 验收，C-01 已关闭。真实 Windows N 前后实机工具和验收器已就绪，实际机器证据归入 C-05/发布门禁；下一节点为 C-02。证据见 [C01_1_FFMPEG_REPRODUCIBLE_CANDIDATE_AUDIT.md](C01_1_FFMPEG_REPRODUCIBLE_CANDIDATE_AUDIT.md)、[C01_2_1_VIDEO_RUNTIME_ADMISSION_AUDIT.md](C01_2_1_VIDEO_RUNTIME_ADMISSION_AUDIT.md) 与 [C01_2_2_INSTALLED_RUNTIME_AND_SIGNED_DELTA_AUDIT.md](C01_2_2_INSTALLED_RUNTIME_AND_SIGNED_DELTA_AUDIT.md)。

- 选择固定版本、固定哈希、可重现的 Windows x64 构建；
- 首期采用可满足项目分发策略的 LGPL 配置，不启用会让整体 FFmpeg 变为 GPL 的组件；
- 随安装包提供许可证、构建配置、来源、版本和哈希；
- 使用 `ffmpeg -version`、`-encoders`、`-filters` 在构建和安装态验证能力；
- 记录新增安装包体积和更新下载体积。

验收目标：CI 与安装包内二进制哈希可追踪；缺失或被替换时应用拒绝开始任务；许可清单完整。

### C-02 探测与配置模型

完成状态（2026-08-28）：**C-02.1 至 C-02.4 全部完成，C-02 已关闭**。产品 ffprobe 有界事实、三档/最大分辨率/估算/流变化规划、不可执行工作区及真实分类/Windows 桌面矩阵全部通过；VFR、旋转、无音频、双音轨、字幕、10 分钟输入和损坏容器均有真实产品运行时证据。下一节点为 C-03，C-04 验证完成前仍不得发布视频输出。完整收口证据见 [C02_4_VIDEO_REAL_MATRIX_AUDIT.md](C02_4_VIDEO_REAL_MATRIX_AUDIT.md)。

- 使用 ffprobe 获取时长、分辨率、帧率、视频/音频编码、码率、旋转和字幕流；
- 提供“清晰、均衡、小体积”三档，并允许设置最大分辨率；
- 默认保持宽高比、旋转方向和音频；
- 明确字幕、章节、封面、HDR 和多音轨的首期策略，不能静默丢弃；
- 在执行前展示预计大小区间和可能发生的流变化。

验收目标：VFR、旋转视频、无音频、多音轨、字幕、长视频和损坏输入均稳定分类；估算标记为估算。

### C-03 执行、进度与取消

完成状态（2026-08-29）：**C-03.1 至 C-03.3.2 全部完成，C-03 已关闭**。参数数组、机器进度、ETA、临时大小/比例、Job Object、容量预检、真实暂存、心跳和清理已通过；唯一 `compress_video_file` 串起权威重规划、精确流变化确认、编码、完整验证和原子发布。视频工作区现已复用统一任务、取消、事件、无覆盖目标规划、最终 `TaskMetricsV1` 与跨重启历史，拒绝风险确认时不创建任务。下一步进入 C-05 真实验收矩阵。证据见 [C03_1_VIDEO_EXECUTION_FOUNDATION_AUDIT.md](C03_1_VIDEO_EXECUTION_FOUNDATION_AUDIT.md)、[C03_2_VIDEO_STAGING_EXECUTOR_AUDIT.md](C03_2_VIDEO_STAGING_EXECUTOR_AUDIT.md)、[C03_3_1_VIDEO_COMMAND_PIPELINE_AUDIT.md](C03_3_1_VIDEO_COMMAND_PIPELINE_AUDIT.md) 与 [C03_3_2_VIDEO_TASK_UI_AUDIT.md](C03_3_2_VIDEO_TASK_UI_AUDIT.md)。

- 通过 `-progress pipe:1` 等机器可解析通道读取进度，不解析本地化控制台文本；
- 参数以参数数组传入，不拼接 shell 字符串；
- 子进程放入 Windows Job Object 或等价生命周期控制，取消和应用退出能终止进程树；
- 临时输出与源文件隔离；
- 展示当前时间点、速度倍数、已输出大小、预计剩余和压缩率；
- 长时间没有进度事件时展示“仍在编码”和最后心跳，而不是假装卡死。

验收目标：路径含中文、空格和特殊字符可处理；取消后 FFmpeg 及子进程全部退出；输出和 passlog 等临时文件全部清理。

### C-04 输出验证

完成状态（2026-08-29）：**C-04.1 至 C-04.3 全部完成，C-04 已关闭**。暂存身份/大小、MP4/H.264/AAC、流数量、尺寸、旋转、时长、完整音视频帧扫描、Mark-of-the-Web、原子发布和最终磁盘事实已实现；真实截断、零字节、FFmpeg 非零退出/终止、源或暂存改写、容量门禁、取消和目标竞态均不发布。C-03.3.1 已将完整安全管线接入唯一后端命令，C-03.3.2 也已接通统一任务 UI、最终指标与历史；后续 C-05.1 至 C-05.4.1 已完成，当前下一接续点为真实 Windows N 前后门禁。证据见 [C04_1_VIDEO_OUTPUT_VALIDATION_AUDIT.md](C04_1_VIDEO_OUTPUT_VALIDATION_AUDIT.md)、[C04_2_VIDEO_ATOMIC_PUBLICATION_AUDIT.md](C04_2_VIDEO_ATOMIC_PUBLICATION_AUDIT.md)、[C04_3_VIDEO_FAILURE_MATRIX_AUDIT.md](C04_3_VIDEO_FAILURE_MATRIX_AUDIT.md)、[C03_3_1_VIDEO_COMMAND_PIPELINE_AUDIT.md](C03_3_1_VIDEO_COMMAND_PIPELINE_AUDIT.md) 与 [C03_3_2_VIDEO_TASK_UI_AUDIT.md](C03_3_2_VIDEO_TASK_UI_AUDIT.md)。

- 用 ffprobe 验证输出容器、视频流、音频流、时长和可解码性；
- 输入有音视频时，输出不得静默缺少对应流；
- 时长偏差设置可解释阈值；
- 输出无效、零字节、时长异常或流丢失时任务失败且不发布；
- 只有验证通过后才允许回收源文件。

验收目标：截断输出、模拟编码器崩溃、磁盘不足和目标竞态均不会覆盖旧目标。

### C-05 真实验收矩阵

进度（2026-08-29）：**C-05.1 至 C-05.4.1 已完成；产品负责人明确将 Windows N 调整为 `v1.1.16` 暂不支持平台后，C-05 在非 N Windows x64 支持范围内关闭。** Release WebView2 已完成两条真实视频执行；开发态矩阵以 5 种格式、4 个分辨率层级和 7 次产品执行覆盖全部三档预设及无音频，并以两个独立产品执行补齐 600 秒/3600 帧和 114,842,332 B 大输入。输出容器、编码、时长、尺寸、流、完整帧和实际字节差异均为 0。C-05.3 用真实 109.52 MiB 编码验证 UI 中途取消后产品 FFmpeg 退出、无暂存/最终输出、取消历史落库；两条完成记录及精确 metrics 跨原生应用完整重启保持一致，发布 MP4 由 Windows 默认应用接收。C-05.4.1 正式 NSIS 生命周期 50/50、安装态视频工作区 20/20，通过候选字节对账、覆盖安装数据保持、真实运行时/UI、卸载与公开 `v1.1.15` 恢复。Windows N 工具和拒绝分类全部保留，`windowsNRealMachinePassed` 继续为 `false`，未来只有取得实机证据后才可移除不支持声明。下一步进入版本身份、公开更新和回下载复验。证据见 [C05_1_VIDEO_DESKTOP_EXECUTION_AUDIT.md](C05_1_VIDEO_DESKTOP_EXECUTION_AUDIT.md)、[C05_2_1_VIDEO_FORMAT_MATRIX_AUDIT.md](C05_2_1_VIDEO_FORMAT_MATRIX_AUDIT.md)、[C05_2_2_VIDEO_LONG_LARGE_MATRIX_AUDIT.md](C05_2_2_VIDEO_LONG_LARGE_MATRIX_AUDIT.md)、[C05_3_VIDEO_RUNTIME_BEHAVIOR_AUDIT.md](C05_3_VIDEO_RUNTIME_BEHAVIOR_AUDIT.md)、[C05_4_1_INSTALLED_VIDEO_LIFECYCLE_AUDIT.md](C05_4_1_INSTALLED_VIDEO_LIFECYCLE_AUDIT.md) 与 [C05_4_2_WINDOWS_N_SCOPE_CHANGE_AUDIT.md](C05_4_2_WINDOWS_N_SCOPE_CHANGE_AUDIT.md)。

- MP4/H.264、MOV、AVI、WMV、WebM 输入；
- 480p、720p、1080p、4K；
- 有/无音频、旋转、VFR、字幕、多音轨；
- 30 秒小样本、10 分钟样本和至少一个大文件；
- 三档预设分别验证输出可播放、时长、分辨率、音视频流和实际大小；
- 安装版执行、取消、历史、输出默认应用播放与公开更新后复验。
- `v1.1.16` 公开支持限定为非 N Windows x64；Windows N 暂不支持，未来只有在 Media Feature Pack 安装前后同机生产验证及独立证据验收通过后才可纳入。

### C-06 后续硬件编码节点

硬件编码不和首个视频节点混发。后续先探测 Media Foundation、Intel QSV、NVIDIA NVENC、AMD AMF 的真实可用性，再按设备白名单开放；任何失败必须自动回退软件编码并在日志说明。该节点完成后可再提升一个补丁版本。

## 7. 大节点 D：PDF 安全优化

建议目标版本：`1.1.17`。

### 开发目标

提供不上传文件、默认保留内容结构、不会夸大压缩效果的 PDF 优化。首期采用 Apache-2.0 的 qpdf，定位为“安全优化”，不宣称等同于会降采样整页图像的强力压缩。

Ghostscript 采用 AGPL/商业双许可，未完成法律与分发方案前不得随 MIT 安装包内置，也不得在构建脚本中静默下载。

### D-01 qpdf 能力与样本基线

- 固定 qpdf 版本、哈希、许可证和安装态能力检查；
- 明确使用流重压缩、对象流、结构整理及可选图片优化参数；
- 样本覆盖文本型、扫描型、混合型、表单、注释、书签、附件、加密和数字签名 PDF；
- 记录每类样本允许变化和禁止变化。

验收目标：所有公开参数有官方依据；无法安全处理的加密、签名或特殊文档在执行前提示，不静默破坏。

进度（2026-08-30）：**D-01 已关闭。** D-01.1 已锁定两种白名单模式与十类真实 PDF 基线；D-01.2.1 将 qpdf 12.4.0 五文件运行时及许可纳入正式资源并实现生产预检；D-01.2.2 同提交签名构建实测 NSIS/updater 各增加 3,603,012 B，正式安装态完整/缺失/替换和公开 v1.1.16 恢复 49/49 通过。产品仍只开放身份预检，PDF 分析、执行和 UI 冻结；下一步为 D-02。证据见 [D01_1_QPDF_CAPABILITY_AND_FIXTURE_BASELINE_AUDIT.md](D01_1_QPDF_CAPABILITY_AND_FIXTURE_BASELINE_AUDIT.md)、[D01_2_1_QPDF_RUNTIME_ADMISSION_AUDIT.md](D01_2_1_QPDF_RUNTIME_ADMISSION_AUDIT.md)和 [D01_2_2_INSTALLED_QPDF_AND_SIGNED_DELTA_AUDIT.md](D01_2_2_INSTALLED_QPDF_AND_SIGNED_DELTA_AUDIT.md)。

### D-02 前端配置

- 模式分为“无损整理”和“兼容图片优化”；
- 展示页数、输入大小、是否加密、是否包含签名/表单/附件等可可靠探测的信息；
- 明确显示“签名可能失效”“图片优化可能有损”等影响；
- 默认输出新文件，禁止默认覆盖原 PDF。

验收目标：用户在执行前能看到风险；危险组合必须显式确认；页面不使用“压缩率保证”等误导文案。

进度（2026-08-31）：**D-02.1 已关闭。** 产品后端新增固定参数的 qpdf 只读分析，返回输入字节、页数、加密/密码状态、签名、普通表单、附件和书签事实；密码通过 `--password-file=-` 从 stdin 传递，不进入参数或稳定错误文本。十类真实 PDF、无密码/正确密码/错误密码、分析前后源 SHA-256 共 12 组预期—实际比较差异为 0。产品 UI 和转换仍冻结，下一步为 D-02.2 两模式配置与风险界面。证据见 [D02_1_PDF_READONLY_ANALYSIS_AUDIT.md](D02_1_PDF_READONLY_ANALYSIS_AUDIT.md)。

### D-03 执行与校验

- 参数数组调用 qpdf，临时输出、取消、磁盘保护和原子发布复用公共事务；
- 输出必须通过 qpdf 检查并能读取页树；
- 页数、加密状态、附件/表单等按所选策略复核；
- 若结果更大，默认不发布并说明原因；用户可选择保留结果但不得自动替换源文件；
- 签名 PDF 默认只分析，不执行会使签名失效的转换。

验收目标：错误输入、损坏 PDF、密码错误、取消、磁盘不足均无半成品；源文件哈希保持不变。

### D-04 真实验收矩阵

- 文本、扫描、图文混合、中文字体、透明图片；
- 表单、注释、书签、附件、加密和签名；
- 大页数和大图片 PDF；
- 两种模式分别记录输入/输出大小、页数、结构检查和人工可见抽查；
- 安装版完成批量处理、失败提示、历史和默认 PDF 阅读器打开。

节点发布门禁：D-01 至 D-04 全部通过后才允许提升补丁版本。

## 8. 跨节点工作项

### X-01 数据库迁移兼容

- 新迁移只增加可空字段或新表，不重写旧历史；
- 旧数据库升级后原有压缩/解压历史数量、状态和日志不变；
- 新版本降级到上一正式版本时不得造成启动崩溃；
- 迁移具备空库、旧库、重复执行和中断恢复测试。

### X-02 统一可观测性

媒体任务可选字段建议包括：

```ts
interface WorkloadMetrics {
  kind: 'archive' | 'image' | 'video' | 'pdf'
  inputBytes?: number
  outputBytes?: number
  savedBytes?: number
  savedPercent?: number
  width?: number
  height?: number
  durationMs?: number
  pageCount?: number
  inputCodec?: string
  outputCodec?: string
}
```

字段必须来自真实探测或最终输出；估算值使用独立 `estimated` 标记，不能写入最终统计代替真实值。

### X-03 性能和资源边界

- 图片并发受内存预算控制；
- 视频首期默认单任务编码，防止 CPU/磁盘饱和；
- PDF 批量并发以文件数和总输入大小双重限制；
- 所有外部进程都有超时、心跳、取消和进程树回收；
- 性能基线必须记录机器指纹，跨机器结果不直接判定回归。

### X-04 安全边界

- 默认应用打开属于执行外部内容的边界，必须传播 Mark-of-the-Web；
- 不把密码、媒体路径或用户内容发送到网络；
- 不从任务模板导入任意 FFmpeg/qpdf 参数；
- 高级参数使用结构化白名单，不接受原始命令行字符串；
- 所有第三方二进制固定版本、哈希、来源和许可证。

## 9. 每个大节点的统一验收与发布流程

### 9.1 开发完成审计

每个节点建立 `docs/RELEASE_AUDIT_<version>.md`，至少包含：

- 原始需求逐条映射；
- 计划工作项与实际实现差异；
- 已知边界和未完成项；
- 真实样本清单、来源、大小、哈希与预期；
- 实际结果、失败记录和修正记录；
- 前端、Rust、桌面、安装和更新测试结果；
- 安装包与第三方依赖变化；
- 是否允许发布的明确结论。

不能只记录最终通过；首次失败及其修复必须保留，形成“预期—实际—修正—复验”链路。

### 9.2 代码门禁

最低要求：

```powershell
npm.cmd ci
npm.cmd run type-check
npm.cmd run test:unit:coverage
npm.cmd run build
cargo test --release --all-targets --manifest-path src-tauri/Cargo.toml
cargo clippy --all-targets --manifest-path src-tauri/Cargo.toml -- -D warnings
npm.cmd run test:e2e:desktop:archive-workspace # 节点 A；其他节点建立各自脚本
```

还必须执行与变更相邻的现有桌面门禁。例如浏览中心改动至少复验归档闭环、历史、密码保险箱和响应式布局；媒体任务模型改动至少复验压缩、历史、容量预检、取消和设置持久化。

### 9.3 版本提升

只有节点审计结论为“允许发布”后，才把补丁版本提升一位。版本身份必须同时更新并由脚本核对：

- `package.json`；
- `package-lock.json` 根版本和包版本；
- `src-tauri/tauri.conf.json`；
- `src-tauri/Cargo.toml` 与 `Cargo.lock`；
- `src-tauri/shell-extension/Cargo.toml` 与 `Cargo.lock`；
- 唯一版本化 Shell Extension DLL。

执行：

```powershell
npm.cmd run test:release-identity -- --expected <version>
```

禁止提前占用版本号，也禁止功能未闭环时只为了触发构建而升版。

### 9.4 文档与候选安装包

- 更新根 README 和应用 README 的当前版本、特色、截图、格式/功能边界；
- 新增 `docs/RELEASE_NOTES_<version>.md`；
- 更新 `docs/DEVELOPMENT_HANDOFF.md` 和本计划中的节点状态；
- 构建不含 `desktop-e2e` 的正式 NSIS；
- 检查安装包文件版本、产品名、载荷列表、7-Zip 完整性、大小和 SHA-256；
- 无商业 Authenticode 证书时继续明确标记 `NotSigned`，不分发 Windows 11 第一层菜单身份包。

### 9.5 推送、Release 与更新闭环

1. 功能分支推送并通过 GitHub CI；
2. PR 审计无阻断问题后合入 `master`；
3. 在合并提交创建 `v<version>` 标签并推送；
4. Release 工作流生成 NSIS、updater ZIP、`.sig` 和 `latest.json`；
5. 从公开 Release 重新下载资产，核验哈希、压缩包完整性、版本和签名；
6. 从上一正式版本执行 `npm.cmd run test:public-update`；
7. 验证自动重启、安装位置、用户数据、密码保险箱、历史任务、右键菜单和唯一 Shell DLL；
8. 将公开 Release URL、工作流编号和更新证据回填发布审计。

只有公开资产和应用内更新都通过，节点才算真正关闭。

## 10. 推荐执行顺序与版本节奏

| 顺序 | 大节点 | 建议版本 | 发布前不可缺少的证据 |
| --- | --- | --- | --- |
| 1 | 压缩包浏览中心 2.0 | `1.1.14` | 双击、右键、默认应用、嵌套归档、安装版桌面闭环 |
| 2 | 图片压缩 | `1.1.15` | JPEG/WebP/无损 PNG 真实批量、GIF 明确边界、结果复核、对比、取消和历史 |
| 3 | 视频压缩软件编码 | `1.1.16` | FFmpeg 合规、真实多格式、输出流/时长验证、进程树取消 |
| 4 | PDF 安全优化 | `1.1.17` | qpdf 合规、结构复核、签名/表单边界、真实安装版批量 |
| 5 | 视频硬件编码（可选） | 后续补丁版 | 多厂商真实设备、自动回退和性能/质量对比 |

版本只是建议映射。如果某节点拆成多个对用户独立有价值、且各自满足全部发布门禁的完整闭环，可以分别提升补丁版本；如果只是内部重构、测试准备或半成品 UI，不得升版。

## 11. 当前状态

- 当前正式基线为 `1.1.16`；标签固定在 `a59742265feb961ab51f9b95b4e455aa15b79bf5`，公开 Release 与真实 `v1.1.15 → v1.1.16` 应用内更新门禁均已通过；
- A-01 已完成聚焦/多选分离、直属目录列表、双击/Enter 进入、Backspace/Alt 方向键、面包屑和后退/前进/上一级/刷新；搜索继续匹配完整归档路径；
- A-02 已完成空白区、单项、多选和目录的动态右键菜单；A-03 已接通默认应用安全打开；A-04 已完成 ZIP/TAR 图片与文本分层预览、编码识别、1 MiB 上限和二进制拒绝；A-05 已完成三层只读嵌套工作区、归档链、逐层密码隔离、服务端深度/循环防护与返回状态恢复；
- 全量前端 234/234、生产构建、A-05 Rust 8/8、A-05.2 归档能力 Rust 3/3 和 Clippy 零告警均通过；
- 本机已配置与 WebView2 精确匹配的 EdgeDriver。真实 Windows Release Tauri 门禁使用现场生成长中文路径 ZIP、加密 7Z、固定加密 RAR 与 TXT/PNG/PDF/CMD 混合 ZIP，完成目录右键打开、中文系统剪贴板逐字复核、详情布局、右键精确选择性解压、默认应用打开、NTFS 安全标记、危险内容默认取消及内容/哈希复核；
- 首次桌面运行发现目录切换后焦点离开页面导致 Alt+Left 无响应，现已改为窗口级键盘监听并复验通过。完整预期—实际—修正证据见 [ARCHIVE_WORKSPACE_A01_AUDIT.md](ARCHIVE_WORKSPACE_A01_AUDIT.md)；
- A-05.2 已消除前端归档扩展名第二真相源并拆分请求、能力、导航和目录树边界；现场生成的真实 zstd 流嵌入 ZIP 后，后端动态能力已在 Release WebView2 中真实驱动嵌套右键入口。证据见 [ARCHIVE_WORKSPACE_A05_2_AUDIT.md](ARCHIVE_WORKSPACE_A05_2_AUDIT.md)；
- B-00.1 至 B-05.3 已全部完成并发布为 `v1.1.15`；视频 C-01 至 C-05 已在非 N Windows x64 支持范围内关闭并发布为 `v1.1.16`，Windows N 暂不支持且实机证据状态仍为 false。PDF D-01 与 D-02.1 已关闭；下一步为 D-02.2 两模式配置与风险界面。

## 12. 技术参考

- 好压万能压缩公开能力：<https://compress.2345.cc/index.html>
- FFmpeg 文档：<https://www.ffmpeg.org/documentation.html>
- FFmpeg 许可说明：<https://www.ffmpeg.org/legal.html>
- libcaesium：<https://github.com/Lymphatus/libcaesium>
- qpdf：<https://github.com/qpdf/qpdf>
- qpdf 文件体积优化边界：<https://qpdf.readthedocs.io/en/stable/cli.html#optimizing-file-size>
- Ghostscript 许可说明：<https://ghostscript.com/faq/index.html>
