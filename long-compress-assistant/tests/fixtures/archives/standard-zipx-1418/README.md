# Bandizip 原生 ZIPX 标准分卷

2026-10-18（北京时间）使用本机已有 `bz 7.38(Beta,x64)` 原工具生成，未改名、未修改卷字节；命令、工具 SHA-256、每卷与载荷 SHA-256 见 `manifest.json`。`scripts/create-zipx-1418-fixtures.mjs` 可生成新的同结构样本；随机载荷和加密随机数意味着重新生成的哈希不同，回归优先使用这里固定的原卷。

每组为 `.zx01` / `.zx02` / `.zipx`，每卷上限 65536 字节；XZ 方法，`none` 无密码，`password` 使用 AES-256，公开测试密码 `ZipxTest-1418`。包含中文路径、170123 字节跨卷随机内容、说明文本和空目录。生成后由 Bandizip `t` 完整检查通过。不得将这些标准磁盘卷与 `.zipx.001` 通用切片混淆。

原工具 SHA-256：`d69bc4d2605f0ecac20e8cfae11e313fb011649ed7f182cfcb25d6be73dcf4e2`。工具不随产品或样本分发。
