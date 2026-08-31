# D-03.3.1 低容量卷门禁换机交接

交接日期：2026-08-31

当前状态：**开发暂停；真实低容量产品事务已经在 GitHub Runner 通过，但 PR 总门禁尚未全绿，D-03 仍未关闭。**

版本判断：**继续保持 `1.1.16`，不得打包、更新 README Release 区或创建 `v1.1.17` GitHub Release。** D-04 产品命令、任务/历史、批量和正式安装版验收尚未开始。

## 1. Git 与远端位置

- 仓库：`https://github.com/Longyuyeee/long_Decompress.git`
- 基线分支：`master`
- 当前开发分支：`codex/pdf-d03-3-1-low-capacity-gate`
- 当前 PR：[PR #103](https://github.com/Longyuyeee/long_Decompress/pull/103)，目标为 `master`
- 已推送代码提交：`6021346 test(pdf): add controlled low-capacity VHD gate`、`eb11036 fix(pdf): run low-capacity gate in-process`
- 本次交接还包含 `pdf_publish.rs` 的两处 Clippy `needless_borrow` 修正和本文档；以远端开发分支最新 HEAD 为准。

换机后不要从 `master` 另起实现。先执行：

```powershell
git clone https://github.com/Longyuyeee/long_Decompress.git
Set-Location .\long_Decompress\long-compress-assistant
git fetch origin
git switch codex/pdf-d03-3-1-low-capacity-gate
git pull --ff-only
git status --short
gh pr checks 103
```

如果仓库已经存在，则从 `git fetch origin` 开始。必须确认工作区干净且分支名称完全一致，再继续。

## 2. 本轮真实代码做了什么

- `scripts/test-d03-pdf-low-capacity.ps1` 只允许在 `GITHUB_ACTIONS=true` 且存在 `RUNNER_TEMP` 时运行；它在 Runner 临时目录内创建 96 MiB 动态 VHDX、快速格式化为 NTFS、使用文件夹挂载，并在 `finally` 中分离 VHD 和删除明确的测试根目录。
- `scripts/run-d03-pdf-low-capacity.mjs` 提供独立复现实入口，执行真实 Rust 事务并生成被 Git 忽略的 `test-results/d03-pdf-low-capacity/result.json`，逐字段比较预期与实际。
- `pdf_publish.rs` 新增最小但结构有效的真实一页 PDF、1 MiB 实写探针和低容量断言。GitHub Actions 路径在同一个 Rust 测试进程内创建 VHD 并直接调用 `execute_pdf_publication_transaction`，不再递归运行 Cargo。
- `storage_preflight.rs` 在 Windows 上比较原生卷挂载点与 `sysinfo` 挂载点的具体程度；文件夹挂载 VHD 比宿主盘更具体时采用原生容量，避免把低容量卷误判为 `C:\`。
- 产品边界没有变化：没有注册 PDF Tauri 执行命令，没有创建任务或历史，也没有开放 UI 转换。

## 3. 预期—实际与两轮修正

真实门禁的固定预期是：NTFS；卷总量与可用量均低于 128 MiB 安全预留；卷上真实写入 1 MiB 成功；产品事务返回 `PDF_TRANSFORM_RESOURCE_PREFLIGHT_BLOCKED`；最终输出不存在；`.pdf-transform-*` 暂存数为 0；源 PDF SHA-256 不变。

| 轮次 | 预期 | 实际 | 处理结果 |
| --- | --- | --- | --- |
| CI run `33376718024` | 外层全目标测试内完成低容量门禁 | VHD 路径进入后又递归启动 `cargo test --release`，两个 Cargo 争用同一 exe，出现 `LNK1104` | 已改为同一 Rust 进程内直接执行事务；这不是产品容量判断失败 |
| CI run `33378117324` 的真实后端测试 | 创建/挂载/写入/阻断/清理全部成立 | `github_actions_runs_real_low_capacity_volume_gate ... ok`；主库 `374 passed, 0 failed, 9 ignored`；日志确认 VHD 已成功 detach | 真实低容量事务本身通过 |
| 同一 run 的严格 Clippy | 零警告 | 两处 `&volume` 产生 `needless_borrow`，因此 Rust job 总结仍为 failure，installer 被跳过 | 工作区已改成 `probe_storage(volume)` 与 `read_dir(volume)`；尚需由最新推送 CI 复验 |

第二轮其余结果：Browser shell E2E 通过（51 秒）、Frontend checks 通过（1 分 33 秒）、Windows desktop E2E build 通过（5 分 57 秒）。不能因为真实 VHD 子测试已通过就把总门禁写成完成；PR 必须在修正后的最新提交上全绿。

## 4. 当前合同与文档状态

- `config/pdf-optimization-contract.json` 的 `controlledLowCapacityVolumeEvidence` **仍为 `false`**。
- `scripts/check-pdf-optimization-contract.mjs` 仍要求该值为 `false`。
- [D-03.3 审计](D03_3_PDF_SAFE_PUBLICATION_AUDIT.md)、README、路线图和总计划仍把唯一接续点写为 D-03.3.1。
- 这是刻意保留的真实状态：最新提交尚未获得完整 CI 通过，不能先改合同制造“文档完成”。
- 本机 `test-results` 被忽略且可能包含绝对路径，不需要复制到新电脑；Git 跟踪的脚本、代码、CI 日志和审计文档才是换机依据。

## 5. 换机后的唯一接续步骤

1. 拉取开发分支后先执行 `gh pr checks 103`，确认本次交接推送触发的最新 CI，而不是引用旧 run。
2. 如果最新 CI 失败，只检查失败 job 的真实日志，继续记录“预期、实际、差异、修正”；不得跳过、改成 mock 或删除断言。
3. 如果最新 CI 全绿，记录 run URL、提交 SHA 和真实 VHD 测试结果，然后才把 `controlledLowCapacityVolumeEvidence` 与对应检查器改为 `true`。
4. 新增/更新 D-03.3.1 收口审计，并同步 README、`DEVELOPMENT_HANDOFF.md`、`PRODUCT_ENHANCEMENT_ROADMAP.md`、`ARCHIVE_WORKSPACE_AND_MEDIA_COMPRESSION_PLAN.md`、当前开发审计；明确 D-03 已关闭、下一唯一接续点为 D-04。
5. 在合同改为 true 的同一候选上重新运行真实 PDF 回归（D-03.1、D-03.2、D-03.3）、合同/架构门禁、严格 Clippy、Rust 全目标、前端测试与构建；逐项记录预期—实际差异。
6. 提交并推送收口文档与合同，等待 PR #103 最新提交所有必需 CI 全绿，然后合并到 `master`；切回 `master`、`git pull --ff-only` 并确认工作区干净。
7. 合并后从 D-04 开始：产品 PDF 执行命令和任务编排、真实批量矩阵、失败提示、任务历史、默认 PDF 阅读器打开、正式安装版桌面闭环。

## 6. 明确禁止提前做的事

- 不得在 PR #103 最新提交全绿前把 D-03 写成关闭。
- 不得用系统盘填充、mock 错误或容量推断替代隔离 VHD 证据。
- 不得在 D-04 和发布门禁完成前提升版本、打包 `v1.1.17`、修改 Release notes 为已发布或创建 GitHub Release。
- 不得把内部 Rust 发布事务描述成当前用户已经可以使用的 PDF 压缩功能。

