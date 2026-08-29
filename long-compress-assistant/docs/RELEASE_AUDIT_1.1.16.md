# Long解压 v1.1.16 发布审计

审计日期：2026-08-29（Asia/Shanghai）

状态：**功能节点与本地候选身份门禁完成；允许进入 PR/正式 NSIS 候选验证，尚未允许创建公开标签或 Release。**

## 结论

视频 C-01 至 C-05 已在获批的非 N Windows x64 支持范围内关闭。版本身份统一提升为 `1.1.16`，本地前端、Rust Release、严格 Clippy、Shell Extension、媒体合同和生产依赖安全门禁通过。下一步必须由干净 GitHub Windows Runner 构建正式 NSIS，并以公开 `v1.1.15` 为基线完成候选覆盖、安装态视频复验、卸载和基线恢复；随后才允许通过受保护主分支 PR、标签 Release 和公开更新闭环。

## 需求对齐

| 范围 | 实际结果 | 判定 |
| --- | --- | --- |
| 输入与规划 | MP4/MOV/AVI/WMV/WebM，真实 ffprobe，三档和最大分辨率 | 已完成 |
| 流变化 | VFR、旋转、无音频、多音轨、字幕和损坏输入稳定分类并显式确认 | 已完成 |
| 安全执行 | 参数数组、机器进度、心跳、容量预检、Job Object 进程树取消 | 已完成 |
| 输出验证与发布 | MP4/H.264/AAC、完整帧、时长/尺寸/流、竞态和原子发布 | 已完成 |
| 任务与历史 | 统一 `compression/video`、真实指标、取消和跨完整重启历史 | 已完成 |
| 真实矩阵 | 5 格式、4 分辨率、三预设、600 秒、109.52 MiB、默认应用 | 已完成 |
| 正式安装功能链 | 既有 C-05.4.1 候选生命周期 50/50、视频工作区 20/20 | 已完成 |
| Windows N | 未取得实机证据；产品负责人明确批准暂不支持并移出本版发布矩阵 | 已授权缩小范围 |
| `1.1.16` 正式候选与公开更新 | 干净 CI NSIS、版本候选覆盖、Release 资产和应用内更新 | 待完成 |

## 开发偏移与纠正

1. 原始计划和生产实现固定公开输出为 MP4/H.264/AAC，但 `media-release-gates.json` 曾把 `h265-mp4` 列为公共输出。实际 H.265 只作为输入矩阵用例。现已将公共输出收敛为唯一 `h264-mp4`，同时由静态门禁要求继续保留 `h265-input` 真实用例。
2. 视频安装包校验脚本曾把默认文件名硬编码为 `Long解压_1.1.15_x64-setup.exe`。现改为读取 `tauri.conf.json` 的实际版本生成默认路径，避免版本提升后脚本继续检查旧产物。
3. Windows N 实机门禁无法在唯一 Professional 主机取得。经产品负责人明确授权，本版本支持面缩小为 `windows-x86_64-non-n`；`windowsNRealMachinePassed` 继续为 `false`，两阶段工具和生产缺失拒绝均保留。

## 版本身份

- `package.json`、`package-lock.json` 根与 workspace、`tauri.conf.json`、主 Cargo 清单/锁、Shell Extension Cargo 清单/锁统一为 `1.1.16`。
- 唯一版本化 Shell Extension：`long_compress_shell_extension_1_1_16.dll`，246,784 B，SHA-256 `FDE0C00141BF06B5AB87313C81DD615D3125F97725D486DEBE5D90F1E9D92A7D`。
- `test:release-identity -- --expected 1.1.16` 通过，并联动验证媒体依赖、指标、发布合同和图片基线。

## 本地候选门禁

- TypeScript 类型检查通过；前端单元测试 45 文件、262/262；Vite 生产构建通过。
- Rust `cargo test --release --all-targets`：库 362 通过/4 个既定外部条件忽略，主程序及全部集成目标无失败；基准目标按声明不执行破坏性/大体积基线。
- 主后端全目标/全特性 Clippy `-D warnings` 通过；Shell Extension Release 5/5 与严格 Clippy 通过。
- 媒体依赖、17 个生产媒体文件架构和媒体发布合同通过；npm 生产依赖审计为 0 个已知漏洞。
- 本机非提升权限的 `test:context-menu-package` 按设计拒绝向 `LocalMachine\\TrustedPeople` 临时写入自签信任。这不是公开资产阻塞：仓库没有商业 Authenticode 证书，正式无签名构建必须跳过 Windows 11 identity package；经典菜单和 Shell DLL 继续由 Rust/安装门禁验证。

## 候选与公开发布待办

1. 推送版本候选并触发完整 GitHub CI，要求五个 job 全绿并回下载正式 NSIS。
2. 核对 NSIS、主程序、Shell DLL 和 8 项视频运行时的大小、SHA-256、版本与包内完整性。
3. 从公开 `v1.1.15` 覆盖候选，执行安装生命周期和无测试桥视频工作区，卸载后恢复公开基线。
4. 通过 PR 合入受保护 `master`，在合并提交创建 annotated `v1.1.16` 标签。
5. 等待 Release workflow 生成 NSIS、updater ZIP、`.sig` 和 `latest.json`；公开回下载逐字节复核。
6. 执行真实 `v1.1.15 → v1.1.16` 应用内更新，复核重启、位置、用户数据、自动启动、经典菜单、唯一 Shell DLL 和视频能力。

只有以上待办全部完成并回填精确提交、工作流、资产和结构化证据后，状态才能改为“正式发布完成”。
