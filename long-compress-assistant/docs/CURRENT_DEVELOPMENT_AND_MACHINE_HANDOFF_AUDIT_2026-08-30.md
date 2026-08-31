# 当前开发与换机接续审计（2026-08-30）

审计基线：`master` / `e3282ddb8f00b6f12d091c2720e930768effea64`

审计目的：以实际代码、版本源、测试入口、GitHub 运行和本机验收目录为准，确认当前开发位置，纠正文档时态，并把更换电脑后继续 D-02 所需的需求边界、证据索引和复验步骤固化到仓库。

> 2026-08-31 增量状态：D-02、D-03.1 与 D-03.2 已完成，D-03.3 内部安全发布核心完成。规范化输出锁、候选验证、源/候选 SHA-256 复核、Mark-of-the-Web 和原子发布已串成内部事务；16 个真实最终 PDF 及失败矩阵共 29 组差异为 0。产品命令、任务和历史仍冻结。当前结论与下一接续点以 [D03_3_PDF_SAFE_PUBLICATION_AUDIT.md](D03_3_PDF_SAFE_PUBLICATION_AUDIT.md) 为准：下一步为 D-03.3.1 受控低容量卷证据。

## 1. 结论

- A 归档工作区已由 `v1.1.14` 发布，B 图片压缩已由 `v1.1.15` 发布，C 视频软件编码已由 `v1.1.16` 在非 N Windows x64 范围内发布并完成公开更新闭环。
- D PDF 安全优化的 D-01 已关闭：qpdf 12.4.0 能力与十类 PDF 基线、正式运行时及许可、生产预检、签名安装包增量和正式安装态恢复均已有证据。
- 当前唯一接续点是 **D-03.3.1 受控低容量卷发布失败证据**。内部 PDF 安全发布核心已存在，但仍没有 PDF 产品执行命令；不得把内部原子发布测试误写成用户可用的 PDF 压缩功能。
- 当前核心路线已经进入最后的 PDF 执行/验收阶段。D-01、D-02、D-03.1 与 D-03.2 已关闭，D-03.3 仍缺隔离低容量卷证据，之后还有 D-04 和 `v1.1.17` 发布闭环；完成比例只作排期参考，节点证据才是完成依据。
- C-06 硬件编码是后续独立可选节点，不阻塞 PDF 与本轮核心路线收尾；Windows N 仍未取得实机证据并明确暂不支持，不得在换机后回填为已验证。

## 2. 仓库与发布事实

| 项目 | 审计结果 |
| --- | --- |
| Git 远端 | `origin = https://github.com/Longyuyeee/long_Decompress.git` |
| 审计分支/提交 | `master` / `e3282ddb8f00b6f12d091c2720e930768effea64` |
| 与远端差异 | `HEAD...origin/master = 0/0`，审计开始时工作区干净 |
| 当前产品身份 | `package.json`、Tauri Cargo 与 `tauri.conf.json` 均为 `1.1.16` |
| 当前公开标签 | `v1.1.16`，固定在 `a59742265feb961ab51f9b95b4e455aa15b79bf5` |
| 当前 PDF 代码 | 正式 qpdf 资源、固定字节身份、能力预检及安装态报告；无分析/转换命令，无可执行 PDF UI |
| 当前 GitHub 主分支 CI | 运行 `33319829140`，对应审计基线 `e3282ddb`；Frontend、Rust/Shell、Browser E2E、Windows desktop build、NSIS installer 5/5 成功 |
| 非当前接续 PR | PR #91 是 C-01.2.1 之后的旧状态审计，仍为 Open，但内容早于当前主线；不得从该分支接续 D-02 |

本审计只记录 `e3282ddb` 的功能状态；随后产生的文档提交不改变上述产品代码基线。

## 3. 原路线与实际实现对齐

| 路线节点 | 实际状态 | 判断 |
| --- | --- | --- |
| A 归档工作区 | A-01 至 A-06 完成，`v1.1.14` 已发布 | 已关闭 |
| B 图片压缩 | B-00/B-01 至 B-05.3 完成，`v1.1.15` 已发布 | 已关闭 |
| C 视频软件编码 | C-01 至 C-05 完成，`v1.1.16` 已发布；Windows N 排除在支持范围外 | 已关闭，无路线偏移 |
| C-06 硬件编码 | 未开始，路线明确为后续独立节点 | 非当前阻塞项 |
| D-01 qpdf 基线 | 十类样本、两模式白名单、正式运行时、许可、生产预检、安装态与增量证据完成 | 已关闭 |
| D-02 前端配置 | D-02.1 只读分析与 D-02.2 两模式风险配置 UI 均完成；只保存页面内草稿 | 已关闭 |
| D-03 执行与校验 | D-03.1 暂存、D-03.2 验证和 D-03.3 内部原子发布核心完成；没有产品转换命令 | 下一步 D-03.3.1 受控低容量卷证据，仍不得开放产品执行 |
| D-04 真实验收 | 未开始 | D-02/D-03 关闭后进行 |

### D-02 的准确开发顺序

原需求要求界面在执行前展示页数、输入大小、加密、签名、表单和附件等真实事实。2026-08-30 基线后端只做 qpdf 运行时身份预检，因此 D-02 不能只开发前端静态卡片；截至 2026-08-31 以下五项均已关闭：

1. ~~读取现有图片/视频工作区、统一任务模型、密码脱敏和资源预检的实际代码，建立 PDF 输入候选及结构化分析结果类型。~~ D-02.1 已完成。
2. ~~新增只读 PDF 分析后端，使用固定 qpdf 参数数组返回页数、大小、加密、签名、表单、附件及可执行性；密码不得进入日志、历史或可持久命令行文本。~~ D-02.1 已完成，密码经 stdin 传递。
3. ~~实现“无损整理”和“兼容图片优化”配置；图片优化明确标记有损，签名默认只分析，加密文件必须正确密码后才能形成计划。~~ D-02.2 已完成。
4. ~~默认输出新文件且不得覆盖源文件；风险必须在执行前可见，危险组合需要显式确认，不承诺一定缩小。~~ D-02.2 已完成并由真实桌面门禁验证。
5. ~~D-02 只形成真实分析与冻结配置，不把 D-03 的转换、发布或源文件处理提前塞入本节点。~~ 架构门禁确认没有 PDF 转换/发布命令或任务写入。

每一个可独立关闭的小步继续执行：代码与原需求对账 → 自动化/真实验收 → 审计文档 → 提交 → PR/CI → 合入主线。D-01 至 D-04 全部关闭前不得提升 `1.1.17`。

## 4. 可传递验收证据

### 4.1 已随 Git 永久传递

- `config/pdf-optimization-contract.json`：两模式参数、允许/禁止变化、加密/签名策略和执行边界。
- `tests/fixtures/media/manifest.json`、`scripts/inspect-d01-pdf.py`、`scripts/run-d01-pdf-baseline.mjs`：十类样本声明、独立结构检查和可重复基线入口。
- `src-tauri/resources/pdf-engine/`：qpdf 12.4.0 五文件正式运行时、来源与四份许可/NOTICE；`.gitattributes` 将其固定为不做文本换行转换。
- `src-tauri/src/services/pdf_engine.rs` 和 `commands/pdf_engine.rs`：10 文件大小/SHA-256、版本、crypto、JSON v2、图片优化能力及缺失/替换拒绝。
- D-01 三份审计文档：能力/样本、运行时准入、安装态与签名增量的完整结论和追溯信息。

这些内容足以在新电脑重新生成样本和结果，不依赖复制当前电脑的 `node_modules`、`target` 或 `test-results`。

### 4.2 GitHub 可回查证据

| 证据 | 位置/身份 | 结论 |
| --- | --- | --- |
| D-01.2.2 签名双构建增量 | Actions run `33318192852`，测量提交 `a27ecc0c9fff7968a5d952b5647e25e82ee5f650` | qpdf 使 NSIS/updater 各增加 `3,603,012 B` |
| D-01.2.1 合入 CI | PR #95 / run `33317897159` | 成功 |
| D-01.2.2 合入 CI | PR #96 / master run `33319829140` | 5/5 成功，含 Windows NSIS artifact |
| v1.1.16 Release | run `33263384953` / tag `v1.1.16` | 四项公开资产与签名发布完成 |
| v1.1.16 发布审计 | `docs/RELEASE_AUDIT_1.1.16.md` | 公开回下载及 `v1.1.15 → v1.1.16` 更新通过 |

Actions 的运行日志和构建 artifact 可能受 GitHub 保留期影响。长期接续以已提交的合同、脚本、哈希和审计文档为准，不把临时 artifact 的持续可下载性作为唯一证据。

### 4.3 当前电脑存在但不会推送的原始产物

`long-compress-assistant/test-results/` 被 `.gitignore` 明确排除。以下原始目录仅用于现场复核，不是新电脑继续开发的必要输入：

| 本机相对路径 | 文件/字节 | 关键 JSON SHA-256 |
| --- | ---: | --- |
| `test-results/d01-pdf-baseline` | 17 / 159,895 B | `result.json = EACB289E…C30584` |
| `test-results/d01-signed-delta-33318192852` | 8 / 69,580,679 B | `pdf-result.json = 428D2BB0…A055` |
| `test-results/d01-ci-installer` | 1 / 19,201,569 B | 安装器身份已写入 D-01.2.2 审计 |
| `test-results/installed-release-validation/20260830-225829` | 16 / 41,703,342 B | `result.json = 3BFCEB88…8EF2`；PDF 子结果 `5C582954…5269` |

上述目录包含安装包、隔离安装副本、绝对路径和本机用户数据指纹，不应直接提交到公开仓库。需要长期离线保存时，应由项目负责人放入受控的私有备份；换机开发不应复制其中的用户数据或凭据。D-04 和 `v1.1.17` 发布仍必须在届时的候选提交上重新产生新证据，不能拿 D-01 的旧机器结果替代。

## 5. 新电脑恢复与基线复验

### 5.1 获取准确主线

```powershell
git clone https://github.com/Longyuyeee/long_Decompress.git
Set-Location long_Decompress
git checkout master
git pull --ff-only origin master
git status --short --branch
git log -5 --oneline
```

必须从合入本审计后的 `origin/master` 接续，不要检出仍处于 Open 状态的旧 PR #91 分支。新电脑需自行配置 GitHub 认证；任何 PAT、签名私钥、密码保险箱或用户目录都不进入 Git。

### 5.2 工具链

- CI 使用 Node.js 24；在新电脑使用 Node 24 LTS/当前兼容版本并执行 `npm ci`，不要复制旧 `node_modules`。
- 安装 Rust stable、Windows MSVC/C++ 构建工具、Windows SDK 和 WebView2；正式 Tauri/NSIS 与桌面验收必须在 Windows x64 上运行。
- D-01 真实样本脚本需要可调用的 Python；脚本会在 `test-results` 下准备固定 Python 包与媒体样本，不应复制旧缓存冒充新结果。
- qpdf 与 FFmpeg 产品运行时已经入库，新电脑不应改用系统 PATH 中的同名工具替代产品资源。

### 5.3 接续前最小门禁

```powershell
Set-Location long-compress-assistant
npm ci
npm run test:release-identity -- --expected 1.1.16
npm run test:pdf-contract
npm run test:media-architecture
npm run test:media-dependencies:real
npm run test:pdf-d01-baseline:real
npm run type-check
npm run test:unit:coverage
npm run build
Set-Location src-tauri
cargo test --release --all-targets
cargo clippy --all-targets -- -D warnings
```

如果只是首次恢复环境，可先运行到 PDF 合同、真实依赖和 D-01 基线；开始提交 D-02 前必须至少补齐类型、单测/覆盖率、构建和相关 Rust 测试。正式安装态、桌面交互、取消、磁盘不足和公开升级不能由浏览器测试替代，应在对应节点重新执行专门门禁。

### 5.4 本次审计实测

- 发布身份 `1.1.16`、PDF 合同、媒体架构、六项真实依赖、类型检查和生产构建通过。
- D-01 可重复门禁重新生成 11 张图片、2 个视频和 10 个 PDF，真实 qpdf 基线 10/10 通过；结论继续是“运行时已准入，产品转换与 UI 未开放”。
- 前端覆盖率门禁 48 个测试文件、284/284 通过；全仓严格 Clippy 通过。
- Rust `cargo test --release --all-targets` 第二次完整运行通过；同一代码的 GitHub Windows clean runner Rust job 也通过。
- 但本机第一次完整运行曾在 `services::archive_diagnostics::tests::real_seven_zip_reports_health_and_wrong_password` 出现一次错误密码分类不一致；随后两次隔离运行一败一成，第二次全量运行通过。该测试与本次纯文档变更和 PDF D-01 无关，但已证明存在非确定性，不能删去失败结果或把重试包装成首次全绿。新电脑应首先复跑全量 Rust 门禁；若再次出现，须在 D-02 首个功能 PR 前单独定位共享临时状态/引擎输出分类问题。

## 6. 未完成项与风险

- D-02 已完成；D-03、D-04 尚未完成。PDF 页面当前可用于真实只读分析和风险配置，但不能执行转换。
- D-03 必须复用共享临时输出、取消、磁盘空间、原子发布和历史模型；不得另建弱化事务。
- 输出大于输入时默认不发布；用户即便选择保留，也不得自动替换源文件。
- 数字签名 PDF 默认只分析；未来若允许执行，必须有独立、明确的签名失效确认流程。
- Windows N 暂不支持且 `windowsNRealMachinePassed=false`；C-06 硬件编码仍是可选后续节点。
- 公开仓库不保存更新签名私钥、GitHub Token、本机用户数据和忽略目录中的安装副本；换机后需要重新配置受控凭据，不能从文档猜测或恢复。
- 归档诊断的错误密码真实测试在本机出现过非确定性分类；虽然后续全量和 clean CI 通过，仍作为已知测试可靠性债务传递，不得静默忽略。

## 7. 换机后的唯一接续点

从 D-03.1“PDF 执行事务基础”开始：执行前必须重新分析并核对冻结配置，复用共享容量预检、取消、输出锁和同目录暂存；验证与原子发布闭环完成前不得写成功历史。完成一小步后更新本文件或新的节点审计，通过对应自动化与真实门禁，再提交、推送、等待 PR CI 并合入。任何偏离上述安全边界的实现应先纠偏需求文档，不得通过扩大描述掩盖代码偏移。
