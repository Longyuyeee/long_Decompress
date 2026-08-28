# C-01.1 FFmpeg 可复现候选审计

审计日期：2026-08-28

目标版本：`1.1.16` 前置开发，不修改当前 `1.1.15` 版本身份

结论：**最小 LGPL Windows x64 候选构建、双构建复现和真实软件转码已通过；产品集成仍保持阻断，C-01 尚未整体关闭。**

## 1. 需求对齐

| 目标 | 当前实际 | 结论 |
| --- | --- | --- |
| 固定版本、来源、哈希 | FFmpeg `9.0.1` 官方源码，12,036,420 B，SHA-256 `cf38e0e2...7f635`；分离签名和发布指纹已由 `test:media-dependencies:real` 验证 | 通过 |
| Windows x64 可复现构建 | 同一脚本在 `formal-build-a`、`formal-build-b` 两个干净绝对目录构建，两个 EXE 均逐字节一致 | 通过 |
| LGPL 且不启用 GPL/nonfree | `ffmpeg -version` 报告 `LGPL version 2.1 or later`；配置不含 `--enable-gpl`、`--enable-nonfree`、libx264、libx265、libopenh264 | 通过 |
| 首期软件 H.264/AAC | 使用 Windows Media Foundation `h264_mf`，显式 `-hw_encoding 0`；内置 AAC 编码器 | 通过 |
| 能力和动态依赖可审计 | 真实执行 `-version`、`-encoders`、`-filters`；PE 只导入 5 个 Windows 系统 DLL | 通过 |
| 真实输入和真实输出 | 5 秒磁盘 MP4/H.264/AAC 输入实际转码，输出重新由候选 `ffprobe` 验证 | 通过 |
| 随安装包携带二进制、许可、来源和配置 | 候选目录已生成二进制、LGPL 2.1/3 文本、`config.mak`、构建日志；尚未进入 Tauri resources | **待 C-01.2** |
| 安装态缺失/替换拒绝与精确 NSIS/更新体积 | 尚未产品集成，不能伪造安装态结论；manifest 中精确保持 `integrationAllowed=false` 和 NSIS delta `null` | **待 C-01.2** |

## 2. 冻结构建身份

- 构建入口：`scripts/build-ffmpeg-c01-windows.sh`。
- 构建环境：WSL2 Ubuntu 24.04.3；MinGW GCC POSIX `13.2.0-6ubuntu1+26.1`、MinGW headers `11.0.1-3build1`、binutils `2.41.90.20240122-1ubuntu1+11.4`、NASM `2.16.01-1build1`、make `4.3-4.1build2`。
- `SOURCE_DATE_EPOCH=1786505700`，取官方源码包顶层目录时间 `2026-08-12 03:35:00 UTC`；配置前缀固定为 `/opt/long-decompress/ffmpeg-9.0.1`。
- 静态构建，网络关闭，所有自动探测关闭；启用的编码器只有 `h264_mf`、`aac`，启用的硬件加速器列表为空。
- `d3d11va` 只作为 Media Foundation 编译所需的 Windows 接口启用，并不启用 FFmpeg hwaccel；公开软件路径仍强制 `hw_encoding=0`。
- 链接参数为 `-static -static-libgcc -Wl,--no-insert-timestamp`。

冻结产物：

| 文件 | 字节 | SHA-256 |
| --- | ---: | --- |
| `ffmpeg.exe` | 12,349,440 | `35c3c8bb7d9371825ba3ee8ee6f6b39205877c5d1172e4a4e925c2d6368672eb` |
| `ffprobe.exe` | 12,131,840 | `2c1df07c649e9499eddd40b445c8721f07b95b8a85524a5e8645a86fb2ba1d98` |
| `COPYING.LGPLv2.1` | 26,517 | `246041b6ecf9bc32d718a62c57877c78b5eb397b6467e74ed7ae2626ab189c30` |
| `COPYING.LGPLv3` | 7,651 | `da7eabb7bafdf7d3ae5e9f223aa5bdc1eece45ac569dc21b3b037520b4464768` |

候选二进制和许可共 24,515,448 B；隔离使用项目 7-Zip LZMA2 压缩为 3,736,540 B。后者只是候选压缩测量，**不是**签名 NSIS 或 updater 的实际增量。

## 3. 真实能力与转码结果

测试输入由 Ubuntu FFmpeg `6.1.1-3ubuntu5` 仅在测试目录生成，不进入产品：MP4/H.264/AAC、640×360、30 fps、5 秒，545,428 B，SHA-256 `147af014fceb84fed015503ac7105ca10ba025f143030489f6ffce4617c148a6`。

候选实际执行软件转码：缩放至 480×270、`h264_mf`、`hw_encoding=0`、AAC 128 kbps、faststart，并通过 `-progress pipe:1` 输出机器事件。最终：

- `frame=150`、`out_time_us=5000000`、`progress=end`；
- 两个最终构建分别写入隔离审计目录；候选 `ffprobe` 均重新确认 MP4、H.264 480×270、AAC、时长 `5.000000` 秒；Media Foundation 码流字节不作为确定性契约；
- 预期与实际语义差异为 0；完整机器结果在忽略目录 `test-results/video-c01-audit/verification/<build>/result.json`。

Windows PE 导入为 `bcrypt.dll`、`KERNEL32.dll`、`msvcrt.dll`、`ole32.dll`、`SHELL32.dll`；不存在 `libwinpthread-1.dll`、`libgcc_s_seh-1.dll` 或 `libstdc++-6.dll`。

## 4. 预期—实际—修正记录

1. 预期 WSL 官方 apt 源可安装 MinGW；实际旧 HTTP 源返回 404。修正为 Ubuntu 官方 HTTPS + 系统 keyring 后安装固定工具链。
2. 预期关闭 hwaccel 后 `h264_mf` 软件编码可直接编译；实际 `mfenc.c` 仍引用 D3D11 类型。修正为启用 D3D11VA 编译接口，同时保持 `Enabled hwaccels` 为空，并在运行时强制软件模式。
3. 预期静态 FFmpeg 为单文件运行时；第一候选实际导入 `libwinpthread-1.dll`，Windows 启动退出 `0xC0000135`。修正静态链接 MinGW 运行库后，Windows `-version` 与真实转码通过。
4. 首次 PowerShell 调 WSL 时 `$(nproc)` 被 PowerShell提前解释。修正为脚本内受控并行度 `LONG_FFMPEG_BUILD_JOBS`，不再跨 Shell 插值。
5. 一轮构建过程中脚本正在补写日志重定向，Shell 读到被替换的文件尾并失败；该轮证据作废。随后从两个干净目录完整重跑，产物哈希一致。
6. 既有 `test:fixtures:media` 所锁定的 BtbN nightly 资产 ID 已被上游轮换，真实下载 4 次均为 HTTP 404。C-01 不改用未冻结 nightly 冒充稳定来源；本节点使用明确记录版本的本地测试工具生成真实输入。夹具长期来源必须在 C-01.2/C-02 前修正。
7. 首次 PE 审计因 `objdump -p` 超过 Node 默认缓冲区而报 `ENOBUFS`。测试提高到 32 MiB 后重新运行通过，没有减少导入检查范围。
8. 最终双候选首次并行转码共用同一输出路径，存在测试写入竞态，因此该轮双验证证据作废。现按 runtime 构建名隔离输出目录后分别重跑，且不把 Media Foundation 码流字节误当成确定性产品契约。

上游 GCC 对 AAC、SBR、VLC、WMA 源码给出若干静态边界分析 warning；构建成功且这些 warning 已保留在 `build.log`。本节点不把 warning 隐藏为零，也不在未形成可复现运行缺陷时私自修改上游源码。

## 5. 下一接续点

C-01.2 必须完成后才能关闭 C-01：

1. 修复视频测试夹具对会轮换的 nightly 资产 ID 的依赖，建立长期可重建且有输入哈希清单的真实视频夹具；
2. 将冻结候选、FFmpeg/MinGW/GCC 许可与来源/配置清单纳入 Tauri resources；
3. 后端启动前验证版本、能力、精确文件大小和 SHA-256，缺失/替换时明确拒绝；Windows N 缺 Media Foundation 时明确分类；
4. 做正式 Release/安装态 `-version`、`-encoders`、`-filters` 和真实转码；
5. 对比相同提交的无 FFmpeg 与含 FFmpeg 签名 NSIS/updater，记录真实增量。完成前不启用视频入口、不升版、不发布。

## 6. 已执行门禁

- 两个不同干净目录执行完整构建：通过，两个 EXE 哈希逐字节一致；
- 使用错误源码输入执行构建入口：按预期在任何清理/配置前因 SHA-256 不匹配拒绝；
- `npm run test:video-c01:real -- --runtime ... --input ...`：两个最终构建分别通过，真实输出语义差异 0；
- `npm run test:media-dependencies:real`：通过，官方源码、签名、指纹和哈希重新验证；
- 静态媒体依赖、媒体架构、类型检查、生产构建、Release identity：全部通过；
- 前端单测：44 个文件、254/254 通过，前置真实图片夹具 11 图 + 1 个 PDF 拒绝边界通过。
