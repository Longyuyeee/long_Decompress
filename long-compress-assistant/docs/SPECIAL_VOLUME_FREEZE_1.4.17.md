# v1.4.17 封板与发布记录

2026-10-18 已封板并[正式发布 v1.4.17](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.4.17)。功能冻结提交 `837683b10b58a8f4fe53cfcf0e29eb5702f103c0`，原生实现提交 `d8890ab`。八处身份统一为 1.4.17，普通生产程序及前端类型/构建、版本化 Shell 资源和身份检查通过。受保护主线 PR #173 已合入，标签为 `3781178f8f5fb06476d93eab29eef48debc023f8`，与通过检查的候选文件树一致；正式四项资产及中文说明已回下载核对。

## 冻结范围

好压原生 `.haozipNN.7z` 内容密码/文件名密码的任意成员识别、乱序去重、原生跨卷浏览与解压、完整性检查、转换和独立回解；缺首/中卷精确列名，缺尾/截断/起始头 CRC 在密码解码前分类，补卷新任务接续并保留独立目录及原失败记录。前轮普通 7Z 浏览预检、具体原因与窄窗卡片修正一并交付。

用户明确要求自行判断此前反复询问的测试及范围选择。已接受自有测试文件容量/兼容提示并固定真正 GUI 原卷；本机原工具没有可验证的 AES ZIP 生成入口，因此本版不承诺该论坛组合，未来取得原卷再验证。保留现有真实 ZipCrypto 论坛 ZIP，不扩大 7Z 预览、特殊卷创建或缺卷恢复承诺。

## 验证与复用

38 项不同 Rust 定向检查、7 个文件的 146 项相关前端回归通过（两组 108 与 38 无重复）；前轮普通 7Z 浏览和其余未改前端证据按范围复用，不重复相加。实际桌面程序 SHA256 `742aa5df740a861651e4abe3fe14efb4f310cfcc9f4116f40ae769dee1b257e7`；两组各三成员完整链与 30 次结构失败/历史原因、真实页面密码纠正及缺中卷补回完成通过。[报告及六张截图](evidence/haozip-forum-7z-1.4.17/results.json)的时间为 `2026-10-17T18:45:55.354Z`（北京时间 10 月 18 日）。该 E2E 程序保留升版前 1.4.16 身份，不是正式安装包。

原卷、生成器、独立工具及载荷哈希见[固定样本](../tests/fixtures/archives/haozip-forum-7z-1417/README.md)，真实失败和脚本修正见[实施审计](SPECIAL_VOLUME_COMPATIBILITY_1.4.17.md)。个人安装、历史、设置未修改，未改安装生命周期及旧格式矩阵按范围复用。

## 发布接续

普通生产 EXE ProductVersion 为 `1.4.17`，33,052,672 字节，SHA256 `de880fc24be1d4309618f61eb3f073d40f22fe771c37cfae4bd6406f92a0fc33`。版本化 Shell 资源 `long_compress_shell_extension_1_4_17.dll` 的 SHA256 为 `9d643ea0615e2031dbe47dd15ea504337947e8970852a1c4a56ebcff05aac0e4`；旧 1.4.16 生成资源由构建脚本替换，可从 Git 历史恢复。本机没有签名配置，Windows 11 身份包按既有规则跳过，不冒称已生成。

候选安装器通过必需 Windows package 后实际回下载核对，再合入并发布标签；正式资产齐全后同步中文说明并再次下载更新清单。发布状态与两份 README 的回填使用独立文档 PR，不移动标签，不重新打包或签名。

## 合入与候选安装器

[PR #173](https://github.com/Longyuyeee/long_Decompress/pull/173) 已合入，正式标签 `v1.4.17` 指向 `3781178f8f5fb06476d93eab29eef48debc023f8`，文件树与候选 `98c0fbfb6b94583dfc9c176f1f005f858b47709f` 完全一致。未绕过保护规则或删除开发分支。

必需 [Windows package / 39059859421](https://github.com/Longyuyeee/long_Decompress/actions/runs/39059859421) 成功。附件 `windows-nsis-installer` ID `12110539942`，20,212,223 字节，附件 ZIP 摘要 `0add5033601df7b77e75046552ce8fe576df791c31b5ced9b1f83b363e83e879`。实际回下载的 `Long解压_1.4.17_x64-setup.exe` 为 20,226,340 字节，ProductVersion `1.4.17`，SHA256 `d994adf08e90fafdd36d66e55b2064bd50c4b8d429150b67af6f57baafaed71b`。

[正式 Release / 39060508551](https://github.com/Longyuyeee/long_Decompress/actions/runs/39060508551) 成功，发布时间 `2026-10-17T19:14:28Z`（北京时间 10 月 18 日 03:14）。正式安装器 ProductVersion 为 `1.4.17`；更新 ZIP 内的 EXE 与下载页安装器哈希相同，更新清单的版本、地址及签名与实际文件一致。中文 Release 正文与重新下载的 latest.json 说明一致，签名、版本、更新 URL 和发布时间未改，仅替换说明清单。候选与正式构建分别记录，不互相冒充；重复主线 CI 已取消。

## 正式资产

以下文件已实际下载，并与 GitHub 资产大小及摘要核对；更新清单为中文说明同步后的最终文件。

| 文件 | 字节数 | SHA256 |
| --- | ---: | --- |
| Long-Decompress_1.4.17_x64-setup.exe | 20232120 | c70bdbcb7ac2fc06a1c9e4a0c6af59cd3ffcf5bb43323574e0400116136c91df |
| Long-Decompress_1.4.17_x64-setup.nsis.zip | 20232280 | 23c290d41c5010b2b55f270b26cc3e27e21dc52f25425036de759bb18e176b05 |
| Long-Decompress_1.4.17_x64-setup.nsis.zip.sig | 428 | a7e71697b27203033efe8482bf655f645f81ec553fb583a494a821d67c33ac99 |
| latest.json | 2533 | c0c89d5190cddc8019028f19a6ae7c73e9a34705eb6867a9fa02463373b534c5 |

## 下一版

按已批准路线接续 v1.4.18 ZIPX 标准多卷：先取得真正 `.zxNN/…/.zipx` 原卷并明确方法/密码边界，再接任意成员读取、创建、转换和重新分卷及缺卷补回。当前 ZIPX 数字卷继续保留，不把数字切片改名冒充标准多卷；尚未开始新版本实现，不需要重跑本版已完成矩阵。
