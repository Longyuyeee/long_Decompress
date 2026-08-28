# C-01.2.2 正式安装运行时与 updater 增量审计

审计日期：2026-08-28

状态：**进行中；生产实现和同布局隔离验证已完成，正式 NSIS 安装生命周期、真实 Windows N 机器和 GitHub updater 签名增量尚待完成。**

## 1. 原始需求与本轮边界

C-01.2.2 只关闭 C-01 的分发与平台边界：正式安装目录生产预检、真实软件转码、安装资源缺失/替换拒绝、Windows N 无 Media Feature Pack 的稳定分类，以及同提交、同配置的无/含 FFmpeg NSIS 与 updater ZIP 精确增量。本节点不增加视频 UI、输入探测模型、任务编排、硬件编码或公开 Release。

项目没有商业 Authenticode 证书。文档中的“签名 updater”指 Tauri updater 完整性签名；普通 NSIS 可以无 Authenticode 签名，Windows 11 原生上下文菜单身份包仍不得在无证书时生成。

## 2. 已完成实现

- 生产 `validate_video_engine` 在八文件身份校验后，从 Windows System32 加载 `mfplat.dll`、`mf.dll` 和 `mfreadwrite.dll`；任一缺失统一返回 `VIDEO_ENGINE_MEDIA_FOUNDATION_UNAVAILABLE:<module>:win32=<code>`。
- `VideoEngineStatus` 明确返回 `mediaFoundationAvailable=true`，不以发现 `h264_mf` 名称代替平台运行条件。
- 正式应用新增内部参数 `--internal-video-engine-preflight-report <path>`；该路径在单实例、数据库和窗口初始化前，只用当前 EXE 所在目录解析 `resources` 并调用同一个生产验证器。成功、验证拒绝和报告写入失败分别返回 0、2、3。
- `test-installed-video-runtime.mjs` 从正式应用自身读取生产预检报告，用安装目录内 `ffmpeg.exe` 对冻结 H.264/VFR/AAC/90°/字幕 MP4 执行 `h264_mf -hw_encoding 0` 与 AAC 真实转码，再由同目录 `ffprobe.exe` 复核 H.264、AAC、480×854 和 1.2 秒事实。
- 缺失和替换测试只复制应用 EXE 与视频资源到唯一隔离目录；不会改写正式安装文件。隔离副本分别返回 `VIDEO_ENGINE_RESOURCE_MISSING` 和 `VIDEO_ENGINE_RESOURCE_HASH_MISMATCH`。
- 既有安装生命周期脚本新增 `-RunVideoRuntimeMatrix`，候选覆盖安装并校验菜单后执行上述矩阵，随后仍走卸载、用户数据保持和上一公开版本恢复。

## 3. 预期—实际—修正

| 项目 | 首次预期 | 实际 | 修正 |
| --- | --- | --- | --- |
| Windows N 分类 | FFmpeg 能力清单可代表 Media Foundation 可用 | `-encoders` 只证明编译能力，不能证明系统 DLL 存在 | 生产预检先从 System32 加载三个必需 MF 模块；单元测试固定缺失分类 |
| 安装负向测试 | 可直接移动安装目录资源 | 会不必要地触碰正式安装文件并扩大恢复风险 | 在唯一隔离安装副本中执行缺失/篡改，正式目录只读 |
| 真实输出尺寸 | 沿用 C-01.1 临时横屏样本的 480×270 | 冻结产品夹具带 90° Display Matrix；FFmpeg 自动应用方向后输出 480×854 | 按冻结夹具真实可见方向修正固定事实，不降低编码/探测断言 |
| 真实输出时长 | 沿用 C-01.1 临时 5 秒样本 | 冻结产品夹具实际输出为 1.2 秒 | 固定为该跟踪夹具的真实结果；C-04 才定义通用时长阈值 |

## 4. 当前验证

- `cargo test video_engine --lib`：4/4 通过，包括真实候选、缺失、篡改和 Windows N 稳定分类。
- `cargo check --bin long-compress-assistant`：通过，证明内部安装态入口进入正式二进制。
- 与正式安装目录相同的唯一隔离布局：生产预检、真实软件转码、同目录 ffprobe、缺失拒绝、替换拒绝全部通过，差异 0。
- `cargo clippy --all-targets -- -D warnings`、`npm run type-check`、媒体架构和依赖门禁：通过。

## 5. 未完成项与下一动作

1. 构建正式 NSIS，执行 `test-installed-release.ps1 -RunVideoRuntimeMatrix`，保留覆盖安装、真实安装目录、卸载和上一版本恢复证据。
2. 新增只读测量工作流，在同一提交、同一工具链与同一 updater 密钥下分别打包去除视频资源和包含视频资源的 NSIS/updater ZIP；验证 updater 内 EXE 与 NSIS 一致并记录精确差值。
3. 在真实 Windows N 且未安装 Media Feature Pack 的机器运行正式安装预检。当前单元负向路径不能冒充真实 Windows N 机器证据。
4. 上述三项完成后才更新 `compressedInstallerDeltaBytes`、关闭 C-01 并进入 C-02。

测量工作流实现检查点：`video-c01-2-2.yml` 已建立同提交双构建和机器报告；测量器使用公开 v1.1.14/v1.1.15 的 NSIS、updater ZIP 与 Base64 包装 minisign 签名完成真实演练。演练中纠正两项假设：Tauri 额外配置会叠加资源数组，基线必须在一次性 runner 内备份后原位过滤并无条件恢复；公开 NSIS 使用英文资产名，而 updater ZIP 内为中文产品名，必须枚举唯一 EXE 后按字节比较，不能按 basename 推断。
