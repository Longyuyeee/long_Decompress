# 归档性能基线

> 基线日期：2026-07-26
> 发布版本：v1.0.13
> 平台：Windows x64，Rust `release` 配置

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

## 运行方法

```powershell
cd src-tauri
cargo test --release --test archive_performance_regression -- --ignored --nocapture

$env:LONG_DECOMPRESS_PERF_SIZE_MIB = "1024"
cargo test --release --test archive_performance_regression -- --ignored --nocapture
Remove-Item Env:LONG_DECOMPRESS_PERF_SIZE_MIB

$env:LONG_DECOMPRESS_PERF_FILE_COUNT = "10000"
cargo test --release --test archive_performance_regression real_zip_many_small_files_baseline -- --ignored --nocapture
Remove-Item Env:LONG_DECOMPRESS_PERF_FILE_COUNT
```

测试允许通过 `LONG_DECOMPRESS_PERF_SIZE_MIB` 设置 16–2048 MiB 的输入，并限制压缩、解压全程的额外工作集小于 256 MiB。

## 后续优化顺序

1. 为加密 AES 容器增加兼容的新流式容器版本；现有格式必须先完成迁移设计，不能直接破坏旧文件读取。
2. 在保持密码、冲突策略、时间戳、事务回滚和路径安全语义一致的前提下，再评估受控并行解压；当前未启用尚未覆盖这些语义的实验性并行提取器。
3. 在固定硬件的 CI 性能任务中积累多次样本后，再设置吞吐回归阈值；单次开发机结果不适合直接作为硬门槛。
