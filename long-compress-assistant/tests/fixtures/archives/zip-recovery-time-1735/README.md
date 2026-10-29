# v1.7.35 ZIP 恢复日期样本

`controls-full.zip` / `controls-lost.zip` 为独立 Python 标准库生成并明确标记的元数据控制，分别保留/去掉中央目录。覆盖中央与本地 UT 布局、唯一/重复/冲突、缺失/无效/高位歧义、epoch 0、2038 正值上限、DOS 缺失和仅 DOS 日期；另有 CRC 坏内容，不能因存在有效时间而被恢复。

`manifest.json` 的其余配方引用仓库既有 bsdtar / libarchive 3.8.9 ZIP/AES 原件和 CPython BZip2/LZMA 流式原件。只截去中央目录或按指定位置派生 CRC/密文损坏，不修改方法号伪造算法，也不复制大份相同原件；记录原件 SHA、派生来源 SHA、预期内容/日期和未取回条目。密码是公开测试密码。

配方：[prepare-zip-recovery-time-1735.py](../../../../scripts/prepare-zip-recovery-time-1735.py)。中央完整时保持中央 UTC 优先；目录重建时仅使用可确认的本地时间。两类来源的故意差异均在清单中分别声明。
