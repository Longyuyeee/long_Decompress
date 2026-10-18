# EGG 原始样本与载荷

`upstream/` 保留 unegg 提交 `9929009ca4d010e8cd88bb796f7a1868162026d4` 的原始测试包与 CC0 声明；没有改名、重打包或重新生成以替代原格式。`manifest.json` 的十三组期望文件由独立 Bandizip 7.38 Beta 回解建立，固定名称、长度、SHA-256。复杂样本只用于确认本版明确拒绝，不当作本版支持证据。

`original-alzip/` 是原版 ALZipCon 11.0.1.16（来自官方 ALZip 12.37.0.4 安装器）生成的四组公开自有载荷：UTF-8 中文名称、空目录、普通/ZipCrypto/原生卷及 CP936 中文密码。原始工具哈希、生成参数和原卷身份见 manifest；独立 Bandizip 与产品回解比较见 independent.json。原工具/安装器不随产品分发。中文密码原包须显式选 CP936，默认 UTF-8 不自动猜测。

`scripts/prepare-egg-153-fixtures.mjs` 在库存存在时只核对固定原卷，不重生成期望。`create-egg-153-originals.mjs` 同样保留现有原件。`verify-egg-153-originals.mjs` 用独立工具及产品核对内容；再次运行请使用新的测试输出目录，不能覆盖已提取文件来绕过产品 create-new 约束。

真实暂停/恢复/取消另外使用原工具生成的 256 MiB 原包，位于忽略的 test-results/egg-153-control；不将通用切片充作原生 EGG 分卷。其准确身份随桌面报告交接，不把体积庞大的临时载荷加入源仓库。
