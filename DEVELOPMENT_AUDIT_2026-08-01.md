# Long解压开发与发布审计（2026-08-01）

## 结论

当前 `master` 可以作为稳定开发基线。审计最初建议等待产品运行时代码增量后再发布；用户随后明确要求立即升级版本，
因此 v1.0.22 定位为发布治理与验收基础设施维护版，交付 v1.0.21 发布后的公开更新生命周期加固、文档和追溯更新。

发布判断：**按用户确定的版本策略发布 v1.0.22；不引入 Windows 商业代码签名。**

## 已核验证据

- `master` 与 `origin/master` 一致，审计开始时工作区干净，没有开放 PR 或 Issue。
- 最新主线 CI 为 [run 30660733646](https://github.com/Longyuyeee/long_Decompress/actions/runs/30660733646)，结论为 `success`。
- [v1.0.21 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.0.21) 是 Latest，安装器、updater ZIP、签名和 `latest.json` 四项资产齐全。
- v1.0.20 → v1.0.21 公开应用内更新为 21/21 `PASS`；安装目录、两套用户数据、独立重启、唯一版本化 Shell DLL 和传统菜单四根/17 命令均通过。
- 版本身份门禁通过：前端、Tauri、Cargo lock、Shell Extension 和版本化 DLL 均为 `1.0.21`。
- `npm audit --omit=dev` 为 0；生产依赖没有已知漏洞。
- 完整 npm 审计仍有 7 项开发工具链问题（3 moderate、2 high、2 critical），主要位于旧 Vite/Vitest/esbuild 链路。完整修复要求跨主版本迁移，不应通过 `npm audit fix --force` 混入补丁发布。
- `npm outdated` 显示 Tauri 1→2、Vite 5→8、Vitest 2→4、Tailwind 3→4、Pinia 2→4 等均属于独立迁移项目。

## 当前开发状态

### 已收口

- 压缩、解压、密码判断、事务发布、取消、冲突处理和磁盘写满清理均有真实归档回归。
- ZIP、7Z、TAR 系列、单文件流与 AES v2 已拆分为独立原生模块，核心服务职责已明显收窄。
- Windows 传统右键菜单已改为按用户内联级联命令，并通过安装版和公开更新版的严格验证。
- 应用内更新具备签名验证、活动任务保护、安装交接、自动重启和真实公开版本验证。
- 固定机器性能工具已覆盖大文件 ZIP、小文件 ZIP、原生 7Z 和 AES v2；只有同机、同配置且双方各至少 10 次样本时才应用阈值。
- 主线保护覆盖前端、浏览器 E2E、Rust/Shell Extension、Windows 桌面构建和 NSIS 安装器。

### 尚未收口

| 优先级 | 事项 | 当前边界 | 下一验收条件 |
| --- | --- | --- | --- |
| P1 | 真实桌面 E2E 无人值守运行 | GitHub 托管 Windows runner 无交互桌面，无法稳定运行 WebView2 GUI | 接入固定、交互式 self-hosted Windows runner，并让安装器依赖真实桌面结果 |
| P1 | 开发工具链主版本迁移 | 生产审计为 0，但开发审计仍有 7 项；Vite/Vitest 等需要跨主版本 | 每个生态分独立 PR，完成类型、覆盖率、浏览器、桌面构建与安装器回归 |
| P1 | 扩展格式证据 | HFSX 仍缺可再分发、非空且可逐字节核验的可靠样本 | 固定来源、版本和 SHA-256，解压后核对已知载荷 |
| P2 | Windows 11 顶层右键菜单 | 实现已存在，但公开身份包需要可信 Windows 代码签名证书 | 获得证书后验证身份包、SmartScreen、升级与卸载 |
| P2 | 实验性并行解压器 | 尚未进入生产路径 | 先对齐密码、冲突、时间戳、路径安全、取消和事务回滚，再决定接入或删除 |

## 下一阶段建议

1. 先做开发工具链迁移预研，优先隔离 Vite/Vitest/vue-tsc 测试链，避免与 Tauri 2、Tailwind 4 同时升级。
2. 在可控 Windows 主机上部署 self-hosted runner；部署前不要添加会永久排队的必需检查。
3. 继续积累固定机器性能结果，但不把跨机器数据当作回归。
4. 只在获得真实非空样本后扩展对外格式声明。
5. 下一次正式版本必须至少包含一项用户可感知修复或能力增量，并重新执行候选安装生命周期与上一正式版公开应用内更新。

## 发布与打包决定

- 不移动 `v1.0.21` 标签，也不覆盖其已发布资产。
- v1.0.22 使用新的八源版本身份、唯一版本化 Shell DLL、NSIS 安装器和 updater 资产独立发布。
- 不配置 Windows Authenticode/商业代码签名证书；Tauri updater 的应用内更新校验签名继续保留。
- 发布后仍需从公开 v1.0.21 执行真实应用内更新，生成独立验收 Issue 和机器可读报告。
