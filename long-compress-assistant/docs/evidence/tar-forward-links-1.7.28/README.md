# v1.7.28 检查记录

- `producer.json`：三组 GNU tar 原件配方、固定 SHA、两次有序 GNU 提取的内容/inode 基准与独立 TAR 头日期。
- `browse.json`：真实普通/GNU/PAX 与数字卷浏览、文本/图片预览、完整性和诊断结果；文本预览仅保留 SHA 与字节数，避免重复保存整段内容。本机原始文本仍在忽略的结果目录，未为整理记录重跑检查。
- `native.json`：9 份实际完整/所选最终输出，原件、读取源、精确范围、各自日期与逻辑/实际数据统计。
- `choices.json`：目录、筛选、平铺和关闭时间选项的结果。冲突接续及独立副本关系由同一原生专项检查。
- `invalid.json`：缺失、歧义、链条、循环、目录及带正文目标的明确拒绝原因。
- `verified.json`：官方 LZOP 解码及独立 Python/GNU 原件基准，核对 44 个编码条目、9 份输出/101 个条目/83 个文件内容和自身日期。
- `checks.json`：本次原生、相关回归、Shell DLL 和版本身份摘要。

本机样本位置记录在文件中；自动生成结果位于忽略的 `test-results/tar-forward-links-1728/`，固定原件位于 `tests/fixtures/archives/tar-forward-links-1728/`。正式 PR、CI、合入、标签与 Release 附件由本版发布流程接续，不为回填运行号再次打包。
