# 开发交接

## 2026-09-02 v1.2.2 正式发布关闭（当前基线）

- 六项冻结需求已由公开 `v1.2.2` 完成：图片/视频设置 Modal、三类重复文案精简、即时 Tab 入场与按钮动效、文件/压缩包/文件夹统一系统动作、Explorer 精确定位、特殊目录属性友好提示。
- Explorer 已不再把手工引号塞进单个参数；文件使用独立 `/select,` 与真实路径，目录使用真实路径。真实 Windows Tauri/WebView2 门禁已打开精确中文空格目录并选中精确中文空格文件，未回落桌面。
- 功能门禁：类型检查通过，前端 48 文件 280/280，Explorer Rust 6/6，Chromium 11/11，Rust 全量和 Shell 扩展全量通过，严格 Clippy 零告警；隔离 E2E Release 的真实 WebView2 右键链路、Explorer 精确定位、junction 属性提示和设置 Modal 几何全部通过。
- 候选身份、唯一 Shell DLL、无测试桥 NSIS 和 `v1.2.1 → v1.2.2 → 卸载 → v1.2.1` 49/49 均通过。PR #114 五项 CI 全绿后合入 `master@6d9012b908e7b1bf24d0fbb32af669021c22b2f4`；annotated `v1.2.2` 和四项资产已发布并回下载一致。
- 真实公开 `v1.2.1 → v1.2.2` 更新 25/25、失败 0；当前机器安装公开 `1.2.2` 于 `E:\Long\Long解压`，相关进程为 0。下一次开发必须从最新 `master`/公开 `v1.2.2` 重新审计接续目标。完整证据见 [RELEASE_AUDIT_1.2.2.md](RELEASE_AUDIT_1.2.2.md)。

## 2026-09-01 v1.2.1 正式发布关闭（历史，已由上方 v1.2.2 取代）

- `v1.2.0` 已由 PR #112 合入 `master@c4f1549` 并公开发布，四资产回下载一致；但真实 `v1.1.19 → v1.2.0` 更新在第 19 项发现旧 updater 尾部清理会删掉新进程刚注册的目录菜单，因此没有将公开更新记为通过。
- 重新启动同一 1.2.0 后，四根菜单在进程内和退出后都保持，确认是时序竞态。`v1.2.1` 增加启动后 2 秒/6 秒延迟复核，测试增加首次完整后 8 秒稳定观察；八处版本身份和唯一 DLL 已提升到 1.2.1。
- 完整回归、无测试桥 NSIS及 `v1.2.0 → v1.2.1 → 卸载 → v1.2.0` 安装生命周期 50/50 已通过。PR #113 五项 CI 全绿后合入 `master@7b05bdc659aa246f6397e8aa362b8ec49f7c7bf9`；annotated `v1.2.1` 标签和 Release 已发布，四项公开资产回下载一致。
- 真实公开 `v1.2.0 → v1.2.1` 更新 25/25、失败 0；菜单在首次完整、8 秒清理窗口后和应用退出后三次均保持 4 根/17 命令/4 快捷动作。当前机器安装公开 `1.2.1` 于 `E:\Long\Long解压`，相关进程为 0。
- `v1.2.1` 已关闭。下一次开发必须从最新 `master`/公开 `v1.2.1` 重新审计未完成目标，不再从 PR #113、候选分支或本机被忽略的证据目录接续。首轮失败、修正与正式证据见 [RELEASE_AUDIT_1.2.1.md](RELEASE_AUDIT_1.2.1.md)。

## 2026-09-01 v1.2.0 候选收口（历史，已由上方 v1.2.1 修复节点取代）

- 当前分支为 `codex/ux-convenience-1-2-0`，功能提交 `4e341e0` 与 `e87fb30` 已推送。特殊压缩紧凑布局、`Ctrl+Shift+S`、双栏显式多选、可点击面包屑、方向图标和空白区跨栏菜单均已完成。
- 完整单元 276/276、Chromium 10/10、生产构建与隔离 Release WebView2 聚焦门禁通过；真实 IPC 复制/移动/属性为 2 文件、2 目录、9 B，预期与实际差异 0。
- 八处版本身份现已同步为 `1.2.0`，唯一候选 DLL 为 `long_compress_shell_extension_1_2_0.dll`。Rust/Shell 严格门禁、无测试桥 NSIS 和 50/50 安装生命周期已通过并恢复公开基线。下一步只允许候选审计、PR 五项 CI、合并/标签/Release、公开资产回下载和真实 `v1.1.19 → v1.2.0` 更新。
- 在公开更新完成前，公开稳定版仍是 `v1.1.19`，不得把候选写成正式发布。完整证据见 [UX_CONVENIENCE_1_2_0_AUDIT.md](UX_CONVENIENCE_1_2_0_AUDIT.md) 与 [RELEASE_AUDIT_1.2.0.md](RELEASE_AUDIT_1.2.0.md)。

## 2026-09-01 v1.1.19 正式发布关闭（历史，已由上方 v1.2.0 候选节点取代）

- PR [#111](https://github.com/Longyuyeee/long_Decompress/pull/111) 已以 merge commit `f003ac675b250cf9c3f923ca5fbdb9905d6b5932` 合入 `master`；annotated `v1.1.19` 标签指向该提交，Release workflow [33472694078](https://github.com/Longyuyeee/long_Decompress/actions/runs/33472694078) 成功，公开四资产已回下载逐项核验。
- 双栏文件浏览器默认直接浏览真实磁盘，跨栏复制、移动、压缩、解压以及重命名、新建目录、回收站和属性已收口；原归档工作区继续承担受限预览、嵌套浏览和选择性解压。真实文件系统 3/3、完整单元 273/273、Browser shell Chromium 9/9、双栏 Release 桌面聚焦门禁和 PR 五项 CI 均通过。
- 真实公开 `v1.1.18 → v1.1.19` 应用内更新 24/24、失败 0。当前机器安装 `1.1.19` 于 `E:\Long\Long解压`，两套用户数据、4 个经典菜单根/17 条命令/4 条快捷动作及自启动保持，唯一 Shell DLL 为 `long_compress_shell_extension_1_1_19.dll`，相关应用进程为 0。
- `v1.1.19` 已收口。下一次开发必须从最新 `master`/公开 `v1.1.19` 重新审计路线图未完成目标；不要从 PR #111、候选分支或被忽略的本机证据目录继续。若扩展文件管理器，优先审计同名冲突交互、跨卷低容量恢复和标签页/书签，且不得削弱默认不覆盖、拒绝重解析点和跨卷先校验后删源的安全边界。

## 2026-09-01 v1.1.18 正式发布关闭（历史，已由上方 v1.1.19 关闭取代）

- PR #109 已以 merge commit `0cdb53a239265ab2b00514029d293f44f120c15e` 合入 `master`；annotated `v1.1.18` 标签指向该提交，Release workflow run `33464702924` 成功，公开四项资产已回下载并逐项核验。
- 精确候选六组安装生命周期 64/64；真实公开 `v1.1.17 → v1.1.18` 应用内更新 24/24。当前机器安装 `1.1.18` 于 `E:\Long\Long解压`，用户数据、经典菜单与自启动保持，唯一 Shell DLL 为 `long_compress_shell_extension_1_1_18.dll`，相关应用进程为 0。
- 自动化窗口干扰已经同时在安装矩阵和公开更新 UI 中纠正：窗口移到虚拟桌面之外但保持渲染，不拉起外部播放器/阅读器；真实输出、签名更新交接和重启断言均保留。
- `v1.1.18` 已收口。下一次开发必须从最新 `master`/公开 `v1.1.18` 重新审计路线图未完成目标，不再从 PR #109、head `045d9d9` 或本机被忽略的候选证据目录继续。

## 2026-09-01 v1.1.18 候选实测后接续点（历史，已由上方正式发布关闭取代）

- 分支仍为 `codex/special-compression-navigation`，PR 仍为 [#109](https://github.com/Longyuyeee/long_Decompress/pull/109)。纠偏 head `045d9d9` 的 CI run `33462432485` 五项全绿；其正式 NSIS、包内 EXE/DLL 身份及六组安装生命周期已经核验，精确最终候选同轮结果为 64/64、失败 0。
- 实测发现 Node 25.2.1 会在 PDF 工作区稳定崩溃，Node 24.14.0 对同一候选通过；发布测试现只接纳 Node 20/22/24 LTS major。视频取消改用 10 分钟真实输入，完成继续使用 114,842,332 B 输入；这修复短输入过快完成造成的观察竞态，没有接受“已完成”冒充“已取消”。
- 为避免自动化干扰当前桌面，四个安装工作区会将真实应用窗口移到虚拟桌面之外，并且不点击会启动外部播放器/阅读器的按钮；仍验证真实输出、格式/页数、源哈希、取消、历史重启和入口可用。直接最小化曾使 WebView2 停顿，已经撤销且未通过延长超时掩盖。
- 当前机器已恢复公开 `1.1.17`，安装位置 `E:\Long\Long解压`，用户数据、经典菜单与自启动均恢复，相关测试进程为 0。原始最终同轮证据位于被忽略的 `test-results/installed-release-validation/20260901-104120`。
- 最终 NSIS 为 19,315,298 B / `1DA61456…43B852`；包内 EXE 为 29,389,312 B / `A546CBCD…C0F6BB`；唯一 `long_compress_shell_extension_1_1_18.dll` 为 253,952 B / `49E619AF…45AAA9`。
- 下一步只允许：提交/推送本次最终审计文档 → 确认该提交相对 `045d9d9` 仅文档变化且 CI 五项全绿 → 合并 → 打 annotated `v1.1.18` 标签 → 验证四项公开资产 → 跑真实 `v1.1.17 → v1.1.18` 更新 → 最后把 README 从候选改为公开稳定版。

## 2026-09-01 v1.1.18 暂停与换机交接（历史，已由上方实测接续点取代）

### 当前状态

- 当前开发分支为 `codex/special-compression-navigation`，目标主线为 `master`，继续使用 [PR #109](https://github.com/Longyuyeee/long_Decompress/pull/109)，不要从主线重做。公开稳定版仍是 `v1.1.17`；README 有意写作“`v1.1.18` 候选，公开稳定版仍为 `v1.1.17`”。当前没有 `v1.1.18` 标签或公开 Release。
- 产品负责人批准将图片、视频、PDF 从压缩中心移出；左侧新增唯一“特殊压缩”入口，内部保留三种选择，压缩中心只做归档文件压缩。这是对 2026-08-26 原始界面路线的正式替换，不是无意偏移。统一 `compression/{archive,image,video,pdf}` 任务/历史、取消、容量预检、暂存校验和原子发布边界不变。
- 功能提交 `81c2649` 与候选准备提交 `54b3f5e` 已在远端分支。页面纠偏已通过 270/270 单元、生产构建、17 文件媒体架构门禁，以及图片、视频、PDF 三类隔离 Windows Release 真实桌面门禁；八处 `1.1.18` 版本身份、README、Release Notes、Rust/Shell 全回归也已完成。
- PR #109 首轮 CI run `33426078666`（头提交 `54b3f5e096bb7b87d22ea5500571b2e0a5b0b0a7`）中 Frontend、Rust/Shell、Windows desktop 三项通过；Browser shell E2E 因旧断言仍要求 7 个导航而失败；Windows installer 按依赖跳过。浏览器断言现已纠正为 8 个，并直接验证“特殊压缩”入口；本地 Chromium 已从头通过 9/9。推送本交接提交后，以 PR #109 最新头提交的新一轮 CI 为准，不复用首轮失败结果。
- 本机 Tauri 生产主程序编译成功，但本机 NSIS 在启动 `makensis.exe` 时以 `系统找不到指定的文件 (os error 2)` 失败。该本地产物不是候选安装器，不得复制到新电脑、安装或发布；精确 NSIS 必须来自 PR 最新提交的干净 GitHub Windows Runner `windows-nsis-installer` artifact。
- Windows N 按产品授权暂不保证支持，不再是 `v1.1.18` 实机发布门禁；不得把普通 Windows 的结果写成 Windows N 支持证据。

### 新电脑恢复与严格接续顺序

1. 在仓库执行 `git fetch origin`、`git switch codex/special-compression-navigation`、`git pull --ff-only`，确认 `git status --short --branch` 干净；打开 PR #109，核对最新 head 与 CI，不要固定使用上面的首轮 head。
2. 必须等 PR 最新头提交的五项 CI 全绿：Browser shell E2E、Frontend checks、Rust and shell-extension checks、Windows desktop E2E build、Windows installer。若任何一项失败，先按真实原因修复、补审计并重新推送；不得合并。
3. 从该次全绿 run 回下载 `windows-nsis-installer`，记录 run ID、精确 head、安装器文件名/字节/SHA-256；用正式 7-Zip 运行时验证 NSIS 完整性，并复核包内主程序 ProductVersion/FileVersion、主程序 SHA-256、唯一 `long_compress_shell_extension_1_1_18.dll` 的字节与 SHA-256。不得引用本机失败 NSIS 或另一提交的 artifact。
4. 修改系统安装前，先复核当前公开 `v1.1.17` 基线、安装位置、进程、用户数据、经典菜单和自动启动。上一台电脑结束时公开版安装在 `E:\long\Long解压` 且进程为 0，但新电脑必须采集自己的实际基线，不能照抄该路径或状态。
5. 对精确候选运行 `npm.cmd run test:installed-release -- -PreviousInstaller <v1.1.17公开NSIS> -CandidateInstaller <CI-v1.1.18-NSIS> -PreviousVersion 1.1.17 -CandidateVersion 1.1.18 -RunArchiveWorkspaceMatrix -RunImageWorkspaceMatrix -RunVideoRuntimeMatrix -RunPdfRuntimeMatrix -RunVideoWorkspaceMatrix -RunPdfWorkspaceMatrix`。必须验证生产包没有 E2E bridge、左侧恰有 8 个入口、压缩中心没有图片/视频/PDF、特殊压缩内有三种工作区，并完成三类真实流程、历史/重启、默认应用、候选卸载与公开基线恢复。
6. 将安装器、包内身份、安装生命周期及恢复证据写入 [RELEASE_AUDIT_1.1.18.md](RELEASE_AUDIT_1.1.18.md)，审计并推送；等待新 head 五项 CI 再次全绿后，才允许合并 PR #109。
7. 合并后拉取 `master`，确认合并提交与 PR 最终文件树一致；只在该合并提交创建 annotated `v1.1.18` 并推送标签。等待 Release workflow 成功，核验 NSIS、updater `.nsis.zip`、`.sig`、`latest.json` 四项资产，逐项记录字节/SHA-256、签名、URL、版本与包内身份。
8. 运行 `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/test-public-update.ps1 -PreviousVersion 1.1.17 -TargetVersion 1.1.18`，完成真实公开应用内更新。最后把 README 从“候选”改成公开稳定版、补齐最终审计并推送；在这些步骤全部完成前不得宣称 `v1.1.18` 已发布。

完整功能证据见 [SPECIAL_COMPRESSION_NAVIGATION_AUDIT.md](SPECIAL_COMPRESSION_NAVIGATION_AUDIT.md)，候选与发布门禁见 [RELEASE_AUDIT_1.1.18.md](RELEASE_AUDIT_1.1.18.md)。本轮在浏览器门禁修复和交接文档推送后暂停，不继续合并、打标签或发布。

## 2026-09-01 v1.1.17 PDF 安全优化正式发布关闭

- D-01 至 D-04 已关闭并经 PR #106 合入 `master`；收口提交 CI run `33412364228` 五项全绿。
- package、Cargo、Tauri 与 Shell Extension 的八处版本身份已同步提升为 `1.1.17`；唯一版本化 DLL 为 `long_compress_shell_extension_1_1_17.dll`，本地 Release 构建 246,784 B、SHA-256 `59646AF395A422192E78B2F1EE1EFB637A7048C6394F9FDD98444EE89C32DD4A`。
- PR #107 已经完整 CI 合入受保护主线；annotated `v1.1.17` 标签、四项公开资产、回下载与真实应用内更新全部完成。`v1.1.17` 现为公开稳定版。详见 [RELEASE_AUDIT_1.1.17.md](RELEASE_AUDIT_1.1.17.md)。
- 精确候选提交 `c51ba38` 的 CI run `33414816383` 五项全绿；NSIS 19,317,221 B、SHA-256 `B0E9AA641F755325839A3AD82EEE8CCD62614FE151912D60FA888888E826BA80`，包内主程序和唯一 `1.1.17` Shell DLL 身份正确。
- 正式安装生命周期 54/54、无测试桥 PDF 工作区 23/23；公开更新 24/24。最终机器安装公开 `v1.1.17` 于原路径，数据、菜单和自动启动保持，运行进程为 0；下一开发节点需重新依据总路线立项。

## 2026-09-01 D-04.3 完整矩阵与正式安装态关闭

- 真实产品命令现覆盖 11 类 PDF × 两种模式，另含签名/加密两类稳定阻断，共 24 个用例、差异 0；新增中文字体、300 页、6000×4000 图片夹具。
- Poppler 人工可见抽查已通过；透明源及两种输出渲染哈希完全一致。隔离 Windows Tauri 已完成真实取消、首项失败不中断后续两项、真实指标、默认阅读器和四条历史完整重启。
- 提交 `81c03b9` 的 CI run `33410249727` 五项全绿；同提交 NSIS 为 19,313,019 B、SHA-256 `2a703383ea1dd60bf69c59692a480452f5ae1ffee1779c4aaff8573ea9466304`。生产包明确不含测试 bridge，通过真实 Tauri IPC 完成取消、2 完成/1 失败隔离、输出重开、默认阅读器、4 条历史重启及签名/加密阻断。
- 正式安装生命周期 53/53、PDF 工作区 23/23；候选卸载后公开 `v1.1.16`、安装路径、用户数据、经典菜单和自动启动均恢复。D-04.3 与 D-04 总节点关闭，下一唯一接续点为 `1.1.17` 提版、正式资产、Release Notes、公开发布/更新和回下载验证。详见 [D04_3_PDF_FULL_MATRIX_AUDIT.md](D04_3_PDF_FULL_MATRIX_AUDIT.md)。

## 2026-08-31 D-04.2 PDF 统一任务与真实桌面闭环完成

- PDF 页面已接入统一 `compression/pdf` 任务、共享取消、终态历史、实测输入/输出/页数、批内安全目标规划、显式保留较大结果和默认阅读器；签名及密码已验证的加密 PDF 都继续只分析。
- 真实 Windows Tauri 使用正式 qpdf 完成表单 PDF 转换/验证/原子发布/历史/默认阅读器，三个源哈希不变，4 组差异为 0，双尺寸横向溢出为 0；真实产品命令继续为 6 组差异 0。
- 全量 Rust 首轮复现交接文档登记的加密 7Z 错误密码偶发误分类；已按实际底层错误族纠正并新增测试，真实场景连续 10/10，第二轮主库 377/0/10 且其余集成测试通过。
- 下一唯一接续点为 D-04.3 完整内容/结构/大文件及正式安装态批量、失败、取消、重启历史矩阵。D-04.3 关闭前保持 `1.1.16`。详见 [D04_2_PDF_TASK_UI_AUDIT.md](D04_2_PDF_TASK_UI_AUDIT.md)。

## 2026-08-31 D-04.1 PDF 产品命令管线完成

- D-03 内部安全发布事务现已由单一产品命令 `compress_pdf_file` 调用；每次执行重新核验 qpdf 和源 PDF，复用统一取消注册表，只报告转换、验证、发布三个事实阶段，并返回原子发布后的真实输出。
- 默认仍拒绝大于输入的结果；为对齐最初需求，后端新增仅由产品请求显式启用的“仍保留较大结果”策略。签名与加密 PDF 继续禁止执行，不把正确密码分析误写成可安全保留加密的转换能力。
- 真实产品命令 6 组 expected/actual 差异为 0；Rust 主库 375/0/10，其他集成测试、严格 Clippy、前端 267 项、类型、PDF 合同与媒体架构门禁均通过。
- 本步仍不创建任务/历史，PDF 页面也尚未接执行按钮。下一唯一接续点为 D-04.2 统一批量任务、取消、结果历史与默认阅读器；之后 D-04.3 才做完整真实/安装态矩阵。详见 [D04_1_PDF_COMMAND_PIPELINE_AUDIT.md](D04_1_PDF_COMMAND_PIPELINE_AUDIT.md)。

## 2026-08-31 D-03.3.1 低容量卷门禁完成，D-03 关闭

- PR #103 提交 `03cd16d5e844c5133a79450a5d0f49e34123dce8` 的 CI run `33379106430` 五组门禁全部通过。Windows Runner 在 `RUNNER_TEMP` 创建并挂载 96 MiB NTFS VHD，真实调用 PDF 发布事务后稳定返回 `PDF_TRANSFORM_RESOURCE_PREFLIGHT_BLOCKED`；最终输出不存在、暂存数为 0、源 SHA-256 不变，VHD 成功 detach。
- Rust 主库为 374 通过、0 失败、9 条按外部条件忽略；本节点没有新增产品命令、任务、历史或 UI 执行。合同已把 `controlledLowCapacityVolumeEvidence` 固化为 true，D-03 至此关闭。
- 当前版本仍为 `1.1.16`。下一唯一接续点是 D-04：用现有统一任务/历史/取消/输出事实架构接入 PDF 产品执行，再完成批量、失败、默认 PDF 阅读器和正式安装版矩阵。D-04 完成前不得提升或发布 `1.1.17`。详见 [D03_3_1_LOW_CAPACITY_CLOSEOUT_AUDIT.md](D03_3_1_LOW_CAPACITY_CLOSEOUT_AUDIT.md)。

## 2026-08-31 D-03.3.1 暂停与换机交接（历史，已由顶部收口节点取代）

- 当前开发分支为 `codex/pdf-d03-3-1-low-capacity-gate`，继续使用 [PR #103](https://github.com/Longyuyeee/long_Decompress/pull/103)，不要从 `master` 重做。
- 96 MiB NTFS VHD 的真实低容量产品事务已在 CI run `33378117324` 通过：主库 374/374，通过后成功 detach；但严格 Clippy 随后因两处 `needless_borrow` 失败，因此 PR 总门禁尚未完成。
- 两处 Clippy 修正已随本次交接准备推送，换机后第一步是检查 PR #103 最新 CI。合同仍保持 `controlledLowCapacityVolumeEvidence=false`；只有最新提交全绿后才能改为 true、关闭 D-03 并进入 D-04。
- 版本继续为 `1.1.16`。D-04 产品命令、任务/历史、批量和安装版验收未开始，所以不得打包或发布 `v1.1.17`。完整命令、两轮差异和严格接续顺序见 [D03_3_1_LOW_CAPACITY_MACHINE_HANDOFF_2026-08-31.md](D03_3_1_LOW_CAPACITY_MACHINE_HANDOFF_2026-08-31.md)。

## 2026-08-31 D-03.3 PDF 内部安全发布核心完成

- `pdf_publish.rs` 已将规范化跨任务输出锁、D-03.1 转换、D-03.2 验证、发布前取消/源 SHA-256/候选 SHA-256 复核、Mark-of-the-Web 和同目录原子重命名绑定为单一内部事务。
- 8 类 PDF × 2 模式均产生真实最终 PDF；验证 SHA-256、最终文件 SHA-256 和独立 pypdf 结构事实一致。连同源/候选变化、目标竞争、验证后取消、锁释放和 ADS 共 29 组预期—实际差异为 0。
- 仍没有 PDF 产品执行命令、任务或历史。当前非管理员机器没有 `New-VHD`/`Mount-VHD`，不得使用填满用户盘或 mock 错误冒充磁盘不足。
- 下一唯一接续点为 D-03.3.1：在管理员 CI/隔离 Windows 机使用受控 VHD/配额卷取得真实低容量失败证据。完成后关闭 D-03，再进入 D-04。版本保持 `1.1.16`。详见 [D03_3_PDF_SAFE_PUBLICATION_AUDIT.md](D03_3_PDF_SAFE_PUBLICATION_AUDIT.md)。

## 2026-08-31 D-03.2 PDF 候选验证与失败矩阵完成

- 新增内部 qpdf 候选验证：只接受 `--check` 退出码 0，并比较页数、加密、MediaBox、表单身份、注释页/类型、书签标题/目标页和附件名称/字节/SHA-256；仍不发布、不注册命令、不写任务历史。
- 8 类真实 PDF × 2 模式由产品 qpdf 验证和独立 pypdf 双重对账；损坏候选、目标竞争、变大拒绝、启动后取消和损坏输入一并验证，共 23 组 expected/actual 差异为 0。
- 首轮把两个夹具名按语义写错，真实独立检查器报文件不存在；按 manifest 实际名称纠正后完整复跑，没有删减断言。真实低容量卷不能在用户系统盘上危险制造，继续列为发布前门禁。
- 该节点关闭时下一点为 D-03.3；D-03.3 内部安全发布核心现已由文档顶部节点完成，当前从 D-03.3.1 接续。D-03.2 证据见 [D03_2_PDF_CANDIDATE_VALIDATION_AUDIT.md](D03_2_PDF_CANDIDATE_VALIDATION_AUDIT.md)。

## 2026-08-31 D-03.1 PDF 内部暂存执行基础完成

- 新增内部 Rust 暂存执行器：固定参数 qpdf 两模式、共享容量预检、取消/600 秒超时、同目录暂存、Drop 自动清理和源 SHA-256 前后复核；没有注册 Tauri 产品命令，没有最终发布、任务或历史。
- 真实文本/图文 PDF 分别执行两模式，页数保持 1、源哈希不变、最终目标不存在；签名、加密和预启动取消稳定拒绝，结构化 expected/actual 差异为 0。样本变小仅为观察值，不构成压缩率保证。
- 真实 qpdf 探针证明保持加密需要命令行密码参数，与凭据边界冲突，因此加密执行继续阻断，没有为了“可用”降低安全要求。
- 该节点关闭时下一点为 D-03.2；D-03.2 已关闭且 D-03.3 内部安全发布核心已完成，当前从 D-03.3.1 接续。D-03.1 证据见 [D03_1_PDF_STAGING_FOUNDATION_AUDIT.md](D03_1_PDF_STAGING_FOUNDATION_AUDIT.md)。

## 2026-08-30 当前开发与换机接续审计

- 审计基线为 `master` / `e3282ddb8f00b6f12d091c2720e930768effea64`；审计开始时与 `origin/master` 差异 0/0、工作区干净，产品版本仍为 `1.1.16`。
- 实际代码确认 D-01 和 D-02 已关闭：产品以固定参数 qpdf 返回结构化真实事实，并已开放“无损整理/兼容图片优化”的执行前风险配置 UI；配置仅为页面内草稿，没有转换命令、任务或历史。下一唯一功能接续点是 D-03.1 执行事务基础。
- 换机证据分为三类：Git 已跟踪的合同/脚本/运行时/审计、GitHub Actions/Release 可回查证据，以及被忽略的本机 `test-results` 原始产物。后者含绝对路径、安装副本和用户数据指纹，不推送且不作为换机开发的必要输入；D-04/发布必须在届时候选上重新取证。
- 新电脑的工具链、最小复验命令、原始证据摘要、未完成风险和旧 PR #91 的非接续状态见 [CURRENT_DEVELOPMENT_AND_MACHINE_HANDOFF_AUDIT_2026-08-30.md](CURRENT_DEVELOPMENT_AND_MACHINE_HANDOFF_AUDIT_2026-08-30.md)。

## 2026-08-30 D-01.2.2 安装态与签名 updater 增量完成

- GitHub 同提交双构建证明 qpdf 10 文件使 NSIS/updater 各增加 3,603,012 B；两份 updater 完整，包内 NSIS 与独立 NSIS 字节一致。运行 `33318192852`，测量提交 `a27ecc0`。
- 正式 CI 候选安装后，qpdf 10 文件、版本、crypto、JSON v2、图片优化能力全部通过；隔离副本的缺失和替换稳定拒绝。候选卸载、公开 v1.1.16、用户数据、菜单和自启动恢复最终 49/49 通过。
- 第一次恢复遇到用户目录瞬时重建并按安全策略失败，自动恢复成功；第二次完整通过。旧临时备份因主机安全策略拒绝删除而保留，没有改写当前数据。
- 该节点关闭时 D-01/D-02 已完成、下一点为 D-03.1；D-03.1 现已由文档顶部节点关闭，当前应从 D-03.2 接续。D-02.2 的真实 Windows Tauri 表单/签名/密码/源哈希/布局门禁差异为 0，历史证据见 [D02_2_PDF_RISK_CONFIGURATION_AUDIT.md](D02_2_PDF_RISK_CONFIGURATION_AUDIT.md)。

## 2026-08-30 D-01.2.1 qpdf 正式资源与生产预检完成

- qpdf 12.4.0 官方 MinGW64 五文件子集已经从测试候选提升为仓库正式资源；二进制 12,637,211 B，连同 Apache-2.0、NOTICE、GCC/MinGW notice 与来源说明共 10 文件、12,765,477 B，全部锁定字节和 SHA-256。
- 新增与产品共用的生产预检：先逐字节核验资源，再验证版本 12.4.0、OpenSSL/native、JSON v2 与图片优化能力；缺失或替换均在启动 qpdf 前失败关闭。Tauri 目前只开放身份预检，不开放 PDF 优化命令或 UI。
- 首轮探针与 qpdf 实际帮助输出不符，已按 DCT/JPEG 与三个 `--oi-min-*` 事实纠正；意外触发的全仓 Rust 格式化漂移也已在提交前精确撤回，只定向格式化新增文件。
- 首轮干净 CI 又发现 qpdf 许可文本会被 Windows checkout 转换为 CRLF 并破坏固定字节；根 `.gitattributes` 已将整个 `pdf-engine` 设为 `-text`，与既有 video-engine 的逐字节策略一致。
- Rust 4/4、严格 Clippy、真实依赖、PDF 契约、媒体架构、类型、前端 284/284 覆盖率和生产构建均通过；Release 主程序内部预检报告也以 10 文件身份通过。下一唯一接续点为 D-01.2.2：同提交正式 NSIS/updater 精确增量、安装态完整/缺失/替换预检及公开版本恢复。详见 [D01_2_1_QPDF_RUNTIME_ADMISSION_AUDIT.md](D01_2_1_QPDF_RUNTIME_ADMISSION_AUDIT.md)。

## 2026-08-30 D-01.1 qpdf 能力契约与 PDF 样本基线完成

- 实际代码确认 qpdf 12.4.0 官方 Windows x64 候选、上游校验文件和五文件运行时子集此前已经锁定，但仍为 `integrationAllowed=false`；旧样本只有文本、扫描、透明、表单、签名和加密六类，缺少原路线要求的图文混合、注释、书签和附件。
- 新增只接受结构化白名单的“无损整理”和“兼容图片优化”契约；锁定对象流、通用流重压缩、Flate 重压缩、压缩等级及图片优化阈值，禁止原始 qpdf 参数、源文件改写、Ghostscript 和半成品 UI。签名保持只分析，加密必须先取得正确密码。
- 合成真实 PDF 扩为 10 类。8 个可执行样本在两种模式下均通过 qpdf 12.4.0/OpenSSL `--check` 和独立 pypdf 对账；页数、页面尺寸、可搜索文本、表单字段/值、注释内容、书签目标和附件字节保持一致。签名字段被实际识别并阻止执行；加密文件无密码返回 `invalid password`，正确密码检查通过。
- 首轮脚本只比较 qpdf 页数与结构名称，审计认为弱于原需求；现已增加独立检查器复核附件 SHA-256、书签页目标、注释内容、表单值、页面尺寸和文本，并从头复验通过。现有单测/覆盖率 pretest 已接入静态 PDF 合同门禁。
- 本节点不升版、不把 qpdf 放入产品资源、不新增后端命令或 UI。下一唯一接续点为 D-01.2：正式资源与许可载荷、生产身份/能力预检、缺失替换拒绝和同提交 NSIS/updater 精确增量。详见 [D01_1_QPDF_CAPABILITY_AND_FIXTURE_BASELINE_AUDIT.md](D01_1_QPDF_CAPABILITY_AND_FIXTURE_BASELINE_AUDIT.md)。

## 2026-08-29 v1.1.16 候选身份与本地发布门禁完成

- 8 个版本源统一提升为 `1.1.16`，唯一 Shell Extension 为 246,784 B / `FDE0C001…92A7D`；视频安装包校验脚本改为从 Tauri 实际版本推导默认 NSIS 路径，不再硬编码 `1.1.15`。
- 发布合同偏移已纠正：公开输出只承诺 MP4/H.264，H.265 保留为真实输入用例；Windows N 明确暂不支持且真实证据状态保持 false。
- 类型检查、45 文件 262/262 前端单测、生产构建、Rust Release 全目标、全特性严格 Clippy、Shell Extension 5/5、媒体门禁及生产依赖审计全部通过。
- 首轮干净 CI `33261161074` 的 Browser、Frontend、Windows desktop Release build 通过，Rust 361/362 因 FFmpeg 官方允许的 `out_time_us=N/A`/有符号启动时间被旧 `u64` 解析器拒绝而失败，安装器按依赖未运行。现已按冻结 FFmpeg 源码契约接受明确未知值、将负启动时间显示为 0，其他畸形值仍失败关闭；视频 Release 42/42 和严格 Clippy 通过，等待新 CI 复验。
- 当前允许推送候选并运行干净 CI/正式 NSIS 安装复验；尚未允许标签和公开 Release。完整待办见 [RELEASE_AUDIT_1.1.16.md](RELEASE_AUDIT_1.1.16.md)。

## 2026-08-29 C-05 Windows N 支持范围变更并关闭功能节点

- 产品负责人明确批准取消 Windows N 实机发布门禁，并将 Windows N 调整为暂不保证支持。本次变更只缩小 `v1.1.16` 支持范围，不把 Professional 主机结果伪装成 Windows N 通过。
- `windowsNRealMachinePassed` 保持 `false`；发布合同明确只支持 `windows-x86_64-non-n`，Windows N 实机脚本、独立验收器和生产 Media Foundation 缺失拒绝均完整保留。
- C-05.1 至 C-05.4.1 的真实功能与正式安装证据已经满足缩小后的发布范围，C-05 功能节点关闭。下一接续点为 `v1.1.16` 版本身份、候选构建、安装态复验、公开 Release 与回下载应用内更新；所有公开材料必须注明 Windows N 暂不支持。
- 变更依据、未验证事实和未来重新纳入条件见 [C05_4_2_WINDOWS_N_SCOPE_CHANGE_AUDIT.md](C05_4_2_WINDOWS_N_SCOPE_CHANGE_AUDIT.md)。

## 2026-08-29 C-05.4.2 Windows N 门禁准备完成

- 实际代码审计发现旧 Windows N 脚本只锁候选 EXE、假定安装由人工完成，不能证明目标机使用了正式 NSIS。现要求前阶段校验锁定安装器身份、干净安装基线，并由脚本执行 `/P /NS /NR` 正式安装；独立验收器同时复核安装器和主程序。
- 工具提交固定为 `b717f973a79035bbf1a475885d84f493de398906`；候选仍为 `71a9572` / CI run `33258733949`。前后阶段不得拉取不同脚本，后阶段会以 SHA-256 拒绝生成器变化。
- Professional 主机负向自检稳定返回 `WINDOWS_N_MACHINE_REQUIRED`，安装器未执行，公开 `v1.1.15` 版本/位置不变且无进程。PowerShell AST、Node 语法、真实媒体依赖、媒体架构和发布门禁通过。
- 本段保留范围变更前的工具准备事实。后续产品授权已把 Windows N 排除出 `v1.1.16` 支持范围，因此该实机矩阵不再阻塞发布；工具继续保留供未来恢复支持时使用，见顶部范围变更节点。

## 2026-08-29 C-05.4.1 正式安装生命周期完成

- 修正提交 `71a9572` 的 CI run `33258733949` 全绿；NSIS 为 15,604,389 B / `C4FF2374…D718`，解包及安装主程序均为 28,853,760 B / `0B443647…5EF0`，8 项视频运行时共 24,631,334 B 且差异为零。
- 正式安装生命周期 50/50，其中视频工作区 20/20：真实 109.52 MiB 输入完成取消无残留、MP4/H.264 1280×720 32 秒发布、默认应用和完整重启历史；候选卸载、用户数据保持及公开 `v1.1.15` 恢复全部通过。
- C-01 历史候选身份继续保留；新增 `c05InstalledCandidate`，Windows N 生产脚本和独立验收器只接受本次候选，避免用旧二进制取得新阶段证据。
- 本段记录范围变更前的状态：C-05.4.1 当时已关闭而 C-05 仍等待 Windows N。后续产品授权已由顶部节点将 Windows N 排除出 `v1.1.16` 支持范围，当前接续点不再是实机 N 门禁。正式安装证据见 [C05_4_1_INSTALLED_VIDEO_LIFECYCLE_AUDIT.md](C05_4_1_INSTALLED_VIDEO_LIFECYCLE_AUDIT.md)。

## 2026-08-29 C-05.4.1 正式安装资源布局纠偏

- 提交 `2b45350` 的干净 GitHub Windows Runner 已完整通过 CI 并生成 NSIS；包内 8 项视频运行时资源和安装态 CLI 预检/真实软件转码均通过，但真实安装 UI 以错误的安装根目录寻找 `video-engine/ffmpeg.exe`，首轮生命周期按设计失败并恢复公开版。
- 实际 NSIS 布局为 `resources/video-engine`。现新增唯一 `bundled_resource_root`，让预检、输入探测、规划和执行四个生产入口共同保留配置中的 `resources/` 前缀；架构门固定该约束，消除“CLI 通过、UI 失败”的双路径漂移。
- 现场基线意外为 `v1.1.13`，已用哈希匹配发布审计的公开 `v1.1.15` 恢复；安装位置和两处用户数据指纹不变。修正后无测试桥 Release UI 20/20、视频引擎 5/5、媒体架构、类型、生产构建和 Clippy 均通过。
- 本段记录首轮纠偏过程；后续修正候选生命周期已通过并关闭 C-05.4.1，见顶部完成节点。旧候选仍不得复用。

## 2026-08-28 C-02 完成：真实分类与 Windows 桌面矩阵

- 产品 FFmpeg 对冻结输入纯流复制，现场生成 30 秒 ENG/ZHO 双音轨+字幕及 10 分钟真实 MP4；生产探测 7/7、C-01/C-02 视频测试 17/17 通过，不提交大二进制、不使用系统 FFmpeg。
- 真实 Release Tauri/WebView2 完成系统选择、生产预检、两输入规划、三档重算、估算/字幕/额外音轨提示、执行冻结、历史零写入、模式保持和 1100×720/760×560 无横向溢出。
- 桌面门禁实际纠正缺 EdgeDriver、standalone feature/资源布局和窄窗口 171 px 溢出，未降低断言；独立资源副本仍由生产大小/SHA-256 校验。
- C-02 已关闭，唯一接续点为 C-03 执行、进度与取消；C-04 验证前不发布输出，C-05/Windows N 前后实机门禁完成前不发布 v1.1.16。详见 [C02_4_VIDEO_REAL_MATRIX_AUDIT.md](C02_4_VIDEO_REAL_MATRIX_AUDIT.md)。

## 2026-08-28 C-02.3 视频探测与配置工作区

- 压缩中心视频 Tab 已从计划占位升级为真实探测/规划工作区：批量和单项三档/最大分辨率调用后端唯一规划命令，展示输入事实、强标记估算和中文流变化。
- 视频草稿复用现有 compression store 的隔离集合；未知扩展交由真实容器探测，目录明确拒绝；revision 防止旧异步规划覆盖新设置。
- 执行按钮始终禁用，源码和架构门禁禁止创建任务或调用视频编码；归档队列、图片草稿和统一历史均不受污染。
- 下一接续点严格为 C-02.4 冻结真实多音轨/较长输入及 Windows Tauri/WebView2 矩阵。通过后才关闭 C-02 并进入 C-03。证据见 [C02_3_VIDEO_PLANNING_WORKSPACE_AUDIT.md](C02_3_VIDEO_PLANNING_WORKSPACE_AUDIT.md)。

## 2026-08-28 C-02.2 视频配置与估算模型

- 后端新增清晰/均衡/小体积三档规划器，按可见方向设置默认最大尺寸，允许受 UHD 面积和 3840 单边限制的成对自定义宽高；输出只缩小、不放大，并采用等比偶数尺寸舍入。
- 目标视频码率由输出像素、平均帧率和档位包络派生；有音频时规划 192/128/96 kbps AAC，无音频时保持 `null`。估算区间强制返回 `isEstimate=true`、依据和偏差说明。
- 规划结果在执行前汇总容器/编码、尺寸、旋转、VFR、音频以及所有丢弃/阻断变化；字幕等有损变化要求显式确认，HDR 令规划不可编码。
- 当前唯一接续点是 C-02.3 不可执行视频工作区和剩余真实分类矩阵；不能因 `canEncode` 字段存在就提前开放执行。完整证据见 [C02_2_VIDEO_COMPRESSION_PLAN_AUDIT.md](C02_2_VIDEO_COMPRESSION_PLAN_AUDIT.md)。

## 2026-08-28 C-02.1 视频输入事实与首期流策略

- 新增有界产品 ffprobe 服务和唯一 Tauri 命令；每次探测先复用 C-01 运行时校验，再以参数数组执行，限制 20 秒、8 MiB 元数据和 2,048 字符错误详情。
- 结构化返回编码/可见尺寸、旋转、VFR 保守分类、时长、码率、视频/音频/字幕流、章节、封面和 HDR。额外音轨、字幕、章节、封面必须显式确认丢弃；HDR 在首期编码前拒绝。
- 两个冻结真实 MP4、损坏容器、空文件和合成多流/HDR 矩阵共 5 项 Rust 测试通过；类型检查及 11 文件媒体架构门禁通过。
- 当前仍无可用视频 UI、转码、进度、取消和发布。唯一接续点为 C-02.2 三档配置、最大分辨率、派生尺寸及明确标记的估算区间，详见 [C02_1_VIDEO_PROBE_FACTS_AUDIT.md](C02_1_VIDEO_PROBE_FACTS_AUDIT.md)。

## 2026-08-28 C-01 完成与门禁归位

- 生产视频预检现验证 System32 中 `mfplat.dll`、`mf.dll`、`mfreadwrite.dll`，Windows N 无 Media Feature Pack 统一返回稳定错误；状态明确返回 Media Foundation 可用事实。
- 正式应用新增无 UI 的内部安装审计入口，严格从当前 EXE 同目录解析资源并复用生产预检。安装生命周期脚本新增视频矩阵：安装目录内真实软件转码与 ffprobe 复核，缺失/替换只在隔离副本验证。
- 同布局隔离验证已通过；旋转产品夹具的实际输出为 480×854、1.2 秒，已纠正沿用 C-01.1 临时横屏 5 秒样本的错误预期。
- GitHub Actions `33173219785` 在提交 `6b95f5c` 上完成同提交、同工具链、同 updater 密钥的双构建：视频资源令 NSIS 和 updater ZIP 均精确增加 `6,821,970 B`，两侧包内 NSIS 字节一致且 updater 签名结构有效。
- 正式安装生命周期已通过：本机 v1.1.13 覆盖到候选 v1.1.15 后，从真实安装目录完成生产预检、软件转码、ffprobe 复核及隔离的缺失/替换拒绝；候选卸载、用户数据保持、v1.1.13 与菜单状态恢复全部通过。
- `test-windows-n-video-runtime.ps1` 已把剩余实机门禁固化为前后两阶段：真实 N/无 Media Feature Pack 必须被生产预检拒绝；安装组件并重启后，必须在同一机器通过生产预检与真实安装态转码。当前普通专业版会以 `WINDOWS_N_MACHINE_REQUIRED` 拒绝，不能误充证据。
- `verify-windows-n-video-runtime-evidence.mjs` 独立复核 schema 2 的前后报告，并以 SHA-256 把前报告字节链入后报告；候选、生成脚本、机器、生产预检、真实输出和两个资源负向控制任一不一致即拒绝。PowerShell 5 UTF-8 BOM 已兼容，但哈希仍覆盖原始 BOM 字节。
- 历史回溯确认真实 Windows N 实机证据被错误前移：最初 C-01 验收要求哈希/许可/安装态能力/缺失替换拒绝和体积；“缺 MF 时明确分类”已经由生产实现与稳定测试满足。真实多平台安装证据归回 C-05 和发布门禁。
- C-01 已关闭，下一接续点为 C-02 探测与配置模型。Windows N 两阶段工具与验收器全部保留，C-05 完成前仍不得发布 v1.1.16。详见 [C01_2_2_INSTALLED_RUNTIME_AND_SIGNED_DELTA_AUDIT.md](C01_2_2_INSTALLED_RUNTIME_AND_SIGNED_DELTA_AUDIT.md)。

## 2026-08-28 C-01.2.1 合并后当前状态审计

- `master` 与 GitHub 同步在 `7bc44a1`；公开版本仍为 `v1.1.15` / `82b1b8f`。主分支比标签多 4 个提交，公开 v1.1.15 资产不包含标签之后准入的 FFmpeg，不能把开发树能力冒充已发布能力。
- A 归档、B-00 公共边界和 B 图片压缩均已完成；视频只完成 C-01.1 与 C-01.2.1。当前只有后端 FFmpeg 资源/能力预检，前端没有调用封装，也没有探测模型、转码服务、进度取消或可用视频 UI。
- PR #90 五组 CI 全部通过；官方 npm registry 生产依赖审计为 0 个已知漏洞。默认 npmmirror advisories API 首次返回 404，已按真实失败记录后切换官方源复验。
- 唯一接续点为 C-01.2.2：正式安装目录真实执行、缺失/替换拒绝、Windows N 分类及同提交签名 NSIS/updater 精确差值。完成前不进入 C-02、不启用视频入口、不升版。完整证据见 [CURRENT_DEVELOPMENT_AUDIT_2026-08-28_POST_C01_2_1.md](CURRENT_DEVELOPMENT_AUDIT_2026-08-28_POST_C01_2_1.md)。

## 2026-08-28 C-01.2.1 视频运行时准入

- C-01.1 的 FFmpeg 9.0.1 可复现候选已进入 Tauri 产品资源；8 个二进制、来源/配置和 FFmpeg/MinGW/GCC 许可文件总计 24,631,334 B，后端在启动进程前逐项验证大小与 SHA-256。
- 生产预检真实执行 ffmpeg/ffprobe，固定 LGPL-only、h264_mf 软件默认、AAC 和必需过滤器；缺失和篡改负向测试通过。视频 UI 仍禁用，版本仍为 `1.1.15`。
- 已移除会轮换的 BtbN nightly 依赖，改为仓库跟踪且逐字节锁定的两个真实 MP4；完整 11 图片/2 视频/6 PDF 夹具由产品 ffprobe 验证通过。
- 未签名 NSIS 包内 8/8 资源、能力和真实 MP4 探测差异为 0；当前包 15,554,236 B，相对父提交 CI 包聚合增加 6,895,417 B。该数不是正式签名/updater 精确增量。
- 完整 Rust 首轮暴露 watch-folder 测试线程调度竞态（预期 1、实际 2），改为第一次快照后同步改写；定向 10/10、完整 322/322 通过。下一接续点严格为 C-01.2.2：正式安装目录执行、Windows N 分类、替换拒绝和同提交签名 NSIS/updater 精确测量。详见 [C01_2_1_VIDEO_RUNTIME_ADMISSION_AUDIT.md](C01_2_1_VIDEO_RUNTIME_ADMISSION_AUDIT.md)。

## 2026-08-28 C-01.1 FFmpeg 可复现候选

- FFmpeg `9.0.1` 官方源码的最小 LGPL Windows x64 构建已脚本化；双干净目录产出的 `ffmpeg.exe`（12,349,440 B，`35c3c8bb...8672eb`）与 `ffprobe.exe`（12,131,840 B，`2c1df07c...ba1d98`）逐字节一致。
- 能力只开放 `h264_mf` 软件 H.264、内置 AAC、固定 demux/mux/filter/decoder 集；硬件加速器列表为空，运行路径强制 `hw_encoding=0`，不存在 GPL/nonfree/libx264/libx265/libopenh264。
- Windows 真实 5 秒 MP4/H.264/AAC 输入已由两套最终候选分别转码并由对应 ffprobe 复验：480×270、H.264/AAC、5.000 秒，progress pipe 正常结束，预期与实际语义差异为 0；PE 仅导入 Windows 系统 DLL。
- 开发中真实纠正 D3D11 编译接口、动态 `libwinpthread-1.dll`、跨 Shell 并行参数和 Node `objdump` 缓冲区问题；两次正式复现前的受干扰构建证据已作废，没有混入通过结论。
- 既有完整媒体夹具锁定的 BtbN nightly 资产已被轮换，4 次真实下载均 HTTP 404；C-01.1 使用明确记录版本的测试工具生成真实磁盘输入，没有把新 nightly 偷换成冻结资产。
- FFmpeg 仍为 `integrationAllowed=false`：视频入口未启用，当前版本仍是 `1.1.15`。下一接续点严格为 C-01.2：修复长期视频夹具来源、产品资源与完整许可清单、后端哈希/能力拒绝、Windows N 分类、安装态真实转码及签名 NSIS/updater 精确增量。详见 [C01_1_FFMPEG_REPRODUCIBLE_CANDIDATE_AUDIT.md](C01_1_FFMPEG_REPRODUCIBLE_CANDIDATE_AUDIT.md)。

## 2026-08-28 v1.1.15 正式发布

- 图片压缩 B-01 至 B-05.3 已完成后，8 个版本源统一提升为 `1.1.15`，唯一版本化 Shell Extension 已重新编译；Release notes 与候选审计已建立。
- v1.1.15 正式 NSIS 为 8,691,488 B、SHA-256 `85CFBAD4230D3C1948278B34CFEC6327AC67368BC3730F66F35A8A99DBF8765A`；主程序为 28,400,640 B、SHA-256 `7D11ED9673865B4F9BBF2B617AE215B8413EDEB4317154FFACB84BB78E476E05`，PE 版本 1.1.15，14 项载荷完整。
- 真实 `v1.1.14 → v1.1.15 → 卸载 → v1.1.14` 已通过：候选 EXE 字节一致、图片 17/17、安装生命周期 50/50、两处用户数据、经典菜单 17+4 和旧版恢复均符合预期，最终无运行进程。
- 全回归通过：前端 254/254、集成 6/6；Rust debug 全目标及 Release workflow 同命令均通过（库 319/319、4 项既定忽略）；Clippy 零警告；真实媒体依赖/指标/图片基线、9 样本格式矩阵和资源/故障边界全部通过；npm 生产依赖漏洞为 0。
- PR #87 已合入受保护主分支，`v1.1.15` annotated tag 固定在 `82b1b8f`；Release run 33146766724 的版本身份、真实图片夹具单测、Rust Release、签名 Tauri/updater 构建、identity package 排除和公开 manifest 回读全部通过。
- PR #87 首轮 Browser E2E 通过，但 Frontend coverage 暴露 4 个真实图片夹具 `ENOENT`：本地已有忽略目录掩盖了干净检出的前置条件。`test:unit` 与 `test:unit:coverage` 现都通过 npm 前置生命周期生成并冻结校验真实图片；两次从项目内无夹具目录开始复验分别通过 276/276 和 254/254，类型、生产构建及发布身份也通过。
- 四项公开资产已回下载并对账：NSIS 8,658,170 B / `DBFF77AE...DD2C51`，updater ZIP 8,658,330 B / `DE591FDB...241D00`，`.sig` 428 B / `BE82F7E0...02E2AC`，`latest.json` 950 B / `8265EFE6...3A3E78`；ZIP 内唯一 EXE 与独立 NSIS 字节一致，manifest 签名逐字一致。
- 首轮公开更新在安装前发现恢复测试基线 `autoStart=true` 但 Run 项缺失；没有放宽断言。使用公开 v1.1.14 真实设置 UI 同步后显式开启，生产 Tauri 命令恢复精确注册，再从头重跑通过 24/24。
- 当前机器已由生产 WebView2 更新 UI 从公开 `v1.1.14` 升至 `v1.1.15`：安装位置、自动重启、两处用户数据指纹、自动启动、经典菜单 17+4、唯一 Shell DLL 和无 MSIX identity package 全部符合预期，最终无运行进程。图片大节点与 v1.1.15 发布至此关闭；下一接续点为视频压缩软件编码前置审计。详见 [RELEASE_AUDIT_1.1.15.md](RELEASE_AUDIT_1.1.15.md)。

## 2026-08-28 B-05.3 正式安装版图片全流程

- 新增 `test:installed-image-workspace` 与 `-RunImageWorkspaceMatrix` 安装门禁；正式候选不含桌面 E2E 桥，通过生产 Tauri `Event.emit` 文件投递监听接收真实路径，不直接注入 store、任务或结果。
- 最终安装态图片链 17/17：三个真实 JPEG/PNG/WebP 输入与尺寸、可见质量 67/保持格式/限制尺寸/rename 设置、执行前结果为空、三个真实发布文件、源哈希变化 0、三组前后预览、三条完成历史、完整重启历史和三个输出重新导入解码全部通过。
- 正式 NSIS 为 8,688,052 B、SHA-256 `404A9BC533F64C8688A05B6E08118817902A4B21131D7B19333F7E81A18DBA2C`；14 个载荷完整。覆盖/卸载/公开 v1.1.14 恢复生命周期 50/50，用户数据和菜单恢复，最终无应用进程。
- 真实差异包括全局 Tauri API 不存在、Windows 文件对话框自动化假成功、ready 文案、滚动区点击和 Explorer 标签页 COM 观察不可靠；均按实际结果修正测试口径，没有放松磁盘文件、预览、历史或重启断言。并发 Cargo 首轮出现一次 7Z 分类差异，单项与完整串行复验均通过，未形成稳定产品缺陷。
- B-05.1 至 B-05.3 功能矩阵至此完成，当前仍不升版、不更新 Release；下一接续点严格为 `v1.1.15` 版本身份、Release notes、正式资产/签名、公开更新和回下载复验。完整证据见 [B05_3_INSTALLED_IMAGE_FULL_FLOW_AUDIT.md](B05_3_INSTALLED_IMAGE_FULL_FLOW_AUDIT.md)。

## 2026-08-28 B-05.2.2 图片资源与故障边界

- 新增 `test:image-boundaries:real`，直接调用生产图片服务验证 96 MP 可解码与 100.01 MP 有效 PNG 拒绝、340 UTF-16 中文长路径、skip/rename/replace-if-smaller 和编码期目标竞态、标准 StorageFull 注入、编码启动后取消。
- 最终所有功能预期与实际差异为 0；长路径真实发布且源文件不变，超限/竞态/磁盘满/取消均不留下错误输出或事务暂存。StorageFull 仅在真实解码和编码后的最终写入点安全注入，不冒充物理磁盘耗尽。
- 首轮发现新增观察器参数的多余 `mut` 编译警告及 Pillow 有意大图告警，均已修正并从头复跑。前端 276/276、Rust 319 通过/4 忽略、Clippy、生产构建、媒体门禁和 B-05.1 九样本矩阵全部通过。
- B-05.2 整体已完成；当前图片配置没有删除源文件选项，不得把归档回收能力冒充完成。B-05 整体仍不升版、不更新 Release；下一接续点严格为 B-05.3 正式安装版拖入—配置—对比—执行—历史—重开输出。完整证据见 [B05_2_2_IMAGE_FAILURE_BOUNDARIES_AUDIT.md](B05_2_2_IMAGE_FAILURE_BOUNDARIES_AUDIT.md)。

## 2026-08-28 B-05.2.1 百图真实批处理

- 新增 `test:e2e:desktop:image-batch`：生成 JPEG 34、PNG 33、WebP 33 共 100 个不同的真实磁盘输入，在 Windows Release Tauri/WebView2 中通过可见按钮执行生产批处理。
- 最终实际为 100/100 ready、100/100 completed、100 个唯一发布文件、100 条统一图片历史、源 SHA-256 变化 0，生产批处理耗时 17,059 ms；输出格式分布与预期 34/33/33 完全一致。
- 首轮测试在 100 个输入全部 ready 后因 Selenium `getText()` 不读取视口外摘要而误报；现通过真实“批量设置”按钮收起面板，让摘要可见后断言，并从头复跑完整批次通过。没有修改产品实现或放松结果断言。
- 前端 276/276、Rust 318 通过/4 忽略、Clippy、普通生产构建、媒体门禁和 B-05.1 九样本生产矩阵全部通过。B-05 整体仍未完成，不升版、不更新 Release；下一接续点严格为 B-05.2.2：超大像素、中文长路径、目标冲突、磁盘不足和取消。完整证据见 [B05_2_1_IMAGE_BATCH_AUDIT.md](B05_2_1_IMAGE_BATCH_AUDIT.md)。

## 2026-08-28 B-05.1 三格式真实样本矩阵

- B-05.1 已完成：新增 9 个冻结真实输入，JPEG、PNG、WebP 各 3 个，覆盖小图、大图、透明、EXIF 方向元数据；`test:image-matrix:real` 直接调用生产 `compress_single_image`，不是候选编码器或模拟返回。
- 每个发布文件均重新解码并核对磁盘字节、格式、矩阵、可见尺寸、Alpha 和元数据；PNG 三例像素完全一致，有损样本按冻结的逐样本 PSNR 下限验收，透明 WebP 按白底合成后的可见像素比较且 Alpha 平面必须完全一致。最终 9/9 通过、解码差异 0。
- 首轮真实矩阵发现无元数据透明 WebP 经 `img-parts` 无效重写后丢失 VP8X Alpha 标志，生产复验拒绝 `ALPH` 块；现仅在编码器元数据状态与目标状态不一致时重写容器。同轮还纠正透明隐藏 RGB 被纳入 PSNR 的错误口径，没有降低 Alpha 或可见质量要求。
- B-05 整体尚未完成，不升版、不更新 Release。下一接续点严格为 B-05.2：100 张混合批量、超大像素、中文长路径、冲突、磁盘不足和取消；其后 B-05.3 才做安装版全流程。完整证据见 [B05_1_IMAGE_FORMAT_MATRIX_AUDIT.md](B05_1_IMAGE_FORMAT_MATRIX_AUDIT.md)。

## 2026-08-28 B-04.5 图片真实结果 UI 与 B-04 总收口

- 图片工作区已接入 `useImageCompressionBatch`；按钮按真实 ready 状态开放，运行中提供统一取消，逐项状态和阶段读取 task store/task-log，批量百分比只按终态文件数计算。
- 结果卡从统一 Task 指标展示真实发布路径、输出字节、格式、应用方向后的尺寸和实际字节差；最终路径经后端授权后在 WebView2 加载真实结果预览。失败、取消和 skip 不使用原图或估算伪装结果。
- 第一轮 Windows 真实桌面执行中 JPEG 15,788→7,909 B、WebP 3,884→3,814 B 成功，透明 PNG 因默认 `lossy + keep PNG` 实际失败；现按 PNG 格式能力映射为无损优化并公开说明，复跑得到 PNG 1,546→601 B，三输出、三历史和双尺寸 UI 全部通过。
- 开发中同时修正含失败/取消批次仍显示绿色成功汇总的语义偏差。前端 276/276、Rust 317 通过/4 忽略、Clippy、生产/正式 Tauri 构建、媒体门禁和真实图片基线通过；最终本地正式 NSIS 候选 8,671,911 B，SHA-256 `472E144050AEB50ADD96D2F4995F61168AD624E9EC6845886AF013CE59B17F27`，B-05 前不更新公开基线。
- B-04.1 至 B-04.5 已全部完成；不升版、不更新 Release。下一接续点严格为 B-05 安装版真实验收矩阵，完整证据见 [B04_5_IMAGE_RESULT_UI_AUDIT.md](B04_5_IMAGE_RESULT_UI_AUDIT.md)。

## 2026-08-28 B-04.4 图片统一队列与历史收口

- 每张图片现注册为统一 `compression/image` Task，复用现有 task-log、状态机、取消和唯一 `save_task_history` 写入口；图片批量返回前等待每条终态历史的持久化结果。
- published 使用后端真实 input/output 建立字节和双侧图片指标；候选不更小时以实际保留源文件作为最终事实；failed/cancelled/未编码 skip 不制造媒体指标。
- 真实透明 PNG 完成记录及 failed/cancelled 已写入实际 SQLite，关闭连接后重新打开仍完整读取。开发中发现并修正嵌套 `encodedBytes` 严格字段泄漏和终态守卫阻断密码重试两项偏差。
- 前端 273/273、Rust 317 通过/4 忽略、Clippy 零警告、生产构建、媒体门禁及真实图片基线通过。本节点当时仍不启用按钮；B-04.5 现已由顶部节点完成，下一接续点为 B-05。完整证据见 [B04_4_IMAGE_QUEUE_HISTORY_AUDIT.md](B04_4_IMAGE_QUEUE_HISTORY_AUDIT.md)。

## 2026-08-28 B-04.3 图片安全批量编排收口

- 后端新增真实文件系统目标规划：稳定生成 `.compressed` 名称，rename 同时避让磁盘现有文件和同批预留目标，skip 返回结构化跳过；replace-if-smaller 只映射候选大小策略，既有目标继续失败关闭，未放松 B-03 禁止覆盖事务。
- 前端新增强类型规划/压缩命令封装和逐图批量执行器；每张图使用唯一 task id，进度只按文件终态数计算，取消复用 `cancel_compression` 且不会在规划返回后误启动编码。
- 真实方向 JPEG、透明 PNG 和 WebP 已完成请求映射、冲突、规划后实际编码与取消复核；前端 270/270、Rust 316 通过/4 忽略、Clippy 零警告、生产构建、媒体门禁及真实图片基线通过。
- 本节点要求的 B-04.4 与后续 B-04.5 均已由顶部节点收口；当前下一接续点为 B-05，完整编排证据见 [B04_3_IMAGE_SAFE_ORCHESTRATION_AUDIT.md](B04_3_IMAGE_SAFE_ORCHESTRATION_AUDIT.md)。

## 2026-08-28 B-04.2 图片真实阶段事件收口

- 图片服务新增可测试的 `decoding/resizing/encoding/validating/publishing` 观察器；缩放仅在可见尺寸实际变化时出现，候选不更小时不会记录发布，预取消不生成任何阶段。
- Tauri 图片命令将阶段映射为现有 `task-log`；没有新增日志 store，也没有发送要求数值百分比的 `task-progress`。架构门禁会阻止后续误加虚假图片进度。
- 固定真实方向 JPEG、透明 PNG、WebP 和 GIF 已完成预期—实际序列复核；前端 244/244、Rust 314 通过/4 忽略、Clippy 零警告、生产构建和五项媒体门禁通过。
- 本节点要求的 B-04.3 至 B-04.5 均已由顶部节点收口；当前下一接续点为 B-05，完整阶段证据见 [B04_2_IMAGE_STAGE_EVENT_AUDIT.md](B04_2_IMAGE_STAGE_EVENT_AUDIT.md)。

## 2026-08-28 B-04.1 图片输入/输出事实契约收口

- Rust `ImageCompressionOutcome` 已从输出单侧事实改为 `input/output`；仅更小策略返回 `input/candidate`。两侧均包含真实文件字节、格式、编码矩阵、方向后可见尺寸、方向、帧数和 Alpha。
- 前端新增同构请求/响应类型；历史 `MediaMetricsV1` 向后兼容增加可选 `image.input/output`，Rust `deny_unknown_fields` 同步并拒绝未知字段、无效格式、零尺寸/帧数和方向范围外数据。
- 固定真实方向 JPEG、透明 PNG 和 WebP 已复核预期—实际差异；前端 244/244、Rust 312 通过/4 忽略、Clippy 零警告、生产构建和五项媒体门禁通过。
- 本节点收口时尚未接入阶段事件；B-04.2 至 B-04.5 现均已由顶部节点完成。当前以 B-05 安装版真实验收矩阵为唯一接续点，完成前仍不升版、不发布。完整事实契约证据见 [B04_1_IMAGE_FACT_CONTRACT_AUDIT.md](B04_1_IMAGE_FACT_CONTRACT_AUDIT.md)。

## 2026-08-28 B-03 后跨设备接续审计（已由 B-04.1 更新）

- GitHub 开发分支为 `codex/archive-media-roadmap`，代码锚点 `5e396c6`；审计时本地与远端一致，相对 `origin/master` 领先 23、落后 0。公开 `v1.1.14` 标签仍在 `cfc58ec`，当前不升版、不更新 Release。
- 实际代码确认 B-03 后端已完成，但前端仍有“B-02/B-03 待开放”旧文案且没有调用 `compress_image_file`。更关键的是，后端结果与现有历史指标尚不能同时表达输入/输出格式、可见尺寸、编码矩阵和方向事实，图片命令也没有阶段事件。
- 该审计当时要求从 B-04.1 事实契约开始；B-04.1 至 B-04.5 现均已完成，当前以文档顶部的 B-05 接续点为准。后续仍不得用浏览器尺寸或预计值覆盖真实结果历史。
- 新电脑的 clone/switch 命令、工具版本、验证命令、工作区噪声和完整阻断表见 [CURRENT_DEVELOPMENT_STATUS_AUDIT_2026-08-28.md](CURRENT_DEVELOPMENT_STATUS_AUDIT_2026-08-28.md)。

## 2026-08-28 B-03.2 图片变换、命令执行与 NSIS 增量收口

- B-03 后端执行与发布事务已完成：JPEG/PNG/WebP 支持同格式压缩、格式转换和按可见尺寸等比例缩放；输出发布前重新解码并核对格式、矩阵、方向、可见尺寸、帧数、Alpha，以及配置承诺的 EXIF/ICC 字段。
- 新增受治理的 `img-parts 0.4.0`（仅 `std`）处理 EXIF/ICC 容器字段；转换/缩放时烘焙方向并归一 Orientation，同格式移除元数据时仍保留最小方向语义。XMP/任意 PNG ancillary chunk 不在既有产品承诺内，未被扩张为虚假支持。
- `compress_image_file` 已复用现有容量预检、统一取消注册表、`spawn_blocking` 和共享原子发布事务；失败、取消、空间不足、目标冲突、结果不更小均不发布且清理暂存。当前没有“删除源文件”选项，因此源文件始终只读；未来只能在发布成功后接共享回收站。
- 同版本 NSIS 前后对照完成：`f4ea25b` 基线 7,736,450 B，当前 8,613,866 B，净增 877,416 B（11.3413%），两份 SHA-256 已进入机器门禁。宿主 NSIS 缓存无法启动，使用同一透明转发工具链生成两包，不影响对照口径。
- 下一步进入 B-04：接统一进度、阶段日志、真实指标和跨重启历史，事实来源完成后再启用前端按钮；B-05 负责安装版完整矩阵。当前不升版、不更新 Release。完整证据见 [B03_2_IMAGE_TRANSFORM_EXECUTION_AUDIT.md](B03_2_IMAGE_TRANSFORM_EXECUTION_AUDIT.md)。

## 2026-08-28 B-03.1 单文件图片编码与发布事务

- 产品运行时首次严格准入 `libcaesium 0.21.0`（仅 `jpg,webp`）、`oxipng 10.2.0`（仅 `parallel,zopfli`）和 `image 0.25.10`（仅 `jpeg,png,webp` 验证）；门禁同时锁定 Cargo 声明、lockfile 精确版本并拒绝 gifski/imagequant，FFmpeg/qpdf 继续冻结。
- 审计发现架构扫描只识别驼峰 `imageCompression`、会漏掉 Rust 的 `image_compression`；现已修正命名匹配，新服务进入共享发布/历史/回收禁止绕过扫描。
- 新服务以魔数猜测和真实完整解码确认 JPEG/PNG/WebP，拒绝 GIF/动画及 1 亿像素以上输入；输出只写目标旁唯一暂存，编码后再次完整解码并核对格式、编码矩阵、EXIF 方向、方向后可见尺寸、帧数和 Alpha，再复用共享事务原子发布。
- 默认“仅在更小时替换”、预取消、编码/验证失败、目标已存在和发布竞态都不修改源文件、不留下暂存；真实方向 JPEG、透明 PNG、WebP、扩展名伪装与拒绝边界测试已通过。
- B-03 尚未收口。下一步只做 B-03.2：同一服务内补输出格式转换、缩放、元数据逐字段保持/移除验证、任务取消注册表与容量预检接线，并测最终 NSIS 增量；之后才连接前端执行。当前不升版、不发布，执行按钮继续禁用。完整证据见 [B03_IMAGE_ENCODING_TRANSACTION_AUDIT.md](B03_IMAGE_ENCODING_TRANSACTION_AUDIT.md)。

## 2026-08-27 B-02 验证基础设施纠偏

- 恢复 B-02 时真实复现 Windows CRLF 使 `test:image-baseline` 误报缺少 libcaesium 锁项；现只规范化换行，版本、feature 与禁止依赖检查均未放宽。
- 图片桌面门禁此前错误依赖完整视频/PDF/FFmpeg 夹具，170,676,191 B 固定测试工具首次下载停在 467,520 B 且无限等待。现新增图片专用夹具路径，固定五图片和一个真实 PDF 拒绝样本；B-01 真实图片基线复验通过。
- 完整媒体下载增加 `.part`、Range 续传、停滞超时、重试、进度和哈希后原子发布，B-00 完整矩阵保持不变。下一步仍是 B-02 Release/WebView2 双尺寸复验和原生选择/拖放，不进入 B-03。证据见 [B02_VALIDATION_INFRASTRUCTURE_AUDIT.md](B02_VALIDATION_INFRASTRUCTURE_AUDIT.md)。

## 2026-08-27 B-02 图片前端工作区暂停交接

- 已实现压缩中心四模式入口、现有压缩 store 内隔离的图片草稿、JPEG/PNG/WebP 输入边界、GIF/PDF 明确拒绝、批量/单项同源配置、预计范围标识以及原图/结果对比框架；视频和 PDF 只展示计划节点，不创建假任务。
- 暂停审计发现首稿曾新建图片 store，偏离 B-00“统一任务/不新建媒体 store”边界；现已并回压缩 store，并扩大架构门禁以阻止用 `imageCompression` 命名绕过检查。
- 第一次 Windows Release/WebView2 真实运行发现原图全部无法解码，实际原因是 Tauri 本地资产协议未启用。现已改为默认空 scope，并由 Rust 在普通文件、大小、魔数和扩展名一致性校验后逐文件授权；聚焦 Rust 安全测试通过。
- 修正后的隔离 Release 重新构建在用户要求暂停时终止，所以 B-02 不能收口。恢复后第一步必须完成双尺寸桌面复验和一次原生选择/拖放，不得直接进入 B-03、升版或发布。完整现状与命令见 [B02_IMAGE_WORKSPACE_PAUSE_AUDIT.md](B02_IMAGE_WORKSPACE_PAUSE_AUDIT.md)。

## 2026-08-27 B-00.4 真实媒体样本基线收口

- 已建立 5 图片、2 视频、6 PDF 的合成真实夹具：透明/EXIF/WebP/动图/9600 万像素，H.264/H.265/VFR/AAC/旋转矩阵/字幕，以及文本/扫描/透明/表单/CMS 签名/AES-256 拒绝边界。
- 首轮真实运行依次发现 ReportLab API 名称、pyHanko 参数、加密 PDF 检查顺序、MP4 旋转标签不产生 Display Matrix 和扫描字体过小，均按实际差异修正并重跑。最终结构化 `differences=0`，六个 PDF Poppler 渲染已逐张视觉复核。
- B-00.1 至 B-00.6 总审计和 B-01 图片依赖/固定哈希基线已完成。libcaesium 仅启用 JPEG/WebP，PNG 独立使用 oxipng；五个输入哈希稳定，三种输出真实解码复核，GIF 不落盘，隔离候选压缩增量 1,077,127 B。候选仍未进入产品运行时，本步骤不升版、不发布；下一步为 B-02 图片前端工作区。完整证据见 [B01_IMAGE_DEPENDENCY_BASELINE_AUDIT.md](B01_IMAGE_DEPENDENCY_BASELINE_AUDIT.md)。

## 2026-08-27 B-00.3 媒体依赖身份与许可门禁收口

- 已固定 `libcaesium 0.21.0`、`oxipng 10.2.0`、FFmpeg 9.0.1 官方源码和 qpdf 12.4.0 官方 MinGW64 候选的来源、字节、SHA-256、许可、平台、链接方式、禁用功能与安全责任；Ghostscript 明确阻断。
- 真实审计纠正了顶层许可误判：libcaesium 默认/GIF/PNG 会引入 AGPL/GPL 依赖，因此未来只允许显式 JPEG/WebP，无损 PNG 独立走 MIT oxipng。FFmpeg 官方源码 PGP 签名与发布指纹已真实验证；qpdf 真实运行版本/crypto 与 12,637,211 B 运行子集已核对。
- CI 和 Release 增加失败关闭静态门禁，真实网络门禁可复验下载与执行身份；四个候选仍为 `integrationAllowed=false`，生产代码中没有媒体引擎。本步骤不升版、不发布；下一步为 B-00.4 固定真实媒体样本。完整证据见 [B00_MEDIA_DEPENDENCY_AUDIT.md](B00_MEDIA_DEPENDENCY_AUDIT.md)。

## 2026-08-27 B-00.2 共享事务边界收口

- 非分卷归档压缩已从服务内部重命名逻辑迁移到公共单文件发布事务：同目录唯一暂存、校验后发布、取消/目标竞争/缺失暂存/写满清理均有真实文件证据；分卷压缩和目录解压事务保持原边界。
- Windows 系统回收站调用已提取为公共服务，整组临时分卷真实移入电脑回收站；没有建立应用内回收站。架构门禁会阻止未来媒体生产代码直接重命名最终文件、调用回收站、另写历史或建立媒体 store。
- Release/WebView2 真实归档闭环通过。首次门禁依次发现 EdgeDriver 环境变量缺失和前端测试桥未编入，均按实际原因修正后重建复验。B-00.2 不升版、不发布；下一步为 B-00.3 第三方依赖身份、许可、哈希和再分发门禁。完整证据见 [B00_SHARED_TRANSACTION_AUDIT.md](B00_SHARED_TRANSACTION_AUDIT.md)。

## 2026-08-27 S-00 跨步骤总验收收口

- S-00.1 至 S-00.4 已通过同一轮跨步骤复验：Windows DPAPI/迁移、真实加密 7Z 保险箱命中与当天趋势、真实 ZIP 历史跨完整进程重启均通过。
- 当前代码重建的无测试桥 NSIS 已覆盖正式安装，安装态工作区、用户数据、经典菜单、卸载和公开 `1.1.14` 恢复共 48/48 通过；真实归档矩阵覆盖加密 7Z/RAR、TAR、嵌套、安全负向与 18 万条目取消。
- 首次安装验收分别被运行中的正式进程、缺少 EdgeDriver 和缺少固定 RAR 夹具阻断；未发现产品回归。安装脚本已增加覆盖安装前的无损前置检查，文档补齐准备顺序。S-00 正式关闭，本节点不升版、不发布；下一步进入 B-00 架构门禁。完整证据见 [S00_TOTAL_ACCEPTANCE_AUDIT.md](S00_TOTAL_ACCEPTANCE_AUDIT.md)。

## 2026-08-27 S-00.4 格式支持与仓库卫生收口

- 新增 A/B/C/D 格式支持等级，明确动态处理器、扩展名识别和空镜像不能作为公开支持证据；README、全格式验证文档与当前权威清单已经统一。
- HFSX 使用固定 `libdmg-hfsplus` 提交生成包含 `Firefox/known-payload.txt` 的非空镜像，随包 7-Zip 与 Windows Release Tauri 均完成真实解压和内容校验；首次夹具哈希兼容问题及首次误用普通 Release 可执行文件均已记录并修正。
- 无构建引用的 `src-tauri/TranslateSoftware` 已以可追溯 Git 重命名移入 `archive/legacy-projects`。前端类型检查、40/40 测试文件（235/235）通过；本步骤不升版本、不发布，下一步为 S-00 跨步骤总验收。完整证据见 [FORMAT_SUPPORT_AND_REPOSITORY_HYGIENE_S00_4_AUDIT.md](FORMAT_SUPPORT_AND_REPOSITORY_HYGIENE_S00_4_AUDIT.md)。

## 2026-08-27 S-00.3 运行队列与历史任务边界收口

- 已删除 `task.ts` 中无调用方、无后端实现的 `fetchTasks` 占位接口；当前任务 store 只维护运行队列，历史页面只从 `history.ts` 和后端 SQLite 读取，不再存在第二套历史读取入口。
- 真实 Windows Release + WebView2 使用 2 MiB 随机文件完成 ZIP 压缩—解压，压缩与解压历史在应用完全退出重启后仍存在；正常/760×520 窗口、状态胶囊和不透明详情抽屉均通过。
- 类型检查、40/40 测试文件（235/235）通过。本步骤不升版本、不发布；下一步为 S-00.4 文档、格式支持等级和未参与构建旧工程治理。完整证据见 [TASK_HISTORY_BOUNDARY_S00_3_AUDIT.md](TASK_HISTORY_BOUNDARY_S00_3_AUDIT.md)。

## 2026-08-27 S-00.2 归档密码领域模型收口

- 活动 WebView/Tauri CRUD 已只保留归档密码字段，新增与编辑不再计算或提交传统网站密码强度，账号、网址、到期和自定义登录字段只留在磁盘兼容模型；编辑旧条目时由 Rust 合并并保留旧字段与生命周期统计。
- 手动密码成功解压后的调用统计已改走后端原子事件；搜索只覆盖归档名称、备注和标签。真实 Windows Release + WebView2 在应用重启后使用保险箱密码解开加密 7Z，当天趋势正确增加。
- 类型检查、40/40 前端测试文件（234/234）、Rust 领域边界 2/2、生产前端构建和真实桌面门禁均通过。本步骤不升版本、不发布；下一步为 S-00.3 历史任务占位接口治理。完整证据见 [ARCHIVE_PASSWORD_MODEL_S00_2_AUDIT.md](ARCHIVE_PASSWORD_MODEL_S00_2_AUDIT.md)。

## 2026-08-27 S-00.1 密码保险箱本机保护收口

- Windows 安装密钥现使用当前用户 DPAPI 保护，密码正文使用 AES-256-GCM v2 密文；旧明文安装密钥和旧明文密码条目在成功读取后原子迁移，密文损坏、密钥不匹配或迁移失败不会覆盖原数据。
- 前端不再获取安装密钥，只调用 Rust 后端无参数就绪命令；内存数据密钥仅在解锁时生成一次并在锁定时清零，避免多条密码重复执行慢密钥派生。
- 真实 Windows Tauri 门禁已检查磁盘文件不含密码明文、DPAPI/v2 标识、完全退出重启后密码保险箱命中真实加密 7Z，以及本地当天趋势为 1；前端 234/234、Rust 聚焦 4/4、全目标 Clippy和正式生产构建均通过。
- 本步骤不升版本、不发布。S-00 下一步为归档密码领域模型收敛，媒体压缩 B-00 继续冻结。完整预期—实际—修正见 [PASSWORD_VAULT_PROTECTION_S00_1_AUDIT.md](PASSWORD_VAULT_PROTECTION_S00_1_AUDIT.md)。

## 2026-08-26 开发目标对齐审计

- 当前分支与 `origin/master` 均为 `d985556`，公开版本为 `v1.1.14`。归档主流程、浏览工作区、密码自动尝试、历史任务、系统回收站、经典右键菜单及公开更新仍然对齐最初基本需求；媒体引擎尚未进入生产代码，没有发生战略性开发偏移。
- 审计发现密码保险箱“安装实例密钥保护”的公开描述与当前直接 JSON 存储不一致；传统网站密码模型也仍有残留。进入媒体功能前先执行 S-00，确定 Windows 无感数据保护/诚实降级方案、完成旧数据迁移验证并收敛归档密码领域模型。
- 路线图已补充可执行的 B-00 门禁。S-00 与 B-00 通过前，不接入图片、视频或 PDF 编码引擎，不提前增加媒体 UI。
- 完整需求矩阵、偏移证据、优先级和下一步顺序见 [DEVELOPMENT_ALIGNMENT_AUDIT_2026-08-26.md](DEVELOPMENT_ALIGNMENT_AUDIT_2026-08-26.md)。

## 2026-08-26 v1.1.14 正式发布

- 全部版本源和唯一 Shell Extension DLL 已统一为 `1.1.14`，README、Release Notes 和发布审计已更新。
- 前端 234/234、类型检查、Rust Release 全目标、Clippy、生产 NSIS 构建和 14 文件安装包完整性均通过；本地 NSIS SHA-256 为 `B085A5D92319BB9095AA87FD933030FF1DE04AA87513C0BDAFF2B4B54543731B`。
- 真实 `1.1.13 → 1.1.14 → 卸载 → 1.1.13` 安装闭环及 A-06 工作区矩阵通过，原菜单和用户数据指纹已恢复。首次 WebView2 会话启动竞态已通过有界重试修正并复验。
- 标签 `v1.1.14` 固定在 `cfc58ec`；Release 工作流 `32947440127` 已通过，四项公开资产、签名、`latest.json`、安装包/ZIP 完整性和本机真实 `1.1.13 → 1.1.14` 应用内更新均已复验。
- 公开更新门禁新增开机启动偏好与 Windows Run 项的一致性检查；最终 24 项全部通过，用户数据指纹、安装位置、自动重启、经典菜单和唯一 Shell Extension 均保持正确。本机现安装 `1.1.14`。
- 大节点 A 已完整收口。下一步进入媒体压缩前置节点 B-00，只做统一任务/历史模型、第三方依赖身份许可与发布事务边界设计；图片、视频、PDF 引擎在 B-00 审计通过前不落地。完整发布证据见 [RELEASE_AUDIT_1.1.14.md](RELEASE_AUDIT_1.1.14.md)。

## 2026-08-26 A-06 正式安装态综合矩阵收口

- 正式公开版仍为 `v1.1.13`；A-06 已完成，本步骤不升版、不发布。下一步可独立进入 `v1.1.14` 发布收口。
- 新增正式安装版归档工作区门禁：候选覆盖安装后校验 EXE 字节、版本、扩展 DLL、经典右键命令和生产模式，并在 150% 缩放下真实操作 ZIP、加密 7Z、固定加密 RAR、TAR/TAR.GZ、中文八层路径、混合内容和三层嵌套。
- 真实测试发现并修复 Esc 无法关闭内部预览；同时纠正完成判定、RAR 目录加密假设和 Toast 正文竞态。最终危险 CMD 默认取消、损坏包中文提示和 18 万条目 TAR 78 ms 取消均通过。
- 候选卸载后已恢复 `v1.1.13`、原菜单目标和两处用户数据指纹。完整预期—实际—修正见 [ARCHIVE_WORKSPACE_A06_AUDIT.md](ARCHIVE_WORKSPACE_A06_AUDIT.md)。
- 下一步只做 `v1.1.14` 版本源、README、Release Notes、不可变资产、公开更新 JSON 与发布验证；图片/视频/PDF 压缩继续冻结。

## 2026-08-26 A-05.2 能力来源与浏览边界收口

- 正式公开版仍为 `v1.1.13`；开发分支 `codex/archive-media-roadmap` 已完成 A-05.2，不升版、不发布。
- 后端能力响应新增可浏览、可嵌套、可有界预览格式及图片/文本预览扩展策略；浏览筛选、嵌套入口与预览入口不再维护前端第二份归档扩展名清单。
- 审计发现内置 7-Zip 26.02 真实报告 `zstd zst tzst`，旧解析表却漏掉 zstd。现已修正并用现场生成的真实 zstd 流嵌入 ZIP，在 Release WebView2 右键菜单中验证“进入压缩包”确实可用。
- `ArchiveBrowserView.vue` 的请求生命周期、能力映射、目录/历史导航已拆为独立组合式函数，目录树拆为独立组件；相应单元测试覆盖取消、错误分类、能力策略、前后导航与刷新状态协调。可见交互、测试选择器与安全命令保持不变。
- 当前验证：前端 234/234、归档能力 Rust 3/3、Clippy 零告警、生产前端构建、`custom-protocol,desktop-e2e` Release 构建和真实归档浏览桌面门禁全部通过；18 万条目 TAR 取消 94 ms。完整差异证据见 [ARCHIVE_WORKSPACE_A05_2_AUDIT.md](ARCHIVE_WORKSPACE_A05_2_AUDIT.md)。
- 下一步严格执行 A-06 安装态综合矩阵与发布门禁。只有 A-06 全部通过才提升 `1.1.14`、打包并更新 README/Release；图片、视频、PDF 压缩继续冻结。

## 2026-08-26 A-05.1 归档读取可取消化收口

- 正式公开版仍为 `v1.1.13`；开发分支 `codex/archive-media-roadmap` 已完成 A-05.1，不升版、不发布。
- 浏览与刷新现在使用唯一请求 ID；取消会到达 Rust 服务。ZIP/RAR/TAR 遍历协作停止，通用 7-Zip CLI 停止等待，所有路径有 30 秒用户等待上限；极快点击取消的登记竞态也已封闭。
- UI 增加明确“取消读取”，取消后回到可继续选文件的空闲态；底层英文异常改为取消、超时、密码、损坏、不支持和通用失败的中文分类。
- 真实 Release Tauri/WebView2 使用现场生成的 18 万目录项 TAR 验证，点击取消后 55 ms 恢复；随后普通/加密 ZIP、7Z、固定加密 RAR、默认应用打开、文本预览、三层嵌套和精确解压全部回归通过。
- 当前验证：前端 229/229、归档 Rust 2/2、取消回归 8/8、Clippy 零告警、生产前端构建、`custom-protocol,desktop-e2e` Release 构建和归档浏览桌面门禁均通过。完整差异证据见 [ARCHIVE_WORKSPACE_A05_1_AUDIT.md](ARCHIVE_WORKSPACE_A05_1_AUDIT.md)。
- 下一步先完成 A-05.2：消除前端归档扩展名第二真相源，并拆分过大的浏览主视图；之后才进入 A-06 安装态综合矩阵。只有 A-05.2、A-06 与发布门禁全部通过才提升 `1.1.14`。图片、视频、PDF 压缩继续冻结，避免开发偏移。

## 2026-08-26 A-05 嵌套归档工作区收口

- 正式基线仍为 `v1.1.13`。A-05 已完成，压缩包浏览中心可在同一只读工作区进入 ZIP→7Z→ZIP，显示可返回的归档链，并在第三层明确阻断第四层。
- 嵌套条目复用 A-03 会话缓存与单文件/会话预算；后端自行推导登记深度、校验归档魔数和父子 SHA-256、拒绝祖先重复内容，不能靠前端伪造深度绕过。
- 每层密码独立：进入子层会清空父层手工密码，再按当前缓存归档匹配保险箱。真实加密 7Z 已完成错误密码失败、正确密码进入和内层 ZIP 密码为空的验证。
- 返回会恢复外层目录、筛选、搜索、焦点、多选和导航栈；迟到的内层读取结果会被序列号丢弃。真实桌面门禁确认外层 2/2 选择恢复、最内层只发布选中内容、损坏内层可稳定返回。
- 当前验证为前端 228/228、A-05 Rust 8/8、Clippy 零告警、测试桥生产前端构建、Release Rust 构建和真实 Windows Tauri/WebView2 归档浏览门禁全部通过。完整预期—实际—修正证据见 [ARCHIVE_WORKSPACE_A05_AUDIT.md](ARCHIVE_WORKSPACE_A05_AUDIT.md)。
- A-05 后新增的 A-05.1 已封闭真实取消和超时问题；先按 A-05.2 收口能力来源与组件边界，再由 A-06 用正式安装态综合矩阵回归普通/加密 ZIP、7Z、RAR、TAR/TAR.GZ、长路径、混合内容、三层嵌套、负向安全场景、资源管理器经典右键入口和选择性解压。A-05.2、A-06 与发布门禁通过后才提升 `1.1.14`、构建安装包并更新 Release；媒体压缩继续冻结。

## 2026-08-26 下一阶段：归档工作区与媒体压缩

- 当前正式基线仍为 `v1.1.13`；A-01 至 A-05 已完成并通过真实桌面门禁，但浏览中心 2.0 还缺 A-06 安装版综合矩阵，因此不升版本、不打包发布。
- 浏览中心现已区分聚焦与多选，右侧可像文件管理器一样双击/Enter 进入文件夹，并提供面包屑、后退、前进、上一级、刷新、Backspace 和 Alt 方向键。键盘监听经过真实 WebView2 焦点丢失复现后改为窗口级处理。
- 右键菜单按对象动态提供文件夹打开、Windows 默认应用安全打开、内部图片/文本查看、嵌套归档进入、当前/指定目录解压、复制名称/归档内路径、详情和刷新；右键未选项不会破坏既有多选，Enter、Ctrl+Enter、Shift+F10、Alt+E、Alt+Enter 等键盘入口复用同一语义。第三层会把嵌套入口明确禁用并解释上限。
- 真实门禁使用现场生成中文八层长路径 ZIP、文件名加密 7Z、固定加密 RAR 和 ZIP→加密 7Z→ZIP；A-03 以真实 TXT/PNG/PDF/CMD 验证默认应用、NTFS 安全标记与默认取消；A-04 覆盖 ZIP UTF-8/超大日志/伪装二进制、UTF-16LE TAR 和 7Z 禁用边界；A-05 覆盖错/正确密码、三层链、第四层阻断、损坏内层和返回恢复。全量前端 228 项通过。证据见 [ARCHIVE_WORKSPACE_A01_AUDIT.md](ARCHIVE_WORKSPACE_A01_AUDIT.md)、[ARCHIVE_WORKSPACE_A02_AUDIT.md](ARCHIVE_WORKSPACE_A02_AUDIT.md)、[ARCHIVE_WORKSPACE_A03_AUDIT.md](ARCHIVE_WORKSPACE_A03_AUDIT.md)、[ARCHIVE_WORKSPACE_A04_AUDIT.md](ARCHIVE_WORKSPACE_A04_AUDIT.md) 与 [ARCHIVE_WORKSPACE_A05_AUDIT.md](ARCHIVE_WORKSPACE_A05_AUDIT.md)。
- A-04 已完成 ZIP/TAR 有界文本查看器，A-05 已完成嵌套归档工作区。下一步严格执行 A-06 安装版综合矩阵；通过后才允许发布 `v1.1.14`。
- 后续按图片压缩、视频压缩软件编码、PDF 安全优化推进。三个模式都放在压缩中心内部，继续使用 `compression` 顶层任务类型，通过可选工作负载分类扩展历史，不复制任务、日志和发布事务。
- 图片 JPEG/WebP 仅评估关闭默认功能并显式启用 `jpg,webp` 的 Apache-2.0 `libcaesium`，无损 PNG 独立评估 MIT `oxipng`；禁止 libcaesium 默认/GIF/PNG 路径引入 AGPL/GPL 依赖。视频使用经过许可与哈希审计的 FFmpeg LGPL 构建；PDF 首期采用 Apache-2.0 qpdf。Ghostscript 在 AGPL/商业许可方案未明确前不得内置。
- 每个大节点的开发步骤、真实样本、验收目标、阻断条件、版本提升和 Release 闭环统一以 [ARCHIVE_WORKSPACE_AND_MEDIA_COMPRESSION_PLAN.md](ARCHIVE_WORKSPACE_AND_MEDIA_COMPRESSION_PLAN.md) 为准。
- 媒体引擎在大节点 A 完成前继续冻结，避免把归档交互、安全临时提取和第三方编解码依赖混入同一发布审计。

## 2026-08-22 解压主流程修复收口（v1.1.13 候选）

- 密码候选阶段不再推进解压百分比；真实输出开始前保持 0，RAR 进度由事务暂存目录的实际产出字节驱动。
- 移除正确 RAR 密码可能触发的固定 10 秒误判，收窄错误分类；固定 libarchive 加密 RAR 已通过错误密码拒绝、正确密码校验和逐文件 SHA-256 桌面回归。
- `.long-extract-*` 事务目录创建后立即隐藏，完成清理后通知 Explorer 刷新；文件冲突改为保留同一暂存并逐项提交，不再取消后重新解压。
- Windows 容量预检增加原生 Win32 后备；不存在的目标子目录也可解析到真实卷。新历史会保存实际识别格式，并对长密码候选日志保留里程碑与均匀样本。
- 前端 218 项、Rust 全目标 373 项、5 组真实 Windows 桌面聚焦门禁和 Clippy 均通过。完整事实、旧数据边界及尚未执行的外部验证见 [EXTRACTION_CLOSEOUT_AUDIT_2026-08-22.md](EXTRACTION_CLOSEOUT_AUDIT_2026-08-22.md)。
- 发布动作现已完成：`v1.1.13` Release 工作流、四项公开资产、公开清单/签名/完整性复验，以及本机 `1.1.12 → 1.1.13` 应用内更新 22 项门禁全部通过。后续开发从 `master` 继续，发布标签固定在 `3438d76`；标签后的提交只补充公开证据与增强验证脚本，不修改发布二进制。
- 普通 CI 曾因集成测试仍断言 4 个监听器而失败；实际产品新增 `archive-format-detected` 后应为 5 个。断言已更新并验证事件写入任务格式，完整 coverage 套件现为 40 个文件、240 项通过。

## 2026-08-22 v1.1.12 显式开机自启动发布收口

- 设置中心恢复 Windows 开机自动启动开关，但持久化写入严格限制在用户点击开关的交互路径；设置加载只读取实际注册状态，不注册、不迁移、不修复。
- 启用值固定为带引号的当前可执行文件绝对路径和 `--autostart` 参数，重复启用幂等；禁用和卸载清理当前及历史品牌值，覆盖更新保留用户已明确开启的当前值。
- 登录启动期间主窗口在 setup 与页面加载阶段保持隐藏，只驻留托盘；携带 `--autostart` 的第二实例不会唤醒现有窗口。
- 前端错误路径在注册失败后回读真实状态，不把失败写成成功；重置普通设置也不会绕过专用开关偷偷改变 Windows 启动项。
- Windows Release Tauri 门禁已覆盖初始关闭、单次启用、注册表精确命令、重复启用、禁用清理和独立进程隐藏启动。托管 CI 只构建隔离测试二进制并校验脚本；真实 GUI 闭环按既有约束在交互式 Windows 环境执行。完整边界与 Defender 验证状态见 [AUTOSTART_SECURITY_AUDIT.md](AUTOSTART_SECURITY_AUDIT.md)。
- 正式发布说明见 [RELEASE_NOTES_1.1.12.md](RELEASE_NOTES_1.1.12.md)，候选审计见 [RELEASE_AUDIT_1.1.12.md](RELEASE_AUDIT_1.1.12.md)。

## 2026-08-21 v1.1.11 发布收口

- 大型分卷的加密探测改为结构化目录元数据读取；超时不再等同于加密，未加密分卷不会进入密码保险箱和字典流程。
- 通用 7-Zip 解压接入回车刷新进度、当前文件、两位小数百分比和实际产出统计；压缩详情同步展示真实落盘大小、压缩比和空间变化。
- 自动密码尝试展示候选来源、序号、结果和耗时但不记录明文；完整解压成功后可将整组源分卷移入 Windows 系统回收站。
- 用户提供的五卷真实 ZIP 样本已确认未加密并能持续进入真实解压；详细变更见 [RELEASE_NOTES_1.1.11.md](RELEASE_NOTES_1.1.11.md)，发布审计见 [RELEASE_AUDIT_1.1.11.md](RELEASE_AUDIT_1.1.11.md)。
- 普通、AES 与多文件 ZIP 已在真实 Windows Release Tauri/WebView2 会话中验证输出体积事件，最终值与磁盘归档大小逐字节一致，补齐此前只构建未执行的桌面断言。
- 公开 v1.1.10 到候选 v1.1.11 的安装态门禁 43/43 通过，覆盖版本与 PE 身份、Shell Extension、经典菜单、覆盖安装、卸载清理、候选数据保持，并最终恢复 v1.1.10、用户数据和原菜单。

## 2026-08-21 分卷解压启动停滞修复

- `digital-rural-platform.zip.001/.002` 真实样本共 621,344,015 字节、15,800 个文件和 5,438 个目录，独立 7-Zip 完整性测试通过，归档未加密。
- 原加密检测误用 `7z t`，会在显示任何解压进度前完整读取所有分卷；真实样本耗时约 6.1 秒。现改为 `7z l -slt -ba -p-` 仅读目录元数据，真实样本约 0.34 秒，并避免慢盘超时被误判为加密包。
- 所有通用引擎子进程禁止隐藏交互输入；元数据超时返回明确错误。未加密归档会显示“加密状态检测完成，正在准备解压”，不再长时间停留在检测日志。
- 新增真实 7-Zip 分卷 ZIP 元数据回归；普通分卷与加密 ZIP 两条聚焦测试、Clippy 全目标严格检查及完整 Rust release 测试均通过（244 通过、1 个固定机性能基准按设计忽略）。

## 2026-08-21 v1.1.10 发布收口

- 分卷归档导入已按用户实际拖入方式归一：任意分卷会解析到同组起始卷，重复分卷不会生成重复任务，缺首卷、断档和混组会提供明确诊断。
- 压缩与解压详情统一为有界左右双栏；左栏内容保持真实高度并纵向滚动，右栏长路径受限且日志独立纵向滚动，不再产生横向延伸。
- 资源预检在解压左栏不再被 Flex 压扁，压缩中心也归入左侧配置语义；响应式门禁覆盖应用最小窗口 760×520 至 1440×900。
- 详细发布变更见 [RELEASE_NOTES_1.1.10.md](RELEASE_NOTES_1.1.10.md)，发布审计见 [RELEASE_AUDIT_1.1.10.md](RELEASE_AUDIT_1.1.10.md)。

## 2026-08-21 v1.1.9 发布收口

- v1.1.7 的 Defender 启发式隔离根因已收敛：未签名主程序不再提供或写入用户级开机自启动，安装/更新会清理三个历史 `Run` 值，不删除密码保险箱、历史任务或其他用户数据。
- 加密归档会读取实际文件型密码保险箱，优先使用收藏、高频和最近使用项；命中后原子更新调用次数、最近使用时间与本地自然日趋势，读取失败会显示真实原因。
- 历史任务详情表面、紧凑状态行、侧栏版本角标、解压密码统计语义和压缩/解压共享资源预检布局均已纳入前端与真实 Windows 桌面门禁。
- Windows 11 第一层菜单仍因没有商业 Authenticode 证书而不随公开安装包分发；“显示更多选项”中的 Long解压 经典菜单、一键解压、一键打包和浏览压缩包是本版本支持范围。
- 本轮逐项收口与剩余非阻塞研究项见 [RELEASE_AUDIT_1.1.9.md](RELEASE_AUDIT_1.1.9.md)，正式变更见 [RELEASE_NOTES_1.1.9.md](RELEASE_NOTES_1.1.9.md)。

## 2026-08-20 TAR 系列真实遥测收口

- `tar / tar.gz / tar.bz2 / tar.xz / tar.zst` 已使用共享有界计数读取器报告真实输入字节，4 MiB 中间事件、文件边界终值、任务起始速率基线和 ETA 均已接通；不使用百分比推算字节。
- 真实回归保留长路径与元数据，并修复读取中取消被 `Interrupted` 自动重试的问题。64 MiB 随机非空载荷逐格式通过 Windows Release + WebView2 可见遥测、独立 7-Zip 校验、应用实际解压和 SHA-256 回环；复验命令为 `npm.cmd run test:e2e:desktop:tar-telemetry`。
- 全量审计修正历史任务 schema v6 后遗留的 v5 迁移测试预期，迁移回归现在同时确认 watch 生命周期表与 `task_operation_history` 表。
- 下一阶段转向外部 CLI 路线可证明的真实落盘观测；若引擎只能返回百分比，继续不显示伪造速度。真实 HDD/网络盘矩阵与 HFSX 非空已知载荷仍保持独立未完成项。

## 2026-08-20 密码全景与历史任务中心

- 左侧已在“文件完整性”与“设置中心”之间增加“历史任务”，覆盖真实压缩/解压的完成、失败和取消结果；统一历史持久化来源、目标、格式、耗时、真实处理量、错误摘要与脱敏日志，清理当前队列不会删除历史。
- 该阶段最初实现的传统密码强度、更新年龄和风险视图已在 2026-08-21 的解压密码语义审计后被替换；当前整库统计以真实调用趋势、归档线索覆盖、最近使用时效和长期活跃为准。
- 独立 Windows Release 门禁已用 2 MiB 随机非空载荷完成真实 ZIP 压缩与解压，应用重启后两条历史仍存在，760×520 最小窗口无横向溢出；复验命令为 `npm.cmd run test:e2e:desktop:history`。
- 本阶段详细审计、预期/实际差异和隐私边界见 [DEVELOPMENT_AUDIT_2026-08-20_VAULT_HISTORY.md](DEVELOPMENT_AUDIT_2026-08-20_VAULT_HISTORY.md)。

## 2026-08-20 v1.1.6 后续增强审计

- 已从远端切换并快进到最新 `master@0694fac`；`agent/p0-archive-flow-alignment` 与 `agent/release-1.1.6` 均已通过 PR #81/#82 合入主线，公开 `v1.1.6` 的 CI、Release 和四项更新资产完整。
- 普通 ZIP 与 AES ZIP 的真实输入字节遥测已经完成；64 MiB 普通/AES 单文件及 24 MiB + 40 MiB 双文件的 Windows Release + WebView2 聚焦门禁通过，中间字节、单调累计、最终总量、可见速度/ETA、错误密码拒绝、独立 7-Zip 和 SHA-256 全部有真实证据。
- 真实门禁同时修正了完成阶段 `0/0` 占位事件覆盖前端总量/ETA 的问题；独立复验命令为 `npm.cmd run test:e2e:desktop:zip-telemetry`。
- TAR 真实字节已在后续阶段完成；外部 CLI、真实 HDD/网络盘矩阵与 HFSX 非空载荷继续作为独立阶段，避免把进度语义、文件格式语义和调度策略混在一次修改中。
- 详细预期/实际差距及验收矩阵见 [DEVELOPMENT_AUDIT_2026-08-20.md](DEVELOPMENT_AUDIT_2026-08-20.md)。

## 2026-08-15 v1.1.6 发布收口

- 原计划以公开版 `v1.1.5` 为基线提升一个补丁版本，本次版本身份统一为 `1.1.6`，没有覆盖或重发旧版本。
- PR #81 已通过全部 GitHub CI 并合并主线；密码状态机、任务并发、同目录串行、真实速度/ETA、固实加密 7Z 和密码格式转换确认均已收口。
- Windows Release 可见桌面门禁与 43 项安装态门禁通过，覆盖经典右键菜单、覆盖安装、卸载清理、候选数据保持和旧版环境恢复。
- 发布仍使用无商业 Windows 代码签名证书的既定边界：提供 NSIS 安装包和带更新签名的应用内更新资产，不分发 Windows 11 第一层菜单身份 MSIX。
- 后续优先补齐外部 CLI/流式路线的真实字节可观测性，并继续积累 HDD、网络盘和跨盘性能基线；在证据充分前不自动提高默认并发。

## 2026-08-15 归档主流程 P0 对齐

- 密码解压只保留一套后端状态机：先查密码本；只有用户开启“密码字典尝试”时，才按导入字典、内置推荐字典的顺序继续。前端不再重复调用密码验证或字典服务，也不会把未加密归档误报为“密码破解成功”。
- 批量压缩与解压已接通真实任务并发；同一解压输出目录始终串行。为遵守既定性能路线，旧版本中实际未生效的“并发 4”会在首次升级时迁移为 1，新安装默认也是 1；用户可在设置中心明确提高。
- 任务进度事件新增实际处理字节、实时速度和 ETA。只有引擎提供真实字节时才显示，不用文件数或模拟进度伪造吞吐；当前不报告字节的外部 CLI/部分流式路线仍只显示进度。
- 7Z“固实压缩”已进入正式 `CompressionOptions` 并由原生 `sevenz-rust` 写入单压缩块；真实回归覆盖双文件、AES 密码、元数据单 folder 和完整解出。创建端分卷能力收敛为当前真实可用的无密码 ZIP 普通文件，7Z 不再误报可创建分卷。
- TAR/GZ/BZ2/XZ/Zstd/LZMA 等非原生密码格式不再静默变成 7Z：设置面板即时显示最终 `.7z` 扩展名和 AES-256 说明，提交前再次确认；拒绝后不创建任务或输出。
- 独立提交顺序为 `e7b1e82`、`05da9c3`、`8a5e2f7`、`4719c28`，均已通过 PR #81 合并主线。收口自动化通过前端 200 项、类型检查、生产构建、Clippy 全目标全特性零警告、Rust 库测试 232 项通过/1 项固定机基准按设计忽略、全部测试目标编译、能力矩阵 13 项、密码状态机 6 项、真实流水线 2 项、通用格式烟雾 1 项和完整格式回环 19 项。
- Windows Release 可见归档门禁已在本机交互桌面通过：两项 128 MiB 真实 7Z 任务最大同时活跃数为 2；两项写入同一目录的解压最大同时活跃数为 1；全局进度面板显示真实速度与剩余时间；AES 密码双文件 7Z 经独立 `7z l -slt` 确认 `Solid = +` 并完整解出；TAR 密码回退拒绝时零任务零输出，确认后才创建 `.7z` 并成功解密。独立复验命令为 `npm.cmd run test:e2e:desktop:archive-flow`。
- 现有真实桌面取消回归继续覆盖原生 7Z 中途取消及最终文件清理；本阶段新增门禁与相关 TypeScript/调度测试均通过。
- 无签名 NSIS 安装态门禁以本机 `1.0.22` 为可恢复基线、当前 `1.1.5` 分支构建为候选，43 项检查全部通过：覆盖安装保持目录，版本与 PE 身份一致，只安装一个 `1.1.5` Shell 扩展；4 个经典菜单根的 17 条子命令与 4 条快捷命令全部指向候选；卸载清理程序、产品注册表及全部菜单键且不修改候选运行态数据；最后恢复旧版用户数据、`1.0.22` 安装和测试前原始菜单命令。
- 安装态门禁同时修正两项验证器缺陷：首次运行允许产生数据库迁移，但卸载必须保持迁移后的逐文件指纹；恢复阶段比较测试前实际菜单命令，不再把原本的开发菜单误判为必须指向安装目录。停止应用后现在等待进程完全退出，避免数据库句柄竞态。当前 P0 代码、真实样本、可见桌面和安装态证据齐全，PR #81 已完成总审计并合并，进入 v1.1.6 发布流程。

## 2026-08-12 v1.1.5 发布收口

- 解压与压缩详情面板改为固定双列网格，响应式宽度缩小时仍保持左侧配置、右侧阶段/进度/日志；内部字段自行换行，页面与详情区不产生横向滚动。
- 配置与执行面板统一使用强调色细滚动条，补齐轨道、角落和原生按钮样式，修复 Windows 下左侧灰色滚动条与箭头突兀的问题。
- 浏览器 E2E 新增 1440、1024、760、390 四档左右结构位置断言，并继续验证压缩、解压详情全链路只允许纵向滚动。
- 项目 README 已更换为用户提供的五张完整 Windows 实机截图；历史根目录材料继续集中保存在 `archive/legacy-root`。
- 版本身份提升至 `1.1.5`；发布仍不包含需要商业代码签名证书的 Windows 11 第一层菜单身份包。

## 2026-08-11 v1.1.4 发布候选

- 本补丁版收口压缩包浏览工作区重构、拖入即浏览、层级目录树和资源管理器“浏览压缩包内容”直达链路，不改变压缩与解压引擎、默认并发或安全事务。
- 版本身份已统一提升到 `1.1.4`；公开发布继续使用无商业 Windows 代码签名证书的既定路径，不打包 Windows 11 第一层菜单身份 MSIX。
- 发布门禁以版本身份、前端单测/构建、Rust 菜单测试和 Shell Extension COM 测试为准；可见 Windows Release 桌面拖放与安装态右键仍需在不占用 WebView2 会话的环境复验。

## 2026-08-11 压缩包浏览工作区重构

- 浏览中心已把目录与文件区提升为页面主工作区：顶部只保留紧凑的压缩包与密码入口，格式、文件数、展开大小和加密状态改为摘要标签，输出目录收进底部操作栏。
- 左侧目录由完整路径平铺改成可折叠的文件夹树，只显示当前层名称并保留层级缩进；文件列表继续按当前目录、搜索和类型筛选工作。
- 页面空状态和已打开状态均接入 Windows 原生拖放，并保留标准 Web 拖放兜底；拖入新压缩包可直接替换当前浏览对象。
- 资源管理器的 Long解压 子菜单（经典注册与 Windows 11 原生扩展）均新增“浏览压缩包内容”，通过独立 `context-browse-archive` 动作直达浏览中心，不再误进解压中心；现有命令 GUID 保持不变。发布前仍需在 Windows Release 安装态重新注册菜单并完成一次可见拖放与右键实机验收。

## 2026-08-11 v1.1.3 发布收口

- 相对公开 `v1.1.2`，本补丁版集中交付四项 Windows Release 候选门禁：智能压缩、归档浏览、Mark-of-the-Web、压缩后校验与源文件删除保护。
- 候选验收发现并修复 ZIP Unicode Path Extra Field 与旧解压路径解码不一致的问题；明确选择条目却零输出时会失败，不再误报完成。
- 完整 Windows 桌面回归继续覆盖诊断、ZIP 修复、ZIP/TAR 图片预览、资源预检、任务模板、监控目录、托盘隐藏和真实重启恢复。
- 发布审计将间接依赖 `nanoid` 从 3.3.16 补丁更新到 3.3.18；干净 `npm ci` 后官方生产依赖安全审计为 0。
- `v1.1.3` 不改变默认并发，不加入无证书 Windows 11 新式第一层菜单；后续开发回到真实 HDD 性能样本与独立调度策略研究。

## 2026-08-11 压缩后校验桌面门禁

- `test:e2e:desktop:compression-verification` 已在独立 Windows Release Tauri + WebView2 会话中通过，从可见压缩中心选择真实中文源文件、修改全局高级设置并启动正式压缩任务。
- “校验但保留源文件”场景确认任务按“正在校验 → 校验通过 → 压缩完成”排序，最终 ZIP 通过独立 7-Zip 完整性检查和内容解出复核，源文件保持不变。
- “校验后删除源文件”场景先在界面主动取消校验，再开启删除源文件；界面立即重新勾选并锁定校验。提交任务的 `verify_after=true`，最终归档完成独立校验与内容复核后源文件才消失。
- 既有 Rust 回归继续保护校验失败清理、既有目标不覆盖、密码 ZIP/7Z、AES、分卷和取消。压缩后校验与删除保护首阶段可以收口；P0 两项候选桌面证据均已补齐。

## 2026-08-11 Mark-of-the-Web 桌面门禁

- `test:e2e:desktop:mark-of-web` 已在独立 Windows Release Tauri + WebView2 会话中通过，使用真实 NTFS `Zone.Identifier` 和正式解压 IPC，不以 Mock 或仅 Rust 单测代替候选桌面证据。
- 可见压缩包浏览中心对带 `ZoneId=3` 的真实 ZIP 解压普通文本、Office、PowerShell、EXE 和嵌套目录文件；默认开启时逐文件 ADS 与源标记完全一致，任务日志报告安全标记传播。
- 测试随后进入可见设置中心关闭“保留互联网来源安全标记”，再次解压同一归档并确认全部输出均未创建 ADS。既有 Windows Rust 回归继续保护畸形/超限标记、取消清理、重命名提交和失败回滚。
- 本阶段不改变 Windows 下载安全策略、不伪造互联网标记，也不把 ADS 行为扩展到不支持的文件系统。Mark-of-the-Web 首阶段可以收口；下一项按 P0 顺序验收压缩后校验与删除源文件保护。

## 2026-08-11 压缩包浏览中心桌面门禁

- `test:e2e:desktop:archive-browser` 已在独立 Windows Release Tauri + WebView2 会话中通过，直接驱动可见浏览中心和正式解压 IPC，不依赖 Mock 归档结果。
- 门禁覆盖 7-Zip 生成的中文八层长路径 ZIP、密码与文件名加密 7Z，以及固定上游加密 RAR；逐项验证搜索、取消全选、精确单选、输出目录选择、任务完成状态、未选文件不落盘和页面无横向溢出。RAR 使用固定密码 `12345678`，并校验 `foo.txt` 的固定 SHA-256。
- 实机验收发现浏览元数据使用的新 ZIP 引擎能读取 Unicode Path Extra Field，而旧解压引擎会按 CP437 解释未设置 UTF-8 标志的中文路径，导致明确选择的文件在暂存事务中被全部过滤。解压现已统一使用 `zip_aes` 8.6；若非空选择最终匹配到 0 个文件，事务会失败，不再允许“完成但空输出”。
- 完整 `test:e2e:desktop:watch-folder` 回归也已通过，继续覆盖诊断、非破坏 ZIP 修复、ZIP/TAR 图片预览、智能压缩、可靠容量阻止、任务模板、托盘监控、退出重启和持久恢复。下一步优先完成本独立 PR 审核；随后按路线图补固定 Windows 性能趋势与真实 HDD 证据，不改变默认并发。

## 2026-08-11 智能压缩桌面门禁

- `test:e2e:desktop:smart-analysis` 已在独立 Windows Release Tauri + WebView2 会话中通过，不依赖 Mock IPC。
- 大目录场景使用 24 个真实文本文件、总量超过 8 MiB，断言最多抽样 16 个文件、2 MiB 内容，15 秒内完成并给出 7Z L7 固实建议；点击“采用建议”后复核现有配置组实际变更。
- 低收益场景组合真实 PNG 与 MP4 类高熵文件，断言全部字节进入低收益统计并建议 ZIP L1；两种场景均检查可见解释文本和详情卡无横向溢出。
- 该门禁只扩展 `VITE_DESKTOP_E2E` 测试桥接，不向正式前端暴露测试接口，也不改变智能分析算法或默认压缩设置。

## 2026-08-11 v1.1.2 收口审计

- 归档浏览、诊断、非破坏式 ZIP 修复、受限图片预览、任务模板、监控文件夹、资源预检与固定机器 I/O 基线已汇入同一发布候选链。
- 自动化门禁通过：Rust 227 项通过、1 项固定机器基准按设计忽略；前端 191 项通过；类型检查、生产构建及 Playwright 32 项通过、13 项按能力跳过。
- 独立 Windows Release Tauri + WebView2 门禁通过真实 ZIP 压缩/解压、诊断、修复、ZIP/TAR 图片预览、可靠容量阻止、模板导入与草稿、监控目录、托盘隐藏和真实重启恢复。
- 固定机器双 NVMe SSD 矩阵已有 10 次样本；HDD 数据仍是后续性能研究项，不阻塞发布，也不改变默认并发。
- 本版本不包含商业 Windows 代码签名证书，不发布 Windows 11 新式第一层右键菜单身份包；经典右键菜单仍是公开安装包的支持范围。
- 后续优先补充真实 HDD 矩阵并观察性能波动，再决定是否进行受控并发实验；不要仅凭 SSD 单机结果修改调度策略。

## 2026-08-10 产品增强路线对齐

- 自适应资源调度第一阶段已进入 `agent/resource-preflight`：没有启用旧任务调度器或改变默认并发，而是先在压缩/解压真实执行路径加入结构化资源预检。目标卷可用空间、文件系统、介质和位置进入任务详情与日志；解压从真实归档元数据读取展开体积，压缩复用未过期的智能分析或普通文件保守估算，固定预留 128 MiB。
- 空间明确不足会在调用引擎前阻止任务；网络位置、移动设备、容量未知或无法可靠估算时只警告，并继续使用既有事务式暂存、运行时容量检查和回滚。预检不可用采用 fail-open 并留下警告，避免元数据探测故障制造新的任务不可用。
- Windows Release 桌面门禁已在 `agent/resource-preflight-desktop-gate` 完成：真实资源管理器一键打包与一键解压会逐字节闭环非空 ZIP，并断言本机卷容量、文件系统、介质、普通文件保守估算、归档元数据展开体积和任务日志；压缩/解压详情卡片均可见且无横向溢出。可靠超大估算经正式 Tauri IPC 返回 blocked，失败任务显示“已阻止”，目标目录未创建。独立复验命令为 `npm.cmd run test:e2e:desktop:resource-preflight`。
- 固定机器 I/O 基线工具已进入 `agent/fixed-io-baseline`：`performance:io-baseline` 复用现有结构化趋势脚本和正式 ZIP 路径，可显式指定源目录与目标目录；压缩和解压均按同一源端→目标端方向计时，单大文件与大量小文件继续执行 CRC32、长度和数量校验。脚本以 Windows 卷/分区/磁盘/物理介质信息区分同卷、同盘跨卷与跨物理盘，记录 SSD/HDD、NTFS、容量和稳定指纹，并在创建夹具前执行 128 MiB 预留与保守空间检查；跨拓扑基线不能互相比对。
- 正式矩阵编排进入 `agent/io-baseline-matrix`：`performance:io-matrix` 共享同一个目标卷，顺序运行目标卷→目标卷和另一物理盘→目标卷，随后核对机器、干净 Git 提交、应用版本、规模、目标卷及预期拓扑，并汇总四项吞吐中位数、相对差异和样本范围。正式 10 样本在工作区不干净、仓库内输出未被忽略或采样根会产生未跟踪夹具时会在开始前拒绝；两组也可分开采集后只读汇总，避免外层执行时限浪费已完成结果。汇总结论固定为 `baseline_only`，不会据此自动改变并发。
- 当前开发机已完成首份合格 SSD 矩阵：干净提交 `8dc8b94` 上的 C→C 同卷与 E→C 跨物理 NVMe SSD 都以 100 MiB/10000 小文件各采集 10 次，恢复分析再次核对同机、同版本、同提交、同规模和同一 C 盘目标。跨盘四项吞吐中位数相对同卷为 -9.05% 至 -24.63%，但样本范围最高达到 92.94%，矩阵结论保持 `baseline_only`，不改变默认并发。本机没有可证明 HDD，下一步是在同配置重复观察波动，并在真实 HDD 机器补齐同样矩阵；实验性并行解压仍不进入生产。
- 最终审计通过前端 190 项单测、类型检查和生产构建；Rust `--all-features` 完整测试退出 0（库测试 222/223 通过，1 项固定机器性能基准按设计忽略，其余集成目标通过），全目标全特性 Clippy 零警告。I/O 基线另以 Release Rust 真实执行两种拓扑、每种两个 ZIP 场景，结果均通过内容校验；拓扑错配、非 I/O 模式误传目录和空间不足三类负向护栏也在引擎启动前正确拒绝。
- 持久监控目录生命周期已进入独立分支：用户必须先执行一次性规则预览，再明确“保存并启用”；每项授权支持启动、暂停、停用和删除，启用或恢复时目录现状只建立去重基线，不把旧文件批量写入草稿。
- Rust `notify` 事件以 900 ms 固定窗口合并，之后复用既有 750 ms 有界稳定性扫描；跨事件以规范路径、大小和修改时间去重。数据库 v5 持久保存授权和待确认批次，最多 20 项授权、每目录 100 批，应用或托盘重启不会直接丢失已发现批次。
- 监控批次只复用现有压缩中心等待草稿：固定密码、额外引擎参数和删除源文件不会进入草稿，自动启动标记强制清除。仍有监控授权时禁止删除对应配置组，根授权及最终候选均拒绝符号链接/Windows 重解析点。
- 持久监控自动化阶段已完成真实 Windows Release Tauri 生命周期门禁：实际目录事件覆盖启用后发现、暂停不响应、恢复时把暂停期间文件纳入基线、托盘隐藏时继续生成待确认草稿、真实进程退出并重启后恢复活动监听、停用不响应、重新启用建立新基线，以及删除后不再产生草稿或持久批次。每轮发现都断言不创建任务、不请求自动启动、不携带密码且不删除源文件。
- 本阶段复验命令为 `npm.cmd run test:e2e:desktop:watch-folder`；同时通过前端 184 项单测、类型检查、无测试桥的生产构建和 Chromium 任务模板交互回归。监控生命周期可以收口，下一阶段先审计自适应资源调度如何复用现有并发设置、磁盘空间预检、任务事务和固定机器性能基线，不直接改变默认并发参数。
- 监控文件夹的第一道安全门已进入独立分支：配置组卡片可对用户当次选择的目录执行一次性只读预览，返回稳定且通过规则的文件、排除原因和扫描边界；结果不持久化，不创建草稿或任务，也没有后台监听器。
- Rust 扫描固定为最多 1000 个普通文件、32 层目录、稳定排序且不跟随符号链接/Windows 重解析点；包含规则支持相对路径，排除规则保持最高优先级。稳定性只依据间隔 750 ms 的大小与修改时间双快照，界面明确说明它不等于持续监控。
- Windows 候选桌面门禁已经完成：真实 Release Tauri/WebView2 会话覆盖“导出→预览→导入”“选择文件→计划审计→创建等待草稿”和“选择目录→只读规则预览”，并验证模板不携带固定密码、删除源文件或额外引擎参数，导入后自动应用关闭，草稿不自动执行。
- 文件夹预览关闭后再写入新的匹配文件，不会创建任务或草稿，证明一次性预览没有残留后台监听器；桌面门禁同时继续覆盖真实格式回环、取消清理、活动任务退出确认、更新阻断和托盘二次实例恢复。
- 当前阶段的下一步不是直接自动压缩：单独设计持久目录授权、事件合并、跨事件文件身份去重、启动/暂停/停用生命周期。即使以后启用监控，首版也只能进入现有待确认草稿，不得默认执行、带入密码或删除源文件。
- 可执行任务模板第二阶段已进入独立分支：配置组和 `long-decompress-task-template` v1 增加包含/排除 glob，排除优先；数据库 v4 迁移会保留已有配置，并为旧记录补空排除列表。
- “从模板创建待确认草稿”已经接入现有压缩中心。用户显式选择文件后，Rust 重新读取最多 1000 个候选的真实元数据、去重并返回接受/排除原因；确认前不改变任务状态，确认后也只创建等待分组，不调用压缩命令。
- 安全边界保持不变：固定密码、保险箱绑定、删除源文件、额外引擎参数和自动写入保险箱不跨设备；草稿密码强制清空、删除源文件强制关闭、自动启动标记强制清除。规则型模板首阶段不递归展开目录，目录候选会提示用户显式选择文件。
- 当前阶段累计验证为 Rust 211 项通过、1 项固定机性能基准按设计忽略，Clippy 零警告；本轮前端 181 项单测、类型检查和无测试桥的生产构建通过，Playwright 五类桌面/移动配置按 CI 单 worker 执行，32 项通过、13 项按浏览器策略跳过。真实 Windows 桌面矩阵和生命周期门禁通过；可选 WSL GPT/MBR 生成在冷启动机器上使用 120 秒实际生成预算，仍执行非空载荷和最终内容校验。
- 实机门禁还发现成功 Toast 会覆盖后续工具栏按钮：现在提示主体允许点击穿透，仅关闭按钮接收点击，并有组件回归保护。桌面测试桥改为在原生事件监听和窗口恢复完成后才暴露，避免退出确认等生命周期检查抢跑。
- 归档内图片受限预览首阶段已进入独立分支：ZIP 与 TAR 系列支持 PNG/JPEG/GIF/WebP/BMP 单条目内存预览，严格限制解压后 8 MiB、单边 8192 和总计 1600 万像素；TAR 流在到达目标条目前最多扫描 64 MiB，声明长度与实际载荷不一致时拒绝预览；魔数不符、SVG/AVIF/ICO、扩展名伪装、超限与非规则条目不会渲染，也不会创建临时文件。
- 7Z 固实块和当前 RAR 读取无法证明严格有界，因此预览按钮明确禁用并说明原因；浏览、密码识别与选择性解压能力保持不变。真实 ZIP/TAR.GZ、SVG 脚本伪装、超像素与不支持路线样本已通过，下一阶段为可执行任务模板。
- 本阶段累计验证为 Rust 202/203（固定机性能基准 1 项按设计忽略）、Clippy 全目标全特性零警告、前端 173 项单测、类型检查和生产构建通过；Playwright 五类桌面/移动配置 31 项通过、9 项按浏览器策略跳过。全量矩阵曾发现移动端文件区被工具栏压缩，现已改为页面纵向滚动并为工作区保留稳定高度。
- 归档诊断与 ZIP 修复首阶段已进入独立开发分支：文件完整性中心新增第三模式，结构化展示实际格式、加密/分卷、缺卷、条目与展开大小、完整性分类、可恢复性及可复制证据；密码只传给后端验证，不进入报告或证据。
- 已删除原先调用不可靠 7-Zip `r` 命令的伪修复路线。新修复只针对未加密 ZIP，将可完整读取且路径安全的条目重建到新文件，完整校验后才发布；已有目标、原包、截断中央目录、取消和失败路径均不会被覆盖。
- 真实诊断样本覆盖健康 ZIP/7Z、CRC 损坏、截断、ZIP/7Z/RAR 缺卷、加密 ZIP/7Z 缺少/错误密码、结构诊断和取消。正式合并前仍需固定 RAR 样本与 Windows 候选桌面抽查；图片受限预览首阶段现已完成，之后进入可执行任务模板阶段。
- 本阶段验证：归档诊断/修复后端 10 项真实样本通过，Rust 全库 197 项通过、1 项固定机性能基准按设计忽略，Clippy 全目标全特性零警告，前端 171 项单测、类型检查和生产构建通过；Playwright 五类桌面/移动配置 29 项通过、6 项按浏览器策略跳过，并覆盖 1440/1024/760/390 四档诊断页无横向滚动。
- 智能压缩分析首阶段已进入独立开发分支：原 `estimatedSize` 占位现已由 Rust 有界抽样结果驱动，限制为 100,000 个元数据条目、16 个抽样文件和 2 MiB 内容样本；分析可取消，结果按单文件/压缩组隔离，并可映射到现有格式、等级和固实压缩设置。
- 智能建议默认不会自动覆盖用户设置；格式或等级改变后结果会标记过期。普通非分卷任务完成后回填实际输出体积和预测误差，分卷任务暂不使用单卷大小制造错误误差。
- 真实后端样本已覆盖高可压缩文本、既压缩型高熵内容、混合目录和预取消；前端覆盖分析、采用建议、结果过期、取消、实际体积与误差。候选桌面仍需抽查真实图片/视频/大型目录；其后续的归档诊断与 ZIP 修复首阶段已在本分支完成。
- 当前累计审计通过 Rust 202/203（固定机性能基准 1 项按设计忽略）、Clippy 全目标全特性零警告、前端 173 项单测、类型检查、生产构建，以及 Playwright 五类桌面/移动配置 31 项通过/9 项按浏览器策略跳过。
- 压缩包浏览中心第一阶段已进入独立开发 PR：结构化浏览、目录/搜索/类型筛选、精确选择性解压和任务重试已闭环；ZIP/7Z/RAR 密码元数据不进入命令行，留空密码复用保险箱候选。真实矩阵已覆盖中文长路径 ZIP、普通/加密 7Z 与 TAR 系列，正式候选仍需随既有固定 RAR 样本执行桌面格式矩阵。
- Mark-of-the-Web 第一阶段已进入独立开发 PR：默认开启并可在设置中心关闭；源归档仅接受 Internet/Restricted 区域标记，标记在暂存区写入后随事务提交，取消和回滚均有真实 Windows ADS 回归。合并前还需完成候选桌面界面验收。
- 压缩后校验已进入独立开发分支：普通/加密/分卷/AES 路线在最终发布前执行完整性验证，删除源文件会强制校验；配置组新增安全默认字段和 v3 数据库迁移。合并前仍需候选桌面界面验收。
- 下一阶段不再以增加格式数量为主，主线调整为安全解压、压缩后校验、归档浏览、智能压缩和诊断修复。
- 完整实施顺序、竞品参考、融合约束和验收条件见 [PRODUCT_ENHANCEMENT_ROADMAP.md](PRODUCT_ENHANCEMENT_ROADMAP.md)。
- 第一项开发为 Mark-of-the-Web 传播：必须接入现有解压事务，不能在事务外直接写最终输出。
- 第二项为压缩后自动校验：复用临时输出和完整性检测；“删除源文件”必须在校验成功后执行。
- 浏览中心复用 `list_archive_contents`、格式能力矩阵、密码保险箱和选择性解压事务；修复中心复用 `repair_zip`，且始终先输出新文件。
- `estimatedSize` 应扩展为受限抽样和智能建议，不另建压缩设置模型；自动化任务必须继续进入现有任务中心。
- 每项增强独立 PR、独立真实样本和桌面闭环。无签名 Windows 11 顶层菜单、自解压 EXE、原地归档编辑和实验性并行解压暂不抢占主线。

## 2026-08-07 经典右键菜单实机修复

- v1.0.23 实机发现 `ExtendedSubCommandsKey` 布局只显示 Long解压 父项，没有二级菜单；点击父项会触发 Windows“没有与之关联的应用”错误。
- 当前修复改用空 `SubCommands` 值配合父项下的内联 `shell` 子键，并让 Long解压、一键解压、一键打包统一声明 `Position=Top`，保持同组排列。
- 注册状态、已安装版本和公开更新脚本必须同时验证空 `SubCommands`、内联命令数量、目标程序路径和顶部排序；不能只检查注册表键是否存在。
- 本修复不改变 Windows 11 新式第一层菜单的签名边界：没有可信代码签名证书时仍使用“显示更多选项”中的经典菜单。
## 2026-08-07 前端工具链安全迁移

- Vite、Vue 插件、Vitest、V8 覆盖率和 vue-tsc 已迁移到 `8.2.1 / 6.0.8 / 4.1.10 / 4.1.10 / 3.3.9`，Node 约束固定为 `^20.19.0 || >=22.12.0`，与 CI 的 Node 24 对齐。
- 公开 npm 安全源的完整审计和生产依赖审计均由 7 项开发工具链漏洞降为 0；没有使用 `npm audit fix --force`，也没有迁移 Tauri、Tailwind、Pinia 等业务或平台依赖。
- Vitest 4 的 V8 AST 重映射改变了 Vue/TypeScript 的覆盖率统计口径。配置已移除会产生假 `0/0` 的旧 `all/include` 组合，明确排除测试夹具，并以本次真实结果重新建立 `67/55/58/69` 的 statements/branches/functions/lines 防倒退门槛。
- Vite 8 的严格检查同时消除了 `vitest.config.ts` 中的 `__dirname` 警告和非 scoped 样式中的无效 `:global(...)` 写法。后续升级必须继续通过类型检查、184 项覆盖率测试、浏览器 E2E 和生产构建。
> 2026-07-30 详细代码审计见仓库根目录
> [DEVELOPMENT_AUDIT_2026-07-30.md](../../archive/legacy-root/documents/DEVELOPMENT_AUDIT_2026-07-30.md)。

> 2026-07-31 发布后审计见仓库根目录
> [DEVELOPMENT_AUDIT_2026-07-31.md](../../archive/legacy-root/documents/DEVELOPMENT_AUDIT_2026-07-31.md)。

## 2026-08-01 性能趋势工具

- `npm.cmd run performance:baseline` 会以 Rust Release 配置运行大文件 ZIP、小文件 ZIP、原生 7Z 和 AES v2 真实往返，并生成结构化 JSON。
- 结果包含机器指纹、活动电源计划、Git/工具链、逐次指标及中位数/极值；跨机器基线会被拒绝。
- 少于 10 次样本只用于烟雾检查，不应用回归阈值；固定机器首份 10 次结果建立后，才可用 `-BaselinePath` 做版本趋势门禁。
- 操作方法和约束见 [PERFORMANCE_BASELINE.md](PERFORMANCE_BASELINE.md)。下一批只在有稳定真实样本和可比指标时扩展 TAR 包装或只读格式，避免用模拟数据充数。

## 2026-08-01 同主版本依赖更新

- Vue 及其 compiler/runtime overrides 已统一到 3.5.40；Playwright 为 1.62.1，Vue Test Utils 为 2.4.11。
- GSAP、PostCSS、Autoprefixer 已更新到各自当前主版本内的 3.15.0、8.5.25、10.5.4。
- 干净 `npm ci`、184 项覆盖率测试、25 项多浏览器 E2E、类型检查、生产构建和版本身份通过。
- `npm audit --omit=dev` 仍为 0；完整审计仍为 15 项开发工具链漏洞。不要运行 `npm audit fix --force`，后续按 Vite/Vitest/vue-tsc 等主版本分别迁移。

## 2026-08-01 v1.0.21 发布收口

- `v1.0.21` 已由发布提交 `97cc7e63e15dc0870f7603eb13edcacea754043a` 构建并公开，安装器、updater ZIP、签名和 `latest.json` 四项资产齐全。
- 主程序 166 项 Rust 核心测试、全目标/全特性矩阵、Clippy、Shell Extension、184 项前端覆盖率、生产构建、浏览器/桌面 E2E 和 NSIS CI 均通过。
- 本地生产 NSIS 完成 v1.0.20 → v1.0.21 → 卸载 → v1.0.20 恢复 41 项验证；公开签名更新另完成 21/21 项验证。
- 公开更新确认升级前进程退出、新版独立重启、安装目录和两套用户数据保持、唯一 1.0.21 Shell DLL，以及四类传统菜单根/17 条命令全部有效。
- 真实更新验收曾暴露自动化过早终止新版进程的问题；PR #45 增加 PID 代际、启动迁移等待和 SQLite 校验前精确停进程，五组 CI 通过后已合并。
- 正式证据见 [RELEASE_VALIDATION_1.0.21.md](RELEASE_VALIDATION_1.0.21.md)和
  [Issue #46](https://github.com/Longyuyeee/long_Decompress/issues/46)；v1.0.20 回归 Issue #39 已由该证据关闭。

### 下一阶段优先顺序

1. 为 Windows self-hosted runner 接入真实已安装桌面/更新 E2E，降低发布后才能验证的人工依赖。
2. 分支化升级 Vite、Vitest、vue-tsc 等开发工具链主版本；保持生产审计为 0，禁止使用 `npm audit fix --force` 批量跨主版本改写。
3. 只在获得可复现非空样本与内容校验后补充 HFSX/扩展格式矩阵，不扩大未经桌面闭环验证的公开格式声明。
4. 获取可信 Windows 代码签名证书后，再启用 Windows 11 新式顶层菜单身份包；在此之前继续使用已验证的传统级联菜单。

## 2026-08-01 右键菜单与发布追溯更正

- v1.0.20 的更新、资产、重启、安装路径和用户数据保持通过，但传统右键二级菜单存在发布后回归；总体发布验收为 `INCOMPLETE`。
- 原因是 HKCU CommandStore 子命令不能作为可靠的按用户级联实现；修复改用 `ExtendedSubCommandsKey` 内联子命令。
- `test:installed-release` 与 `test:public-update` 必须验证四类菜单根下合计 17 条命令，不能只检查顶层键。
- 正式记录见 [RELEASE_VALIDATION_1.0.20.md](RELEASE_VALIDATION_1.0.20.md)和
  [Issue #39](https://github.com/Longyuyeee/long_Decompress/issues/39)；修复实现见 PR #38。
- 下一补丁版必须从 v1.0.20 执行真实应用内更新，得到严格菜单证据后才能关闭 Issue #39。

## 2026-07-31 交接点

- 当前稳定基线为 `master` / `5f4505686a8ea0770b5de5178d9ad6433967fb4e`，正式版本为 `v1.0.20`。
- `v1.0.20` GitHub Release 已发布，包含 NSIS 安装器、updater ZIP、签名文件和 `latest.json`。
- 已从公开安装的 v1.0.19 通过应用内更新升级到 v1.0.20，`test:public-update` 验证安装、签名下载、自动重启、安装路径保持、用户数据保持、传统右键菜单资源和版本化 Shell DLL 均通过。
- v1.0.20 发布复验 Issue #39 保持开放，直到下一补丁版取得严格 17 条传统子菜单更新证据；生产依赖审计为 0。
- 完整 npm audit 仍有 15 项开发工具链漏洞，修复需要 Vite/Vitest/vue-tsc/@vue/test-utils 等主版本迁移，必须放到独立迁移分支。
- 下一阶段优先顺序：发布验收记录回链、固定 Windows 性能趋势、self-hosted 桌面 E2E、HFSX/扩展格式真实样本补齐、依赖与平台现代化。
- 不要再按 2026-07-30 的旧步骤创建 `v1.0.20` 标签或等待发布 PR；这些动作已经完成。

## 2026-07-30 阶段暂停点

- 当前工作分支为 `agent/release-1.0.20` 的收口修复分支，版本仍为 `1.0.20`；本阶段不升版本、不创建标签、不发布 Release。
- 已通过 `test:prepare:full-format`、带 `desktop-e2e` feature 的 Release 构建，以及前端 27 个测试文件、163 项单元测试。
- 严格桌面矩阵已实际完成 25 个可创建场景、扩展名别名、虚拟磁盘、文件系统、Windows Installer、NSIS、UEFI、HFS/HFSX、CRAMFS、IHEX、DEB/UDEB 和固定上游 RAR/LHA/RPM/DMG 样本；这些场景均使用非空载荷并校验最终文件内容或 SHA-256。
- 新增固定哈希的加密 RAR 样本，错误密码不得发布明文，正确密码必须匹配两个已知文件哈希。
- 2026-07-30 已修复加密 RAR 使用错误密码时长期无响应的问题：RAR 密码验证加入限时原生预检，通用 7z 加密探测改为非交互并设置超时，密码仍不会进入外部进程命令行。
- 严格全格式真实桌面矩阵已重新通过，覆盖错误 RAR 密码快速失败、正确 RAR 密码解压、非空 HFS/HFSX、NSIS、格式别名、GPT/MBR、CRAMFS、IHEX 和已声明扩展名对账。
- 对外格式声明已收紧：暂时移除只有引擎识别能力、但没有非空桌面闭环证据的 `ppkg`、`apm`、`scap`、`udf`、`arj`、`chm`、`z`、`taz`。以后只有补齐真实样本后才能重新公开。

### 接手后按此顺序继续

1. 将 RAR 密码超时修复合入发布收口 PR，并等待 GitHub CI 全部通过。
2. PR 合并后，从 `master` 重新构建正式 `1.0.20` NSIS、updater ZIP、签名文件和 `latest.json`。
3. 创建 `v1.0.20` Release 后，从保留的 `v1.0.19` 环境执行 `npm.cmd run test:public-update`，回填应用内更新和自动重启证据。
4. 发布完成后再开启下一阶段：固定 Windows 性能趋势、自托管桌面 E2E、依赖主版本迁移和代码签名证书事项。
5. Windows 11 顶层右键菜单的生产签名证书仍然没有，继续作为非阻塞限制记录，不要伪造签名验证结果。

## 当前情况

- 当前正式基线：v1.0.19，默认分支为 `master`。
- 压缩与解压主流程、任务状态、取消、冲突处理和真实桌面 E2E 已建立。
- 已完成 ZIP、7Z、TAR 系列及多种只读归档、文件系统、虚拟磁盘的真实载荷验证。
- 本轮新增 MSI、MSM、MSP、APFS 和 UEFI 固件的可复现测试样本，并已通过 Release Tauri 桌面闭环；MSM 会继续解开内嵌 CAB，不再只输出中间容器。
- 全格式桌面 E2E 新增严格模式；缺少任一外部生成器时会汇总失败，不再允许将静默跳过误记为全格式通过。
- 2026-07-30 已运行 `npm.cmd run test:prepare:full-format` 和
  `npm.cmd run test:e2e:desktop:full-format`：25 种可创建格式、虚拟磁盘、FAT16/NTFS、APFS、
  SquashFS、MSI/MSM/MSP、UEFI 与固定上游样本均完成真实载荷闭环。
- 桌面测试现在为每次运行生成独立实例名、IPC socket、数据目录和 WebView2 用户目录；即使旧 E2E
  进程异常残留，也不会阻断新会话或污染固定样本验收。
- Windows 11 顶层右键菜单仍受签名证书限制，当前不作为开发阻塞项。
- 历史说明：本段当时尚不能生成非空 HFSX；该限制已在 2026-08-27 由固定工具提交、已知载荷和 Release Tauri 门禁解除，当前口径见 `FORMAT_SUPPORT_LEVELS.md`。
- v1.0.18 候选已修复无签名 Windows 11 菜单降级与覆盖安装残留，并通过公开 v1.0.17
  覆盖安装、41 项状态/数据检查、卸载和基线恢复。
- v1.0.18 正式更新的签名下载与覆盖安装成功，但被动安装没有自动重启；v1.0.19 已修复。
  公开 v1.0.18 → v1.0.19 的独立 WebView2 UI 更新验收 18 项全部通过。
- 压缩中心工具栏、状态进度和执行日志已拆为稳定组件；配置组调用层已统一 Rust `snake_case`
  与前端 `camelCase`，复杂密码策略和推荐配置组路径具备直接回归。
- 归档魔数/扩展名识别已提取到 `archive_format.rs`，压缩能力、别名归一化、请求校验和执行路由已提取到
  `compression_format.rs`；公开 `CompressionService` 门面保持兼容，核心文件从 4,281 行降至 3,961 行。
  所有声明别名均有统一路由回归，`.tpz` 已明确按 TAR+GZIP 容器处理。
- 暂存生命周期、资源限制、路径/reparse point 安全、冲突决策和事务提交已提取到
  `extraction_transaction.rs`；核心文件进一步降至 3,613 行。暂存 RAII 守卫覆盖异步提前返回，
  资源扫描同时统计目录和文件，回滚不完整会显式报告。
- TAR、TAR.GZ/BZ2/XZ/Zstandard 与 GZ/BZ2/XZ/Zstandard 单文件流原生解压实现已迁移到
  `native_extraction/`；取消、进度和日志通过 `ExtractionRuntime` 注入，核心文件降至 3,502 行，
  事务、格式路由和密码语义未改变。
- ZIP 原生解压实现已迁移到 `native_extraction/zip.rs`；密码预检会检查全部归档条目，
  混合 ZIP 中后置的加密文件不再漏检，错误/取消路径也会显式归还 I/O 缓冲区。
  继续完成 7Z 拆分后，`compression_service.rs` 已降至 3,074 行。
- 7Z 原生解压已迁移到 `native_extraction/seven_zip.rs`。CRC/密码错误分类结合归档加密元数据判断，并由真实损坏包及真实加密包
  覆盖缺少、错误、正确密码；文件时间戳、仅解压较新文件三态、过滤、取消、跳过损坏和暂存回滚也有真实回归。
  损坏/取消路径会删除半文件，生产路由和暂存事务接口未改变；可注入写入边界已用真实 7Z 验证中途 `StorageFull`，
  会明确返回磁盘空间不足、删除半文件和暂存目录，并保持原目标目录不变。
- 普通、AES-256 加密及分卷 ZIP 写入已迁移到 `native_compression/zip.rs`，新写入模块通过
  `CompressionRuntime` 复用取消、进度和日志能力，条目收集规则迁入 `compression_entries.rs`。
  `compression_service.rs` 已降至 2,977 行；154 项 Rust 测试、Clippy 和严格全格式桌面矩阵通过。
- 普通及 AES 加密 7Z 写入已迁移到 `native_compression/seven_zip.rs`，复用相同运行时与条目收集边界；
  字节进度、中途取消、密码归档和输出清理由直接回归及重新构建的严格桌面矩阵保护。
  `compression_service.rs` 已降至 2,901 行；157 项 Rust 测试和 Clippy 通过。
- TAR/TAR.GZ/BZ2/XZ/Zstandard 写入已迁移到 `native_compression/tar.rs`，
  GZ/BZ2/XZ/Zstandard/LZMA 单文件流写入已迁移到 `native_compression/single_stream.rs`。
  密码回退和 AES 包装调用链保持不变；核心文件降至 2,526 行，158 项 Rust 测试、Clippy 和重新构建的
  严格全格式桌面矩阵通过。
- TAR.AES 与八种 `*.AES` 包装格式的写入编排已迁移到 `native_compression/aes.rs`，临时输入、取消和
  加密失败清理均由模块内回归保护。计划内写入职责拆分完成，核心文件降至 2,364 行；159 项 Rust 测试、
  Clippy、AES 字节往返和重新构建的严格桌面矩阵通过。
- 压缩公共发布出口现在统一使用唯一临时输出并规范化系统级 `StorageFull`；压缩失败、发布竞态或磁盘写满均不会覆盖
  已出现的目标，并会清理未发布输出和临时旁车。连同 7Z 解压写满事务测试，当前 161 项 Rust 测试、Clippy
  和重新构建的 Release Tauri 严格全格式桌面矩阵通过。
- `1.0.20-rc.2` 候选已由 PR #35 合入 `master`；正式发布分支已将八处版本来源和版本化 Shell DLL 收敛为 `1.0.20`。
  不含 E2E 桥的正式生产 NSIS 已从公开 `1.0.17` 完成覆盖安装、原目录保持、两套用户数据指纹保持、传统菜单注册、
  卸载清理和基线恢复；42 项真实安装检查全部通过，机器已恢复 v1.0.17 基线。安装包 SHA-256 为
  `2D9ED1CA8098D258A30D0A18CA3750261F0961550BFAA8D924702E633E4782B5`。验收脚本会备份并精确恢复
  应用自有右键注册表树，预发布版本 DLL 名称也按统一规则校验。
- `rc.1` 已安装候选的 ZIP、7Z 压缩及 7Z 快速解压完成真实界面抽查，普通与中文文件名往返哈希一致。抽查发现并修复了
  “完成的同源 ZIP 行会静默阻止新的 7Z 请求”：终态行现在会被新格式任务替换，活动任务重复请求会明确提示。`rc.2`
  已安装版在不清理完成 ZIP 行的前提下成功生成同源 7Z，并完成最终界面验收：ZIP 行被 7Z 行原位替换，三列表头、
  完成状态、100% 进度及展开后的配置/阶段/实时日志布局均正确，没有任务重叠或操作区下沉。
- 前端 30 个测试文件、182 项通过，覆盖率达到 75.22% 行/语句、78.03% 分支和 63.68% 函数；
  防倒退门槛提高到 75% 行/语句、75% 分支和 60% 函数。
- 生产 npm 依赖审计为 0；完整审计报告的 15 个漏洞均来自开发工具链，自动修复要求跨主版本升级
  vue-tsc/Vite/Vitest，应放入独立迁移阶段，不与归档引擎改动混合。

## 接手后要做什么

1. 找到可再分发、非空的 HFSX 样本或可靠写入工具，补齐 HFSX 真实载荷验证。
2. 原生解压、计划内写入职责、磁盘写满、`1.0.20-rc.2` 安装生命周期及“同一来源完成 ZIP 后直接请求 7Z”已经收口；
   最终任务行界面也已验收。当前只需完成正式 `1.0.20` 版本身份、README、Release 文档和安装包复验。
3. 后续改动继续使用 `npm.cmd run test:e2e:desktop:full-format` 作为严格回归；只识别文件头、空镜像或损坏样本不算通过。
4. 每次正式发布后继续用 `test:public-update` 从上一正式版本执行独立 WebView2 更新验收。
5. 正式发布 PR 通过后合入 `master` 并创建 `v1.0.20` 标签，由 Release 工作流生产签名 updater 资产；发布后立即从
   保留的 v1.0.19 环境执行 `test:public-update`，回填应用内更新和自动重启证据。
## 2026-08-27 B-02 Release/WebView2 桌面矩阵通过

- 隔离 Release/WebView2 双尺寸门禁已真实通过并人工检查截图：JPEG/PNG/WebP 能预览，GIF/PDF 明确拒绝，1100×720 与 760×560 无横向溢出，归档/图片队列互不污染。
- 首次失败确认 Windows `convertFileSrc` 使用 `https://asset.localhost`，CSP 已补齐且 Release 身份门禁会锁定空默认 scope 与两种资产源；随后将 Orientation=6 的公开尺寸明确为应用方向后的 360×640，编码矩阵 640×360 和方向值仍单独验证。
- B-02 仍未收口：下一步只补系统文件选择器或真实拖放路径，然后做全量回归审计；不得提前进入 B-03、升版或发布。证据见 [B02_IMAGE_WORKSPACE_PAUSE_AUDIT.md](B02_IMAGE_WORKSPACE_PAUSE_AUDIT.md)。
## 2026-08-28 B-02 系统选择路径纠偏

- 修复真实 `dialog.open` 路径未读取磁盘元数据、图片大小先显示 0 B 的偏移；文件/目录选择、测试桥和原生拖放现在统一调用 `get_file_info`。
- 图片系统选择器改用“选择图片文件”标题并允许选中 GIF 后由统一业务规则明确拒绝。组件测试与 Windows Release/WebView2 可见入口门禁通过，JPEG 使用真实字节和方向后尺寸，GIF Toast 拒绝且不入队。
- 当前 Codex 宿主可打开、枚举和预选真实 Windows 对话框，但阻止后台测试进程完成受信任点击；实验性调度器已全部撤回。B-02 仍需一次有人值守的系统选择，不能宣称收口或进入 B-03。证据见 [B02_NATIVE_PICKER_PATH_AUDIT.md](B02_NATIVE_PICKER_PATH_AUDIT.md)。
- 有人值守门禁现可通过 `npm.cmd run test:e2e:desktop:image-picker-manual` 启动隔离 Release/WebView2 会话；脚本不注入选择结果，会自动点击可见投放入口打开真实系统对话框，只在用户完成对话框操作后自动审计 JPEG 字节与方向尺寸、预览、GIF 拒绝、队列和焦点，并保存本地证据。
## 2026-08-28 B-02 图片前端工作区收口

- 原生门禁由产品可见投放入口真实打开 Windows `#32770`“选择图片文件”对话框，系统选择状态确认 `exif-orientation.jpg` 与 `animated.gif` 同时选中后由标准 `IDOK` 返回路径；未调用测试桥选择队列，也未把后台坐标输入冒充通过。
- 返回应用后自动证据为 JPEG 15,788 B、360×640、预览完成；GIF Toast 明确拒绝且不入队；WebView 确实失焦并重新获焦。`test:e2e:desktop:image-picker-manual` 退出码 0，本地 PNG/JSON 证据不提交。
- B-02 至此收口，不升版、不发布。下一步进入 B-03，只接入 B-01 已审计图片引擎和 B-00 共享发布事务；真实输出重新解码、失败/取消清理与发布竞态通过前，执行按钮继续禁用。

# 2026-08-28 C-03.1 视频执行基础

- C-03.1 已完成：FFmpeg 参数固定为 `Vec<OsString>`，真实中文/空格/`&`/括号路径使用产品 `h264_mf` 软件编码和 AAC 转码通过，不经过 shell。
- 进度只解析 `-progress pipe:1` 的机器字段；百分比、临时输出大小、速度、比例均保留事实来源，ETA 至少等待两个递增时间样本。
- Windows Job Object 使用 `KILL_ON_JOB_CLOSE`，真实分配和终止进程通过。视频 UI 仍禁止执行，C-04 前不发布、不写最终成功历史。下一接续点为 C-03.2 暂存执行器、统一取消/事件、心跳和完整临时清理。详见 [C03_1_VIDEO_EXECUTION_FOUNDATION_AUDIT.md](C03_1_VIDEO_EXECUTION_FOUNDATION_AUDIT.md)。

# 2026-08-28 C-03.2 视频暂存执行器

- 内部异步执行器已复用容量预检与同目录唯一暂存；机器 stdout、持续排空且保留 64 KiB 的 stderr、5 秒心跳、50 ms 取消轮询、Job Object 与 Future drop 清理均已落地。
- 真实特殊字符输入成功编码但不发布，暂存所有者 drop 后清除；产品 FFmpeg 现场生成约 500 秒输入，心跳触发取消后 5 秒门限内结束，最终/暂存均无残留。
- 为守住 C-04 验证发布边界，本节点不注册 Tauri 命令、不解锁 UI。下一接续点纠正为 C-03.3：复用统一取消注册表、输出锁、任务事件和视频任务 UI，但 C-04 前仍不能标记最终完成。详见 [C03_2_VIDEO_STAGING_EXECUTOR_AUDIT.md](C03_2_VIDEO_STAGING_EXECUTOR_AUDIT.md)。

# 2026-08-28 C-04.1 视频输出验证

- 实际依赖已纠偏为先验证/发布、后接统一命令/UI；否则 C-03.3 会暴露未验证暂存文件。C-04.1 只接受 C-03 所有权对象，不发布。
- 产品 ffprobe 验证 MP4/H.264/AAC、流数量、编码/可见尺寸、0° 旋转、字幕/章节/附图清除和有界时长偏差，并以全流 `-count_frames` 复核音视频可解码帧数。
- 真实 `faststart` MP4 截半后仍保留元数据和 2 个视频帧，首版“至少一帧”会误放行；现已增加视频/AAC 最低帧数和编码后大小不变校验，截断稳定拒绝。下一节点为 C-04.2 Mark-of-the-Web、取消复检、共享原子发布和发布后最终事实。详见 [C04_1_VIDEO_OUTPUT_VALIDATION_AUDIT.md](C04_1_VIDEO_OUTPUT_VALIDATION_AUDIT.md)。

# 2026-08-28 C-04.2 视频原子发布

- 验证结果只能连同其 `StagedVideoOutput` 所有者发布；发布前复核源/暂存字节、取消和 Mark-of-the-Web，再调用共享 `publish_verified_file`，最终字节取发布后文件系统。
- 真实原子发布、NTFS ZoneId=3/HostUrl 逐字节传播、发布前取消和目标竞态通过；竞态目标 `existing-user-bytes` 未被覆盖，失败暂存无残留。
- 源文件回收没有提前进入发布服务。下一步 C-04.3 补齐编码非零退出/终止、源或暂存改写、零字节、容量和竞态跨层矩阵；通过后关闭 C-04，再接 C-03.3 完整任务/UI。详见 [C04_2_VIDEO_ATOMIC_PUBLICATION_AUDIT.md](C04_2_VIDEO_ATOMIC_PUBLICATION_AUDIT.md)。

# 2026-08-28 C-04.3 视频失败矩阵收口

- 新增产品 FFmpeg 非零退出、编码后清零、验证后源改写三条真实负向路径，连同既有截断、Job 终止、容量门禁、取消和目标竞态，全部不发布且无暂存残留。
- `video_` 定向矩阵 35/35 通过。C-04.1 至 C-04.3 已覆盖最初容器/流/时长/解码、无效输出、源回收时序和失败不覆盖要求，C-04 关闭。
- 下一接续点回到 C-03.3：用单个安全命令串起重新探测/规划、编码、验证和发布，复用统一取消/输出锁/任务事件/历史，再解锁视频 UI。详见 [C04_3_VIDEO_FAILURE_MATRIX_AUDIT.md](C04_3_VIDEO_FAILURE_MATRIX_AUDIT.md)。

# 2026-08-29 C-03.3.1 视频单命令安全管线

- 唯一 `compress_video_file` 已注册：冻结引擎校验、重新探测/规划、暂存编码、完整验证和原子发布全部在一次命令内完成，未验证暂存不会返回给前端。
- 命令复用统一取消注册表、输出占用锁、`task-log` 与 `task-progress`；媒体字段包含当前时间、速度倍数、临时输出大小/比例、ETA，以及不制造新进度的 `still-encoding` 心跳。
- 流变化确认从布尔值纠正为完整列表精确比较；规划后源内容变化导致风险列表漂移时拒绝执行并要求重新确认。真实拒绝/成功命令测试和视频 37/37 定向矩阵通过，源文件保持不变。
- 视频工作区按钮仍关闭。下一步 C-03.3.2 接入统一任务创建、显式风险确认、任务详情、取消、最终 `TaskMetricsV1` 与历史；只有命令返回发布事实后才能标记 completed。详见 [C03_3_1_VIDEO_COMMAND_PIPELINE_AUDIT.md](C03_3_1_VIDEO_COMMAND_PIPELINE_AUDIT.md)。

# 2026-08-29 C-03.3.2 视频统一任务 UI

- 视频工作区已开放真实执行：后端安全规划不覆盖的 `.compressed.mp4`，系统对话框逐项确认流变化，拒绝时不创建任务；执行仍只调用唯一安全命令。
- 每个视频复用统一 task store、`cancel_compression` 和历史写入口；当前时间、速度、临时大小/比例、ETA、心跳及最终发布事实可见。只有命令返回后才生成视频 `TaskMetricsV1` 并标记 completed。
- 全局归档重试已限制为 archive，避免失败媒体被错误送入归档命令；当前视频无源回收选项，源文件保持只读。
- 定向前端 33/33、视频 Rust 38/38、视频历史、类型、Clippy 和 17 文件媒体架构门禁通过。C-03 关闭；下一接续点为 C-05 真实格式/分辨率/预设/批量/取消/历史/安装版矩阵。详见 [C03_3_2_VIDEO_TASK_UI_AUDIT.md](C03_3_2_VIDEO_TASK_UI_AUDIT.md)。

# 2026-08-29 C-05.1 视频真实桌面执行

- 原 C-02 只读桌面门禁已纠偏为 C-05.1 真实执行：Release WebView2 从可见工作区确认两条视频，串行进入唯一安全命令并完成验证、原子发布和 measured 历史。
- 实测覆盖 1.2 秒 VFR/90° 旋转/AAC/字幕 MP4，以及 30.536 秒双音轨/字幕 MP4；两份输出均由产品 ffprobe 复核为 MP4/H.264/AAC、360×640，历史字节与磁盘一致。
- E2E 构建补齐视频输出目录和流变化确认的既有测试队列；生产构建仍走原生 Tauri 对话框。完成任务清除陈旧 `Publishing` 阶段，页面节点标识由 C-02 对齐为 C-05。
- 下一接续点为 C-05.2 格式/分辨率/三预设矩阵，并补无音频、10 分钟和大文件；取消、跨重启历史、默认应用、安装版、Windows N 与公开更新仍未完成，不得发布 v1.1.16。详见 [C05_1_VIDEO_DESKTOP_EXECUTION_AUDIT.md](C05_1_VIDEO_DESKTOP_EXECUTION_AUDIT.md)。

# 2026-08-29 C-05.2.1 视频格式/分辨率/三预设矩阵

- 新增固定清单和单命令真实矩阵：MP4、MOV、AVI、WMV、WebM，480p/720p/1080p/4K，7 次唯一产品管线执行覆盖清晰/均衡/小体积及无音频；输出统一由产品 ffprobe 复核，差异为 0。
- 4K 16:9 输入在 854×480 上限内按保持宽高比和偶数尺寸得到 852×480；首次 854×480 预期已纠偏，产品行为无需修改。
- 输入生成使用固定 SHA-256 的 GPL FFmpeg 9.0.1，严格留在忽略的 `test-results/c05-fixture-tool`，不得进入产品资源或安装包；产品探测、规划、执行、验证和发布仍走正式实现。
- 下一唯一接续点为 C-05.2.2：补 10 分钟与至少一个大文件。随后仍需取消、跨重启历史、默认应用、安装版、Windows N 前后和公开更新；不得提前发布 v1.1.16。详见 [C05_2_1_VIDEO_FORMAT_MATRIX_AUDIT.md](C05_2_1_VIDEO_FORMAT_MATRIX_AUDIT.md)。

# 2026-08-29 C-05.2.2 视频长时长/大文件矩阵

- 两个独立 AVI/MPEG-4 输入分别覆盖 600.000 秒/3600 帧和 114,842,332 B（109.52 MiB）/960 帧；均进入唯一产品压缩管线并发布 MP4/H.264，无音轨输入未被虚构音频。
- 长输出为 600.000 秒/3600 帧/59,085,065 B；大输入输出为 32.000 秒/960 帧/8,410,052 B。产品后端和独立 ffprobe 的时长差均为 0，机器结果无差异。
- “大文件”按仓库既有测试层级明确为至少 100 MiB；样本是可完整解码的真实容器，不使用稀疏、尾部填充或元数据伪造。GPL 生成工具继续严格隔离在忽略目录。
- C-05.2 已关闭。下一唯一接续点为 C-05.3：真实中途取消、跨重启历史和默认应用；之后 C-05.4 才做安装候选、Windows N 前后与公开更新。详见 [C05_2_2_VIDEO_LONG_LARGE_MATRIX_AUDIT.md](C05_2_2_VIDEO_LONG_LARGE_MATRIX_AUDIT.md)。

# 2026-08-29 C-05.3 视频桌面运行行为

- Release WebView2 可见工作区用 114,842,332 B 真实 AVI 在编码中取消：对应产品 FFmpeg 进程退出，最终输出和 `.video-encode-*` 暂存均不存在，源文件字节与 SHA-256 不变，统一历史为 `cancelled`。
- 随后两条真实视频完成验证和原子发布；原生应用完全退出并以新进程/WebView profile 重启后，两条完成记录和一条取消记录均保留，完成记录的 ID、路径与完整实测 metrics 深比较一致。
- 实际代码审计发现结果按钮只在 Explorer 定位，偏离“默认应用播放”；现已改为受绝对路径、普通非符号链接文件和 MP4 后缀约束的原生命令，Windows 默认应用接收发布结果。
- 首轮门禁因成功 Toast 的真实角色是 `status` 而测试只查 `alert` 超时；纠正选择器后从头复验通过，没有弱化产品断言。C-05.3 已关闭，C-05 尚未关闭；下一唯一接续点为 C-05.4 正式安装候选、Windows N 前后、`v1.1.16` 版本身份、公开更新和回下载复验。详见 [C05_3_VIDEO_RUNTIME_BEHAVIOR_AUDIT.md](C05_3_VIDEO_RUNTIME_BEHAVIOR_AUDIT.md)。

# 2026-08-29 C-05.4.1 正式安装版视频门禁基础

- 新增无测试桥生产视频工作区门，并接入安装生命周期 `RunVideoWorkspaceMatrix`：真实 114,842,332 B AVI 分别走 UI 取消与完成，复核 FFmpeg 退出、无暂存/发布、产品 ffprobe、默认应用和跨完整重启历史。
- 无测试桥 Release 预演 20 项通过：完成输出 8,410,052 B、MP4/H.264、1280×720、32.000 秒；取消和完成历史在新原生进程/WebView profile 中各保留一条。
- 首轮发现旧安装版进程触发单实例转发、Windows `0xC0000142` 瞬态预检和 WebView2 延迟句柄清理；分别按既有无运行前置、仅一次 150 ms 状态码重试、临时目录有界重试纠正，其他生产错误和产品残留断言未放宽。
- 当前只完成门禁基础，不冒充正式安装证据。下一步从本提交构建正式 NSIS，运行候选覆盖、视频运行时/工作区、卸载与公开 v1.1.15 恢复；通过后才关闭 C-05.4.1。详见 [C05_4_1_INSTALLED_VIDEO_GATE_FOUNDATION_AUDIT.md](C05_4_1_INSTALLED_VIDEO_GATE_FOUNDATION_AUDIT.md)。

# 2026-08-30 v1.1.16 干净候选与安装生命周期

- 首轮 CI `33261161074` 从真实 FFmpeg 发布命令发现 `out_time_us=N/A`/负启动时间协议偏差；按冻结 FFmpeg 9.0.1 源码纠正解析器后，提交 `6d3469f5d57fe29c9accabe369f89c4f86b66bbd` 的第二轮 CI `33261741520` 五个 job 全绿。
- CI NSIS 为 15,608,481 B，SHA-256 `70F4D4B3C1A86E9C92DA4E8B4C286629E4CB1376070024D2CEB157F9E258B1CA`；7-Zip 完整性、主程序 `1.1.16` 身份、唯一版本化 Shell DLL 和 8 项视频运行时精确载荷均通过。
- 从公开 v1.1.15 覆盖该 CI 候选，生产视频运行时差异 0，无测试桥 114,842,332 B AVI 的取消/完成/默认应用/跨重启历史全部通过；生命周期 50/50。候选已卸载，本机恢复公开 v1.1.15、`E:\long\Long解压`、运行进程 0。
- Windows N 经产品负责人授权暂不支持，继续保持 `windowsNRealMachinePassed=false`，不再作为 v1.1.16 阻塞项。下一唯一接续点是创建受保护 `master` PR；合并前等待 PR CI，合并后才可在合并提交创建 annotated `v1.1.16` 标签。公开 Release 资产回下载与真实 `v1.1.15 → v1.1.16` 应用内更新通过前，不得宣称正式发布完成。详见 [RELEASE_AUDIT_1.1.16.md](RELEASE_AUDIT_1.1.16.md)。

# 2026-08-30 v1.1.16 正式发布关闭

- PR #92 五个 job 全绿后以 squash 合入受保护 `master`；合并提交 `a59742265feb961ab51f9b95b4e455aa15b79bf5` 与已验证 PR 头 Git tree 完全一致。annotated `v1.1.16` 标签精确指向该提交。
- Release run `33263384953` 成功生成签名四资产；公开回下载的 manifest、签名、ZIP/NSIS 字节、7-Zip 完整性、主程序和 8 项视频运行时全部对账。真实 `v1.1.15 → v1.1.16` 应用内更新 24/24，通过后本机最终安装公开 v1.1.16、运行进程 0。
- 首轮公开更新在安装前发现偏好 `autoStart=true` 但 Run 值缺失；通过真实设置 UI 显式开启后从头重跑，没有直接写注册表或放宽断言。根因是安装生命周期恢复工具没有恢复 Run 值，且用户数据复制未证明稳定快照。
- 发布后门禁已加固：复制前/后源与备份三方指纹一致、验证暂存后同卷替换、重建竞态拒绝，成功/异常路径均精确恢复 Run 值。前两轮复验分别暴露快照差异与已存在 Run 键上的 `New-Item -Force` 权限问题；纠正后公开 v1.1.16 同版本生命周期 49/49 通过，最终用户数据指纹、菜单、自动启动、安装位置和进程状态全部正确。
- v1.1.16 视频阶段正式关闭。Windows N 仍明确暂不支持，不回填虚假实机证据；下一接续点应从总路线选择 v1.1.16 之后的未完成阶段。完整证据见 [RELEASE_AUDIT_1.1.16.md](RELEASE_AUDIT_1.1.16.md)。
# 2026-09-01 v1.1.19 双栏文件浏览器接续点（历史，已由文件顶部正式发布关闭取代）

- 当前开发分支为 `codex/dual-pane-file-browser-1-1-19`，功能提交 `0d81b3b` 已推送。目标是将“压缩包浏览”纠偏为默认双栏磁盘文件管理器，同时把原安全归档工作区保留为压缩包操作。
- 已完成双栏目录导航、多选、跨栏复制/移动、压缩/解压、重命名、新建目录、回收站删除、属性；写操作默认不覆盖，整批预检、重解析点阻断、目标暂存、BLAKE3 校验和跨卷先校验后删源均已落地。
- 已通过 Rust 真实文件系统 3/3、相关界面/归档回归 41/41、完整单元 273/273、TypeScript/Rust/生产构建和 `1.1.19` 八处身份门禁。本地正式 NSIS 已生成；第一次桌面门禁误用无测试桥正式候选而按设计超时，隔离 E2E 测试体正在用于真实桌面复验，不能把该环境错误写成功能失败或成功证据。
- 接续顺序：完成隔离真实桌面门禁并记录预期/实际 → 审计最终差异 → 提交推送文档与版本 → 等待 PR CI 全绿并合入主线 → 在合并提交打 annotated `v1.1.19` 标签 → 核验公开 NSIS/updater/signature/latest.json → 运行真实 `v1.1.18 → v1.1.19` 更新 → 将 README 的“发布目标”改为“公开稳定版”。
- 完整范围和安全边界见 [DUAL_PANE_FILE_BROWSER_AUDIT.md](DUAL_PANE_FILE_BROWSER_AUDIT.md)，发布说明见 [RELEASE_NOTES_1.1.19.md](RELEASE_NOTES_1.1.19.md)。
