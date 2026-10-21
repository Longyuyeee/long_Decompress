# Long解压 v1.5.64

ZPAQ 原生多部分备份现在可以确认整组后浏览、选择旧快照并取回资料。

- **整组来源更清楚**：从任一编号成员打开，先列出候选部分、顺序和体积；确认后一起读取，也可以明确只打开当前文件。解压中心和转换沿用同一选择，不凭相似名称自动合并。
- **旧快照继续使用**：普通与原生密码组接通文本/图片预览、内容检查、所选/整包取回及所选快照转换。任务保留确认过的成员列表，不自动加入后来出现的文件。
- **失败后接着处理**：缺少成员或来源变化会说明需要处理的文件；草稿重启、历史再取回、补密码和新任务重试保留快照及精确所选范围，旧失败与已有输出保留。
- **熟悉的界面**：成员信息可收起，配置保持单条展开、多选弹窗、左右等宽与窄窗滚动。

本版只读完整原生组，不提供原地追加、原生组创建/更新或损坏事务抢救。候选识别以名称末尾数字编号为界，同一前缀从 1 连续排列；已确认组缺成员会拒绝读取。未确认来源的末尾部分是否丢失，无法仅靠现有文件名判断，请核对成员数量与预期快照。确认记录保存在本机应用数据目录，只含成员路径和版本信息，不复制整包或保存密码；默认输出及单条/全局/批量“同目录”都使用原成员目录。原件保留。

[实施与范围](https://github.com/Longyuyeee/long_Decompress/blob/v1.5.64/long-compress-assistant/docs/ARCHIVE_ROADMAP_1.5.64.md) · [封板记录](https://github.com/Longyuyeee/long_Decompress/blob/v1.5.64/long-compress-assistant/docs/ZPAQ_NATIVE_MULTIPART_FREEZE_1.5.64.md) · [下一版计划](https://github.com/Longyuyeee/long_Decompress/blob/v1.5.64/long-compress-assistant/docs/ARCHIVE_ROADMAP_1.5.65.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.64)
