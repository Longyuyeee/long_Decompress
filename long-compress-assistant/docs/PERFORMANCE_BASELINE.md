# 归档性能基线

> 基线日期：2026-07-27
> 发布版本：v1.0.13
> 平台：Windows x64，Rust `release` 配置

> 2026-08-01 起，本页的历史数字继续保留作参考；新增的结构化采样工具负责后续版本趋势，
> 不再依赖人工复制控制台输出。

## 本轮优化

- ZIP 与单文件流复制统一使用可复用的 256 KiB 缓冲，并在每个块之间响应取消。
- 原生 ZIP 解压按每 4 MiB 最多发送一次字节级进度，文件完成事件仍会发送，避免大文件产生大量 Tauri IPC。
- IO 缓冲池保留已扩容缓冲的容量；连续处理相近大小的归档条目时不再反复分配。
- 通用 7-Zip 引擎的全目录资源扫描由每秒一次调整为每 5 秒一次；磁盘安全余量仍每秒检查。
- 解压后的布局整理与最终资源计数合并，减少一次完整暂存目录遍历，同时保留文件数量、展开体积、压缩比、链接和磁盘余量防护。

## 真实文件结果

测试使用确定性伪随机数据，压缩后重新解压，并以文件长度和 CRC32 校验内容。内存采样覆盖压缩与解压两个阶段。

| 输入大小 | ZIP 压缩 | ZIP 解压 | 峰值工作集增量 |
| --- | ---: | ---: | ---: |
| 100 MiB | 48.30 MiB/s | 1301.19 MiB/s | 5.14 MiB |
| 1 GiB | 48.78 MiB/s | 1364.07 MiB/s | 2.40 MiB |

10,000 个 4 KiB 文件的真实文件系统基线为：压缩约 3,235 文件/秒，解压约 4,660 文件/秒，峰值工作集增加 2.96 MiB。

这些数据用于当前机器上的回归参照，不应直接作为其他磁盘、CPU 或杀毒软件环境的绝对性能承诺。

## AES v2 流式基线

`AESENC02/TARAES02` 使用 1 MiB AES-256-GCM 分块和固定 64 MiB Argon2id KDF。测试覆盖
确定性伪随机输入、加密、解密、长度与 CRC32 校验，并采样整个加解密阶段的峰值工作集。

| 输入大小 | AES v2 加密 | AES v2 解密 | 峰值工作集增量 |
| --- | ---: | ---: | ---: |
| 100 MiB | 215.37 MiB/s | 196.61 MiB/s | 64.53 MiB |
| 1 GiB | 182.36 MiB/s | 162.69 MiB/s | 64.19 MiB |

输入扩大约十倍后峰值增量仍保持约 64 MiB，验证内存主要由固定 KDF 和分块缓冲决定，
不会随文件大小线性增长。测试门槛暂设为额外工作集小于 192 MiB，用于发现退化而不是
跨机器性能承诺。

## 运行方法

推荐从项目目录运行结构化基线：

```powershell
# 日常烟雾检查：只验证三条真实路径和结果格式，不作为性能门禁
npm.cmd run performance:baseline -- -Iterations 1 -LargeFileMiB 16 -SmallFileCount 1000

# 固定机器的正式基线：每个场景至少 10 个样本
npm.cmd run performance:baseline -- -Iterations 10 -LargeFileMiB 100 -SmallFileCount 10000 `
  -OutputPath test-results\performance-baseline\v1.0.20\result.json

# 同机比较；只有基线自身达到 10 次样本资格时才应用回归阈值
npm.cmd run performance:baseline -- -Iterations 10 -LargeFileMiB 100 -SmallFileCount 10000 `
  -BaselinePath test-results\performance-baseline\v1.0.20\result.json `
  -RegressionThresholdPercent 25

# I/O 拓扑烟雾：显式指定源端和目标端，只运行真实 ZIP 单大文件/大量小文件
npm.cmd run performance:io-baseline -- -Iterations 1 -LargeFileMiB 16 -SmallFileCount 1000 `
  -SourceRoot C:\baseline-source -TargetRoot D:\baseline-target

# I/O 拓扑正式样本：相同源卷、目标卷和规模至少重复 10 次
npm.cmd run performance:io-baseline -- -Iterations 10 -LargeFileMiB 100 -SmallFileCount 10000 `
  -SourceRoot C:\baseline-source -TargetRoot D:\baseline-target `
  -OutputPath test-results\performance-baseline\io\c-to-d.json

# 正式 SSD/HDD 矩阵：共享同一个目标卷，比较“目标卷→目标卷”和“另一物理盘→目标卷”
npm.cmd run performance:io-matrix -- -Iterations 10 -LargeFileMiB 100 -SmallFileCount 10000 `
  -TargetRoot C:\baseline-target -CrossSourceRoot D:\baseline-source
```

结果 JSON 包含 Git 提交及工作区状态、Windows/CPU/内存/架构组成的机器指纹、活动电源计划、
Rust 工具链、逐次样本、中位数/最小值/最大值和基线比较结果。大文件 ZIP、小文件 ZIP、原生 7Z 与 AES v2
均由真实写入、读取和内容校验路径产生 `PERF_JSON`，脚本不会解析易变化的展示文本。

同一基线周期必须保持机器、存储设备、电源计划、实时防护策略和输入规模不变。脚本会拒绝跨机器指纹比较；
少于 10 次、Windows 无法证明介质，或 Git 工作区存在未提交改动时，结果都会明确标记
`threshold_eligible=false`，只能用于烟雾观察，不能阻断发布。正式矩阵会在采样前检查工作区，避免完成昂贵测试后
才发现代码状态不可追溯；仓库内的输出目录还必须被 Git 忽略，防止第一组结果污染第二组资格。

I/O 拓扑模式会通过 Windows 卷、分区、磁盘和物理介质信息证明源端/目标端关系，记录文件系统、SSD/HDD、
总容量、可用容量、卷与磁盘指纹，并区分同卷、同物理盘跨卷和跨物理盘。源目录和目标目录必须已经存在；
运行前按最大夹具体积和 128 MiB 安全预留检查空间，无法取得稳定本地磁盘身份时拒绝运行。比较文件还必须与
当前源卷和目标卷指纹完全一致，不能把 C→C 与 D→C、SSD 与 HDD 的结果混在一起。

该模式复用正式 ZIP 压缩服务和真实 ZIP 解码：压缩从源端读取并写到目标端；压缩完成后在计时外把归档放回
源端，解压再从源端读取并写到目标端，因此两个指标都表达同一个 I/O 方向。大文件按长度与 CRC32 校验，
小文件按数量与组合 CRC32 校验。它衡量应用实际路径（会受到 Windows 文件缓存、杀毒软件和电源计划影响），
不是裸盘顺序读写测试；结果只适合同机同配置趋势，不能作为磁盘厂商级带宽结论。

`performance:io-matrix` 在上述单场景能力之上按固定顺序运行两组结果：第一组把共享目标目录同时作为源端，
第二组从另一块物理盘读取但继续写入同一个目标目录。汇总器会验证机器、Git 提交、应用版本、规模和目标卷完全一致，
并要求关系分别为 `same_volume` 与 `cross_physical_disk`；四项吞吐量均输出中位数、跨盘相对变化以及
最小值到最大值相对中位数的波动范围。`matrix.json` 只说明存储拓扑基线，固定写入
`change_default_concurrency=false`，不能被解释为某种调度策略已经获益。

长时间矩阵也可以先分别用 `performance:io-baseline` 采集两份结果，再向 `performance:io-matrix` 同时传入
`-ExistingSameVolumeResult` 与 `-ExistingCrossPhysicalDiskResult` 做只读汇总；两份结果仍必须来自同一干净提交、
同一机器、相同规模和同一个目标卷。正式采样根目录如果位于 Git 仓库内，随机夹具路径必须已被忽略，否则会在
开始前拒绝，避免临时目录把后续样本变成脏工作区。

2026-08-10 的工具烟雾在当前开发机完成：C→C 被系统证明为同卷 NVMe SSD，E→C 被证明为跨两块物理
NVMe SSD；两条路线均以 16 MiB 单文件和 1000 个 4 KiB 小文件完成真实往返。每场景只有 1 个样本，
`threshold_eligible=false`，仅证明工具、拓扑识别、空间护栏和内容闭环可用。本机没有可证明的 HDD，
因此 HDD 仍明确标记为未覆盖，且本轮数据不支持任何默认并发或调度调整。

2026-08-11 在同一开发机、应用 `1.1.1`、干净提交 `8dc8b94` 上完成首份合格 SSD 矩阵。C→C 同卷与
E→C 跨物理盘均使用同一个 C 盘目标，分别执行 100 MiB 单文件和 10,000 个 4 KiB 文件各 10 次；两份
结果均为 `threshold_eligible=true`，恢复分析生成的矩阵也再次通过机器、提交、版本、规模、目标卷和拓扑校验。

| 真实 ZIP 吞吐指标 | C→C 同卷中位数 | E→C 跨物理盘中位数 | 跨盘变化 | 同卷样本范围 | 跨盘样本范围 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 100 MiB 压缩 | 37.60 MiB/s | 32.52 MiB/s | -13.51% | 54.93% | 69.31% |
| 100 MiB 解压 | 682.70 MiB/s | 514.57 MiB/s | -24.63% | 63.97% | 92.94% |
| 小文件压缩 | 764.03 文件/s | 694.86 文件/s | -9.05% | 23.26% | 54.94% |
| 小文件解压 | 769.22 文件/s | 598.15 文件/s | -22.24% | 34.77% | 47.52% |

这里的“样本范围”是最小值到最大值相对中位数的跨度。跨盘中位数在四项指标上均较低，但大文件解压等路线
波动明显，且矩阵没有比较不同任务调度策略，因此结论固定为 `baseline_only`、
`change_default_concurrency=false`。原始 JSON 是本机忽略的测试产物，不提交仓库；本机仍没有可证明 HDD，
不得把这组 NVMe SSD 数据外推为 HDD 结论。

底层测试仍可单独运行：

```powershell
cd src-tauri
cargo test --release --test archive_performance_regression -- --ignored --nocapture

$env:LONG_DECOMPRESS_PERF_SIZE_MIB = "1024"
cargo test --release --test archive_performance_regression -- --ignored --nocapture
Remove-Item Env:LONG_DECOMPRESS_PERF_SIZE_MIB

$env:LONG_DECOMPRESS_PERF_FILE_COUNT = "10000"
cargo test --release --test archive_performance_regression real_zip_many_small_files_baseline -- --ignored --nocapture
Remove-Item Env:LONG_DECOMPRESS_PERF_FILE_COUNT

cargo test --release --test aes_stream_performance real_aes_stream_100_mib_baseline -- --ignored --nocapture
cargo test --release --test aes_stream_performance real_aes_stream_1_gib_baseline -- --ignored --nocapture
```

测试允许通过 `LONG_DECOMPRESS_PERF_SIZE_MIB` 设置 16–2048 MiB 的输入，并限制压缩、解压全程的额外工作集小于 256 MiB。

## 后续优化顺序

1. 在固定 Windows 环境周期运行 `performance:baseline`；首份合格结果至少包含 ZIP 大文件、ZIP 小文件、7Z 大文件和 AES v2 每场景 10 个样本，之后才启用同机趋势告警。
2. 当前双 NVMe SSD 的 10 样本同卷/跨物理盘矩阵已经完成；后续在相同机器和输入规模重复采样以观察高波动指标，并另找系统可明确识别的 HDD 重复同样矩阵，缺失场景不得用盘符或设备名称推测。
3. 只有后续矩阵和独立调度策略实验显示稳定、可复现的任务级收益，才设计与现有用户并发设置兼容的策略；在保持密码、冲突、时间戳、事务回滚和取消语义前，不接入实验性并行提取器，也不改变默认并发。
