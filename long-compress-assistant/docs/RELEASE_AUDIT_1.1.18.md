# Long解压 v1.1.18 发布审计

审计日期：2026-09-01（Asia/Shanghai）

状态：**PR 候选安装矩阵已通过；测试工具纠偏待推送并以新 head 重新取最终候选，尚未创建公开 Release。**

## 结论

特殊压缩导航纠偏、八处版本身份、PR head `252ae03` 五项 CI 和该 head 的正式 NSIS 六组安装矩阵已经通过。真实测试暴露并纠正了 Node 25 崩溃、短视频取消竞态、WebView 临时目录占用和桌面窗口干扰；这些测试工具修正推送后必须重新取得新 head 的正式 NSIS 并复验，之后才允许合并、打标签和公开更新。在全部发布证据闭合前，公开稳定版仍为 `v1.1.17`。

## 需求对齐

| 范围 | 实际结果 | 判定 |
| --- | --- | --- |
| 压缩中心 | 仅保留归档文件压缩，不导入媒体工作区 | 已完成 |
| 特殊压缩 | 左侧单一入口，页面内选择图片/视频/PDF | 已完成 |
| 统一底层 | `compression/{archive,image,video,pdf}`、任务/历史/事务不分叉 | 已完成 |
| 功能回归 | 单元、架构、生产构建和三类隔离桌面门禁 | 已完成 |
| 版本身份与正式 NSIS | 八处 `1.1.18` 已完成；head `252ae03` 的干净 CI NSIS 身份/完整性已核验 | 已完成（需在最终 head 重取） |
| 安装态 | `v1.1.17 → 候选 → 卸载 → v1.1.17` 与六组真实矩阵 64/64 | 已完成（需在最终 head 复验） |
| 公开更新 | 合并、标签、Release 四资产回下载与应用内更新 | 待完成 |

## 已完成证据

- 功能提交 `81c2649` 已推送远端分支；完整预期—实际—修正记录见 [SPECIAL_COMPRESSION_NAVIGATION_AUDIT.md](SPECIAL_COMPRESSION_NAVIGATION_AUDIT.md)。
- Node.js `v24.19.0` 下 270/270 单元、类型、生产构建及 17 文件媒体架构门禁通过。
- 隔离 Windows Release 图片、PDF、视频聚焦门禁全部通过；视频长/大文件前置矩阵同时通过。
- PR #109 最新已完成 run `33457637334` 锁定 head `252ae03a7fae2a473ccf16c5324aa34c3b054cb4`，Browser、Frontend、Rust/Shell、Windows desktop、Windows installer 五项全绿。
- 该 run 的 `windows-nsis-installer` 为 19,317,594 B，SHA-256 `FE59CD76B96D46E5DD7E829B60325A7EB5120F622A49F1DC739FFB336BF144A1`；7-Zip 26.02 识别为 NSIS-3 Unicode，32 文件、展开 69,639,600 B，完整性为 `Everything is Ok`。
- 包内正式无测试桥主程序为 29,389,312 B，ProductVersion/FileVersion 均为 `1.1.18`，SHA-256 `1B8F8064251E635E861AFFDF1147FCF8412F2B45528FFA5B4A1D3DB6F48D1327`。唯一 DLL 为 `long_compress_shell_extension_1_1_18.dll`，253,952 B，SHA-256 `6472456C8971FAB383600AE704E9500B36D84195CD6093CAACABED85E1C02C0E`。这些 CI 实物事实取代本机编译尺寸/哈希，后者不再作为候选身份。
- 前端 270/270、生产构建、npm 生产依赖 0 漏洞；Rust 主库 377 通过/10 条按声明忽略且所有集成目标无失败，严格 Clippy 通过；Shell 5/5 与严格 Clippy 通过。
- 本机 Tauri 已完成生产编译，但 NSIS 在调用 `makensis.exe` 时以 `系统找不到指定的文件 (os error 2)` 失败。该失败不冒充候选包；与既有发布口径一致，精确 NSIS 必须由干净 GitHub Windows Runner 构建并回下载验证。
- Node 25.2.1 在同一正式候选的 PDF 安装工作区稳定以 `0xC0000409` 崩溃；Node 24.14.0 对同一矩阵通过。因此 package engine 与安装发布前置门禁只接纳 20/22/24 LTS major，显式拒绝 Node 25，而非放宽产品断言。
- 视频取消首轮使用 114,842,332 B、32 秒输入时，实际压缩可在测试观察 `compressing` 前完成。现改用现有 10 分钟、30,163,318 B 输入专测取消，保留 114,842,332 B 输入专测完成；两条源路径、输出和 SHA-256 独立，真实 FFmpeg 启动/退出、无输出、暂存为 0、源哈希不变均通过。
- 测试窗口曾干扰当前桌面；直接最小化会使 WebView2 停顿，因此改为将窗口移到虚拟桌面之外但保持渲染，并停止点击会启动系统播放器/PDF 阅读器的入口。真实成品、格式/页数和入口 enabled 状态仍验证。最终同轮安装生命周期证据 `test-results/installed-release-validation/20260901-101618/result.json` 为 64/64、失败 0；六组矩阵全部 exit 0，候选卸载、公开 `1.1.17`、用户数据指纹、经典菜单和自启动均恢复，相关进程为 0。

## 当前唯一接续点

- 接续标识固定为分支 `codex/special-compression-navigation` 和 PR #109。先提交并推送本轮测试工具及审计修正，等待新 head 五项 CI 全绿。
- 只使用新 head 生成的 `windows-nsis-installer`，重新核验安装器/包内 EXE/DLL 身份，并在 Node 24 LTS 下从头运行同一六组正式安装生命周期；不得用 head `252ae03` 的通过结果替代最终 head。
- 最终候选再次 64/64 后才能合并、创建 annotated `v1.1.18` 标签、等待 Release workflow、回下载四项资产并运行真实公开更新。README 当前的候选措辞必须保留到公开更新完成。
- Windows N 暂不保证支持且不再阻塞本版本；不得伪造 Windows N 实机通过。换机命令、安装矩阵参数和发布顺序见 [DEVELOPMENT_HANDOFF.md](DEVELOPMENT_HANDOFF.md) 顶部。

## 待关闭门禁

- [x] 八处版本源及唯一 `1.1.18` Shell Extension 身份。
- [x] 本地发布身份、前端/Rust/Shell 回归和正式无测试桥主程序。
- [x] head `252ae03` 干净 CI 正式无测试桥 NSIS及安装生命周期；最终工具修正 head 仍需重取并复验。
- [ ] 受保护主线 PR CI、annotated `v1.1.18` 标签与 Release workflow。
- [ ] 四项公开资产回下载、哈希/签名/包内身份和真实 `v1.1.17 → v1.1.18` 应用内更新。
