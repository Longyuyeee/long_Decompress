# D-03.3 PDF 安全发布事务审计

审计日期：2026-08-31

节点状态：**核心事务完成；D-03.3.1 已补齐受控低容量卷证据，D-03 总节点关闭**

执行边界：**仅 Rust 内部事务；不注册 Tauri 产品命令、不创建任务或历史、不开放 UI 执行**

## 1. 目标与需求对齐

D-03.3 把 D-03.1 暂存与 D-03.2 验证收进一个内部安全发布事务：转换开始前锁定规范化最终路径，随后依次执行固定参数 qpdf 转换、候选验证、发布前取消复核、源文件 SHA-256 复核、候选 SHA-256 复核、Mark-of-the-Web 策略和同目录原子重命名。发布结果只能来自发布后真实文件系统字节和 SHA-256。

本节点没有把内部事务注册为产品命令。当前 PDF 页面仍只有 D-02.2 风险配置草稿；任务、历史、批量、结果打开和安装版闭环属于 D-04。

## 2. 真实代码边界

- `pdf_publish.rs` 的进程级 `ACTIVE_PDF_OUTPUTS` 使用规范化绝对目标路径作为键；等价路径的第二个任务返回 `PDF_PUBLISH_OUTPUT_LOCKED`，守卫 Drop 后释放。
- `execute_pdf_publication_transaction` 持有输出锁贯穿转换、验证和发布；候选所有权守卫在任一错误路径清理本事务暂存族。
- D-03.1 现在把转换前真实源 SHA-256 绑定进 `PdfStagedOutput`。发布前重新流式读取源和候选；源变化返回 `PDF_PUBLISH_SOURCE_CHANGED`，候选变化返回 `PDF_PUBLISH_STAGING_CHANGED`。
- Mark-of-the-Web 只在用户策略启用且源文件含 Internet/Restricted zone 时写入候选 ADS；写入发生在原子重命名前。真实 Windows 测试确认最终 PDF 的 `ZoneId=3`。
- 最终提交复用共享 `publish_verified_file`：目标出现时不覆盖，同目录 `rename` 提交。提交后再读取普通文件身份、字节和 SHA-256，必须与 D-03.2 验证报告完全相同。

## 3. 真实预期—实际结果

`npm run test:pdf-d03-publication:real` 使用打包 qpdf 12.4.0 和真实 PDF 文件。文本、扫描、图文混合、透明、AcroForm、注释、书签、附件 8 类文件分别执行两种模式，共 16 个最终 PDF：

| 门禁 | 预期 | 实际 |
| --- | --- | --- |
| 原子发布 | 16 个候选均成为真实最终文件 | 16/16 存在；最终 SHA-256 与验证报告逐个相同 |
| 独立结构检查 | 发布文件与源文件事实相同 | pypdf 对 16/16 的文本、页面尺寸、图片、表单值、注释、书签和附件对账相同 |
| 源文件保护 | 发布不改变源字节 | 16/16 源 SHA-256 不变 |
| 暂存清理 | 成功和失败后均无事务暂存 | 每次发布及最终扫描均为 0 个 `.pdf-transform-*` 文件 |
| 跨任务输出锁 | 等价目标不可并发占有，Drop 后可重试 | 第二次占有被拒绝；释放后重新占有成功 |
| 发布前源变化 | 拒绝且无最终输出 | `PDF_PUBLISH_SOURCE_CHANGED`，目标不存在 |
| 发布前候选变化 | 拒绝且无最终输出 | `PDF_PUBLISH_STAGING_CHANGED`，目标不存在 |
| 目标竞争 | 不覆盖用户文件 | `PDF_PUBLISH_TARGET_APPEARED`，`existing user bytes` 逐字保持 |
| 验证后取消 | 不发布并清理 | `PDF_PUBLISH_CANCELLED`，目标不存在 |
| Mark-of-the-Web | 标记在提交前传播并随重命名保留 | `applied`，最终 ADS 读取 `ZoneId=3` |

结构化证据 `test-results/d03-pdf-publication/result.json` 共 29 组预期—实际比较，`differenceCount=0`。该目录被 Git 忽略；仓库提交可复验脚本和审计结论。

## 4. 首次差异与修正

| 项目 | 预期 | 首次实际 | 修正 |
| --- | --- | --- | --- |
| 测试编译 | 最终 ADS 在 Mark-of-the-Web 场景中读取 | 补丁匹配把读取语句插入较早的源变化场景，Rust 报 `motw_destination` 不在作用域 | 将 ADS 读取移动到真实标记文件发布之后，完整真实矩阵重跑为 29/29、差异 0 |
| 格式化范围 | 只格式化 3 个目标 Rust 文件 | `cargo fmt --manifest-path ... -- 文件` 仍格式化整个 crate，产生 100 多个无关机械差异 | 撤回本轮产生的全部无关格式化，只保留目标文件，并改用文件级 `rustfmt` 检查 |

## 5. D-03.3.1 后续证据

已完成回归：D-03.1 真实暂存 5 组差异 0、D-03.2 真实验证 23 组差异 0、D-02.1 真实分析 12 组差异 0；PDF 契约和媒体架构门禁通过；严格 Clippy 零警告；前端 49 个文件 289/289，通过覆盖率和生产构建；Rust 全目标主库 373 通过、8 条按明确外部条件忽略，全部集成目标通过。

本机非管理员边界没有被绕过。D-03.3.1 改在管理员 GitHub Actions Windows Runner 的 `RUNNER_TEMP` 内创建 96 MiB NTFS VHD，真实运行同一产品事务。CI run `33379106430` 证明容量预检稳定阻断、最终输出不存在、暂存数为 0、源 SHA-256 不变，且 VHD 成功卸载；五组总门禁全绿。合同据此把 `controlledLowCapacityVolumeEvidence` 固化为 `true`，详细证据见 [D03_3_1_LOW_CAPACITY_CLOSEOUT_AUDIT.md](D03_3_1_LOW_CAPACITY_CLOSEOUT_AUDIT.md)。

## 6. 版本判断

版本保持 `1.1.16`。D-03 已关闭；D-04 产品/批量/安装版/默认 PDF 阅读器/历史，以及公开更新和 Release 门禁均未完成，当前仍不允许打包或发布 `v1.1.17`。
