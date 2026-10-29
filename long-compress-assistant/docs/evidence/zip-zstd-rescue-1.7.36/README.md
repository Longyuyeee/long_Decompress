# v1.7.36 检查证据

- [汇总](checks.json)、[独立内容与日期核对](verified.json)：36 份恢复新包、132 个条目、38 次 7-Zip 读回（包括真实原件和 ZIP64 结构控制）；每份新包也经本软件最终解压提交后读回日期和内容。28 个停止控制和 5 个不发布场景通过。
- 5 项新增原生：消费量/限制/取消；普通/ZIP64 有无签名及混合方法；实际完整数字切片与缺中间卷；坏帧和描述符停止；首文件失败、取消、已有结果、未知加密与修正原件后重试。首轮 2 项通过，3 项被测试辅助中的运行时嵌套打断；仅修正辅助调用并补查该 3 项后通过。3 项相关 Deflate/BZip2/LZMA 旧方法组通过。
- [冻结来源及配方](../../../tests/fixtures/archives/zip-zstd-rescue-1736/)、[生成脚本](../../../scripts/prepare-zip-zstd-rescue-1736.py)、[原生检查](../../../src-tauri/src/services/zip_zstd_rescue_1736_tests.rs)、[独立检查](../../../scripts/verify-zip-zstd-rescue-1736.py)。ZIP64 和无签名等均为明确标记的结构控制，不冒充独立生产者原件。
- 下一版仅规划探测：[两个非空帧](next-nonempty-probe.json)、[非空帧后接空帧](next-empty-probe.json)。实际 libarchive 帧按累计 CRC/尺寸重新封装，由 7-Zip 读回；尚未加入恢复支持。

完整运行日志、临时原件副本和最终目录留在本机忽略路径 `test-results/zip-zstd-rescue-1736/`；不携入安装包。正式发布见 [v1.7.36](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.7.36)，检查只覆盖新增及受影响流程，前端没有修改。
