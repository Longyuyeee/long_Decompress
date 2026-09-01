# Long解压 v1.2.1 发布审计

> 审计日期：2026-09-01
> 状态：正式发布关闭

## 真实差异

`v1.2.0` 已按 `c4f1549` 发布，Release workflow `33482500232` 成功，四项公开资产版本、URL、签名、包内安装器和哈希一致。但真实 `v1.1.19 → v1.2.0` 更新在第 19 项检查失败：新应用首次启动时菜单为 4 根/17 命令/4 快捷动作，退出后目录和目录空白处入口被旧 updater 尾部清理删除。应用重新启动后会自动修复且退出后保持，证明根因是更新时序竞态，不是注册实现缺失。

## 修正

- Windows 新进程启动后在 2 秒和 6 秒对“原本已存在”的菜单执行延迟复核；菜单从未启用时保持关闭。
- 公开更新脚本首次看到完整菜单后继续观察 8 秒，覆盖两次复核与 updater 清理，再停止应用并做最终保持检查。
- Rust 合同测试固定产品延迟复核与发布脚本稳定观察，不允许以后退回瞬时成功断言。

## 发布门禁

| 门禁 | 当前结果 |
| --- | --- |
| 八处版本身份与唯一 Shell DLL | `1.2.1` 一致；`long_compress_shell_extension_1_2_1.dll` 唯一 |
| 更新菜单合同与严格 Clippy | 通过 |
| 完整前端 / Rust / Shell | 前端 276/276；Chromium 按 CI 单 worker 10/10；Rust 主库 380/380、主程序 1/1、全目标无失败；Shell 5/5；严格 Clippy 通过 |
| 无测试桥 NSIS 与安装生命周期 | `Long解压_1.2.1_x64-setup.exe` 已生成；真实 `v1.2.0 → v1.2.1 → 卸载 → v1.2.0` 50/50、失败 0 |
| 真实公开 `v1.2.0 → v1.2.1` 更新 | 25/25、失败 0；首次完整、8 秒清理窗口后及应用退出后三次均为 4 根/17 命令/4 快捷动作 |

真实公开更新已经覆盖竞态发生窗口并全部通过，本竞态正式关闭。

本机首次把 Chromium 8 workers 与 Rust Release 并行运行时，两个随机懒加载路由超过原 30 秒；第二次 8 workers 又有三个不同页面超时。截图均显示导航按钮已经激活但页面仍停留在解压中心，没有业务异常。未增加重试、未延长超时；按仓库 CI 的单 worker 配置从头运行后 10/10，因此将差异归类为本机并行开发服务器资源竞争，而不是接受失败或修改产品断言。

本地候选 NSIS 为 19,393,909 B / SHA-256 `B8294CB4FC7CC7257AFD544358B91B943BBB69BC412C6F81A4EC231D0CE85217`；主程序为 29,687,296 B / SHA-256 `294A676E02861A207ACFA786C7F6A921D012BE19CF60A0E9B6A98E7E537E3337`，ProductVersion `1.2.1`；唯一 Shell DLL 为 246,784 B / SHA-256 `AC6E9E3CE8E36A02749ACE0E4A72A01C26B10D04122AF5B288A1ED3C3CCF110B`。安装生命周期原始证据位于被忽略的 `test-results/installed-release-validation/20260901-161557/result.json`，结束后已恢复公开 `v1.2.0`，相关应用进程为 0。

## 正式发布关闭

- PR [#113](https://github.com/Longyuyeee/long_Decompress/pull/113) 五项 CI 全绿后合入 `master@7b05bdc659aa246f6397e8aa362b8ec49f7c7bf9`；annotated `v1.2.1` 标签精确指向该提交。
- PR CI run [33486409060](https://github.com/Longyuyeee/long_Decompress/actions/runs/33486409060) 与 Release run [33487650509](https://github.com/Longyuyeee/long_Decompress/actions/runs/33487650509) 均成功。
- 公开 `latest.json` 为 952 B / SHA-256 `E0684D176B4428A4E0A5FCEC4CA57EC6586F921CA8FDF1028FA95F4CAAFD8967`；NSIS 为 19,369,135 B / SHA-256 `62335C948538CCC1A6BC18C240B9EBE0991DEC68E08E5C741928DC31CFA5DCC7`；updater ZIP 为 19,369,293 B / SHA-256 `6E0383D29FF3F8800438C32749067D42834C956F17C79290538F9A9F5E64403C`；签名为 428 B / SHA-256 `4B049F12F098634FC5003B8BC929186666A2FEC3462B2507B36418319958BB58`。manifest 的版本、下载 URL 和签名与公开资产一致。
- 真实公开更新证据位于被忽略的 `test-results/public-update-validation/20260901-165644/result.json`：`v1.2.0 → v1.2.1` 共 25/25、失败 0，安装路径、用户数据、自启动和唯一 `long_compress_shell_extension_1_2_1.dll` 保持。
- 当前机器最终安装公开 `1.2.1` 于 `E:\Long\Long解压`，相关 Long解压进程为 0。下一次开发必须从最新 `master`/公开 `v1.2.1` 开始，不再从候选分支、本地候选安装包或被忽略的测试证据目录接续。
