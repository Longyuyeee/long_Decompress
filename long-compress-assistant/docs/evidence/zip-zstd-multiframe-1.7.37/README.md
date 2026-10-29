# v1.7.37 检查证据

- [汇总](checks.json)、[独立读回](verified.json)：5 项新增原生首轮通过，2 项相关旧方法组通过；33 份恢复新包、115 个条目及 33 份本软件最终目录内容/日期一致，38 次外部 7-Zip 读回通过。26 个停止控制、2 个不发布场景和缺中间卷拒绝通过。
- [来源与配方](../../../tests/fixtures/archives/zip-zstd-multiframe-1737/)、[生成](../../../scripts/prepare-zip-zstd-multiframe-1737.py)、[原生检查](../../../src-tauri/src/services/zip_zstd_multiframe_1737_tests.rs)、[独立核对](../../../scripts/verify-zip-zstd-multiframe-1737.py)。多帧包是实际压缩帧的结构派生控制，不冒充独立生产者直接输出的多帧 ZIP。
- 最后补齐帧间真实读取错误原因，仅补查受影响两组及更新产物读回，通过后封板。
- 实际普通帧在空帧前后组合；累计尺寸和 CRC 才能确认成员。4096 帧边界及超界、累计展开限制、取消、无签名 CRC 等于 Zstandard magic、后帧损坏不产生半个文件均已确认。
- 下一版仅规划探测：[7-Zip 流式 ZIPX / XZ 原件](next-xz-stream.zipx)、[SHA / 消费量 / CRC 记录](next-xz-probe.json)。Python liblzma 独立确认 EOF、实际消费量与描述符，尚未加入本软件未知边界恢复。

完整日志、临时副本和最终目录留在本机忽略路径 `test-results/zip-zstd-multiframe-1737/`。正式发布见 [v1.7.37](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.7.37)，前端没有修改，检查仅覆盖新增和受影响流程。
