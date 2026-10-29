# v1.7.38 检查证据

- [汇总](checks.json)、[独立核对](verified.json)：5 项新原生最终通过，首轮 4 项通过、1 项因测试缺口位置与名称断言不一致中断；只修正该辅助位置并补查受影响组，产品代码无需调整。2 项相关旧方法组通过。
- 34 份恢复新包、120 个条目及本软件最终目录的内容/路径/日期一致，34 次新包 7-Zip 严格读回通过。28 个停止控制、2 个不发布场景、缺中间卷拒绝、原件 SHA 与暂存清理通过。
- 四份来源派生控制中有零字节 XZ 成员：Python liblzma 与本软件能完整读回，7-Zip 26.04 对该空成员报告附加数据。四次来源警告完整记录，未算作成功的外部读回；取回后新包使用普通 ZIP，严格读回通过。来源中央 NTFS 的较细日期不作为本期恢复承诺，最终按已有 DOS/UT 规则核对。
- [来源及配方](../../../tests/fixtures/archives/zip-xz-rescue-1738/)、[生成](../../../scripts/prepare-zip-xz-rescue-1738.py)、[原生](../../../src-tauri/src/services/zip_xz_rescue_1738_tests.rs)、[独立检查](../../../scripts/verify-zip-xz-rescue-1738.py)。
- 下一版仅规划：[探测记录](next-xz-probe.json)、[多流控制](next-concatenated-single.zipx)、[填充控制](next-padding-single.zipx)。两个实际 XZ 流或一个流后八字节零填充，按累计尺寸/CRC 重封装，7-Zip 均以退出码 0 完整读回；不是独立生产者直接输出的多流 ZIPX，尚未加入本软件兼容支持。

完整日志和临时目录在本机忽略路径 `test-results/zip-xz-rescue-1738/`，正式发布见 [v1.7.38](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.7.38)。前端未改，不控制桌面，不重跑无关矩阵。
