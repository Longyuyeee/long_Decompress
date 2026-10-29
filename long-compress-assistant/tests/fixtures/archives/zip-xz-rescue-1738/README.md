# ZIPX（XZ）流式恢复来源

[生成脚本](../../../../scripts/prepare-zip-xz-rescue-1738.py)、[SHA / 偏移 / 配方](manifest.json)。

`stream-xz.zipx` 是 7-Zip 26.04 向 stdout 实际写入的不可寻址 ZIPX，非空文件 method 95、flags 8、本地尺寸为 0；目录与原空文件使用 Store。`empty.xz` 是 Python 标准 liblzma 实际编码的空 XZ 流，`bzip-tail.zip` 是 CPython zipfile/bz2 的不可寻址尾记录。

四个 `xz32/64-signed/unsigned.zipx` 明确为结构控制：保留实际 XZ 压缩内容，按顺序整理记录，以真实空 XZ 替换 Store 空文件，接 BZip2 尾记录，重算描述符/中央信息。ZIP64 只拓宽本地未知尺寸和描述符，不冒充独立生产者超过 4 GiB 原件。丢目录、坏块校验/索引/尾部、尺寸/CRC 矛盾、过大字典、多个流与填充等在临时副本按配方派生，不当作已支持范围。

Python liblzma 检查每个实际 XZ 流 EOF、预读剩余量、CRC 和内容 SHA。7-Zip 对四份完整控制的零字节 XZ 条目报告附加数据，该差异单独记录，不伪装为干净通过；新生成的普通 ZIP 和最终目录严格读回。来源中央 NTFS 可能有奇数秒，本期沿现有规则使用可确认 DOS/UT；未扩展 NTFS/亚秒，不以外部工具读出的更细时间充当已支持能力。
