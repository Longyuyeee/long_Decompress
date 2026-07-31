# 开发交接

> 2026-07-30 详细代码审计见仓库根目录
> [DEVELOPMENT_AUDIT_2026-07-30.md](../../DEVELOPMENT_AUDIT_2026-07-30.md)。

> 2026-07-31 发布后审计见仓库根目录
> [DEVELOPMENT_AUDIT_2026-07-31.md](../../DEVELOPMENT_AUDIT_2026-07-31.md)。

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
- HFSX 目前只能可靠生成空镜像，尚未找到可写入已知载荷的可信方案，因此不能标记为完整通过。
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
