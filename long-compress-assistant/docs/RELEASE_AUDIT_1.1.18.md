# Long解压 v1.1.18 发布审计

审计日期：2026-09-01（Asia/Shanghai）

状态：**已完成并公开发布。**

## 结论

`v1.1.18` 已在受保护主线合并并公开发布。特殊压缩导航纠偏、八处版本身份、精确候选六组安装矩阵、Release 四资产回下载和真实 `v1.1.17 → v1.1.18` 应用内更新全部闭合；当前公开稳定版为 `v1.1.18`。

## 需求对齐

| 范围 | 实际结果 | 判定 |
| --- | --- | --- |
| 压缩中心 | 仅保留归档文件压缩，不导入媒体工作区 | 已完成 |
| 特殊压缩 | 左侧单一入口，页面内选择图片/视频/PDF | 已完成 |
| 统一底层 | `compression/{archive,image,video,pdf}`、任务/历史/事务不分叉 | 已完成 |
| 功能回归 | 单元、架构、生产构建和三类隔离桌面门禁 | 已完成 |
| 版本身份与正式 NSIS | 八处 `1.1.18` 与 head `045d9d9` 的干净 CI NSIS 身份/完整性 | 已完成 |
| 安装态 | 精确最终候选的 `v1.1.17 → 候选 → 卸载 → v1.1.17` 与六组真实矩阵 64/64 | 已完成 |
| 公开更新 | 合并、annotated 标签、Release 四资产回下载与应用内更新 24/24 | 已完成 |

## 已完成证据

- 功能提交 `81c2649` 已推送远端分支；完整预期—实际—修正记录见 [SPECIAL_COMPRESSION_NAVIGATION_AUDIT.md](SPECIAL_COMPRESSION_NAVIGATION_AUDIT.md)。
- Node.js `v24.19.0` 下 270/270 单元、类型、生产构建及 17 文件媒体架构门禁通过。
- 隔离 Windows Release 图片、PDF、视频聚焦门禁全部通过；视频长/大文件前置矩阵同时通过。
- PR #109 CI run `33462432485` 锁定纠偏后的 head `045d9d98113175294fa2528be2cf3bd25367fb7a`，Browser、Frontend、Rust/Shell、Windows desktop、Windows installer 五项全绿。
- 该 run 的 `windows-nsis-installer` 为 19,315,298 B，SHA-256 `1DA61456456E4026F5327417B0630EE50A0F988956AA47AA98C263C14843B852`；7-Zip 26.02 识别为 NSIS-3 Unicode，32 文件、展开 69,639,600 B，完整性为 `Everything is Ok`。
- 包内正式无测试桥主程序为 29,389,312 B，ProductVersion/FileVersion 均为 `1.1.18`，SHA-256 `A546CBCDF6F70C0DB0C0C444EDE35FB76D9E26C0CB63178363E58C5C20C0F6BB`。唯一 DLL 为 `long_compress_shell_extension_1_1_18.dll`，253,952 B，SHA-256 `49E619AF283F03577AE05B804BABACEC3BB9B2F8C230DFF11C42AA936B45AAA9`。这些 CI 实物事实取代本机编译尺寸/哈希，后者不再作为候选身份。
- 前端 270/270、生产构建、npm 生产依赖 0 漏洞；Rust 主库 377 通过/10 条按声明忽略且所有集成目标无失败，严格 Clippy 通过；Shell 5/5 与严格 Clippy 通过。
- 本机 Tauri 已完成生产编译，但 NSIS 在调用 `makensis.exe` 时以 `系统找不到指定的文件 (os error 2)` 失败。该失败不冒充候选包；与既有发布口径一致，精确 NSIS 必须由干净 GitHub Windows Runner 构建并回下载验证。
- Node 25.2.1 在同一正式候选的 PDF 安装工作区稳定以 `0xC0000409` 崩溃；Node 24.14.0 对同一矩阵通过。因此 package engine 与安装发布前置门禁只接纳 20/22/24 LTS major，显式拒绝 Node 25，而非放宽产品断言。
- 视频取消首轮使用 114,842,332 B、32 秒输入时，实际压缩可在测试观察 `compressing` 前完成。现改用现有 10 分钟、30,163,318 B 输入专测取消，保留 114,842,332 B 输入专测完成；两条源路径、输出和 SHA-256 独立，真实 FFmpeg 启动/退出、无输出、暂存为 0、源哈希不变均通过。
- 测试窗口曾干扰当前桌面；直接最小化会使 WebView2 停顿，因此改为将窗口移到虚拟桌面之外但保持渲染，并停止点击会启动系统播放器/PDF 阅读器的入口。真实成品、格式/页数和入口 enabled 状态仍验证。精确最终候选的同轮安装生命周期证据 `test-results/installed-release-validation/20260901-104120/result.json` 为 64/64、失败 0；六组矩阵全部 exit 0，候选卸载、公开 `1.1.17`、用户数据指纹、经典菜单和自启动均恢复，相关进程为 0。

## 公开发布关闭证据

- PR #109 以 merge commit `0cdb53a239265ab2b00514029d293f44f120c15e` 合入 `master`；合并前最终文档 head `3e86b9e` 的 CI run `33463907564` 五项全绿，且相对已实测候选只修改三份 Markdown。合并后的主线 CI run `33464673807` 也成功。
- annotated `v1.1.18` 标签指向该 merge commit；Release workflow run `33464702924` 成功，公开 Release 非 draft、非 prerelease，发布时间 `2026-09-01T03:22:44Z`。
- 回下载四资产与 GitHub digest 一致：`latest.json` 1,075 B / `997EB09C…9EECF`；NSIS 19,316,096 B / `A7FBC962…E05EC`；updater zip 19,316,256 B / `FB60A395…45551E`；签名 428 B / `85039ADC…08D777`。
- `latest.json` 版本为 `1.1.18`、URL 指向公开 updater zip，内嵌签名与独立 `.sig` 完全一致。updater zip 完整且仅含一个 19,316,096 B NSIS，其 SHA-256 与独立 NSIS 完全相同。
- 公开 NSIS 完整性通过；包内主程序 29,389,312 B，ProductVersion/FileVersion 为 `1.1.18`，SHA-256 `09CF09758703542A480F7877292B1DD50A2AA65EC6F6DD3F236F9CD0617F9E92`。唯一 Shell DLL 为 `long_compress_shell_extension_1_1_18.dll`，253,952 B，SHA-256 `C97CE1312CB63F706898A6534D581EAC19DE06B0081D33573CEB9696532520D4`。
- 真实公开更新证据 `test-results/public-update-validation/20260901-112516/result.json` 为 24/24、失败 0：从已安装 `1.1.17` 读取公开 `latest.json`，经真实桌面更新对话框交接签名资产并升级到 `1.1.18`；安装路径、两处用户数据指纹、经典菜单 4 根/17 命令/4 快捷项和自启动保持，旧 Shell DLL 被替换且无未签名 Windows 11 identity package。最终安装版本为 `1.1.18`，应用进程为 0。
- 更新 UI 测试也改为在虚拟桌面之外运行，避免自动化抢占当前桌面；真实签名下载、交接、应用退出、更新安装和重启断言均保留。

## 后续接续点

`v1.1.18` 已收口，不再从 PR #109 或候选安装矩阵接续。下一开发步骤应从 `master` 当前公开基线重新审计路线图中的未完成目标，立项后继续遵守“真实代码—原始需求—真实预期/实际差异—审计—推送”的闭环。
- Windows N 暂不保证支持且不再阻塞本版本；不得伪造 Windows N 实机通过。换机命令、安装矩阵参数和发布顺序见 [DEVELOPMENT_HANDOFF.md](DEVELOPMENT_HANDOFF.md) 顶部。

## 待关闭门禁

- [x] 八处版本源及唯一 `1.1.18` Shell Extension 身份。
- [x] 本地发布身份、前端/Rust/Shell 回归和正式无测试桥主程序。
- [x] head `045d9d9` 干净 CI 正式无测试桥 NSIS及 64/64 安装生命周期。
- [x] 受保护主线 PR CI、annotated `v1.1.18` 标签与 Release workflow。
- [x] 四项公开资产回下载、哈希/签名/包内身份和真实 `v1.1.17 → v1.1.18` 应用内更新。
