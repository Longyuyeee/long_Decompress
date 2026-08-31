# Long解压 v1.1.18 发布审计

审计日期：2026-09-01（Asia/Shanghai）

状态：**候选验证进行中，尚未创建公开 Release。**

## 结论

特殊压缩导航纠偏的代码、单元、生产构建、媒体架构与图片/视频/PDF 隔离桌面门禁已经关闭。版本身份、正式 NSIS、安装态功能、受保护主线 CI、标签、公开资产回下载和 `v1.1.17 → v1.1.18` 更新仍需逐项完成；在这些证据齐全前，公开稳定版仍为 `v1.1.17`。

## 需求对齐

| 范围 | 实际结果 | 判定 |
| --- | --- | --- |
| 压缩中心 | 仅保留归档文件压缩，不导入媒体工作区 | 已完成 |
| 特殊压缩 | 左侧单一入口，页面内选择图片/视频/PDF | 已完成 |
| 统一底层 | `compression/{archive,image,video,pdf}`、任务/历史/事务不分叉 | 已完成 |
| 功能回归 | 单元、架构、生产构建和三类隔离桌面门禁 | 已完成 |
| 版本身份与正式 NSIS | 八处 `1.1.18` 与新 Shell DLL已完成；干净 CI NSIS 待生成 | 进行中 |
| 安装态与公开更新 | 覆盖、卸载/恢复、三入口、资产回下载、公开更新 | 待完成 |

## 已完成证据

- 功能提交 `81c2649` 已推送远端分支；完整预期—实际—修正记录见 [SPECIAL_COMPRESSION_NAVIGATION_AUDIT.md](SPECIAL_COMPRESSION_NAVIGATION_AUDIT.md)。
- Node.js `v24.19.0` 下 270/270 单元、类型、生产构建及 17 文件媒体架构门禁通过。
- 隔离 Windows Release 图片、PDF、视频聚焦门禁全部通过；视频长/大文件前置矩阵同时通过。
- 八处版本源统一为 `1.1.18`；发布身份门禁通过。唯一 DLL 为 `long_compress_shell_extension_1_1_18.dll`，246,784 B，SHA-256 `B6A050AF53A12717B41B1EA6BE5D570A7781D0758BA1C05911C8A7EA4E863A31`。
- 正式无测试桥主程序编译成功：29,457,920 B，ProductVersion/FileVersion 均为 `1.1.18`，SHA-256 `FB1721C51AE7D00C941A8D8C90657F28BD09B6E7EB20FAD950D8EFA7BCE6DDFB`。
- 前端 270/270、生产构建、npm 生产依赖 0 漏洞；Rust 主库 377 通过/10 条按声明忽略且所有集成目标无失败，严格 Clippy 通过；Shell 5/5 与严格 Clippy 通过。
- 本机 Tauri 已完成生产编译，但 NSIS 在调用 `makensis.exe` 时以 `系统找不到指定的文件 (os error 2)` 失败。该失败不冒充候选包；与既有发布口径一致，精确 NSIS 必须由干净 GitHub Windows Runner 构建并回下载验证。

## 待关闭门禁

- [x] 八处版本源及唯一 `1.1.18` Shell Extension 身份。
- [x] 本地发布身份、前端/Rust/Shell 回归和正式无测试桥主程序。
- [ ] 干净 CI 正式无测试桥 NSIS。
- [ ] 精确候选的安装生命周期与图片/视频/PDF 工作区入口。
- [ ] 受保护主线 PR CI、annotated `v1.1.18` 标签与 Release workflow。
- [ ] 四项公开资产回下载、哈希/签名/包内身份和真实 `v1.1.17 → v1.1.18` 应用内更新。
