# Long解压 v1.5.62

PPMd ZIP 现在能继续整理、对比和取回资料，补齐方法 98 在现有工作区中的兼容。

- **整理 PPMd ZIP**：独立普通包和 AES/ZipCrypto 密码包可添加、替换、改名或移动，再沿原清单另存新 ZIP。未加密保留条目直接保留原压缩数据，密码条目按 PPMd 重新编码；新增/替换仍明确选择 Deflate 或 Store。
- **对比与所选取回**：文件夹与 ZIP、ZIP 与 ZIP 的 PPMd 来源可核对真实内容，精确取回所选文件，或作为补充来源生成新 ZIP。完整数字/标准卷、密码编码、正常重启草稿、历史与失败纠正沿原流程，密码仅在本次会话使用。
- **完整性判断更准确**：修正提前断流被当作正常结束的问题；读取需要实际结束标记、正确大小及完整压缩数据。截断、尾部多余数据、大小/CRC 或加密校验异常，不会被当作完整相同内容或发布新包。
- **参数问题更清楚**：合法 Restart／CutOff 继续读取；不支持的恢复方式与异常阶数在分配解码内存前说明原因。保留原资源范围，不增加解码器或依赖。
- **熟悉的界面继续保留**：单条展开、多选弹窗、自然继承、左右等宽和窄窗滚动保持，原包方法与新增/替换方法分别说明。

本版针对 ZIP 方法 98，不是 7z PPMd 或独立 PPMd 容器的扩展。Freeze 等未支持恢复方式仍明确拒绝；完整分卷可比较和取材后另存独立 ZIP，不提供原生分卷回写、损坏包修复或全部元数据无损保证。

[实施与范围](https://github.com/Longyuyeee/long_Decompress/blob/v1.5.62/long-compress-assistant/docs/ARCHIVE_ROADMAP_1.5.62.md) · [封板记录](https://github.com/Longyuyeee/long_Decompress/blob/v1.5.62/long-compress-assistant/docs/ZIP_PPMD_COMPATIBILITY_FREEZE_1.5.62.md) · [下一版计划](https://github.com/Longyuyeee/long_Decompress/blob/v1.5.62/long-compress-assistant/docs/ARCHIVE_ROADMAP_1.5.63.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.62)
