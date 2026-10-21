# Long解压 v1.5.60

ZIP LZMA 现在能继续编辑、对比和取回资料，补齐方法 14 在现有工作区中的兼容。

- **整理 LZMA ZIP**：独立普通包和 AES/ZipCrypto 密码包可添加、替换、改名或移动，再沿原清单另存新 ZIP。保留条目继续使用 LZMA；未加密原条目可保留压缩数据，密码条目按原方法重新编码。新增/替换仍按明确选择的 Deflate 或 Store 写出。
- **对比与取材接通**：文件夹与 ZIP、ZIP 与 ZIP 的 LZMA 来源可核对真实内容，精确取回所选文件，也可作为补充来源生成新 ZIP。密码编码、完整数字/标准卷、正常重启草稿、历史与失败纠正沿原流程，密码仅在本次会话使用。
- **LZMA 结束检查更准确**：声明带结束标记的条目需要真正读到结束，合法无结束标记条目仍按大小读取。截断、实际大小不符、CRC 或属性异常不会被当作完整可用内容；小文件继续按实际大小限制解码字典占用。
- **熟悉的界面继续保留**：单条点击展开、多选弹窗、自然继承、左右等宽与窄窗滚动保持；原包方法与新增/替换方法分别说明。

本版针对 ZIP 方法 14，不是 .lzma 单流或 ZIPX（XZ）。原包不覆盖，分卷可比较和取材后另存独立 ZIP，仍不提供原生分卷回写、损坏包修复或全部元数据无损保证。

[实施与范围](https://github.com/Longyuyeee/long_Decompress/blob/v1.5.60/long-compress-assistant/docs/ARCHIVE_ROADMAP_1.5.60.md) · [封板记录](https://github.com/Longyuyeee/long_Decompress/blob/v1.5.60/long-compress-assistant/docs/ZIP_LZMA_COMPATIBILITY_FREEZE_1.5.60.md) · [下一版计划](https://github.com/Longyuyeee/long_Decompress/blob/v1.5.60/long-compress-assistant/docs/ARCHIVE_ROADMAP_1.5.61.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.60)
