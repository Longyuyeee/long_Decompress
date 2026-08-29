# C-03.1 视频执行基础审计

日期：2026-08-28

范围：FFmpeg 参数数组、机器进度契约、Windows 进程树生命周期

结论：**C-03.1 已完成；C-03 尚未关闭，不开放视频执行或发布。**

## 1. 与最初需求的对齐

| 最初 C-03 要求 | 本节点实现与证据 | 状态 |
| --- | --- | --- |
| 只解析机器通道 | `VideoProgressParser` 只消费 `out_time_us`、`total_size`、`speed`、`progress`；真实产品 FFmpeg 到达 `progress=end` | 通过 |
| 参数数组，不拼 shell | `build_ffmpeg_arguments` 返回 `Vec<OsString>`；中文、空格、`&`、括号路径作为单个 OS 参数通过真实转码 | 通过 |
| Windows Job Object 管理进程树 | Job 设置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`，支持显式 `TerminateJobObject`；真实分配并终止进程测试通过 | 通过 |
| 指标必须有事实来源 | 百分比来自 `out_time_us / ffprobe duration`；`total_size` 标记 provisional；速度来自 FFmpeg；ETA 等待两个递增时间样本 | 通过 |
| 首期固定 H.264/AAC 软件路径 | 参数显式使用 `h264_mf`、`-hw_encoding 0`、`-rate_control cbr`，有音频时使用 AAC；`-fps_mode passthrough` 对齐 VFR 时间戳策略，输出固定 MP4 | 通过 |

## 2. 真实代码边界与纠偏

- 没有新增任务队列、取消注册表、进度事件或发布事务；后续执行命令必须复用 `commands/compression.rs` 的统一取消注册表和 `task-progress/task-log`。
- 暂存输出继续采用 B-00 已有的同目录唯一命名；C-03 只负责写暂存文件及失败/取消清理。
- C-04 才负责用 ffprobe 验证编码、时长、尺寸、流和文件事实，并调用共享原子发布事务。本节点没有把 FFmpeg 退出码冒充最终成功。
- C-02 工作区仍保留 `execute-disabled`，因此本节点不会产生用户可见的占位成功、任务历史或未经验证的视频输出。

## 3. 验证结果

- `cargo test video_encoding --lib`：5/5 通过。
  - 真实 `h264_mf`/AAC 产品 FFmpeg 转码：通过；输入与暂存路径含中文、空格、`&` 和括号。
  - 机器进度、两样本 ETA、临时输出大小/比例：通过。
  - 非法机器字段拒绝：通过。
  - Windows Job Object 创建、分配、终止真实进程：通过。
- `cargo test --lib`：341 通过、0 失败、4 个显式忽略的外部/破坏性门禁。
- `cargo clippy --lib -- -D warnings`：通过。
- `npm run test:media-architecture`：通过。
- `npm run type-check`：通过。

仓库既有全量 `cargo fmt --check` 会报告大量本节点之外的历史格式差异；本节点只对新增 Rust 文件运行 `rustfmt`，未机械改写无关代码。

## 4. 下一接续点

C-03.2：建立后端暂存编码执行器，复用统一取消注册表、容量预检和任务事件；并发读取 progress/stdout 与受限 stderr，提供无进度心跳，取消时终止 Job Object 并清除暂存与 passlog 家族。仍不得发布最终文件。随后 C-03.3 才接入统一视频任务 UI 和真实桌面取消矩阵，C-04 验证通过前不能把任务标记为最终完成。
