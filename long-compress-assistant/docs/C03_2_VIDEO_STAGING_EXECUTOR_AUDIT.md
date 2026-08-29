# C-03.2 视频暂存执行器审计

日期：2026-08-28

范围：真实异步编码、容量预检、心跳、取消与暂存所有权

结论：**C-03.2 已完成；能力仍为后端内部边界，不注册 Tauri 命令、不解锁 UI、不发布最终文件。**

## 1. 实现与最初需求对齐

| 要求 | 实际实现 | 状态 |
| --- | --- | --- |
| 真实 FFmpeg 异步执行 | 使用冻结产品 `ffmpeg.exe`，stdout/stderr 分管；stdout 只进入机器进度解析器，stderr 持续排空且最多保留 64 KiB | 通过 |
| 容量预检与暂存隔离 | 编码前复用 `preflight_operation_resources`；输出使用共享 `staged_output_path` 在最终目录生成唯一隐藏文件 | 通过 |
| 取消与应用退出终止进程树 | 每 50 ms 检查传入的统一取消令牌；取消时终止 Job Object 并等待退出；异步 Future/App 退出时 Job、`kill_on_drop` 与暂存清理守卫共同收口 | 通过 |
| 无进度时仍有真实生命信号 | 5 秒无完整 progress block 时发 `Heartbeat`，只报告距最后真实进度的时间，不制造百分比 | 通过 |
| 失败/取消不留半成品 | 启动、管道、解析、退出码、缺失 `progress=end`、空输出、取消及 Future drop 全部由暂存守卫清理事务家族 | 通过 |
| C-04 前不得发布 | 成功只返回拥有暂存生命周期的 `StagedVideoOutput`；对象 drop 即清理；最终路径始终不存在 | 通过 |

当前首期是单遍编码，不生成 FFmpeg passlog；共享事务家族清理仍覆盖同一暂存前缀的潜在 sidecar。最终目标竞态、ffprobe 复验、Mark-of-the-Web、原子发布和最终事实归 C-04，未提前实现或伪装通过。

## 2. 真实验证

- `cargo test video_encoding --lib`：7/7 通过。
- 特殊字符 VFR/旋转/AAC/字幕真实输入：编码成功，机器进度结束，最终路径未发布，暂存对象 drop 后文件消失。
- 现场用产品 FFmpeg 流复制生成约 500 秒输入；10 ms 测试心跳触发取消，Job Object 在 5 秒门限内结束，最终文件与 `.video-encode-*` 暂存家族均不存在。
- 非法 progress 值、目标已存在、无效输出等错误保持稳定分类；stderr 保留有界但持续排空，避免子进程管道阻塞。
- `cargo clippy --lib -- -D warnings`：通过。
- `cargo test --lib`：343 通过、0 失败、4 个显式忽略的外部/破坏性门禁。
- `npm run test:media-architecture`、`npm run type-check`：通过。

## 3. 边界纠偏与下一接续点

上一审计把“暂存执行器、统一命令/事件接入”合写为 C-03.2。实际代码核对后拆开更安全：如果当前注册可调用命令，调用方会拿到一个 C-04 尚不能验证和发布的暂存结果。因此 C-03.2 只关闭内部执行与清理；C-03.3 才把它接到 `commands/compression.rs` 的唯一取消注册表、输出占用锁和现有 `task-progress/task-log`，并接入统一视频任务 UI。C-03.3 仍只能以“待验证/最终化”结束，不能标记 completed；C-04 随后补齐验证与发布。
