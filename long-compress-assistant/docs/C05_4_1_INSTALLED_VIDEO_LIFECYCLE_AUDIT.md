# C-05.4.1 正式安装版视频生命周期审计

日期：2026-08-29

## 结论

**C-05.4.1 已完成，C-05 尚未关闭。** 提交 `71a95729a17352c2c711b2f34764739175954099` 的干净 GitHub Windows Runner 候选完成包级校验、公开 `v1.1.15` 覆盖安装、候选主程序字节对账、安装态视频运行时、可见 UI 取消/完成/默认应用/跨完整重启历史、候选卸载、用户数据保持及公开 `v1.1.15` 恢复。安装生命周期 50/50、视频工作区 20/20，差异为零。

本节点没有提升版本、创建标签或发布 Release。下一接续点严格为同一候选在真实 Windows N 上的 Media Feature Pack 安装前后证据；完成前不得进入 `v1.1.16` 正式发布。

## 候选身份与 CI

- GitHub Actions run：`33258733949`，Browser、Frontend、Windows Release desktop、Rust/Shell Extension 和 NSIS 全部通过；
- NSIS：15,604,389 B，SHA-256 `C4FF2374AA033A6EE892EF8BF6313CDB2B987BDD3BA244930C45AF081391D718`；
- 解包主程序：28,853,760 B，SHA-256 `0B443647D39B817993794FA3E6F6D52600515B06700030F565CB4DBF5D795EF0`；
- 安装目录主程序与解包主程序大小、SHA-256 完全一致；
- NSIS 内 8 项冻结视频运行时共 24,631,334 B，归档完整性、大小和 SHA-256 差异为零。

本机 NSIS 的 32 位 `makensis.exe` 因 `0xC0000135` 缺失运行库无法启动，因此没有把本地失败产物冒充候选；改用精确 head SHA 的干净 CI Runner 生成，并回下载做独立包级复核。

## 预期—实际—修正

| 场景 | 预期 | 首次实际 | 修正 | 最终实际 |
| --- | --- | --- | --- | --- |
| 公开基线 | 当前安装为公开 `v1.1.15` | 注册表实际为 `v1.1.13` | 回下载公开 NSIS，并按 8,658,170 B / `DBFF77AE…DD2C51` 对账后安装；核对安装位置和两处数据指纹 | `v1.1.15`，位置不变，数据指纹不变，无进程 |
| 安装资源根 | CLI 与 UI 使用同一 `resources/video-engine` | 首个候选 CLI 通过，UI 错查安装根下 `video-engine`，返回 `VIDEO_ENGINE_RESOURCE_MISSING` | 新增唯一 `bundled_resource_root`，四个生产入口共同保留 `resources/` 前缀；架构门锁定 | 修正候选安装态运行时和 UI 均通过 |
| Windows N 候选锁 | 后续实机脚本只接受本次正式候选 | 脚本仍读取 C-01 历史候选主程序身份 | 保留 C-01 历史测量，新增 `c05InstalledCandidate`，脚本和独立验收器改读新身份 | 后续只接受 `71a9572` 的 28,853,760 B 主程序及精确哈希 |

## 正式安装生命周期结果

结构化主证据位于本机忽略目录 `test-results/installed-release-validation/20260829-230820/result.json`：

- 覆盖安装保持 `E:\long\Long解压`，候选主程序字节一致，Shell Extension 版本唯一，经典右键菜单 4 根/17 条命令/4 条快捷命令完整；
- 两处用户数据在覆盖安装前后指纹一致；
- 安装态生产预检、真实 H.264/AAC 软件转码、独立 ffprobe、隔离缺失资源和替换资源拒绝全部通过；
- 114,842,332 B AVI 在可见 UI 中取消后，已观察的产品 FFmpeg 退出，最终输出不存在、暂存为零、源 SHA-256 不变，并落一条取消历史；
- 完成输出 8,410,052 B，SHA-256 `53F6281593C8584D4FBAC7CC760208D7A8ADBEC0A2FA80DD0F460337F4215F45`，为 MP4/H.264、1280×720、32.000 秒、无虚构音轨；
- 默认应用收到输出；原生应用完全退出并以新进程、新 WebView profile 重启后，一条取消和一条完成历史仍精确保留；
- 候选卸载后无产品 EXE、卸载器或安装器所属菜单键残留，运行期用户数据保持；随后恢复公开 `v1.1.15`、原安装位置、菜单模式/目标和原始用户数据，当前无应用进程。

## 回归审计

- 视频引擎聚焦测试：5/5；
- Rust 全特性串行：库 362 通过/4 按既有外部条件忽略，全部集成目标及文档测试无失败；
- 全 target/全 feature Clippy `-D warnings`：通过；
- 类型检查、Vite 生产构建、媒体架构、媒体依赖真实复核、媒体发布门与版本身份：通过；
- 无测试桥 Release 视频工作区：20/20；
- 正式安装生命周期：50/50，其中正式安装视频工作区 20/20。

## 下一接续点

在真实 Windows N x64 上使用后续准备审计锁定的工具提交，由 `MissingMediaFeaturePack -CandidateInstaller` 阶段校验并安装本审计候选；安装 Media Feature Pack 并重启后，在同一证据目录运行 `MediaFeaturePackInstalled` 和独立验收器。当前主机 `EditionID=Professional`，不能替代或伪造该证据。完整跨机器手册见 [C05_4_2_WINDOWS_N_GATE_PREPARATION_AUDIT.md](C05_4_2_WINDOWS_N_GATE_PREPARATION_AUDIT.md)。
