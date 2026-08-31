# Long解压 v1.1.17 发布审计

审计日期：2026-09-01（Asia/Shanghai）

状态：**正式发布、公开资产回下载与应用内更新验证全部完成。**

## 结论

PDF D-01 至 D-04 已关闭。PR #107 经完整 CI 合入受保护主线，annotated `v1.1.17` 标签固定在合并提交 `49156c0e4de6f5beb3f9daf5674f2c389552b84a`。Release workflow 已生成四项签名公开资产；公开回下载、哈希/签名/包内身份对账及真实 `v1.1.16 → v1.1.17` 应用内更新 24/24 全部通过。`v1.1.17` 现为公开稳定版。

## 需求对齐

| 范围 | 实际结果 | 判定 |
| --- | --- | --- |
| 输入与分析 | 页数、大小、加密、签名、表单、附件真实读取 | 已完成 |
| 风险配置 | 无损整理/兼容图片优化；有损、可能变大和显式保留可见 | 已完成 |
| 安全执行 | 固定 qpdf、stdin 密码、重分析、取消与源变化拒绝 | 已完成 |
| 输出验证与发布 | 暂存、页数/结构复核、容量/竞态、原子发布 | 已完成 |
| 任务与历史 | 统一 `compression/pdf`、真实指标、失败/取消和完整重启 | 已完成 |
| 真实矩阵 | 11 类 × 两模式、签名/加密阻断、人工渲染、大页数/大图片 | 已完成 |
| 正式安装功能链 | 精确 `1.1.17` 候选生命周期 54/54、生产 PDF 工作区 23/23 | 已完成 |
| `1.1.17` 精确候选 | 干净 CI NSIS、包内身份、覆盖/卸载及公开基线恢复 | 已完成 |
| `1.1.17` 公开发布与更新 | 受保护主线、签名 Release 资产、回下载和应用内更新 | 已完成 |

## 路线偏移审计

1. 实现没有把 PDF 做成旁路任务系统，继续复用统一队列、取消、历史、容量预检和安全发布事务，与最初总路线一致。
2. 加密 PDF 即使密码正确，也因当前 qpdf 输出无法证明保持原加密策略而保守阻断；这不是功能遗漏，而是对“不得静默降级安全属性”的原始边界落实。
3. 本机 32 位 NSIS 工具以 `STATUS_DLL_NOT_FOUND` 失败，未被写成候选通过；功能冻结候选和后续精确版本候选均要求由干净 GitHub Windows Runner 构建。
4. 安装态门禁最初误用了测试桥入口，已纠正为生产 `__TAURI_IPC__`，并断言正式包不存在 E2E bridge；Node 运行时固定为与 CI 一致的 24 系列以消除 WebDriver/FastFail 偶发差异。

## 版本身份

- `package.json`、`package-lock.json` 根与 workspace、`tauri.conf.json`、主 Cargo 清单/锁、Shell Extension Cargo 清单/锁统一为 `1.1.17`。
- 唯一版本化 Shell Extension 为 `long_compress_shell_extension_1_1_17.dll`；本地 Release 构建 246,784 B，SHA-256 `59646AF395A422192E78B2F1EE1EFB637A7048C6394F9FDD98444EE89C32DD4A`。正式候选仍以干净 CI 包内字节为准，不混用不同构建环境哈希。
- `test:release-identity -- --expected 1.1.17` 已通过，并联动验证媒体依赖、指标、发布合同和图片基线。

## 功能冻结证据

- D-04.3 代码提交 `81c03b9` 的 CI run `33410249727` 五项全绿；同提交 NSIS 19,313,019 B、SHA-256 `2A703383EA1DD60BF69C59692A480452F5AE1FFEE1779C4AAFF8573EA9466304`。
- 生产安装生命周期 53/53、PDF 工作区 23/23：取消无输出、2 完成/1 失败隔离、输出重开、默认阅读器、四条历史完整重启及签名/加密阻断全部成立。
- D-04 收口 PR #106 的最终头提交 CI run `33412364228` 五项全绿，并 squash 合入 `master` 为 `c8b2395`。

## 本地候选门禁

- 使用 Node `24.19.0` 完成 TypeScript 类型检查、前端单元测试 47 文件/270 项与 Vite 生产构建，差异为零。
- Rust `cargo test --release --all-targets` 全部目标无失败；主库 377 项通过、10 项按声明忽略，主程序 1 项及各集成目标均通过。主后端全目标/全特性 Clippy `-D warnings` 通过。
- Shell Extension Release 5/5 与严格 Clippy 通过；重新编译产生唯一 `1.1.17` 版本 DLL，没有仅重命名复用旧二进制。
- 媒体架构 17 个生产文件、PDF 合同 13 类夹具、发布身份八处版本源均通过；真实 PDF 产品矩阵 24 项预期—实际差异为零。
- npm 生产依赖审计为 0 个已知漏洞。首轮并行编排曾让两个夹具生成入口竞争同一忽略目录并触发 `EBUSY`；已按实际共享目录约束改为独占后使用 Node 24 串行重跑，前端全门禁通过，未把测试竞争写成产品失败或降低断言。

## 干净 CI 与精确候选

- PR #107 头提交 `c51ba38` 的 GitHub Actions CI run `33414816383` 五项全绿：Browser 1 分 4 秒、Frontend 1 分 24 秒、Rust/Shell 8 分 29 秒、Windows desktop 6 分 23 秒、Windows installer 6 分 3 秒。
- 回下载 `windows-nsis-installer` 得到 `Long解压_1.1.17_x64-setup.exe`：19,317,221 B，SHA-256 `B0E9AA641F755325839A3AD82EEE8CCD62614FE151912D60FA888888E826BA80`。
- 冻结 7-Zip 26.02 对 NSIS 实测 `Everything is Ok`，32 个文件、展开大小 69,635,504 B。npm 自带的旧 7-Zip 21.07 不能识别当前 NSIS 3，已按既有发布证据口径改用项目正式 26.02 运行时，没有把工具能力不足写成包损坏。
- 包内主程序 29,385,216 B，ProductVersion/FileVersion 均为 `1.1.17`，SHA-256 `D08E06498FE41A406ECE3A8C9895C78AFB1F1FCFFA6F7A0DDC0313000484EE14`；唯一 Shell DLL 为 `long_compress_shell_extension_1_1_17.dll`，253,952 B，SHA-256 `95E47A0F21A3695C91B6D1210777A2E96B9BF83348405D2EBB2EF06081BFB450`。
- 包内 PDF 运行时含 5 个执行必需文件及来源/四份许可证，共 10 个文件、12,765,477 B；正式安装后生产预检通过，隔离副本的缺失/替换资源均按预期拒绝。

## 精确候选安装生命周期

- 基线为公开 `v1.1.16` NSIS：15,598,022 B、SHA-256 `BC83EA3554EC2631B453CC6EED4D2496C500F932EE2DFDEAA3F7109EB2A29F64`，安装位置 `E:\long\Long解压`，测试前无运行进程。
- `v1.1.16 → CI v1.1.17 候选 → 卸载候选 → 公开 v1.1.16` 生命周期 54/54 通过；候选安装主程序与 NSIS 独立提取字节一致，用户数据、经典菜单、自动启动和原安装路径保持/恢复。
- 无测试桥生产 PDF 工作区 23/23：取消无输出且源哈希不变，三项批量为 2 完成/1 失败且失败不阻断后续，输出通过生产 IPC 重开并读取页数，四条终态历史跨完整重启，签名/错误密码/加密执行继续保守阻断。
- 首两次运行均在修改安装前安全拒绝，分别指出缺少匹配 EdgeDriver 和完整 PDF 大图夹具；安装 EdgeDriver 151.0.4129.107、重新生成 13 类 PDF 夹具后从头执行通过，未产生候选覆盖或恢复副作用。
- 结构化证据：忽略目录 `test-results/installed-release-validation/20260901-005223/result.json` 及 `pdf-workspace/result.json`。结束后机器已恢复公开 `v1.1.16`、原安装位置与用户环境，运行进程为 0。

## 受保护分支合并与标签

- PR #107 最终头提交 `563b97115d7582cd26284699d59fec5095c840ee` 的 CI run `33416953367` 五项全绿：Browser 1 分 3 秒、Frontend 1 分 32 秒、Rust/Shell 7 分 49 秒、Windows desktop 4 分 57 秒、Windows installer 5 分 53 秒。
- PR 以 squash 合并为 `49156c0e4de6f5beb3f9daf5674f2c389552b84a`；PR 头与合并提交的 Git tree 均为 `fe3111d07dc3b424fd84cfd2f6b4646ed6863ca6`，文件树逐字节一致。
- annotated tag `v1.1.17` 精确指向该合并提交；tag object 为 `bb3c3accf13692cae7abe2f5bb70598c07fdf18f`。

## 正式资产与公开更新

- Release workflow run `33418305780` 精确检出合并提交并于 21 分 15 秒后成功；正式 [Long解压 v1.1.17](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.17) 非草稿、非预发布，目标提交正确。
- `latest.json`：2,666 B，SHA-256 `0A0B29126B43B6F95B4AE0C79749F5BB7C34864195D42695E5D8314746115899`；版本 `1.1.17`，Windows x86_64 URL 精确指向本标签 updater ZIP，签名与独立 `.sig` 逐字一致。
- NSIS：19,311,101 B，SHA-256 `BD11E4411896F2F34F2279B8B6A9F3374B0240B72C1FD803ED820623AF3E61B1`。7-Zip 26.02 完整性为 32 文件、`Everything is Ok`。
- updater ZIP：19,311,261 B，SHA-256 `49DCB5D2115D596FB256CE11B194327968AFB45195C00E224C001B9ABD884C20`；解包唯一 EXE 与独立 NSIS 大小和 SHA-256 完全一致。
- `.sig`：428 B，SHA-256 `119E689B72046F60D46EAE395151B153AB8CF3E476E592C0C19EFA309137841A`；正式包内主程序 29,385,216 B、ProductVersion `1.1.17`、SHA-256 `453AC71B3D1BB19026C05D8B133271F7F51D22FC186690BE738EBDE94DB71D01`。唯一 Shell DLL 253,952 B、SHA-256 `E98F86EBF4F4BA12F79BD4DAEE181746B2E164850F299A5AAE5865C70716D0F0`。
- 真实公开 `v1.1.16 → v1.1.17` 更新 24/24：签名更新 UI 交接、旧进程退出、目标安装、原路径、自动重启、经典菜单、自动启动、两处用户数据和唯一版本 DLL 均通过。证据：`test-results/public-update-validation/20260901-013557/result.json`。
- 最终机器安装公开 `v1.1.17` 于 `E:\long\Long解压`，Run 值为 `"E:\long\Long解压\Long解压.exe" --autostart`，运行中应用进程 0。

## 发布闭环

上述六项候选与公开发布待办均已完成。`v1.1.17` 发布阶段至此关闭；后续开发从总路线中的下一独立节点重新立项，不把已完成的 PDF 发布工作重复列为当前阻塞项。
