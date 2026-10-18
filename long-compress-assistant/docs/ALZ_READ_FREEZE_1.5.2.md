# v1.5.2 ALZ 封板与发布

## 功能冻结

2026-10-18，最终功能冻结 `ffa0f3f144b2bf2eb6c20f401a8b7a809b610234`，前一步引擎实现 `f9966b8`、用户流程 `be51cc4`。首次候选 `58fb015` 及其 CI `39087626825` 撤回发布资格：复核补上 CP932 字符别名的往返检查，原断言及两项新 helper 测试通过，真实应用确认不会误报密码错误。按批准路线完成 ALZ 真实只读，不提供 ALZ 创建或修复；范围、原工具与异常证据见[实施审计](ALZ_READ_1.5.2.md)及样本 README/provenance。

73 项不同 Rust（主程序 71、helper 2）、15 文件 235 项前端通过。十二组真实原包覆盖 Store/Deflate/DLZ Bzip2、ZipCrypto、原生多卷、CP949/CP936；独立 Bandizip 固定名称与载荷 SHA，转换 ZIP 由独立 7-Zip 回解。原始负向样本不改名。256 MiB 真原包验证实际进程暂停无 CPU 进展、恢复及暂停中取消，取消不发布输出且源包保留。

真实用户流程覆盖文件管理器双击中卷、显式编码、所选提取、历史恢复编码后重新填写密码并整包执行、缺卷最终原因及补回原卷后的新任务、导入多成员去重、转换和诊断。22 张 1100×700/760×520 截图及横向边界记录均实看；三类批量参数、设置与问号说明一并确认。没有放松验收断言。

发现并修复：完整性检查旧入口误走 7-Zip；诊断方法检查晚于密码提示；局部历史丢失来源编码、后缀误作格式；早期缺卷入口缺少处理指引；显式损坏证据被临时路径中的 `eof` 文本误分类。每项均按真实顺序验证，旧错误记录不改写。

## 发布状态

功能已完成，八处身份统一至 v1.5.2。最终同版本真实桌面 UTC `2026-10-18T01:50:41.468Z` 完成，E2E EXE ProductVersion 1.5.2、33754112 字节、SHA256 `2b658e6a3cd5e3c465bc422d9ec662faa0664de46c65754be50a14e86728372b`；最终 ALZ helper SHA256 `484149b815a05358212be29203d7fe625e3f2c22dcdfe32edcefaeedff806c4d`。22 张图与[桌面收据](evidence/alz-read-1.5.2/results.json)入库。这是带测试桥的真实桌面程序，不是正式安装包。旧引擎和旧 E2E 哈希不覆盖最终别名修正。

普通生产 `tauri build --bundles none` 成功，构建前按正常流程重建 v1.5.2 Shell DLL；八处身份与全部随附资源库存校验通过。必需候选、受保护合入、固定标签及正式四资产均已完成，身份见下。个人安装、设置和历史不改动，测试使用隔离工作区，不执行安装升级。

## 最终候选与合入

最终候选 `b87da7cdc5fde852f823bb85444d224b17521379`，必需 Windows package `39088095487` 成功。安装器 ProductVersion 1.5.2、20780058 字节、SHA256 `a48ef0eeaaf3bb16548ea59545c7051205e8856cc226611cd13eff7eb8c64c45`。从候选安装包提取引擎实际读取十二组原包、从中卷所选提取，并核对 124 份随附许可原字节；helper SHA 与最终库存一致。未执行安装，见[候选收据](evidence/alz-read-1.5.2/candidate.json)。

[PR #185](https://github.com/Longyuyeee/long_Decompress/pull/185) 已于 UTC `2026-10-18T02:02:30Z` 受保护合入，合入/标签 `aee07e9f2e65495a9eceb8c1ce55ae310d9b4e4c` 与候选同树 `420ca8c8e281ab27c7b67c1db47bc21f96b8a46d`。合入后主线 CI `39088649765`、正式构建 `39088654460` 均成功。标签固定，不移动。

## 正式发布资产

[v1.5.2 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.2) 于 UTC `2026-10-18T02:17:29Z`（北京时间 10:17:29）发布，非草稿、非预发布。等待正式任务整体成功后发布中文说明，仅修改 `latest.json.notes`，不改变签名、下载地址或发布时间。

正式四资产均回下载，体积与 GitHub SHA256 一致；ProductVersion 1.5.2，更新 ZIP 内安装器与正式 EXE 同字节，签名/地址/日期与清单一致，Release 和清单中文说明一致。包内 ALZ 引擎/124 份许可与库存一致，并实际读取十二组原包及中卷所选提取；未安装。见[正式收据](evidence/alz-read-1.5.2/release.json)，核对时间 UTC `02:18:53.642`。

| 正式资产 | 字节 | SHA256 |
| --- | ---: | --- |
| Long-Decompress_1.5.2_x64-setup.exe | 20774311 | `65e12bf0059e4746c90c6359d7ca559b8744efd6ccbaedeb37a482348f658a1b` |
| Long-Decompress_1.5.2_x64-setup.nsis.zip | 20774469 | `4b30a1ef0adba29d15fc9947bf8dd942be2f353ee13e8746036d3c040b5ae546` |
| Long-Decompress_1.5.2_x64-setup.nsis.zip.sig | 428 | `80a7a9187a69c7ed0247c3c9327eef3cc0d8aa81977299d77e61289e609823a7` |
| latest.json | 3516 | `bc00499ff5f9e42d59bb1832d95d6eab805a575dca22fe7064b5d8d2a742a667` |

正式资产不与候选安装器或 E2E 哈希混用。发布后文档单独受保护回填，不更改功能或标签。
