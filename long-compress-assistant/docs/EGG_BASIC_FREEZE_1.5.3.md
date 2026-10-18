# v1.5.3 EGG 基础读取封板与发布

## 功能冻结

2026-10-18，从发布后主线 `9efeb92` 接续。初审 `1fdee8c`、实现 `8341b5d`，最终功能冻结 `d3d0a097c34600dde536ac0d5cc4b95e953c3b58`。初冻结 `b57e9dd` 和首候选 `1be7878` 撤回后，补齐损坏头部在独立密码验证入口的原生路由，旧 CI `39092376645` 取消，不作为最终证据。完成真实 EGG 基础只读、原生卷、密码编码和用户闭环，范围见[实施审计](EGG_BASIC_1.5.3.md)。不提供 EGG 创建、AZO/固实/AES/LEA、文件名加密、损坏修复或缺卷部分救援。

77 项不同 Rust（主程序 73、helper 4）、十文件 140 项前端通过。十七组原始归档的文件名、字节数和 SHA256 与独立 Bandizip 固定结果一致；真实用户转换的 ZIP 另经独立 7-Zip 回解。确认原 ALZip 中文文件名是 UTF-8，但中文密码用本机 CP936，因此明确分开两者，不做自动猜测。

最终真实桌面 UTC `2026-10-18T03:10:55.345Z` 完成，27 场景与 26 张 1100×700/760×520 页面。相同 UI 的首轮 26 张已逐一查看，后端入口修正后的最终重跑补查页面。文件管理器打开中卷、所选提取、密码纠正、历史重做、缺卷补回、导入去重、转换/诊断与批量/设置均走真实入口；保留旧失败，密码不进历史。256 MiB 原包验证实际暂停无 CPU 进展、恢复与暂停中取消；源文件保留且无已发布/暂存输出残留。见[桌面收据](evidence/egg-basic-1.5.3/results.json)。

最终 E2E ProductVersion 1.5.3，SHA256 `49241014031a085cac0692e799cc176daf05b7b8e119cb4853fa167300c562c0`；运行时实际 helper 与库存一致，790528 字节，SHA256 `533cd527d52e92ea896347d9c0ba662ee4107cc264a65628e76dd8d90ff7c6e4`。E2E 程序不是正式安装器，初轮旧版本/旧 helper 不覆盖最终边界修正。

普通生产 `tauri build --bundles none` 成功，八处版本身份与所有随附资源库存检查通过。正常构建用 v1.5.3 Shell DLL 替换生成的旧版本资源，旧资源可从 Git 历史恢复。个人安装、设置和历史不改动，没有执行安装升级。

## 最终候选与合入

最终候选 `c5beb80315d400e642cbc66593433e84125da3b6`，必需 Windows package `39092798929` 成功。安装器 ProductVersion 1.5.3，20854650 字节，SHA256 `915da8bb0aecf88a03ea25a7c87eded76da05b0c0f49fd5dac0a0596b993596e`。从安装器提取 EGG 引擎及 67 份许可，逐字节库存核对并实际读取十七组原包、中卷所选提取成功；未执行安装，见[候选收据](evidence/egg-basic-1.5.3/candidate.json)。

[PR #187](https://github.com/Longyuyeee/long_Decompress/pull/187) 于 UTC `2026-10-18T03:21:59Z` 受保护合入。合入/标签 `d31e16b658b11bd93376fba9e79466453478930c` 与最终候选同树 `bf23aa49d7e258eed8967dfa196fbebc66967d34`，固定 `v1.5.3`，不移动标签。主线 CI `39093363899` 与正式工作流 `39093382228` 均成功。

## 正式发布资产

[v1.5.3 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.3) 于 UTC `2026-10-18T03:32:33Z`（北京时间 11:32:33）发布，非草稿、非预发布。等待正式任务整体成功后更新中文 Release 和 `latest.json.notes`，其余签名、下载地址及发布时间保持原值。

UTC `2026-10-18T03:34:02.991Z` 四资产回下载核对通过，体积及 SHA256 与 GitHub 一致；安装器 ProductVersion 1.5.3，更新 ZIP 内安装器与正式 EXE 同字节，签名/地址/时间与清单一致，Release 和清单中文说明一致。包内 EGG helper 与 67 份许可按库存核对，实际读取十七组原包及中卷所选提取成功，未执行安装。见[正式收据](evidence/egg-basic-1.5.3/release.json)。

| 正式资产 | 字节 | SHA256 |
| --- | ---: | --- |
| Long-Decompress_1.5.3_x64-setup.exe | 20855154 | `7877b8781e9abf969c243f1342e88233c2d3724ddda8ac4eae5715f0af53b58f` |
| Long-Decompress_1.5.3_x64-setup.nsis.zip | 20855312 | `e2fe857f2a0515e6c9faa22e74fa02bd0ab342104b88073734c991b64c658e8f` |
| Long-Decompress_1.5.3_x64-setup.nsis.zip.sig | 428 | `3a844c37a1c13974ccfbafb75555b346a4464d6ae16114d53000f2e725f39239` |
| latest.json | 2899 | `669d53cd7579c246157af7030f1a676cc3afc0b8f1ad78fab0fabd21d3e15f72` |

不混用正式包、候选安装器和 E2E 哈希。发布后只回填两份 README、审计/交接和收据，单独受保护合入，不改功能或移动标签。

十二版路线在本版发布后为十版交付，接 v1.5.4 复杂 EGG 和 v1.5.5 外置卷 SFX；不是全部路线完成，不重复规划。
