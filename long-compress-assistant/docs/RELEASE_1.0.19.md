# Long解压 1.0.19

本版本是 v1.0.18 应用内更新真实验收后产生的补丁版本，不覆盖重发已经公开的 v1.0.18。

## 修复

- Tauri updater 使用被动 NSIS 安装时默认重新启动应用，不再停留在“已经安装但没有重新打开”的状态。
- 被动维护和自动化安装增加 `/NR` 选项，可明确禁止重启，避免影响覆盖安装与卸载回归。
- Windows 11 native 菜单只有在签名身份包资源存在且稀疏包注册成功时才视为有效；
  无签名或遗留状态会在启动同步中降级到传统菜单。

## 公开更新验收工具

新增 `test:public-update`，只允许从调用者指定的已安装旧版本升级到指定正式版本：

- 读取公开 `latest.json` 并验证目标版本、Windows 下载地址和签名；
- 备份并计算两个用户数据目录的 SHA-256 指纹；
- 将已安装应用作为独立进程启动，通过页面级 CDP 连接真实 Tauri/WebView2 设置页检查并安装更新；
- 自动化进程不再成为应用的父作业，避免 WebDriver 的 Windows Job Object 误杀 updater 或重启后的应用；
- 验证注册表与主程序版本、原安装目录、自动重启、数据指纹、单一版本化 Shell DLL、
  传统右键菜单和无签名 MSIX 残留；
- 失败时保留用户数据备份和 JSON/截图/驱动日志证据。

## 候选版验证

- 公开 v1.0.18 → v1.0.19 候选覆盖安装、用户数据保持、传统菜单、静默卸载、
  注册表清理和 v1.0.18 恢复：41 项通过。
- 严格全格式真实 Windows Tauri 桌面矩阵通过。
- 前端 27 个文件、170 项测试通过；覆盖率为 72.02% 行、76.49% 分支。
- Rust release 主程序 119 项及全部集成矩阵通过，Clippy `-D warnings` 通过。
- 生产依赖审计为 0 个漏洞，版本一致性与 NSIS 生产打包通过。

## 正式发布验收

v1.0.19 正式资产已由 Release 工作流生成，安装器 SHA-256 为
`8DEEEAF6EEFD3C76F5C9CC1B17DEE50085CF823B901F65D890FAC4AAB8322737`。

公开 v1.0.18 → v1.0.19 的真实应用内更新最终 18 项全部通过：

- 设置页发现 v1.0.19，签名更新包下载并安装成功；
- 应用自动重启，版本与原安装目录正确；
- 两个用户数据目录 SHA-256 指纹完全不变；
- 传统右键菜单正常，仅保留 `long_compress_shell_extension_1_0_19.dll`；
- 不存在无签名 Windows 11 identity MSIX 残留。

正式验收记录见 [Issue #27](https://github.com/Longyuyeee/long_Decompress/issues/27)，
v1.0.18 未重启问题由 [Issue #25](https://github.com/Longyuyeee/long_Decompress/issues/25) 收口。
