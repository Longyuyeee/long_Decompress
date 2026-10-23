# v1.6.33 Deflate 图文原件

`scripts/prepare-sevenz-deflate-preview-1633.py` 使用原版 7-Zip 26.02 生成普通、最高压缩和数字卷三组非固实 Deflate 原件，包内官方 26.04 独立逐文件回解 SHA-256，列表确认 Deflate / Solid -。工具哈希、命令、逐文件与卷哈希见 manifest.json。

包含方括号/相邻文本路径、空项、UTF-8/UTF-16/GBK、二进制、精确 1 MiB 和刚超出；五种有效图片与一张有正确压缩 CRC 的无效栅格。另有 8 MiB/刚超出图片、尺寸超限、固实、AES 文件名加密和 Deflate64 原件。公开密码 `Deflate公开测试密码1633!` 仅用于自制不支持范围样本。

bad-data.7z 是 original.7z 的压缩起始块类型受控改为保留值，独立完整性检查失败，不是原工具自然损坏样本。测试补充构造“正文可解但无终止块”和尾部夹带，不能用普通 Read EOF 假称完整结束。

忽略的 large-unrelated.7z 包含先独立检查、后受控损坏的 65 MiB 无关 COPY 数据块；所选图文所在 Deflate 块不应扫描该块。新检出仅重建这个补充文件，已提交原件保持不变。
