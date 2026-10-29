# 流式 Zstandard ZIP 恢复来源

由 [生成脚本](../../../../scripts/prepare-zip-zstd-rescue-1736.py) 固定，文件 SHA、尺寸、CRC、偏移及原件/控制身份见 [producer.json](producer.json)。

- `stream32.zip`：Windows bsdtar / libarchive 3.8.9（libzstd 1.5.7）通过 stdout 不可寻址输出的真实 method 93 ZIP；四个文件（含空文件）使用未知本地尺寸及描述符，两个明确目录为 Store。固定来源日期有奇数 UTC 秒，正文包含 ZIP 标记文字，随机成员跨越 64 KiB 缓冲区。
- `bzip2-tail.zip`：CPython zipfile + stdlib bz2 的不可寻址独立原件；用于混合方法末尾，保留 DOS 日期，无 UTC 扩展。
- `stream64-control.zip`：以真实 Zstandard 原件为基础，保留全部压缩帧和 CRC，将文件本地尺寸改成 ZIP64 未知占位，增加 ZIP64 本地额外字段并拓宽描述符，修正中央偏移。小样本控制不能代替真实超过 4 GiB 原件或独立生产者直接输出的 ZIP64 声明。
- `lost32/64.zip`：各版本删除中央目录；`unsigned32/64.zip` 进一步删除文件描述符的可选签名，不改变真实帧。
- `mixed.zip`：真实 Zstandard 本地记录前缀接入独立 BZip2 尾记录，删除中央目录；按该尾记录实际 DOS 日期核对。

首次生成的 tar 探测中，Windows 对以 `/` 结尾的输入目录参数报错。生成器改用无尾斜线的明确目录名并检查退出码，随后才冻结以上成功原件；这不涉及产品路径逻辑。

损坏、字典 ID、过大窗口、多帧、旧 method 20、缺口等仅在原生测试临时副本上按配方派生，不能当作真实字典包或已支持范围。独立检查使用 7-Zip 26.04 解码真实来源及结构控制，Python 检查恢复新包内容和本地/中央日期，并核对本软件最终解压目录。
