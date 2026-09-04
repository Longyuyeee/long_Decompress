# Long解压 v1.2.9 发布审计

日期：2026-09-04

状态：正式公开，发布关闭

## 发布范围

- 对 v1.2.8 归档队列、配置来源与真实暂停控制进行发布后总审计。
- 复验真实 Windows 控制链路、停止清理、队列接续和 archive-flow。
- 统一版本身份、README、Release Notes、发布审计与开发交接。

## 预期—实际

| 门禁 | 预期 | 实际 |
| --- | --- | --- |
| 生产依赖安全审计 | npm 官方安全源无已知生产漏洞 | 0 漏洞 |
| 前端单元与覆盖率 | 全部通过 | 56 文件、325/325 |
| 浏览器界面 | Chromium 核心布局与交互无回归 | 13/13 |
| 集成与性能 | 常规集成和性能契约通过 | 6/6、17/17 |
| Rust Release | 主库及常规集成矩阵无失败 | 主库 389 通过、10 项专用环境测试明确忽略；其余通过 |
| 壳扩展 | 单元和严格 Clippy 通过 | 5/5；零告警 |
| 真实暂停控制 | 单项/全部暂停、恢复、停止与清理一致 | Windows Tauri/WebView2 通过 |
| 真实归档主流程 | 并发、遥测、串行、加密与回退保持正确 | archive-flow 通过 |

## 首轮桌面门禁说明

第一次直接运行桌面门禁时，现有候选前端未按 CI 注入 `VITE_DESKTOP_E2E=1`，导致测试桥不存在；这次运行没有进入功能断言。按正式 CI 顺序重建测试前端和 `custom-protocol,desktop-e2e` Release 二进制后，pause-control 与 archive-flow 均通过。该差异归类为本地测试构建前置条件，不归类为产品缺陷。

## 本机正式候选

正式候选已重新构建生产前端，静态检查确认不包含桌面 E2E bridge。未配置生产代码签名时，Windows 11 身份包按既有契约跳过，经典菜单与版本化 Shell 扩展继续随 NSIS 提供。

| 产物 | 字节 | SHA-256 |
| --- | ---: | --- |
| `Long解压_1.2.9_x64-setup.exe` | `19,432,334` | `4593B99291525B129B95796F82542D79DC05B7B2F39B63B1345ECEA37FA139FE` |
| `Long解压.exe` | `29,914,112` | `3143E74DA9D80874B14C4E4A8E105DF6969E5601F8ABE4E1F73708073646BA56` |
| `long_compress_shell_extension_1_2_9.dll` | `246,784` | `3A9F0E79FCDA2C16DD3F3CCEB050C6B730EA3BB031E001AD80E008C075AD0D03` |

本机产物只用于身份、可构建性和哈希审计；正式 updater 与公开资产由 annotated `v1.2.9` 标签触发的干净 GitHub Actions Runner 重新生成。

## 合并、CI 与正式发布

- [PR #122](https://github.com/Longyuyeee/long_Decompress/pull/122) 五项检查全部通过；CI run 为 [33842839787](https://github.com/Longyuyeee/long_Decompress/actions/runs/33842839787)。
- PR 以 merge commit `76479f3d50eddd322437884dc597885ca10553f5` 合入 `master`；annotated `v1.2.9` 标签精确指向该提交。
- [Release workflow 33843995836](https://github.com/Longyuyeee/long_Decompress/actions/runs/33843995836) 在 27 分 44 秒内完成，所有步骤通过。
- [v1.2.9 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.2.9) 为正式公开状态，不是草稿或预发布。

## 公开资产回下载对账

| 公开资产 | 字节 | SHA-256 |
| --- | ---: | --- |
| `latest.json` | `950` | `52D6707503954C06D7A26F973B0E39E123AEC583144C5C03F39FE911CF2D1172` |
| `Long-Decompress_1.2.9_x64-setup.exe` | `19,364,987` | `95D818B4304C886CA31EDDA700B04EB8A2DF702601562DE9CD28381E6D2C86E9` |
| `Long-Decompress_1.2.9_x64-setup.nsis.zip` | `19,365,145` | `9CC94984293B8ABBBEF2A3BCDF5AA625FAF291D2D75162B836E3B619025D807A` |
| `Long-Decompress_1.2.9_x64-setup.nsis.zip.sig` | `428` | `2B30A476901DCDB4FAB33AFE40B1942671548AFA95238288F2A891CE30B3ABF4` |

回下载后重新计算的四项 SHA-256 与 GitHub Release 元数据完全一致。`latest.json.version` 为 `1.2.9`，下载 URL 精确指向 v1.2.9 updater ZIP；清单内 428 字符签名与独立 `.sig` 文件逐字一致。

## 发布关闭与边界

- 版本身份、PR、五项 CI、合并、annotated tag、Release workflow、公开资产及 updater 清单/签名均已完成，v1.2.9 发布阶段关闭。
- 当前会话不是管理员；需要向 `LocalMachine\\TrustedPeople` 写入自签名证书的可逆 AppX 部署测试在权限预检处停止，未修改系统。未配置生产签名时正式构建按既有契约跳过 Windows 11 身份包。
- 本机不替换用户当前安装版本；安装/升级生命周期不在未执行时冒充通过。
