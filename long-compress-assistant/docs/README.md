# 文档导航与维护规则

- **当前版本 v1.7.20**：[说明](RELEASE_NOTES_1.7.20.md)、[范围](ARCHIVE_ROADMAP_1.7.20.md)、[封板记录](TAR_SELECTED_FREEZE_1.7.20.md)。正式身份由本版 PR/Release 接续。
- **下一版仅规划**：[v1.7.21 整包提取与链接内容检查](ARCHIVE_ROADMAP_1.7.21.md)，准确状态见[开发交接](DEVELOPMENT_HANDOFF.md)。

- **历史版本 v1.6.61**：[发布下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.61)、[中文说明](RELEASE_NOTES_1.6.61.md)、[LZOP 路径与名称记录](LZOP_PATHS_NAMES_FREEZE_1.6.61.md)。已完成功能封板，正式发布身份在本版 PR 接续。
- **历史接续**：[本版路线](ARCHIVE_ROADMAP_1.6.61.md) → [v1.6.62 单文件创建](ARCHIVE_ROADMAP_1.6.62.md)仅规划，见[交接](DEVELOPMENT_HANDOFF.md)。

以下为历史阶段记录。

- **历史正式版 v1.5.39**：[发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.39)，准确开发状态见[开发交接](DEVELOPMENT_HANDOFF.md)。
- **历史开发路线**：v1.5.40 [原生 SWM 创建](NATIVE_SWM_CREATE_1.5.40.md)已完成功能封板，发布在本版 PR 接续，后续按[近期 v1.5.40～v1.5.43](ARCHIVE_ROADMAP_1.5.40_TO_1.5.43.md) → [五模块分阶段 v1.5.44～v1.5.58，v1.5.59 起主线回归](ARCHIVE_ROADMAP_1.5.44_TO_1.5.59.md)。大模块拆成可独立使用的功能阶段，小阶段可合并。

以下为历史阶段记录，不覆盖上述当前状态与路线。

- **历史 v1.5.19 功能封板**：[逐文件校验](CHECKSUM_DETAILS_1.5.19.md)、[封板记录](CHECKSUM_DETAILS_FREEZE_1.5.19.md)、[中文说明](RELEASE_NOTES_1.5.19.md)。七项 Rust、26 前端、四条原生 IPC、五条实际 UI、八张实看页面；最终候选/正式状态在本版 PR 与[发布页](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.19)接续。
- **历史阶段原排期**：[v1.5.20～v1.5.22](ARCHIVE_ROADMAP_1.5.20_TO_1.5.22.md)：LZIP 数字分卷、扩展 AES 内层、大清单真实进度与停止/继续。准确状态见[开发交接](DEVELOPMENT_HANDOFF.md)。
- **历史 v1.5.18 已正式发布**：[AES 数字分卷](AES_VOLUMES_FREEZE_1.5.18.md)、[Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.18)、[PR #213](https://github.com/Longyuyeee/long_Decompress/pull/213)。真实 VHD 盘满补测已通过。

- **历史正式版：[v1.5.14 TAR 与真实路由](TAR_ROUTING_FREEZE_1.5.14.md)**已封板发布：203 Rust、170 前端、36 桌面记录、18 张实看页面及普通生产通过，PR #208 受保护合入，正式四资产与中文说明已回下载核对；[下载安装包](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.14)。

- **历史接续：v1.5.15 ZIP64 大边界，再 v1.5.16 过程 I/O**。原十目标按证据调整范围交付八项、余两项，见[交接](DEVELOPMENT_HANDOFF.md)和[路线](ARCHIVE_ROADMAP_1.5.6_TO_1.5.15.md)。v1.5.13 撤回不计交付；ALZ 方法 3 与 EGG 加密固实未完成状态保留，不重复扩张规划。

- **撤回记录：[v1.5.13 TAR 候选](TAR_SPARSE_FREEZE_1.5.13.md)**未正式发布。原标签保留、正式构建取消；原本地封板和候选证据供追溯，不替代当前 v1.5.14 重新验收。

- **历史记录：[v1.5.12 ALZ 完整性](ALZ_STREAM_INTEGRITY_1.5.12.md)**已封板发布，PR #205 受保护合入，正式四资产、中文说明及包内引擎核对完成。见[封板](ALZ_STREAM_INTEGRITY_FREEZE_1.5.12.md)、[交接](DEVELOPMENT_HANDOFF.md)与[路线](ARCHIVE_ROADMAP_1.5.6_TO_1.5.15.md)；[下载安装包](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.12)。方法 3 仍未实现且不支持；此为前版快照，准确接续以上方当前正式版和开发交接为准。

- **历史记录：[v1.5.11 EGG 中文密码组合](EGG_PASSWORD_COMBINATIONS_1.5.11.md)**已封板发布，PR #203 受保护合入，正式四资产、中文说明及包内五十五组原包回归已核对。见[封板](EGG_PASSWORD_COMBINATIONS_FREEZE_1.5.11.md)、[交接](DEVELOPMENT_HANDOFF.md)与[路线](ARCHIVE_ROADMAP_1.5.6_TO_1.5.15.md)；[下载安装包](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.11)。加密固实未验证不承诺，此前接续快照以最新交接为准。

- **历史记录：[v1.5.10 大 EGG LZMA](EGG_LARGE_LZMA_1.5.10.md)**已封板发布，PR #201 受保护合入，正式四资产、中文说明及包内引擎回归均核对完成。准确接续见[封板记录](EGG_LARGE_LZMA_FREEZE_1.5.10.md)、[开发交接](DEVELOPMENT_HANDOFF.md)与[十版路线](ARCHIVE_ROADMAP_1.5.6_TO_1.5.15.md)；[下载安装包](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.10)。此为前版快照，当前接续以上方和开发交接为准。

- **历史记录：[v1.4.16 旧密码编码](LEGACY_ZIP_PASSWORD_ENCODING_1.4.16.md)**已封板发布，[PR #171](https://github.com/Longyuyeee/long_Decompress/pull/171) 已合入，正式标签为 `5b4c317`。438 项相关回归和真实桌面链通过；安装器、签名更新 ZIP、签名及中文更新清单已回下载核对，见[封板记录](LEGACY_ZIP_PASSWORD_FREEZE_1.4.16.md)、[中文说明](RELEASE_NOTES_1.4.16.md)及[正式 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.4.16)。旧版“下一版”不作为当前待办。

- **前版记录：[v1.4.15 封板记录](ARCHIVE_PERFORMANCE_FREEZE_1.4.15.md)**、[实施审计](ARCHIVE_PERFORMANCE_1.4.15.md)、[中文更新说明](RELEASE_NOTES_1.4.15.md)、[十二版本路线](ARCHIVE_ROADMAP_1.4.15_TO_1.5.5.md)。PR #169 及所含 PR #168 已合入，两轮改进统一由 v1.4.15 交付，不单独发布 v1.4.14；历史安装器及更新附件见[v1.4.15 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.4.15)。

- **前版记录：[v1.4.13 缺卷 ZIP 部分抢救](PARTIAL_ZIP_RECOVERY_1.4.13.md)**、[封板与发布记录](PARTIAL_ZIP_RECOVERY_FREEZE_1.4.13.md)、[中文更新说明](RELEASE_NOTES_1.4.13.md)。标准 ZIP 缺卷、密码包与恢复结果接续已交付；安装器及签名更新附件见 [v1.4.13 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.4.13)。

- **前版记录：[v1.4.12 LZIP / TAR.LZIP](LZIP_AND_TAR_LZIP_1.4.12.md)**、[封板与发布记录](LZIP_AND_TAR_LZIP_FREEZE_1.4.12.md)、[中文更新说明](RELEASE_NOTES_1.4.12.md)。[PR #166](https://github.com/Longyuyeee/long_Decompress/pull/166) 已合入；读取/创建、批量配置和转换已交付，安装器及签名更新附件见 [v1.4.12](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.4.12)，保留供追溯。

- **前版记录：[v1.4.11 自解压包与好压分卷](SFX_AND_VOLUME_COMPATIBILITY_1.4.11.md)**、[封板记录](SFX_AND_VOLUME_FREEZE_1.4.11.md)、[中文发布说明](RELEASE_NOTES_1.4.11.md)。[PR #165](https://github.com/Longyuyeee/long_Decompress/pull/165) 已合入，[v1.4.11](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.4.11) 安装器及签名更新附件保留供追溯；下方旧版本记录不覆盖当前状态。

- [v1.4.3 归档升级收口](ARCHIVE_UPGRADE_FREEZE_1.4.3.md)、[更新说明](RELEASE_NOTES_1.4.3.md)：卷组一致性、RAR 分卷与 Unicode 密码、LZ4/Brotli 及通用数字分卷。

`docs/` 同时保存当前接续文档和历史发布证据。历史审计不是当前需求清单，但它们用于追溯真实测试、版本决策和公开资产，因此不得仅因日期较早而删除。

## 当前接续入口

- [开发交接](DEVELOPMENT_HANDOFF.md)、[产品增强路线](PRODUCT_ENHANCEMENT_ROADMAP.md)：当前接续状态及后续候选。
- [v1.4.4 实施审计](ARCHIVE_RELIABILITY_1.4.4.md)、[封板记录](ARCHIVE_RELIABILITY_FREEZE_1.4.4.md)：标准 ZIP 创建、密码兼容和无密码 ZIP 完整条目恢复，已发布。
- [归档升级方向提案](ARCHIVE_COMPATIBILITY_AND_WORKFLOW_UPGRADE_PROPOSAL_2026-10-13.md)、[暂停审计](DEVELOPMENT_PAUSED_AUDIT_2026-10-13.md)：v1.4.3 开发过程的历史记录，不覆盖当前状态。

- [2026-10-13 开发情况审计](DEVELOPMENT_STATUS_AUDIT_2026-10-13.md)：当时主线 `fa611a8` / v1.4.2 的历史快照。
- [v1.4.2 批量转换与待处理草稿恢复](BATCH_CONVERSION_AND_DRAFT_RECOVERY_PLAN.md)、[发布说明](RELEASE_NOTES_1.4.2.md)：已合入并正式发布，保留单条行内展开和多选弹窗配置。
- [设置页样式实施记录](UI_VISUAL_REFRESH_PLAN.md)、[原静态示意图](../design/settings-style/README.md)：设置样式已实现并随 v1.4.2 交付；整体 UI 重构继续取消。

## 历史接续记录

以下保留当时的发布与开发决策，不作为当前待办或版本状态。

- [2026-10-12 开发情况审计](DEVELOPMENT_STATUS_AUDIT_2026-10-12.md)：设置页实施和 v1.4.2 发布之前的快照，当前状态以 2026-10-13 审计为准。

- [1.3.7 更新说明](RELEASE_NOTES_1.3.7.md)、[发布记录](RELEASE_AUDIT_1.3.7.md)。本版交付归档解压体验增强，后续优先级见产品路线。

- [2026-10-10 用户体验增强路线](PRODUCT_ENHANCEMENT_ROADMAP.md)、[开发交接](DEVELOPMENT_HANDOFF.md)、[下一版更新说明与配套计划](NEXT_VERSION_AFTER_1.3.6.md)。批次处理、历史查找与再次解压、输出预览和成功结果入口已本地实现，当前接续版本与安装包准备。冲突增强、跨类型结果工作区及书签/标签页后续推进；性能作为每版持续工作。

- [1.3.6 发布审计](RELEASE_AUDIT_1.3.6.md)、[历史保留场景审计](HISTORY_RETENTION_2026-09-23.md)。这是已交付基线，开发顺序以顶部路线为准，旧版计划仅作追溯。

- [1.3.5 发布审计](RELEASE_AUDIT_1.3.5.md)、[失败指引开发审计](FAILURE_RECOVERY_2026-09-23.md)、[下一版计划](NEXT_VERSION_AFTER_1.3.5.md)。以下旧版计划用于追溯，不代表当前待办全部未完成。

- [1.3.4 发布审计](RELEASE_AUDIT_1.3.4.md)、[媒体跨页场景审计](MEDIA_ROUTE_SESSION_2026-09-22.md)、[下一版方向](NEXT_VERSION_AFTER_1.3.4.md)。

- [`NEXT_VERSION_AFTER_1.3.3.md`](NEXT_VERSION_AFTER_1.3.3.md)：下一轮失败处理、跨页控制、历史保留和恢复的开发顺序及验收条件。
- [`RELEASE_NOTES_1.3.3.md`](RELEASE_NOTES_1.3.3.md)：1.3.3 发布说明；[发布审计](RELEASE_AUDIT_1.3.3.md)记录安装、公开资产状态与验证边界。

- [`RELEASE_CLOSURE_2026-09-22.md`](RELEASE_CLOSURE_2026-09-22.md)：历史重试、真实图片并发隔离、跨页面状态修复及安装发布接续。

- [`NEXT_VERSION_USER_PLAN_2026-09-22.md`](NEXT_VERSION_USER_PLAN_2026-09-22.md)：当前审计、历史筛选找回、1.3.3 收尾范围与后续用户痛点优先级。
- [`ARCHIVE_CENTER_TASK_SCOPE_2026-09-12.md`](ARCHIVE_CENTER_TASK_SCOPE_2026-09-12.md)：归档页取消/清除不影响特殊压缩，用户场景复现与验证边界。
- [`COMPRESSION_HISTORY_CLEAR_CONSISTENCY_2026-09-12.md`](COMPRESSION_HISTORY_CLEAR_CONSISTENCY_2026-09-12.md)：压缩清除尊重历史保存状态、用户场景先红后绿，完整单测 372/372。
- [`USER_SCENARIO_AUDIT_2026-09-12.md`](USER_SCENARIO_AUDIT_2026-09-12.md)：用户口吻场景、失败原因优先展示，以及真实桌面未完成/全量回归未通过的明确接续点。
- [`ROLLBACK_RECOVERY_GUIDANCE_2026-09-12.md`](ROLLBACK_RECOVERY_GUIDANCE_2026-09-12.md)：回滚不完整的历史指引、诊断导出与人工核对边界。
- [`INCOMPLETE_ROLLBACK_RETENTION_2026-09-12.md`](INCOMPLETE_ROLLBACK_RETENTION_2026-09-12.md)：回滚不完整时保留恢复目录、禁用原暂存重试及验证边界。
- [`EXTRACTION_ROLLBACK_STAGING_2026-09-12.md`](EXTRACTION_ROLLBACK_STAGING_2026-09-12.md)：部分提交回滚恢复暂存、实际文件占用重试验证，以及回滚不完整边界。
- [`COMMITTED_EXTRACTION_CLEANUP_2026-09-12.md`](COMMITTED_EXTRACTION_CLEANUP_2026-09-12.md)：解压冲突恢复成功后清理告警纠偏与 Windows 实际占用测试。
- [`OUTPUT_PUBLICATION_FAILURE_2026-09-12.md`](OUTPUT_PUBLICATION_FAILURE_2026-09-12.md)：单文件压缩输出冲突、提交失败、取消与原因链验证。
- [`OUTPUT_VERIFICATION_FAILURE_2026-09-12.md`](OUTPUT_VERIFICATION_FAILURE_2026-09-12.md)：压缩产物校验失败的类别、阶段、取消兼容与验证。
- [`SOURCE_ACCESS_FAILURE_2026-09-12.md`](SOURCE_ACCESS_FAILURE_2026-09-12.md)：源缺失、元数据不可访问、非普通文件分类及阶段，验证与后续边界。
- [`SOURCE_CHANGE_FAILURE_2026-09-12.md`](SOURCE_CHANGE_FAILURE_2026-09-12.md)：预检和解压期间源变化的明确分类、阶段及验证。
- [`STRUCTURED_FAILURE_HISTORY_2026-09-12.md`](STRUCTURED_FAILURE_HISTORY_2026-09-12.md)：schema 8 失败类别/阶段持久化、旧数据兼容及下一步边界。
- [`HISTORY_EXTRACTION_DRAFT_2026-09-12.md`](HISTORY_EXTRACTION_DRAFT_2026-09-12.md)：当前从历史创建完整解压待执行任务的实现、参数边界与验证。
- [`HISTORY_DIAGNOSTICS_2026-09-12.md`](HISTORY_DIAGNOSTICS_2026-09-12.md)：历史错误检索、分类筛选和单任务诊断报告导出的当前实现与边界。
- [`FAILURE_RELIABILITY_FOLLOWUP_2026-09-12.md`](FAILURE_RELIABILITY_FOLLOWUP_2026-09-12.md)：当前错误分类、前端密码重试纠偏、历史保存串行与保存失败恢复的实现和测试。
- [`DEVELOPMENT_AUDIT_AND_UPGRADE_PLAN_2026-09-12.md`](DEVELOPMENT_AUDIT_AND_UPGRADE_PLAN_2026-09-12.md)：v1.3.2 当前代码审计、错误分类遗留、同类产品参考和六步升级规划；后续开发优先从此处接续。
- [`DEVELOPMENT_HANDOFF.md`](DEVELOPMENT_HANDOFF.md)：跨电脑开发接续总入口，最新记录在文件顶部。
- [`RELEASE_NOTES_1.3.2.md`](RELEASE_NOTES_1.3.2.md) / [`RELEASE_AUDIT_1.3.2.md`](RELEASE_AUDIT_1.3.2.md)：当前 v1.3.2 的损坏 RAR 识别、失败任务最终原因和正式发布证据。
- [`RELEASE_NOTES_1.3.1.md`](RELEASE_NOTES_1.3.1.md) / [`RELEASE_AUDIT_1.3.1.md`](RELEASE_AUDIT_1.3.1.md)：历史 v1.3.1 的 RAR 密码判定、源文件稳定性、本机失败记录和正式发布证据。
- [`RAR_PASSWORD_POST_VALIDATION_FIX_AUDIT_2026-09-05.md`](RAR_PASSWORD_POST_VALIDATION_FIX_AUDIT_2026-09-05.md)：RAR 密码终态误分类修复及本机历史失败记录审计。
- [`FAILED_TASK_FINAL_REASON_AUDIT_2026-09-05.md`](FAILED_TASK_FINAL_REASON_AUDIT_2026-09-05.md)：损坏 RAR 不再误入密码尝试，以及所有失败任务最终原因落库审计。
- [`RELEASE_NOTES_1.3.0.md`](RELEASE_NOTES_1.3.0.md)：历史 v1.3.0 的配置、密码工作流、文件浏览器与完整性工作区发布说明。
- [`RELEASE_NOTES_1.2.9.md`](RELEASE_NOTES_1.2.9.md) / [`RELEASE_AUDIT_1.2.9.md`](RELEASE_AUDIT_1.2.9.md)：历史稳定版 v1.2.9 的发布后总审计、真实归档控制复验和正式发布证据。
- [`RELEASE_NOTES_1.2.8.md`](RELEASE_NOTES_1.2.8.md) / [`RELEASE_AUDIT_1.2.8.md`](RELEASE_AUDIT_1.2.8.md)：历史稳定版 v1.2.8 的归档队列、配置、暂停控制和正式发布证据。
- [`ARCHIVE_QUEUE_CONFIGURATION_CONTROL_AUDIT_2026-09-04.md`](ARCHIVE_QUEUE_CONFIGURATION_CONTROL_AUDIT_2026-09-04.md)：归档顺序、并发、配置来源、停止及暂停/恢复的最终实现和真实证据。
- [`WORKSPACE_DENSITY_AND_DUAL_PANE_AUDIT_2026-09-03.md`](WORKSPACE_DENSITY_AND_DUAL_PANE_AUDIT_2026-09-03.md)：当前省略文本完整提示、三个核心工作区密度和窄窗口双栏固定布局审计。
- [`DECOMPRESSION_TASK_LAYOUT_AUDIT_2026-09-03.md`](DECOMPRESSION_TASK_LAYOUT_AUDIT_2026-09-03.md)：v1.2.6 后续任务名称、状态对齐、自然排序、存储预检和低高度版本徽标修复审计。
- [`DECOMPRESSION_RUNTIME_UX_AUDIT_2026-09-03.md`](DECOMPRESSION_RUNTIME_UX_AUDIT_2026-09-03.md)：当前解压运行态界面、速度、进度、跨盘事务和密码流程审计。
- [`../tests/README.md`](../tests/README.md)：当前真实测试目录、命令与证据规则。

## 历史证据

- `RELEASE_NOTES_<版本>.md`：公开版本用户可见变更。
- `RELEASE_AUDIT_<版本>.md`：打包、安装、升级、资产与回下载证据。
- `*_AUDIT*.md`、`*_BASELINE*.md`：已完成阶段的实现与真实验证记录。
- `CURRENT_DEVELOPMENT_*`、`*_HANDOFF*`：过去机器或阶段的交接快照；只用于追溯，当前工作以本页“当前接续入口”为准。

## 清理准则

可以删除：不在任何配置中收集的测试、引用不存在脚本/夹具/界面的示例、被当前权威文档完全替代且没有审计引用的草稿。

必须保留：公开 Release 说明、发布审计、真实测试证据、仍被接续链引用的阶段审计。修改当前行为后，应更新当期审计和 `DEVELOPMENT_HANDOFF.md`，而不是回写历史版本结论。
