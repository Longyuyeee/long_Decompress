# v1.7.34 流式 LZMA ZIP 原件

`stream32.zip` / `stream64.zip` 由独立 CPython 3.12 标准库 `zipfile` 向不可寻址输出生成，实际 LZMA 数据由标准库 liblzma 编码。method 14 成员明确设置通用标志 bit 1，含 4 字节 ZIP LZMA 版本/属性长度前导、5 字节属性及实际 EOS；不是更改方法号伪造的样本。

两组均有 LZMA 文本、随机数据、空文件和空目录，并保留 Deflate 目录、后续 BZip2 文件以核对混合方法接续。`lost32.zip` / `lost64.zip` 仅去中央目录；`unsigned32.zip` / `unsigned64.zip` 另去描述符可选签名，为明确标记的结构控制。运行时的属性、尺寸、结束标记与加密标记控制不冒称新增外部原件或密码能力。

`producer.json` 包含整体 SHA、头/描述符位置、尺寸、CRC 和内容 SHA。配方：[prepare-zip-lzma-rescue-1734.py](../../../../scripts/prepare-zip-lzma-rescue-1734.py)。来源原总清单未知时，取回结果始终保持部分取回。
