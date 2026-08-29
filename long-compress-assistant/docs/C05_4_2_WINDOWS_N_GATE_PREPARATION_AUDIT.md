# C-05.4.2 Windows N 实机门禁准备审计

日期：2026-08-29

## 结论

**C-05.4.2 的跨机器执行工具已完成，真实 Windows N 前后证据尚未取得，因此 C-05.4.2、C-05 和 `v1.1.16` 发布门禁均未关闭。** 当前主机是 `EditionID=Professional`，只能验证拒绝边界，不能代替 Windows N 或通过移除 DLL 伪造证据。

候选产品仍严格锁定为提交 `71a95729a17352c2c711b2f34764739175954099`、CI run `33258733949` 的正式 NSIS；实机证据工具固定使用提交 `b717f973a79035bbf1a475885d84f493de398906`。Windows N 前后两阶段必须保持该工具提交不变，因为后阶段和独立验收器会复核前阶段记录的脚本 SHA-256。

## 与最初需求对齐

C-05 原始验收要求的是“正式安装候选在真实 Windows N、Media Feature Pack 安装前后的生产行为”，不是某个 EXE 被手工复制到目标机器后的命令行探针。原脚本虽然要求正式卸载注册表键和候选主程序精确哈希，但安装动作依赖人工完成，报告没有锁定实际使用的 NSIS。

本次纠偏后：

- `MissingMediaFeaturePack` 必须接收 `-CandidateInstaller`；
- 在任何安装动作前先确认真实 Windows N，普通 Windows 稳定拒绝且不改变安装状态；
- 校验 NSIS 为 15,604,389 B、SHA-256 `C4FF2374AA033A6EE892EF8BF6313CDB2B987BDD3BA244930C45AF081391D718`；
- 要求目标机器此前不存在 Long解压安装记录和运行进程，然后由门禁以审计过的 `/P /NS /NR` 参数安装候选；
- 安装后从正式卸载注册表推导主程序路径，并继续校验 28,853,760 B、SHA-256 `0B443647D39B817993794FA3E6F6D52600515B06700030F565CB4DBF5D795EF0`；
- 前阶段报告新增安装器身份，独立验收器同时复核安装器、安装主程序、脚本、机器和后阶段真实转码。

## 本机负向自检

在 Professional 主机传入精确候选安装器运行前阶段，结果为：

- 退出码：1；
- 稳定错误：`WINDOWS_N_MACHINE_REQUIRED: editionId=Professional`；
- 报告 `passed=false`；
- 安装器未执行；
- 测试前后均为公开 `v1.1.15`，安装位置 `E:\long\Long解压` 不变；
- 测试后无 Long解压进程。

同时通过 PowerShell AST、两个 Node 脚本语法、真实媒体依赖、媒体架构和媒体发布门禁。

## Windows N 目标机执行手册

目标必须是 Windows N x64，尚未安装 Media Feature Pack，且没有现存 Long解压安装。先固定工具提交并安装依赖：

```powershell
git fetch origin
git checkout b717f973a79035bbf1a475885d84f493de398906
npm ci
npm run test:media-dependencies
```

使用有仓库 Actions 读取权限的 GitHub CLI 回下载锁定候选，并让包级门禁再次复核：

```powershell
gh run download 33258733949 -n windows-nsis-installer -D test-results\windows-n-candidate
$candidate = (Resolve-Path 'test-results\windows-n-candidate\Long解压_1.1.15_x64-setup.exe').Path
npm run test:video-runtime-package:real -- "--installer=$candidate"
```

选择一个不会因重启清除、且两阶段完全相同的证据目录：

```powershell
$evidence = 'C:\LongDecompressEvidence\c05-windows-n-71a9572'
npm run test:windows-n-video-runtime -- -Phase MissingMediaFeaturePack -CandidateInstaller "$candidate" -EvidenceDirectory "$evidence"
```

前阶段必须通过后，才通过 Windows 设置安装与当前 N 版本匹配的 Media Feature Pack，并按系统要求重启。重启后不要拉取代码、修改脚本、改变证据目录或重装候选：

```powershell
git status --short
git rev-parse HEAD
$evidence = 'C:\LongDecompressEvidence\c05-windows-n-71a9572'
npm run test:windows-n-video-runtime -- -Phase MediaFeaturePackInstalled -EvidenceDirectory "$evidence"
npm run verify:windows-n-video-runtime -- "$evidence"
```

只接受 `git rev-parse HEAD` 为 `b717f973a79035bbf1a475885d84f493de398906`、工作区干净、最终 `verification.json` 为 `passed=true`。随后审计原始报告，更新 `windowsNRealMachinePassed` 和 C-05 文档，再进入 `v1.1.16` 版本身份与公开发布；不得仅凭命令退出码提前改状态。

## 下一接续点

必须在具备上述条件的真实 Windows N 机器执行两阶段门禁。本机已穷尽所有不伪造平台证据的准备工作；在外部实机结果返回前，不进行版本提升、标签、Release 或公开更新。
