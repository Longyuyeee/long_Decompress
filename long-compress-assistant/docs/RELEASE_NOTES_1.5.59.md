# Long解压 v1.5.59

Bzip2 ZIP 现在能继续整理、核对和取回资料，补齐压缩后进入其他工作区的断点。

- **编辑 Bzip2 ZIP**：支持独立普通包及 AES/ZipCrypto 密码包的添加、替换、重命名与目录整理，沿原清单另存新 ZIP。保留条目继续使用 Bzip2，未加密条目保留原压缩数据，密码条目按原方法重新编码；新增/替换按明确选择的 Deflate 或 Store 写出。
- **对比实际内容**：文件夹与 ZIP、ZIP 与 ZIP 的 Bzip2 来源可读取清单、核对所选内容并显示真实 SHA-256 结论。密码与密码编码沿原弹窗设置；完整数字和标准卷组可从任意成员归组，缺卷先补齐。
- **所选资料接回原流程**：精确取回、重启后的待处理草稿、历史再取回及失败纠正保持原范围；也可作为补充来源，在原多选弹窗确认后生成独立新 ZIP。组包按所选方法重新编码，来源原件保留。
- **损坏内容不误报完成**：编辑核对实际读取大小，CRC 或压缩流损坏不能发布新包；内容核对中的 ZIP 大小异常显示该项未完成，其他已完成结论保留。未知方法、密码问题与缺卷继续提供原处理入口。
- **方法说明更清楚**：编辑清单显示原包方法，说明保留条目和新增/替换的区别；单条展开、多选弹窗、左右等宽和窄窗滚动保持。

本版扩展的是方法 12 在现有工作区中的兼容，不是新增 .bz2 格式或原生分卷编辑。原包不覆盖，分卷可取材并生成独立 ZIP；不承诺损坏包修复、全元数据无损改写或全部外部工具兼容。

[实施与范围](https://github.com/Longyuyeee/long_Decompress/blob/v1.5.59/long-compress-assistant/docs/ARCHIVE_ROADMAP_1.5.59.md) · [封板记录](https://github.com/Longyuyeee/long_Decompress/blob/v1.5.59/long-compress-assistant/docs/ZIP_BZIP2_COMPATIBILITY_FREEZE_1.5.59.md) · [下一版计划](https://github.com/Longyuyeee/long_Decompress/blob/v1.5.59/long-compress-assistant/docs/ARCHIVE_ROADMAP_1.5.60.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.59)
