# C-01.2.2 正式安装运行时与 updater 增量审计

审计日期：2026-08-28

状态：**完成。C-01 原始验收已全部满足；真实 Windows N 前后实机证据保留为 C-05/发布门禁，不再错误阻塞 C-02。视频 UI 仍未实现，不能表述为视频压缩可用。**

## 1. 原始需求与本轮边界

C-01.2.2 关闭 C-01 的分发与平台边界：正式安装目录生产预检、真实软件转码、安装资源缺失/替换拒绝、Windows N 无 Media Feature Pack 的稳定分类实现，以及同提交、同配置的无/含 FFmpeg NSIS 与 updater ZIP 精确增量。本节点不增加视频 UI、输入探测模型、任务编排、硬件编码或公开 Release。

历史回溯确认最初主计划提交 `8ce898b` 的 C-01 验收目标是：CI/安装包二进制哈希可追踪、缺失或替换时拒绝开始任务、许可清单完整，并要求安装态执行版本/编码器/过滤器及记录安装和更新体积。后续候选审计增加的是“Windows N 缺 Media Foundation 时明确分类”，不是“真实 Windows N 必须在 C-02 前完成”。真实多平台、安装版和公开更新复验原本归属 C-05 与第 9 节发布门禁。此前将实机 N 证据设为 C-01.2.2 唯一阻塞属于范围前移，本次予以纠正。

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

### 4.1 同提交签名 updater 精确测量

GitHub Actions 运行 `33173219785` 在提交 `6b95f5c2f6d66fc0a879eebb10f0346eac6792c7`、同一 Windows runner、同一 Tauri 工具链和同一 updater minisign 密钥下完成双构建。基线只从一次性 runner 的规范配置中移除 8 个视频资源，构建后无条件恢复配置再构建集成包。

| 产物 | 无视频资源基线 | 含视频资源集成 | 精确差值 |
| --- | ---: | ---: | ---: |
| NSIS | 8,671,804 B；`bd32dba2...10c9a33` | 15,493,774 B；`118397ce...030b3f` | **6,821,970 B（78.6684%）** |
| updater ZIP | 8,671,964 B；`1e5faa5b...e62ab` | 15,493,934 B；`8e1dfb51...79d24f` | **6,821,970 B（78.6670%）** |

两个 updater ZIP 都通过 7-Zip 完整性测试，包内唯一 NSIS 与各自独立 NSIS 字节一致；两个 428 字符 `.sig` 均解码为 Tauri minisign 结构。`compressedInstallerDeltaBytes` 与 updater 对应字段据此写为 `6,821,970`，不再沿用 C-01.2.1 的跨提交未签名聚合数 `6,895,417`。

### 4.2 正式安装生命周期

本机基线为 Windows x64 `10.0.22621`、已安装 v1.1.13。正式候选来自上述工作流的集成 NSIS，生命周期结果为：

1. v1.1.13 → v1.1.15 覆盖安装成功，安装位置保持 `E:\long\Long解压`，产品 EXE、卸载器和 shell extension 版本一致；
2. 从正式安装 EXE 调用生产预检成功，并使用正式安装目录内 FFmpeg/ffprobe 完成真实软件转码与语义复核；
3. 唯一隔离副本中的 FFmpeg 缺失与替换分别被稳定拒绝，正式安装目录全程只读；
4. 候选卸载成功，安装文件和安装器拥有的菜单键清理干净；
5. 两处用户数据指纹保持，v1.1.13、原安装路径和原菜单模式恢复成功。

机器报告位于忽略目录 `test-results/installed-release-validation/20260828-212031/result.json`；清单固化通过项而不提交包含本机绝对路径的原始机器报告。

### 4.3 完整回归

- 前端单元/组件：44 个测试文件、254 个测试全部通过；Playwright：32 个通过、13 个按既有项目条件跳过；
- Rust：323 个测试通过、0 失败、4 个明确的环境型测试保持 ignored；
- 工作流与安装态证据均来自提交后的干净产物，不用本机无法启动的 `makensis` 结果代替。

### 4.4 Windows N 实机证据入口

新增 `scripts/test-windows-n-video-runtime.ps1`，把最后一个外部平台门禁固定为两个不可互换的阶段：

- `MissingMediaFeaturePack`：只接受 `EditionID` 以 `N` 结尾的真实 Windows N，要求 HKCU 正式安装记录、v1.1.15 和正式候选 EXE 的大小/SHA-256 完全一致；随后必须由正式 EXE 返回非成功，并写出 `VIDEO_ENGINE_MEDIA_FOUNDATION_UNAVAILABLE`。
- `MediaFeaturePackInstalled`：要求读取同一证据目录的前阶段通过报告，并用脱敏后的 MachineGuid 哈希证明是同一机器；生产预检必须转为成功，再复用安装态矩阵完成真实软件转码、ffprobe 输出复核和隔离缺失/替换拒绝。

新增只读验收器 `scripts/verify-windows-n-video-runtime-evidence.mjs`。报告 schema 升级为 2；后阶段固化前阶段报告的 SHA-256，并要求前后阶段使用同一个证据生成脚本、同一机器和同一候选 EXE。验收器独立复核两个阶段、三个 MF 模块清单、生产预检、真实转码 H.264/AAC/480×854/1.2 秒事实及缺失/替换拒绝，任一报告缺失、失败或被替换都不会生成通过结论。

两个阶段均记录 OS caption、EditionID、build、三个 Media Foundation DLL 的存在性/身份以及生产预检原始报告。脚本不查询需要管理员权限且可能本地化的 Windows Capability 文本，也不把“DLL 人工改名”或单元注入视为实机证据。当前 Windows 11 专业版 `EditionID=Professional` 的负向自检会稳定返回 `WINDOWS_N_MACHINE_REQUIRED` 并写失败报告，证明普通 Windows 不能误提交为 Windows N 结果。

本地检查已通过 PowerShell AST、Node 语法、清单字段门禁和上述普通版防误报；验收器会拒绝该普通版失败报告并写出 `passed=false`。首次跨工具读取暴露 Windows PowerShell 5 的 UTF-8 BOM，读取器现只剥离文件开头单个 BOM 后解析；前阶段字节链仍对原始文件（包含 BOM）计算 SHA-256，不弱化篡改检测。正式 N 前/后阶段只能在目标机器执行，当前仍不记为通过。

环境可用性审计：当前主机是 `EditionID=Professional`，三个必需 Media Foundation DLL 均存在；没有可调用的 Hyper-V、VirtualBox 或 VMware 管理命令，项目测试目录及当前用户下载目录也没有明确的 Windows N 安装介质。因此本轮不能在不新增系统镜像/虚拟化环境的前提下取得真实 N 证据。下载许可受约束的 Windows 镜像、安装虚拟化组件或修改宿主系统版本均超出本节点已有授权，不自动执行。

在目标 Windows N 上使用同一证据目录依次执行：

```powershell
npm run test:windows-n-video-runtime -- -Phase MissingMediaFeaturePack -InstalledExecutable '<正式安装目录>\Long解压.exe' -EvidenceDirectory '<证据目录>'
# 通过 Windows 设置安装 Media Feature Pack，并按系统要求重启
npm run test:windows-n-video-runtime -- -Phase MediaFeaturePackInstalled -InstalledExecutable '<同一正式安装目录>\Long解压.exe' -EvidenceDirectory '<同一证据目录>'
npm run verify:windows-n-video-runtime -- '<同一证据目录>'
```

## 5. C-05 保留门禁与下一动作

1. C-02 现在按原始顺序进入输入探测与配置模型；C-01 的分类实现和依赖准入不再重复开发。
2. C-05 必须在真实 Windows N 且未安装 Media Feature Pack 的机器安装候选，运行 `MissingMediaFeaturePack` 阶段；安装组件并重启后运行 `MediaFeaturePackInstalled`。
3. C-05 运行独立验收器并取得 `verification.json` 的 `passed=true`，之后才可把 `windowsNRealMachinePassed` 改为 `true`。该证据仍阻止 v1.1.16 发布，但不阻止 C-02 至 C-04 的开发。

测量实现演练中纠正两项假设：Tauri 额外配置会叠加资源数组，基线必须在一次性 runner 内备份后原位过滤并无条件恢复；公开 NSIS 使用英文资产名，而 updater ZIP 内为中文产品名，必须枚举唯一 EXE 后按字节比较，不能按 basename 推断。正式运行随后证明该方案可重复执行并产生完整签名证据。

本机 `makensis.exe` 即使清空并重新下载 Tauri 官方 NSIS 缓存也在进程启动阶段返回 `0xC0000135`。这属于当前主机的 32 位 NSIS 运行环境问题；正式结论使用 GitHub 干净 Windows runner 构建的同提交产物，并在本机完成安装验证，不把本机构建失败混入产品缺陷。
