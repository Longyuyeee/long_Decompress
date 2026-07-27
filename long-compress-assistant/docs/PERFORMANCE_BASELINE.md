# 归档性能基线

> 基线日期：2026-07-27
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

1. 在固定 Windows 环境周期运行 ZIP 与 AES 基准，积累至少 10 次样本后再设置吞吐告警阈值。
2. 为 AES 输出增加可控的磁盘写满故障注入，验证所有文件系统错误都保持未完成输出清理语义。
3. 在保持密码、冲突策略、时间戳、事务回滚和路径安全语义一致的前提下，再评估受控并行解压；当前未启用尚未覆盖这些语义的实验性并行提取器。
