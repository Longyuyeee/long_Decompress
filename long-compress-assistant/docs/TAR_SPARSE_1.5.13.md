# v1.5.13 TAR 稀疏布局与真实内容

从 v1.5.12 发布后主线 `794fcc9` 接续，按十版路线补标准 TAR 的旧 GNU、PAX 0.0、0.1、1.0 稀疏只读兼容。普通 PAX 中文长路径已有能力，不重复宣传为新增。沿用默认设置，不创建链接或设备项，不改变密码机制及自有加密容器格式。

## 原始需求与实际缺陷

`native_extraction/tar.rs` 和预览用 `EntryType::is_file()` 过滤，旧 GNU 稀疏条目因此被跳过。`tar 0.4.44` 已解码旧 GNU 空洞，却不解码 GNU PAX 稀疏映射；直接读取后者会得到压紧内容或 `GNUSparseFile` 临时名称。浏览、提取、所选、检查和转换必须使用同一逻辑内容视图，不能只让列表看起来正确。

原始代码的四种稀疏布局均在真实原包单元测试失败，普通 PAX 长路径通过。见[红灯](evidence/tar-sparse-1.5.13/red-unit.txt)，不把首个失败后未执行的压缩变体算作红灯证据。旧 GNU 的 `header.size()` 已返回逻辑大小，不将它误判成新缺陷。

## 原包与独立回解

`scripts/prepare-tar-1513.py` 通过 WSL GNU tar 1.35，从自有固定的多段稀疏文件（8 MiB + 33 字节）、全零文件、空文件、中文长路径和末条内容生成五布局、TAR/Gzip/Bzip2/XZ 共二十原包，另有含普通文件、硬链接、符号链接和 FIFO 的原包。来源、原工具身份、命令和全部内容哈希见[固定清单](../tests/fixtures/archives/tar-1513/manifest.json)。四个异常件独立标注为派生故障，不能充当正面原包。

GNU tar 原工具回解及 Linux libarchive 3.7.2 独立回解，所有逻辑大小、完整内容哈希与中文名称匹配，见[独立收据](evidence/tar-sparse-1.5.13/originals.json)。后者来自 Ubuntu noble 的 `libarchive-tools`/`libarchive13t64` 3.7.2-2，仅解包至忽略的 `test-results/tar-1513/linux-oracle`，未安装或随产品分发。初次索引中的更新包已失效，使用仓库仍可取得的基础版本；未以下载失败计通过。

Windows bsdtar 默认本地代码页会误解旧 GNU 中文名称；显式 UTF-8 后仍有长名转换告警。7-Zip 26.04 对本组 PAX 0.0 只回解压紧数据；CPython tarfile 的 PAX 0.1 名称仍留临时占位路径。这些探查不计完整互通通过，也未更改来源名称或内容来迎合工具。最终独立验收使用 Linux libarchive，失败探查日志留在本机忽略的 `test-results/tar1513-*`。

## 实施与封板条件

新增共享逻辑读取适配层，重用当前 tar 库对普通 TAR、GNU 长名和旧 GNU 稀疏的实现，按公开规格解读 GNU PAX 稀疏映射与真实名称。有界流式填零，不整文件缓冲；检查映射块数、顺序、边界和声明存储量，未知版本明确不支持。PAX 长名、尾部压缩流校验、事务输出、所选和失败历史一起核对。遇到本次选择中的链接或设备等条目明确失败，不静默遗漏；普通文件可从浏览器单独提取。

封板需要二十原件完整内容与所选、四派生故障分类和无半成品、正常/窄窗实际页面、真实暂停恢复取消、定向回归与普通生产构建。之后按受保护候选、固定同树标签、中文 README/Release、安装包四资产回下载及文档回填完成发布。ALZ 方法 3 与 EGG 加密固实的未完成状态保持，不因新版本发布而消失。

## 规格

- [GNU PAX 0.0 与 0.1](https://www.gnu.org/software/tar/manual/html_node/PAX-0.html)：重复偏移/长度或逗号映射、逻辑大小与 GNU.sparse.name。
- [GNU PAX 1.0](https://www.gnu.org/software/tar/manual/html_node/PAX-1.html)：正文前置十进制映射、块填充、真实名称与逻辑大小。

规格核对日期：2026-10-19。尚未完成封板或发布时，不把上方条件记为已完成。
