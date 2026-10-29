# v1.7.31 检查记录

- `producer.json`：bsdtar/libarchive 真实外部原件的来源日期、UT/DOS 头、内容 SHA；独立 Python 元数据控制和派生 AES 来源配方。密码是公开测试密码。
- `native.json`：真实 ZIP、AES 密码、完整数字与标准卷的浏览和实际提交结果。
- `controls.json`：中央/本地字段、无效/重复/歧义 UTC 的实际 DOS 回退，以及坏 UT 不阻断 AES 内容读取。
- `choices.json`：实际并行工作线程、所选、平铺、时间关闭、较新判断、保留两份及询问冲突后接续。
- `zipx.json`：测试专用 XZ/AES ZIPX 的 UTC 与明显不同 DOS 日期，未冒称外部原件。
- `conversion.json`：UTC 暂存文件/目录进入已验证 TAR.GZ 和加密 7Z。
- `consistency.json`：密码回退文件日期、资料对比当地日历显示。
- `verified.json` / `checks.json`：独立 Python/7-Zip 对 17 份输出的核对，以及 6 项新原生、2 项相关回归、版本与 Shell 摘要。

固定原件位于 `tests/fixtures/archives/zip-utc-read-1731/`；本机生成物与原始日志位于忽略的 `test-results/zip-utc-read-1731/`。正式 PR/CI/合入/标签和发布附件由本版发布流程接续，不为回填编号再次打包。
