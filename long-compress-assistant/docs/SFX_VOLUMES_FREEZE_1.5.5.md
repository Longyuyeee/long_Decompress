# v1.5.5 外置卷 SFX 封板与发布

## 功能与证据

从已发布 v1.5.4 的主线 `479dcab` 接续。实现 `e2d38a9`、末轮真实页面纠偏 `cab418c`，最终功能与测试冻结 `f5e7050c0d970ce3f9f8ff3b9179081590c17eb4`。真实 RAR4 单 EXE、旧式 EXE/rNN、RAR4/5 新式 part1.exe/partN.rar 和固定 7-Zip 24.09 GUI 外置加载器读取完成；普通、内容密码和文件名密码按十五组原工具样本分别核对。范围及换机入口见[实施审计](SFX_VOLUMES_1.5.5.md)。

94 项不同 Rust、十文件 214 项前端及类型检查通过，另 7 个 ignored 不计成功。最终桌面于 UTC `2026-10-18T06:05:41.835Z` 通过 24 场景、18 张常用/窄窗页面；实看后修正旧浏览错误残留与 SFX 不支持的恢复卷按钮。中卷实际双击、所选提取、EXE/数据卷去重、缺首提示、缺中卷补回新任务、真实错密码失败再纠正、历史重做、转换 ZIP 独立回解及只读诊断通过。见[机器证据](evidence/sfx-volumes-1.5.5/results.json)和[原生控制日志](evidence/sfx-volumes-1.5.5/native-control-logs.json)。

256 MiB 原包先实际写入 20971520 字节，再暂停无增长、恢复增长和暂停中取消；源卷哈希保持，无最终或暂存输出。MSVC UnRAR 缺少 C++ 异常展开导致取消后句柄泄漏，已固定原发布 `unrar_sys 0.5.8` 源码并启用 `/EHsc`，新增立即删除部分输出的真实回归。未保留无效的延迟重试方案。

E2E ProductVersion 1.5.5，SHA256 `f0ecaeadf443f5f685f16e2a0e44999095a62b0f044192b7dad81a8cb350d271`，不是候选或正式安装器。测试输出目录缺失造成的早期假阳性已撤回，最终错密码场景明确排除该原因；没有弱化断言或向生产程序加入验收捷径。

## 候选与发布状态

八处版本已统一 v1.5.5，最终普通生产构建（不带 E2E）成功，资源身份检查通过。最终候选 `b0918a090f424ca3299d7e68040e99f9cb74c2bd` 的必需 Windows package `39103656875` 成功。安装器 ProductVersion 1.5.5，20933324 字节，SHA256 `f18928f41813d4b1faa9ad6b85f60d5a757c78ea1c0d052ed234e6e646e5a79a`；包内中文主程序、原始 UnRAR 许可/来源说明按字节核对。既有 EGG helper、103 份许可、35 组原包及中卷/固实所选回归通过。见[候选收据](evidence/sfx-volumes-1.5.5/candidate.json)。首候选 `e46dc41` 构建成功但中文成员检查脚本失败，后续修正、重建并核对，不算最终候选。

[PR #191](https://github.com/Longyuyeee/long_Decompress/pull/191) 于 UTC `2026-10-18T06:32:11Z` 受保护合入。标签 `v1.5.5` 固定在 `739e5e59f6d02ebe0fbf5f61eaa0ac85c2fd6905`，与最终候选同树 `a7a00d9f7db5346f3fa87773db6e605fcda35c31`，不移动标签。正式构建和四资产以完成后的回下载收据为准。不执行安装升级，个人安装、设置和历史保持。

## 正式发布资产

主线 CI `39104385580` 与正式工作流 `39104426802` 均成功。[v1.5.5 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.5) 于 UTC `2026-10-18T06:47:55Z`（北京时间 14:47:55）发布，非草稿、非预发布。工作流整体成功后更新中文 Release 和 `latest.json.notes`，其余签名、地址及发布时间保持原值。

UTC `2026-10-18T06:49:38.680Z` 正式四资产回下载核对通过：安装器 ProductVersion 1.5.5，更新 ZIP 内安装器与正式 EXE 同字节，签名/地址/时间与清单一致，Release 与更新清单中文说明一致。包内主程序版本及 UnRAR 原始许可/来源说明按字节核对，既有 EGG 引擎/103 份许可及 35 组原包回归通过。见[正式收据](evidence/sfx-volumes-1.5.5/release.json)。SFX 原生运行证据来自前述明确身份的 E2E 程序，不把包检查或 EGG 回归称为正式 SFX 主程序运行；未执行安装升级。

| 正式资产 | 字节 | SHA256 |
| --- | ---: | --- |
| Long-Decompress_1.5.5_x64-setup.exe | 20941840 | `6ac04c3f9bf561eacf5ddae0f40b571d897e398d19ee07863af695980c3bebe0` |
| Long-Decompress_1.5.5_x64-setup.nsis.zip | 20941998 | `1ad5513e9ba161c5114697253a77f024ecf58045271669f90fd91d3c9d8c298b` |
| Long-Decompress_1.5.5_x64-setup.nsis.zip.sig | 428 | `ac420fe27d636556ff64aee58f60ab8c80d8080b9e2f0002a7449c0944eebab8` |
| latest.json | 3181 | `da152ccbd6d23ee155cc175a0975d99e3528b314552abdc1e8ac7b1adc036f39` |

候选、E2E 和正式安装器各自记账，不混用哈希。发布后只回填文档和收据，单独受保护合入，不改功能、不移动标签。

## 支持边界与下一阶段

SFX 仅只读归档，不执行输入 EXE，不创建或修复 SFX，不处理其恢复卷，不承诺任意自解压程序。RAR4 是原 RAR 6.24 产生的旧格式，不等于所有历史模块；独立 7Z 加载器仅本次固定 24.09 GUI 模块。加密 RAR 头隐藏最终卷数时不靠连续卷名猜完整，后续原生完整读取继续验证。RAR 进度目前仍是输出长度估算，精确实际字节进度接下一版。

v1.5.5 已发布，原十二版路线全部交付，按[后续十版目标](ARCHIVE_ROADMAP_1.5.6_TO_1.5.15.md)接续。Windows N 暂不保证支持，智能目录与 ZIP 并行保持默认关闭。
