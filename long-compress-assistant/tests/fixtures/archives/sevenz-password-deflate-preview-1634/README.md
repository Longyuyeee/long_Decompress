# v1.6.34 密码 Deflate 图文原件

`scripts/prepare-sevenz-password-deflate-preview-1634.py` 用原版 7-Zip 26.02 生成 AES 内容加密、文件名加密（最高压缩）和完整数字卷三组非固实 Deflate 原件，包内官方 26.04 独立逐文件回解 SHA-256。工具哈希、命令、逐文件/各卷哈希与 Deflate / 7zAES / Solid - 列表见 manifest.json。公开密码 `Deflate公开测试密码1634!` 仅用于这些自制样本。

包含方括号/相邻路径、空项、UTF-8/UTF-16/GBK、二进制、精确 1 MiB/刚超出，五种有效图片和正确压缩 CRC 的坏栅格；另有 AES 图片 8 MiB/刚超出/尺寸超限、加密固实及单文件原件。damaged-data.7z 为单文件原件载荷受控修改，由独立完整性检查确认失败，不是原工具自然损坏样本。

忽略的 large-unrelated.7z 含先独立检查后受控损坏的 65 MiB 无关 COPY 块，所选文本和图片所在 AES → Deflate 块应直接读取。脚本保留已提交原件，新检出仅重建忽略补充；构造单元另补“正文完整但缺终止块”、额外明文、非零 AES 填充和完整消费回执，不混称独立原件。
