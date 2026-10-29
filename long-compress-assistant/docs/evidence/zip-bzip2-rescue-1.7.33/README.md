# v1.7.33 检查记录

- `native.json`：独立生产者 ZIP32 / ZIP64 去中央目录来源、签名/无签名描述符、混合 Deflate/BZip2 目录与文件，原诊断及已验证新包。
- `numeric.json`：真实完整数字切片从非首卷进入、来源 SHA、缺中卷拒绝与整组恢复。
- `stops.json`：18 组流/CRC/描述符/本地尺寸、截断、缺口和未知方法控制，只保留此前确认前缀。
- `failures.json` / `retry.json`：无完整文件、取消、已有输出及未知加密边界不发布；恢复来源后重试的新包。加密标记是明确的元数据控制，不声明真实加密流的新救援能力。
- `verified.json`：独立 Python/7-Zip 核对 25 组恢复结果的 96 个条目，另有 2 组完整外部原件。
- `diagnostic-verified.json` / `diagnostic-positive-verified.json`：修正“只确认目录而无完整文件”的可恢复状态后，仅补查受影响诊断与重试所生成的 5 个结果，均重新独立读回。
- `checks.json`：5 项新原生、2 项相关旧回归、15 项原界面操作，以及 Shell/版本身份摘要；说明两次定向补查原因。

[原件与配方](../../../tests/fixtures/archives/zip-bzip2-rescue-1733/)保留独立生产者信息及每条记录的真实头/描述符位置、CRC 与内容 SHA。本机生成物与原始日志位于忽略的 `test-results/zip-bzip2-rescue-1733/`；正式 CI/合入/标签及附件由本版发布流程接续，不为回填编号再次打包。
