# Long解压 v1.1.16 发布审计

审计日期：2026-08-30（Asia/Shanghai）

状态：**正式发布与公开更新验证已完成；发布后安装生命周期恢复加固已通过实机复验。**

## 结论

视频 C-01 至 C-05 已在获批的非 N Windows x64 支持范围内关闭。PR #92 经完整 CI 合入受保护主分支，annotated `v1.1.16` 标签固定在合并提交 `a59742265feb961ab51f9b95b4e455aa15b79bf5`。Release workflow 已生成四项签名公开资产；公开回下载、哈希/签名/包内身份对账及真实 `v1.1.15 → v1.1.16` 应用内更新 24/24 全部通过。Windows N 继续明确为暂不支持，不影响已获批的非 N Windows x64 发布结论。

## 需求对齐

| 范围 | 实际结果 | 判定 |
| --- | --- | --- |
| 输入与规划 | MP4/MOV/AVI/WMV/WebM，真实 ffprobe，三档和最大分辨率 | 已完成 |
| 流变化 | VFR、旋转、无音频、多音轨、字幕和损坏输入稳定分类并显式确认 | 已完成 |
| 安全执行 | 参数数组、机器进度、心跳、容量预检、Job Object 进程树取消 | 已完成 |
| 输出验证与发布 | MP4/H.264/AAC、完整帧、时长/尺寸/流、竞态和原子发布 | 已完成 |
| 任务与历史 | 统一 `compression/video`、真实指标、取消和跨完整重启历史 | 已完成 |
| 真实矩阵 | 5 格式、4 分辨率、三预设、600 秒、109.52 MiB、默认应用 | 已完成 |
| 正式安装功能链 | CI 候选生命周期 50/50、生产视频运行时差异 0、视频工作区 20/20 | 已完成 |
| Windows N | 未取得实机证据；产品负责人明确批准暂不支持并移出本版发布矩阵 | 已授权缩小范围 |
| `1.1.16` 正式候选 | 干净 CI NSIS、包内身份、版本候选覆盖与公开基线恢复 | 已完成 |
| `1.1.16` 公开发布与更新 | 受保护分支合并、签名 Release 资产、回下载和应用内更新 | 已完成 |

## 开发偏移与纠正

1. 原始计划和生产实现固定公开输出为 MP4/H.264/AAC，但 `media-release-gates.json` 曾把 `h265-mp4` 列为公共输出。实际 H.265 只作为输入矩阵用例。现已将公共输出收敛为唯一 `h264-mp4`，同时由静态门禁要求继续保留 `h265-input` 真实用例。
2. 视频安装包校验脚本曾把默认文件名硬编码为 `Long解压_1.1.15_x64-setup.exe`。现改为读取 `tauri.conf.json` 的实际版本生成默认路径，避免版本提升后脚本继续检查旧产物。
3. Windows N 实机门禁无法在唯一 Professional 主机取得。经产品负责人明确授权，本版本支持面缩小为 `windows-x86_64-non-n`；`windowsNRealMachinePassed` 继续为 `false`，两阶段工具和生产缺失拒绝均保留。
4. 首轮干净 CI run `33261161074` 的 Browser、Frontend 和 Windows desktop E2E build 通过，但 Rust 后端 361/362 在真实发布命令测试中返回 `VIDEO_ENCODING_PROGRESS_INVALID: out_time_us`，安装器按依赖门禁未运行。冻结 FFmpeg 9.0.1 源码 `fftools/ffmpeg.c` 明确把未知 `out_time_us` / `total_size` 输出为 `N/A`，时间则使用有符号 `PRId64`；旧解析器错误地强制两个字段都是 `u64`。现只对白名单 `N/A` 表示未知，将负启动时间钳制为用户进度 0，负大小及其他畸形值仍拒绝，并新增定向回归。修正提交 `6d3469f5d57fe29c9accabe369f89c4f86b66bbd` 已由第二轮干净 CI run `33261741520` 全量复验通过。
5. 首轮公开更新门禁在安装动作前安全失败：持久化偏好为 `autoStart=true`，但此前候选卸载/基线重装清掉了 Windows Run 值。通过公开 v1.1.15 的真实设置 UI 读取实际关闭状态并显式开启，精确注册 `"E:\long\Long解压\Long解压.exe" --autostart`；没有直接写注册表、放宽断言或伪造更新结果。随后从头重跑公开更新通过。
6. 发布后复盘确认 `test-installed-release.ps1` 只恢复用户数据和菜单，没有恢复自动启动；其用户数据备份也没有证明复制期间源目录稳定。现已加固为复制前/后源指纹与备份指纹三方一致、验证暂存后同卷恢复、恢复竞态拒绝，并在成功/异常路径精确恢复原 Run 值。首两轮实测分别暴露不稳定快照和对已存在 Run 键执行 `New-Item -Force` 的权限问题；纠正后公开 v1.1.16 同版本生命周期 49/49 通过。

## 版本身份

- `package.json`、`package-lock.json` 根与 workspace、`tauri.conf.json`、主 Cargo 清单/锁、Shell Extension Cargo 清单/锁统一为 `1.1.16`。
- 本地预检唯一版本化 Shell Extension：`long_compress_shell_extension_1_1_16.dll`，246,784 B，SHA-256 `FDE0C00141BF06B5AB87313C81DD615D3125F97725D486DEBE5D90F1E9D92A7D`。干净 CI 正式候选内同名 Release DLL 为 253,952 B，SHA-256 `A3E623A26ECE74EEC87EAA8857BEC424C88906F46B11635CACF9668FDCCA7FE1`；发布身份以后者为准，不把不同构建环境的字节差异隐藏成同一哈希。
- `test:release-identity -- --expected 1.1.16` 通过，并联动验证媒体依赖、指标、发布合同和图片基线。

## 本地候选门禁

- TypeScript 类型检查通过；前端单元测试 45 文件、262/262；Vite 生产构建通过。
- Rust `cargo test --release --all-targets`：库 362 通过/4 个既定外部条件忽略，主程序及全部集成目标无失败；基准目标按声明不执行破坏性/大体积基线。
- 主后端全目标/全特性 Clippy `-D warnings` 通过；Shell Extension Release 5/5 与严格 Clippy 通过。
- 媒体依赖、17 个生产媒体文件架构和媒体发布合同通过；npm 生产依赖审计为 0 个已知漏洞。
- 本机非提升权限的 `test:context-menu-package` 按设计拒绝向 `LocalMachine\\TrustedPeople` 临时写入自签信任。这不是公开资产阻塞：仓库没有商业 Authenticode 证书，正式无签名构建必须跳过 Windows 11 identity package；经典菜单和 Shell DLL 继续由 Rust/安装门禁验证。
- 修正 CI 进度协议偏差后，视频定向 Release 测试 42/42，通过真实编码命令、官方未知/负启动值、畸形值拒绝、取消、验证和发布；全目标/全特性 Clippy 继续零警告。

## 干净 CI 与正式候选

- 第二轮 GitHub Actions CI run [33261741520](https://github.com/Longyuyeee/long_Decompress/actions/runs/33261741520) 精确检出 `6d3469f5d57fe29c9accabe369f89c4f86b66bbd`，五个 job 全绿：Browser shell E2E 54 秒、Frontend checks 1 分 24 秒、Windows desktop E2E build 4 分 42 秒、Rust and shell-extension checks 8 分 55 秒、Windows installer 5 分 38 秒。
- 回下载 `windows-nsis-installer` 得到 `Long解压_1.1.16_x64-setup.exe`：15,608,481 B，SHA-256 `70F4D4B3C1A86E9C92DA4E8B4C286629E4CB1376070024D2CEB157F9E258B1CA`。
- 7-Zip 26.02 对 NSIS 实测 `Everything is Ok`，22 个文件、展开大小 56,349,323 B。包内主程序 28,864,512 B，SHA-256 `59ACE5A3BE3A35F5CBEBA4936A0768466597E3CD231E1073FD5D1C9D352A26C8`，ProductName 为 `Long解压`，FileVersion/ProductVersion 为 `1.1.16`。
- 包内 8 项视频运行时精确校验通过，共 24,631,334 B、差异 0；FFmpeg/ffprobe、来源、构建配置和许可证均来自冻结资源，没有混入测试用 GPL 生成器。

## 真实安装生命周期

- 基线安装为公开 Release `v1.1.15`，公开 NSIS 8,658,170 B、SHA-256 `DBFF77AE6C1C642B32EFC03C4726D14284A46CAED9AFA9EEA7F6BF4487DD2C51`，安装位置 `E:\long\Long解压`，测试前无运行进程。
- `v1.1.15 → CI v1.1.16 候选 → 卸载候选 → 公开 v1.1.15` 生命周期 50/50 通过；候选安装 EXE 与从 NSIS 独立提取的主程序 SHA-256 一致，两处用户数据指纹、经典右键菜单模式和原安装路径均保持/恢复。
- 正式安装版视频运行时通过生产预检、真实软件转码、缺失/替换资源拒绝，差异 0。无测试桥视频工作区用 114,842,332 B AVI 分别完成中途取消和正式输出：取消无最终/暂存残留；完成输出为 MP4/H.264、1280×720、32.000 秒，默认应用可接收；源 SHA-256 始终不变，取消与完成历史跨完整应用重启各保留一条。
- 结构化证据为忽略目录 `test-results/installed-release-validation/20260830-001551/result.json`。测试结束后机器已恢复公开 v1.1.15、安装位置不变、运行中应用进程 0。

## 受保护分支合并

- PR [#92](https://github.com/Longyuyeee/long_Decompress/pull/92) 头提交为 `6a1c932d5be2035b1930f38dee264576c131ed01`。PR CI run [33262671671](https://github.com/Longyuyeee/long_Decompress/actions/runs/33262671671) 五个 job 全绿：Browser 55 秒、Frontend 1 分 37 秒、Rust/Shell 8 分 43 秒、Windows desktop 5 分 18 秒、Windows installer 5 分 29 秒。
- PR 以 squash 合并为 `a59742265feb961ab51f9b95b4e455aa15b79bf5`。跑绿 PR 头与合并提交的 Git tree 都是 `e473a6e36fe8afc5b26c233613cc4e30f982322c`，文件树逐字节一致；受保护主分支没有被绕过。
- annotated tag `v1.1.16` 精确指向该合并提交；tag object 为 `2013b0cd7168f9bc982a73b2caabb34e90ce421c`。

## 正式资产与公开更新

- Release workflow run [33263384953](https://github.com/Longyuyeee/long_Decompress/actions/runs/33263384953) 精确检出合并提交并于 21 分 49 秒后成功；正式 Release [Long解压 v1.1.16](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.16) 非草稿、非预发布，目标提交正确。
- `latest.json`：1,317 B，SHA-256 `39783534C4562BDA68C7C67D0DD0261BA35A8AC0A455C26FB82D0E1F62104081`；版本为 `1.1.16`，Windows x86_64 URL 精确指向本标签 updater ZIP。
- NSIS：15,598,022 B，SHA-256 `BC83EA3554EC2631B453CC6EED4D2496C500F932EE2DFDEAA3F7109EB2A29F64`。7-Zip 26.02 完整性为 22 文件、`Everything is Ok`；8 项视频运行时共 24,631,334 B、差异 0。
- updater ZIP：15,598,182 B，SHA-256 `64F24CD9C2A97453F679ADE10316BEBAB5BCA8DB9321A57C815F8EBF3FDB05C6`；解包唯一 EXE 与独立 NSIS 大小和 SHA-256 完全一致。
- `.sig`：428 B，SHA-256 `10F3568220DA8F6015841854A5D6A4D7F1B774891D4BBF6BE4F95E3098B1F261`；内容与 `latest.json` Windows x86_64 签名逐字一致。
- 正式包内及最终安装主程序均为 28,864,512 B、ProductVersion `1.1.16`、SHA-256 `82D91158C1F676FB4CC0701F1F8BBB673ACF11D9A0C7A851D345AEC8F14C1729`，证明实际安装字节与公开 NSIS 一致。正式安装唯一 Shell DLL 为 253,952 B、SHA-256 `A83FFB94B583762181826A3AF31CEE0F6EF382823E37FB43477D4974B6D139A0`；不同 runner 的 Release 字节差异已分别记录，不混用候选哈希。
- 真实公开 `v1.1.15 → v1.1.16` 更新 24/24：签名更新 UI 交接、旧进程退出、目标安装、原路径、自动重启、经典菜单 4 根/17 条子命令/4 条快捷命令、自动启动、两处用户数据、唯一版本 DLL 和无 identity package 均通过。证据：`test-results/public-update-validation/20260830-010059/result.json`。
- 最终机器安装公开 v1.1.16 于 `E:\long\Long解压`，Run 值与偏好一致，运行中应用进程 0。

## 发布后门禁加固

- 修正后的安装生命周期先生成稳定字节快照，恢复时以验证过的同卷暂存目录替换目标，并对目标重建竞态显式失败；自动启动原值在成功与异常恢复两条路径均恢复并核对。
- 媒体架构门禁锁定上述三方快照、原子恢复、竞态拒绝及两条 Run 恢复调用；PowerShell AST 和 17 文件媒体架构门禁通过。
- 用公开 v1.1.16 NSIS 做 `v1.1.16 → v1.1.16 → 卸载 → v1.1.16` 实机生命周期 49/49，通过用户数据精确指纹 `5FA8138FD1E50D04A089F02A7EDB2143B9E492C0D0D580A1B7868210B211D7E5` / `7A939F55BFBEE291E84C9833BDC7BA60C80CD4E00B234686767456C2FDD36BDF`、菜单和已开启 Run 值恢复。结构化证据：`test-results/installed-release-validation/20260830-011312/result.json`。

v1.1.16 发布阶段至此关闭。下一阶段从总路线中的后续节点继续，不再把 Windows N 或已完成的视频发布工作重复列为当前阻塞项。
