# C-02.4 视频真实分类与桌面矩阵审计

日期：2026-08-28

分支：`codex/video-c02-probe-model`

范围：关闭 C-02 剩余的真实多音轨、长视频和 Windows Tauri/WebView2 工作区验收；不进入编码、任务进度、取消或输出发布。

## 结论

C-02 已整体完成，允许进入 C-03。真实分类矩阵现覆盖 VFR、90° 旋转、无音频、双音轨、字幕、10 分钟输入和损坏容器；真实桌面工作区完成系统选择路径、生产运行时校验、ffprobe、三档重规划、估算/流变化展示、执行冻结、历史零写入、模式切换保持和双窗口宽度布局。

新增长样本没有使用系统 FFmpeg，也没有提交大二进制。产品构建本身不含 lavfi source，因此测试用已准入且逐字节冻结的 1 秒 MP4 作为输入，由产品 `ffmpeg.exe` 以 `-stream_loop` 和 `-c copy` 现场生成 30 秒双音轨/字幕容器及 10 分钟音视频容器，再交给产品 ffprobe 和生产 Rust 解析器。流复制不改变编码负载，生成与两次探测总计约 0.22 秒。

## C-02 最初验收闭环

| 验收项 | 真实证据 | 结果 |
| --- | --- | --- |
| VFR | 冻结 H.264 输入及 30 秒/10 分钟派生容器均返回 `variable` | 通过 |
| 旋转 | 编码矩阵 640×360、90°、可见尺寸 360×640；桌面真实展示 | 通过 |
| 无音频 | 冻结 HEVC 输入返回零音轨且规划不虚构音频码率 | 通过 |
| 多音轨 | 产品流复制生成 ENG/ZHO 两条 AAC；生产探测返回 2 并告警额外音轨丢弃 | 通过 |
| 字幕 | 冻结及 30 秒输入保留 mov_text；桌面显示字幕移除与显式确认 | 通过 |
| 长视频 | 产品流复制生成并探测约 600 秒真实 MP4，不扫描媒体包即可稳定返回 | 通过 |
| 损坏输入 | 非容器字节稳定返回 `VIDEO_PROBE_PROCESS_FAILED` | 通过 |
| 估算标记 | 后端 `isEstimate=true`；真实桌面显示“预计输出 · 估算”和中文不确定性说明 | 通过 |
| 三档/最大分辨率/流策略 | 后端权威规划、前端同构配置及桌面档位重规划通过 | 通过 |

## 真实桌面预期—实际—修正

| 预期 | 首次实际 | 修正 | 复验 |
| --- | --- | --- | --- |
| 启动真实 WebView2 | 本机 EdgeDriver 缓存缺失，在应用启动前安全失败 | 用仓库既有安装脚本读取 WebView2 完整版本，下载并验证精确匹配驱动 | 进入 WebDriver |
| Release 加载内置前端和测试桥 | 独立 Cargo 构建遗漏 `custom-protocol`，WebView2 访问 localhost 并被拒绝 | 使用 `VITE_DESKTOP_E2E=1` 前端及 `custom-protocol,desktop-e2e` Release 后端 | 内置页面与测试桥就绪 |
| 独立二进制复用正式资源 | Cargo standalone 不执行 Tauri bundle 资源复制，生产预检正确报告资源缺失 | 聚焦门禁把仓库产品视频资源镜像到 standalone 的 Tauri `resource_dir`；生产校验仍逐文件验证大小与 SHA-256 | 两输入均规划就绪 |
| 窄窗口无横向滚动 | 760×560 时工作区溢出 171 px | 工具栏改为容器宽度自适应换行，工作区和全部直接子项锁定 box/max/min width，横向隐藏、纵向独立滚动 | 1100×720 与 760×560 的 main/workspace/card/facts 溢出均 ≤1 px |

## 验证证据

- `cargo test --manifest-path src-tauri/Cargo.toml video_probe --lib -- --nocapture`：7/7 通过，包含真实 30 秒双音轨和 10 分钟容器。
- `cargo test --manifest-path src-tauri/Cargo.toml video_ --lib`：C-01/C-02 共 17/17 通过。
- `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings`：通过。
- `npm run test:e2e:desktop:video-workspace`：真实 Windows Release Tauri/WebView2 通过；输出两张宽度截图到未提交审计目录。
- `npm run type-check`、`npm run test:media-architecture`、`node --check scripts/test-tauri-desktop.mjs`：通过。
- `git diff --check`：通过。

## 下一接续点

C-03 执行、进度与取消。开始前必须先核对统一任务/取消/容量预检和发布事务的现有实际代码；首段只应建立安全参数计划与子进程生命周期，不得在 C-04 输出复核完成前发布结果或回收源文件。C-05 的真实格式/安装/更新矩阵及 Windows N 前后实机证据仍是 v1.1.16 发布门禁。
