# v1.5.12 ALZ 压缩流完整性与方法边界

从 v1.5.11 发布后主线 `e243a28` 接续，先核实 `tools/alz-engine`、`vendor/unalz`、原生卷和密码验证路径。本版按路线证据取舍规则收口现有 ALZ Deflate 的完整性缺口；Obfuscated Deflate（方法 3）未取得真实正面原包，仍不支持，不以改头普通 Deflate 或模拟编码器替代原厂证据。

## 原计划调查与范围纠偏

[Bandizip 原厂说明](https://en.bandisoft.com/bandizip/help/how-to-open-alz-and-egg-files/)、[XADMaster 方法分派](https://github.com/MacPaw/XADMaster/blob/master/XADALZipParser.m)和 [The Unarchiver 格式说明](https://storage.googleapis.com/google-code-archive/v2/code.google.com/theunarchiver/wiki/AlZipSpecs.wiki)分别列出方法 3 及其随未压缩大小变化的码长表顺序，不能当作本产品已支持。公开 decoder 源码可供后续实现调查，但尚无原厂正面包，故本版不引入或移植其代码及依赖。

原厂 ALZipCon 11.0.1.16（官方 ALZip 12.37.0.4）实际请求三档压缩、普通/中文密码、单包/原生卷共十二组：低、中档八组产物均为方法 2，最高档四次返回 0 但没有归档输出，不能把退出码当成功。`unalz` 作者的既有 CC0 样本没有方法 3；其原作者[历史调查](https://kippler.tistory.com/164)也记录过难以找到创建版本。已只读提取其保存的 ALZip 5.6 安装器和帮助资料，没有执行安装或得到方法 3 正面产物，不据此断言所有旧版本不能创建。新近其他开源库的方法 3 测试使用自建归档，并非原 ALZip 原件，不能替代本路线的原包条件。

## 已复现的真实流程缺口

现有 `extract_deflate` 遇到 `StreamEnd` 就成功返回，未检查已缓冲和未读取的声明压缩输入。由八组原厂归档派生的负面故障组，在 Deflate 结束后插入 5 或 65536 字节并扩展该条声明压缩范围，原载荷大小/CRC 保持。旧随附 helper 对八组全部返回 verified true，这是错误的完整成功。原件和故障派生件分目录、分清单，不把故障组记作原厂产物或兼容证据。

八组真正原件经 Bandizip 7.38 Beta 以 CP936 独立完整回解，全部匹配固定自有载荷；包含中文密码、原生五卷、空文件及空目录。工具身份、参数、哈希与故障变更见[清单](../tests/fixtures/archives/alz-1512/manifest.json)。下一步先保留红灯，再修压缩流完整消耗及最终原因；只有实际内容大小/CRC 已通过时，才把加密条目的尾部残留判为确定损坏并停止重复试密，否则仍保持不确定类别。

本版不增加 ALZ 创建或修复，不改密码默认与 Windows N 边界。封板须覆盖正确/错误密码、原卷补回、新任务与旧失败保留、所选/整包、检查/诊断、转换拒绝及真实暂停取消；常用/窄窗实际查看后才记录通过。完成后受保护合入、固定标签、中文说明及安装包四资产核对，后续按 v1.5.13 TAR 既定目标接续，未验证方法 3 单独保留，不伪记支持。
