# Long解压 v1.5.63

ZIPX（XZ）现在能继续整理、对比和取回资料，补齐 ZIP 方法 95 在原工作区中的兼容。

- **ZIPX 直接编辑**：在原浏览器展开编辑清单，添加、替换、改名或移动，再另存新包。默认建议 `.zipx`，保存位置、标题和任务格式随实际后缀显示；也接收方法 95 的 `.zip` 来源。原包保留，保留的非空条目继续使用 XZ，新增/替换仍明确选择 Deflate／Store。
- **密码和分卷取材**：AES／ZipCrypto、明确密码编码、完整数字卷与 `.zxNN/.zipx` 标准卷接入真实内容对比、精确取回与所选组包。正常重启草稿、历史再取回和失败纠正沿原流程，密码仅在本次会话使用。
- **完整读取再给结果**：检查实际大小、XZ 结束和完整压缩载荷，保留原 CRC／加密校验；截断、异常尾部或未支持的后续流不作为完整相同内容，也不发布整理后的新包。
- **显示保持熟悉**：单条展开、多选弹窗、自然继承、左右等宽与窄窗滚动保持，对比入口和另存提示明确显示 ZIP／ZIPX。

本版仅扩展方法 95 的单 XZ 流条目，不等于全部 ZIPX 方法或独立 .xz 流的一并扩展。空 XZ 条目另存时按既有 ZIPX 创建规则仅存储，内容和密码设置保留，便于接收方正常打开。完整分卷可取材后另存独立包，不提供原生分卷回写、损坏修复或全部元数据无损保证。接收方需使用支持 XZ 方法的工具。

[实施与范围](https://github.com/Longyuyeee/long_Decompress/blob/v1.5.63/long-compress-assistant/docs/ARCHIVE_ROADMAP_1.5.63.md) · [封板记录](https://github.com/Longyuyeee/long_Decompress/blob/v1.5.63/long-compress-assistant/docs/ZIP_XZ_COMPATIBILITY_FREEZE_1.5.63.md) · [下一版计划](https://github.com/Longyuyeee/long_Decompress/blob/v1.5.63/long-compress-assistant/docs/ARCHIVE_ROADMAP_1.5.64.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.63)
