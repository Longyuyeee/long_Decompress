# Long解压 v1.1.14 发布审计

审计日期：2026-08-26

## 结论

归档工作区 A-01 至 A-06 已完成，`v1.1.14` 已正式发布。版本身份、生产构建、单元与 Rust Release 测试、真实覆盖安装、卸载恢复、四项公开资产、updater 签名、公开 `latest.json` 对账及本机 `1.1.13 → 1.1.14` 应用内更新均通过。

## 需求对齐

| 范围 | 实际结果 | 判定 |
| --- | --- | --- |
| 文件管理器式浏览 | 目录树、直属内容区、导航历史、搜索、筛选、键盘与拖入打开 | 已完成 |
| 对象化右键 | 文件、目录、单项和多选使用对应操作，不破坏既有选择 | 已完成 |
| 默认应用安全打开 | 隔离缓存、预算、来源标记；危险内容默认取消并二次确认 | 已完成 |
| 有界预览 | ZIP/TAR 图片和文本受类型、扫描、体积及尺寸上限保护 | 已完成 |
| 嵌套归档 | 最多三层、逐层密码、循环/深度/临时预算保护和状态恢复 | 已完成 |
| 可取消读取 | 唯一请求 ID 到 Rust 服务；慢遍历协作停止，外部引擎终止等待 | 已完成 |
| 能力单一来源 | 格式与预览策略由后端报告，前端不维护第二份扩展名真相 | 已完成 |
| Windows 11 第一层菜单 | 无签名身份无法可靠发布；经典菜单完整保留 | 边界明确 |

## 候选真实证据

- A-06 正式安装态矩阵完整通过，详情见 [ARCHIVE_WORKSPACE_A06_AUDIT.md](ARCHIVE_WORKSPACE_A06_AUDIT.md)。
- 本次正式 `1.1.14` 版本源已重新构建，没有复用 A-06 的临时同号候选。前端类型检查、40 个测试文件 234 项、Rust Release 全目标测试和 Clippy 零警告均通过；真实加密 RAR 用例在本次 Rust 测试中实际执行。
- 本地正式 NSIS 包含 14 个载荷文件，7-Zip 完整性测试通过；大小为 7,785,440 字节，SHA-256 为 `B085A5D92319BB9095AA87FD933030FF1DE04AA87513C0BDAFF2B4B54543731B`。主程序 ProductName 为 `Long解压`，FileVersion/ProductVersion 均为 `1.1.14`，SHA-256 为 `F70B479B7DEEC83A40182AFC3EBDE8B80DECF37B6568D9910A80726C3493F5E6`。
- 真实 `1.1.13 → 1.1.14 → 卸载 → 1.1.13` 门禁通过：安装后 EXE 与候选字节一致，ZIP、加密 7Z/RAR、TAR/TAR.GZ、中文八层路径、三层嵌套、危险/损坏场景和 18 万条目取消重新通过；取消耗时 83 ms。最终恢复原菜单与两处用户数据指纹，证据位于本机 `test-results/installed-release-validation/20260826-161756/result.json`。
- 首次正式候选复验在产品页面启动前遇到 WebView2 `DevToolsActivePort` 竞态。安装门禁已改为最多三次、每次独立用户数据目录并清理残留应用进程的受控重试；修正后完整复验通过，不用手工重跑掩盖环境竞态。
- 从旧电脑复制的本地 updater 密码 DPAPI 文件无法在当前用户/机器状态解密，因此本机没有伪造 updater 签名。updater ZIP、签名与 `latest.json` 只允许由已配置 GitHub Actions Secrets 的干净 runner 生成并在发布后公开复核。

## 正式资产门禁

- npm、Tauri、主程序、Shell Extension、两个 Cargo 锁文件和唯一版本化 DLL 必须统一为 `1.1.14`。
- GitHub Actions 必须生成 NSIS 安装包、updater ZIP、独立签名和 `latest.json` 四项资产。
- `latest.json` 版本、下载 URL 和签名必须与 `v1.1.14` 同一 Release 对账。
- 公开安装包和 updater ZIP 必须重新下载并通过完整性测试；安装升级必须保留用户数据并注册完整经典菜单。

## 发布后审计

- 正式标签固定在提交 `cfc58ec9a14dc8ccb3f0e026986786af5693b6cc`；GitHub Actions Release 运行 `32947440127` 的版本身份、前端检查、Rust Release 测试、Tauri 构建、无证书身份包边界和 updater 清单发布全部通过。
- Release 地址：[Long解压 v1.1.14](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.14)。公开资产共四项：`latest.json` 811 字节，SHA-256 `31FDE77AD58B4505554966158BB4A808F64633FAC387DE01635264F672B3FC66`；NSIS 安装包 7,722,311 字节，SHA-256 `05C2B1D20611F1A063D39EA79623E711CCD3A93F0FB9C0090CF762B6A8D6A427`；updater ZIP 7,722,471 字节，SHA-256 `99697DE09072AB8A88B7433F5761D21725DB216041AC44C5EE051383A6861306`；签名文件 428 字节，SHA-256 `7B8A36D4033D527796BDF3C55F2C3BF95BE0B6341A8021C4CF2552ACFDAD933E`。
- 从公开 Release 重新下载的 NSIS 与 updater ZIP 均通过内置 7-Zip 26.02 完整性检查；`latest.json` 的版本、URL 与独立签名逐项一致。
- 本机通过应用设置界面使用公开 `latest.json` 完成真实 `1.1.13 → 1.1.14` 更新。共 24 项检查通过：安装位置和两处用户数据指纹保持不变，应用自动重启，经典菜单 17 条子命令与 4 条快捷命令完整，目标 Shell Extension 唯一，且无证书版本不包含 MSIX 身份包。证据位于 `test-results/public-update-validation/20260826-164854/result.json`。
- 首轮公开更新已经成功安装，但严格数据指纹门禁发现测试恢复态的设置文件记录 `autoStart=true`、Windows Run 项却不存在；设置页同步真实系统状态后造成单字段变化。验收脚本现增加更新前后“持久化偏好与 Windows 注册必须一致”的明确门禁；修复基线后完整重跑通过，没有通过忽略字段来放宽用户数据保护。
