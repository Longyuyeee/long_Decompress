# D-01.1 qpdf 能力契约与 PDF 样本基线审计

审计日期：2026-08-30（Asia/Shanghai）

状态：**D-01.1 完成；D-01 尚未关闭，qpdf 产品集成继续冻结。**

## 原始需求对齐

| 原始要求 | 实际证据 | 结论 |
| --- | --- | --- |
| 固定 qpdf 版本、哈希、许可 | `12.4.0` 官方 mingw64 ZIP 24,063,155 B，SHA-256 `DCEC940C…B4F5`；上游 checksum 文件对账；Apache-2.0 与 NOTICE URL 固定 | 完成候选身份，许可载荷待 D-01.2 打包 |
| 参数必须有官方依据 | 两种模式只使用官方记录的对象流、流压缩、Flate 重压缩、压缩等级、图片优化与尺寸阈值参数 | 完成 |
| 文本、扫描、混合、表单、注释、书签、附件、加密、签名样本 | 合成且实际解析的 10 类 PDF，另保留透明内容边界 | 完成 |
| 记录允许和禁止变化 | `config/pdf-optimization-contract.json` 分别冻结无损与图片模式允许/禁止变化 | 完成 |
| 危险文档执行前提示而非静默破坏 | 签名样本实际识别 `/Sig` 并只分析；加密样本无密码安全拒绝，正确密码才允许检查 | D-01.1 完成；交互提示属于 D-02 |
| 安装态能力检查 | 本步只允许测试缓存候选，产品运行时未准入 | 待 D-01.2 |

官方依据为 qpdf 12.4.0 [CLI](https://qpdf.readthedocs.io/en/latest/cli.html)、[参数索引](https://qpdf.readthedocs.io/en/latest/qpdf-options.html)、[JSON 结构](https://qpdf.readthedocs.io/en/latest/json.html)和[正式 Release](https://github.com/qpdf/qpdf/releases/tag/v12.4.0)。qpdf 官方明确说明其主要职责不是强力缩小文件；`--optimize-images` 可能用有损 JPEG 重写符合阈值且确实更小的非 JPEG 图片，因此产品模式命名为“兼容图片优化”，不得宣传压缩率保证。

## 实际代码审计与纠偏

1. qpdf 候选身份此前已经存在于媒体依赖清单，实际状态为 `official-runtime-candidate-packaging-blocked` / `integrationAllowed=false`，不能重复下载后直接宣称产品能力。
2. 原六类 PDF 夹具缺少图文混合、注释、书签和附件，与路线 D-01 不一致。现扩为十类并把四项加入发布门禁。
3. 首轮真实脚本只比较 qpdf 页数、字段名、注释类型、书签标题和附件名。虽然 10/10 通过，但不足以证明禁止变化项；现新增独立 pypdf 检查器，精确比较页面尺寸/文本、表单值、注释内容与矩形、书签页目标、附件长度和 SHA-256，再从头复验通过。
4. 真实样本中“兼容图片优化”没有比无损整理进一步缩小扫描/混合 PDF，说明当前图片编码不满足 qpdf“只有结果更小时才采用”的实际条件。该结果保留为基线，不制造图片优化收益。

## 冻结能力契约

- 无损整理：`--object-streams=generate`、`--compress-streams=y`、`--decode-level=generalized`、`--recompress-flate`、`--compression-level=9`、`--newline-before-endstream`。
- 兼容图片优化：在上述参数上增加 `--optimize-images`、`--jpeg-quality=85`、最小宽高 128 和最小面积 16,384；明确标记可能有损。
- 两种模式都禁止原始参数字符串、源文件替换、Ghostscript 和产品 UI 提前开放。
- 数字签名当前只分析；加密文档必须先验证正确密码。密码不得写入日志、证据或持久命令行模板。

## 真实结果

- qpdf 输出：`qpdf version 12.4.0`；crypto provider 同时报告 `openssl` 与 `native`。
- 8 个普通/特殊非签名样本 × 2 种模式，共 16 次真实转换均通过输入/输出 `--check` 和独立结构保持对账。
- 无损整理相对输入减少 352 至 10,699 B；图片模式相对输入减少 342 至 10,699 B。这里只记录当前合成样本事实，不作为未来压缩率承诺。
- 签名 PDF：实际识别 `FixtureSignature`，决定为 `analysis-only-execution-blocked`。
- AES-256 加密 PDF：无密码 `--check` 返回 `invalid password`；正确用户密码检查通过，决定为 `password-required-before-planning`。
- 结构化证据：忽略目录 `test-results/d01-pdf-baseline/result.json`。合成 PDF 可能包含生成时间和随机签名字节，因此每次证据记录实际大小/SHA-256，验收依据是生成后实际结构而不是跨机器伪造固定字节。

## 门禁与下一步

- `npm.cmd run test:pdf-contract`：通过，包含四组负向变异控制。
- `npm.cmd run test:pdf-d01-baseline:real`：通过；依赖真实校验、11 图片/2 视频/10 PDF 夹具与 10 PDF qpdf 基线全部通过。
- 前端单测与覆盖率的既有 pretest 已接入静态 PDF 合同门禁，GitHub Frontend CI 会随覆盖率测试执行；真实下载和转换保持本地/专用证据门禁，不让普通单元测试依赖网络。
- 本步没有修改版本、Tauri resources、Rust 命令、前端 PDF 执行入口或安装包。

下一接续点严格为 D-01.2：把经审计的 qpdf 五文件子集和 Apache-2.0/NOTICE 纳入正式资源，建立后端生产身份与能力预检、缺失/替换拒绝，并在同提交正式 NSIS/updater 中测量精确增量。上述完成前不得进入 D-02，也不得提升 `1.1.17`。
