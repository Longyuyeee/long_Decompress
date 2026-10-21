# Long解压 v1.5.61

Zstandard ZIP 现在能继续整理、对比和取回资料，补齐方法 93 在现有工作区中的兼容。

- **整理 Zstandard ZIP**：独立普通包和 AES/ZipCrypto 密码包可添加、替换、改名或移动，再沿原清单另存新 ZIP。保留条目继续使用 Zstandard；未加密条目保留原压缩数据，密码条目按同方法重新编码。新增/替换仍按明确选择的 Deflate 或 Store 写出。
- **对比与所选取回**：文件夹与 ZIP、ZIP 与 ZIP 的 Zstandard 来源可核对真实内容，精确取回所选文件，也可作为补充来源生成新 ZIP。明确密码编码、完整数字/标准卷、草稿、历史与失败纠正沿原流程，密码仅在本次会话使用。
- **无法读取时更清楚**：完整帧、实际大小、CRC 和加密校验继续检查；外部字典、窗口超出支持范围或帧未完整结束会说明原因和处理办法。截断或损坏内容不作为完整相同结论，也不发布整理后的新包。
- **熟悉的界面继续保留**：单条点击展开、多选弹窗、自然继承、左右等宽与窄窗滚动保持；原包方法与新增/替换方法分别说明。

本版针对 ZIP 方法 93，不是独立 .zst 或旧 ZIP 编号 20 的扩展；不提供外部字典导入。原包不覆盖，完整分卷可比较和取材后另存独立 ZIP，仍不提供原生分卷回写、损坏包修复或全部元数据无损保证。

[实施与范围](https://github.com/Longyuyeee/long_Decompress/blob/v1.5.61/long-compress-assistant/docs/ARCHIVE_ROADMAP_1.5.61.md) · [封板记录](https://github.com/Longyuyeee/long_Decompress/blob/v1.5.61/long-compress-assistant/docs/ZIP_ZSTD_COMPATIBILITY_FREEZE_1.5.61.md) · [下一版计划](https://github.com/Longyuyeee/long_Decompress/blob/v1.5.61/long-compress-assistant/docs/ARCHIVE_ROADMAP_1.5.62.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.61)
