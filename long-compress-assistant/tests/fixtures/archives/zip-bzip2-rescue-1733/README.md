# v1.7.33 流式 BZip2 ZIP 原件

`stream32.zip` / `stream64.zip` 由独立 CPython 3.12 标准库 `zipfile` 向不可寻址输出生成，BZip2 内容由标准库 bz2 编码；并非修改压缩方法号得到的样本。两组都含显式 Deflate / BZip2 空目录、空文件、重复文本、跨读取缓冲区的随机数据和后续 Deflate 文件，本地尺寸未知，尾部带普通/ZIP64 数据描述符。

`lost32.zip` / `lost64.zip` 仅移除中央目录；`unsigned32.zip` / `unsigned64.zip` 另移除描述符的可选签名，属于明确标记的结构控制，不冒称另一种外部生产者原件。运行时损坏/加密标记/尺寸控制从这些原件派生，不声明新增密码支持。

`producer.json` 记录来源 SHA、每条记录真实头位置、方法、尺寸、CRC 和内容 SHA；配方：[prepare-zip-bzip2-rescue-1733.py](../../../../scripts/prepare-zip-bzip2-rescue-1733.py)。部分取回原总清单仍未知。
