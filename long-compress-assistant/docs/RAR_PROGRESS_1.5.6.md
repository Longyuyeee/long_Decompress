# v1.5.6 RAR 实际进度与暂停计时

## 范围与开工审计

从 v1.5.5 发布后主线 `5dc0211` 接续，遵循[十版路线](ARCHIVE_ROADMAP_1.5.6_TO_1.5.15.md)。当前 RAR 每 500 ms 扫描暂存目录的逻辑长度；UnRAR 预分配文件后，进度可能提前接近完成。`ProgressMetric` 用完整墙钟时间计算速度，历史耗时也是开始到结束的差值，均未扣除暂停。

改用原生 `UCM_PROCESSDATA` 的解码块长度，不增加第二次全包读取或输出目录轮询。块回调发生在写入和 CRC 之前，累计值表示实际解码工作，不代表已经落盘、校验成功或发布；完成仍以原有事务提交为准。保留密码、分卷、所选提取、暂停取消及立即清理，不改变已有格式支持边界。

速度与剩余时间扣除任务暂停区间，解压开始时重新建立基线，不把试密计入解压吞吐。历史保留原开始/结束时间，耗时扣除本次会话实际暂停；既有历史不追改。暂停时不显示排队到达事件中的速度，后续空速度/剩余时间清除旧值。

## 封板条件

原 RAR4/5 普通、内容密码、文件名密码，单包、原生卷及 SFX 分别验证字节累计与固定载荷。大包必须先实际解码再暂停、恢复、取消，核对预分配不冒充进度、暂停不计入吞吐、取消后无发布/暂存残留及源卷保持。实际队列入口和历史耗时走真实桌面，查看常用及窄窗页面。定向单元回归、普通生产构建、受保护 CI/合入、两份 README、中文 Release/更新说明和正式四资产回下载完成后才宣布发布。

功能与测试已封板，发布状态见[封板记录](RAR_PROGRESS_FREEZE_1.5.6.md)。个人安装、设置和历史不修改；智能目录/ZIP 并行仍默认关闭，不恢复快速打包，Windows N 暂不保证支持。

## 真实场景发现与纠偏

首轮同目录桌面任务解压 256 MiB 文件后，历史处理量变成 `537332802` 字节。原因不是解码回调重复，而是完成阶段重新扫描整个目标目录，把已存在的九个原压缩卷也计入“实际产出”。改为复用事务提交时成功移动的文件长度累计；跳过的不计，保留两份及覆盖只计本次新文件，备份和其他已有内容不计。确认冲突后继续提交沿用相同返回值。取消/失败仍回滚，不输出成功计数；不为修正统计增加第二次全量读取。

桌面脚本早期误以为所有目录按钮都有对话框测试队列，实际单条配置使用原生目录选择。最终场景改为在独立接收目录中放入原卷，走真实“同目录”操作；不添加生产验收捷径，不把目录未生效的早期结果算通过。

真实停止按钮还复现取消竞态：底层已接受取消并清理，命令层保留 cancelling，但解压中心外层 catch 又将同一个取消拒绝写成 failed，最终记录变成失败。新增两条先失败的回归，外层不再改写 cancelling/cancelled，也不弹失败或密码提示。等待底层确认后仍由原任务控制器收敛为 cancelled。

解码回调首次正字节立即可见，后续界面事件限制为约 100 ms 一次；收到暂停时立即刷新上一块已解码累计，再进入原生等待，避免短任务暂停后仍显示旧采样。没有增加解码延迟或测试专用限速。

首轮桌面数据对照显示，实际解码约 205 MiB 时百分比只有约 8%。`rar_preflight.rs` 使用 processing 模式配合 RAR_SKIP，逐卷续块头重复返回整个逻辑文件的 unpacked_size，分母随卷数膨胀。原 RAR4 旧卷回归先复现 `(9,1324120)` 而正确值为 `(5,275544)`。按原生 split-before 标志只计逻辑条目首头，仍检查并跳过全部物理头；不按文件名粗暴去重，并与实际 RAR 筛选/路径规则一致。最终桌面必须同时核对实际字节、逻辑总量和显示比例，早期未核对分母的报告不作为封板证据。

## 最终证据与换机入口

功能冻结 `aed3eca`；49 项不同 Rust、133 项前端、类型检查通过。最终桌面 12 场景和 8 张实看页面、事件中的实际字节/分母/比例、暂停扣时、真实损坏拒绝及来源身份均在[机器证据](evidence/rar-progress-1.5.6/results.json)。运行 E2E 前先设置 `VITE_DESKTOP_E2E=1`，执行 `node node_modules/@tauri-apps/cli/tauri.js build --features desktop-e2e --bundles none`；构建完成再设 `TAURI_APP_BINARY` 为 `src-tauri/target/release/Long解压.exe` 的绝对路径，运行 `node scripts/test-tauri-desktop.mjs --rar-156-only`。不要与普通生产构建并行，共用 dist 和二进制路径。

新增原包生成脚本 `scripts/prepare-rar-156-fixtures.mjs`，默认验证并保留 `test-results/rar-156-originals` 中已存在原字节；原 RAR7.23 路径 `E:/Winrar/Rar.exe`，独立读取使用随附 7-Zip 26.04。公共测试密码与成员 SHA256 在[原包清单](evidence/rar-progress-1.5.6/originals.json)，不把编码器随软件分发。16 MiB 载荷为前版 256 MiB 控制载荷的前缀；大控制包继续复用 `test-results/sfx-155-control/manifest.json`，来源和恢复脚本见[v1.5.5 审计](SFX_VOLUMES_1.5.5.md)。大包未入 Git；换机重建时须核对本版清单，不能用不同原包宣称相同证据。

定向 Rust 过滤入口：`rar_support::tests`、`rar_preflight::tests`、`task_control::tests`、`progress_telemetry`、`sfx_archive::tests`、`extraction_transaction::tests`、`conflict_confirmation_tests` 和 `rar_decoded_progress`。前端六文件为 task store、taskHistory、AeroTable、DecompressView、HistoryView、useTauriCommands 的对应测试。`node scripts/audit-rar-156.mjs` 核对最终回归、桌面数据、文档链接及两份 README 原始图片保留。
