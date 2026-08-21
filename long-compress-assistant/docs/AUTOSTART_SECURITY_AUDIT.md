# Windows 开机自启动安全审计

审计日期：2026-08-22
当前基线：v1.1.12

## 结论

开机自动启动可以恢复，但必须保持为用户明确选择的可撤销能力。当前实现只在设置页开关的点击路径调用写入命令；应用启动、设置加载、更新、后台监控和只读状态检查均不会注册、迁移或修复启动项。

这项约束能消除 v1.1.7 曾出现的“每次启动重复写入持久化位置”行为，降低 Defender 启发式规则将未签名程序识别为异常持久化的概率。它不能保证任何未签名二进制永远不会被安全产品告警，因此发布前仍需在启用 Microsoft Defender 的干净 Windows 机器上扫描并复验。

## 注册与生命周期边界

| 场景 | 是否写入 Windows 启动项 | 预期行为 |
| --- | --- | --- |
| 打开应用或登录后启动 | 否 | 仅读取参数；`--autostart` 隐藏主窗口并驻留托盘 |
| 打开设置页 | 否 | 只读核对实际注册状态 |
| 用户点击开启 | 是，一次 | 写入当前用户 `Run` 值，命令固定为 `"<当前程序绝对路径>" --autostart` |
| 用户重复开启 | 否 | 精确值一致时保持幂等，不重复写入 |
| 用户点击关闭 | 是，一次 | 删除当前及旧品牌启动值 |
| 覆盖安装或更新 | 否 | 保留用户已经明确启用的当前值，只清理旧品牌值 |
| 卸载 | 是 | 删除当前及旧品牌启动值 |
| 重置其他设置 | 否 | 保留实际自启选择，不能绕过专用开关修改系统状态 |

注册位置限定为 `HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run`，不申请管理员权限，不创建计划任务、服务、驱动、启动文件夹脚本或 Defender 排除项。程序路径必须是绝对路径且不能包含可注入的引号。

## 交互与失败处理

- 开关清楚说明只有点击后才注册，并展示正在修改、启用、关闭和验证失败状态。
- 后端写入后立即回读精确值；返回状态与用户选择不一致时视为失败。
- 写入被权限或安全软件阻止时，前端回滚显示并再次只读查询真实状态，不保存虚假成功。
- 不建议用户关闭 Defender，也不提供添加文件或目录白名单的功能。

## 自动化证据

- Rust 单元测试验证绝对路径引用、固定参数以及相对路径和引号注入拒绝。
- 安装器模板测试验证更新保留当前显式值、清理旧品牌值，卸载清理全部相关值。
- 前端单元测试验证加载只读、点击才写、成功后才持久化以及失败回滚。
- `npm.cmd run test:e2e:desktop:auto-start` 在真实 Windows Release Tauri/WebView2 设置页中点击实际开关，并验证注册表精确值、幂等启用、禁用清理和登录启动隐藏窗口；测试发现已有用户值时直接拒绝执行，只有确认初始无值后才取得清理权。
- Windows CI 构建带隔离测试桥的真实桌面程序、校验脚本并准备匹配的 EdgeDriver；GitHub 托管 runner 是非交互会话，无法可靠创建 WebView2 `DevToolsActivePort`，因此聚焦 GUI 门禁只在本机或交互式 self-hosted Windows runner 执行。

## Defender 验证状态

当前开发机的 Microsoft Defender 服务、实时保护和行为监控均处于禁用状态，因此 `Start-MpScan` 与 `MpCmdRun.exe` 无法完成扫描。本轮只能确认近期检测历史中没有新增记录，不能把它记为“Defender 扫描通过”，也没有擅自修改机器的安全策略。

正式发布前的阻断条件：在 Microsoft Defender 已启用、病毒库已更新的 Windows 10/11 干净环境中，对正式 NSIS 安装包和安装后的主程序执行扫描；随后完成“默认不注册 → 用户开启 → 重启/登录静默托盘 → 用户关闭 → 卸载无残留”的人工或自动化闭环，并保存扫描版本、时间、哈希和结果。

## 平台依据

- Microsoft 说明未打包桌面应用仍可使用传统安装器写入的注册表项或快捷方式：<https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/unpackage-winui-app>
- Windows App SDK 的激活文档说明启动激活可以来自注册表 `Run` 项或启动文件夹快捷方式：<https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/applifecycle/applifecycle-rich-activation>
- `StartupTask` 等依赖包身份的能力属于 MSIX/包身份路线；当前 NSIS Win32 应用不假装具备该身份：<https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/packaging/>

后续如果迁移到具备包身份且签名可信的 MSIX，再单独评估 `StartupTask`；在此之前不混用两套机制，也不以要求用户降低系统防护作为交付条件。
