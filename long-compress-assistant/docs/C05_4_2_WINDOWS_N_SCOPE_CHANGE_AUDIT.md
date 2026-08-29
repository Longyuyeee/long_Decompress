# C-05.4.2 Windows N 支持范围变更审计

日期：2026-08-29  
状态：**范围变更已获产品负责人明确批准；Windows N 不再阻塞 `v1.1.16`，但本版本不得宣称支持 Windows N。**

## 1. 变更授权与原因

产品负责人明确授权：“批准取消 Windows N 实机门禁，Windows N 暂不保证支持。”当前唯一可用开发主机为 `EditionID=Professional`，不具备真实 Windows N、未安装 Media Feature Pack 的前置状态；此前已确认本机没有可复用的 Windows N 虚拟机、镜像或虚拟化工具。

这项授权改变的是公开支持范围，不是测试结果。`windowsNRealMachinePassed` 必须继续为 `false`，不得把 Professional 主机负向自测、人工移除 DLL 或单元注入写成 Windows N 实机通过。

## 2. 原需求与纠偏后的边界

原 C-05 要求 Windows N 在 Media Feature Pack 安装前稳定拒绝、安装并重启后完成生产预检和真实转码。该要求在无法取得目标环境时会阻塞首个视频版本。

纠偏后的 `v1.1.16` 边界如下：

- 支持范围限定为带 Media Foundation 的非 N Windows x64 版本；
- Windows N（无论是否安装 Media Feature Pack）均不列入 `v1.1.16` 已验证支持范围；
- 生产预检仍必须检查 Media Foundation，缺失时继续稳定返回明确错误；
- Windows N 两阶段脚本、独立验收器和锁定候选身份全部保留；未来只有取得真实同机证据后，才允许移除“不支持”声明；
- C-05.1 至 C-05.4.1 的真实桌面、格式、长时/大文件、取消、历史、默认应用和正式 NSIS 生命周期证据不受本次范围变更影响。

## 3. 实际实现

- `config/media-dependencies.json` 保留 `windowsNRealMachinePassed=false`，新增不支持策略、非阻塞发布语义、变更日期和明确授权来源；
- `config/media-release-gates.json` 将 C 节点平台范围固定为 `windows-x86_64-non-n`，并要求真实 Windows N 证据才能在未来移除排除项；
- `check-media-dependencies.mjs` 与 `check-media-release-gates.mjs` 静态锁定上述语义，防止后续把“非阻塞”误改成“已通过”或静默扩大支持面；
- Windows N 生产脚本和独立验收器不删除、不弱化。

## 4. 发布结论

在本次获批支持范围内，C-05 可以依据 C-05.1 至 C-05.4.1 的既有真实证据关闭，并进入 `v1.1.16` 版本身份、候选构建、安装态复验、公开 Release 与回下载更新闭环。所有 README、Release notes 和发布审计必须明确写出 Windows N 暂不支持。

本审计不等于 `v1.1.16` 已发布；公开资产、签名 updater、上一版本应用内更新和回下载复验仍需逐项完成。
