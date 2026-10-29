# v1.7.35 检查记录

- `controls.json`：中央完整与丢目录两类来源的 UT/DOS 保留；flags 1/3/5/7、中央/本地单独存在及故意相异、截短/空/重复/冲突/高位歧义、epoch 0 与 2038 上限、无效 DOS 兼容回退。CRC 坏文件仍不取回。
- `native.json`：冻结 bsdtar 普通/AES 原件的派生损坏来源、坏可选 UT 在 AES 前、完整数字与标准卷、顺序及实际并行最终提交日期；错误密码不发布，来源 SHA 与暂存清理保持。
- `streams.json`：原 BZip2 / LZMA 流式取回保持 DOS 日期，不补造 UTC。
- `verified.json`：独立 Python 核对新包本地/中央头，及外部 7-Zip 对 10 个新包的内容/日期读回；11 份本软件最终目录也核对内容和日期。
- `checks.json`：3 项新增专项、2 项相关回归、Shell/版本身份摘要；说明测试辅助配置和实际并行样本选择的修正，仅补查受影响组。
- `next-zstd-probe.json`：下一版独立原件生成的可行性核对，证明外部工具能生成 method 93、未知本地尺寸 ZIP，不代表本软件已实现 Zstandard 流式恢复。

[样本与配方](../../../tests/fixtures/archives/zip-recovery-time-1735/)引用既有独立原件，明确标记目录、CRC、密文和元数据控制。只保存必要的小型控制包与派生配方，不复制大份相同原件。本机原始日志位于忽略的 `test-results/zip-recovery-time-1735/`；正式 CI、合入、标签与附件由本版发布流程接续，不为回填编号再次打包。
