# D-03.3.1 受控低容量卷门禁收口审计

审计日期：2026-08-31

节点状态：**完成；D-03 总节点关闭**

执行边界：**真实 Windows CI 隔离卷与内部产品事务；仍不注册 PDF 产品执行命令**

## 1. 原始需求与安全边界

D-03 要求磁盘不足时不产生半成品且源文件哈希不变。开发机没有可安全创建隔离卷的管理员能力，因此没有填满用户磁盘，也没有用 mock 或固定错误冒充证据。门禁只允许在 `GITHUB_ACTIONS=true` 且测试根目录位于 `RUNNER_TEMP` 时创建 96 MiB 动态 VHDX；清理阶段始终分离 VHD。

## 2. 实际实现

- `test-d03-pdf-low-capacity.ps1` 创建、格式化并挂载 NTFS VHD，限定测试根目录并在 `finally` 中清理。
- Rust 测试在同一进程调用 `execute_pdf_publication_transaction`，不是容量判断替身。
- 测试先真实写入 1 MiB 探针，确认卷总量与可用量低于 128 MiB 产品安全预留。
- Windows 存储探测优先采用更具体的原生文件夹挂载点，避免把 VHD 错认成宿主 `C:` 卷。

## 3. 预期—实际—修正

| 轮次 | 预期 | 实际 | 修正 |
| --- | --- | --- | --- |
| `33376718024` | 在全目标测试内执行 VHD 门禁 | 子测试递归启动 Cargo，两个进程争用测试 exe，出现 `LNK1104` | 改为同一 Rust 进程直接执行产品事务 |
| `33378117324` | 真实容量阻断且总门禁通过 | VHD 子测试通过，但严格 Clippy 报两处 `needless_borrow` | 修正无意义借用，不删除断言 |
| `33379106430` / `03cd16d5e844c5133a79450a5d0f49e34123dce8` | 五组 CI 全绿 | Frontend、Rust/shell、Browser E2E、Windows desktop、Windows installer 全部成功 | 无差异 |

## 4. 最终证据

CI 日志明确记录：

- `github_actions_runs_real_low_capacity_volume_gate ... ok`；
- `DiskPart successfully detached the virtual disk file.`；
- Rust 主库 `374 passed; 0 failed; 9 ignored`；
- 产品事务返回 `PDF_TRANSFORM_RESOURCE_PREFLIGHT_BLOCKED`；
- 最终输出不存在，`.pdf-transform-*` 暂存数为 0；
- 源 PDF SHA-256 前后相同。

运行地址：<https://github.com/Longyuyeee/long_Decompress/actions/runs/33379106430>

同一收口候选的本机复验结果：

- D-02.1 真实分析 12/12、D-03.1 真实暂存 5/5、D-03.2 真实验证 23/23、D-03.3 真实发布 29/29，预期—实际差异均为 0；
- 本分支涉及的两个 Rust 文件 `rustfmt --check` 通过，严格 Clippy 零告警；
- Rust 全目标主库 374 通过、0 失败、9 条按明确外部条件忽略，其余集成目标全部通过；
- 前端 49 个文件 289/289、覆盖率、类型检查、生产构建、媒体架构和 `1.1.16` 版本身份全部通过。

首轮本机复验曾把完整媒体夹具与 `--images-only` 夹具并行写入同一被忽略目录，PDF 测试因此暂时找不到 `text-vector.pdf`；改为按夹具所有权串行运行后四段真实 PDF 回归全部通过，没有修改产品代码或删减断言。仓库级 `cargo fmt --check` 还会报告大量既存基准、示例和归档模块格式债务；本节点只对实际变更的 Rust 文件执行格式门禁，避免用 PDF 收口机械重写无关代码。

## 5. 合同与下一接续点

`config/pdf-optimization-contract.json` 的节点提升为 `D-03.3.1`，`controlledLowCapacityVolumeEvidence=true`。产品执行边界仍为关闭：没有 Tauri 转换命令、任务、历史或可执行按钮。

D-03 至此关闭。下一唯一接续点为 D-04：复用现有统一任务、取消、事件、历史、容量预检和默认应用能力，接入 PDF 产品执行，并完成真实批量、失败、大文件、结构、默认阅读器与正式安装版矩阵。D-04 关闭前版本保持 `1.1.16`。
