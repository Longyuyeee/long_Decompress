# v1.4.16 封板与发布记录

2026-10-17 已封板并[正式发布 v1.4.16](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.4.16)。功能冻结提交 `436e02eb027c501732c21c117c9290f361b00aea`，[PR #171](https://github.com/Longyuyeee/long_Decompress/pull/171) 已合入，正式标签 `v1.4.16` 指向 `5b4c3175a20c6bfa0b98021c024ae98669910079`，文件树与通过检查的最终候选一致。正式安装器、签名更新包、签名和中文更新清单均已回下载核对。

## 冻结范围

- ZIP/ZIPX 明确选择 UTF-8、当前 Windows、CP932/936/950，输入自动与新包 UTF-8 默认不变，拒绝有损转换，不盲试全部编码。
- 创建后的单包/数字卷/标准卷完整校验、解压与候选错误分类、浏览/嵌套、检查/修复及跨编码转换共享同一选择；原包与新包、原包与恢复结果分别保存。
- 单条、多选批量、配置组/模板、草稿/历史接续完整；非 ZIP 按真实格式过滤，未适用项不脱离全局继承，密码不写入草稿。
- 单条及批量救援结果重开、密码重填、浏览和实际再次解压通过；常用/窄窗口下拉和表单实看，保留现有交互结构。

## 最终审计与证据

438 项相关前端回归、Vue 类型检查及普通生产构建通过。未改的后端证据按范围复用：阶段六 125 项和阶段七 33 项不相加；66 个有效创建组合、原始 Info-ZIP 三代码页包、独立 7-Zip 完整回解及前轮桌面流程均保留。

最终接续报告时间 `2026-10-17T06:17:16.183Z`（北京时间 10 月 17 日），测试 EXE SHA256 `1de0bc549d84ff545aa0f42896a5647c753ab6a1f8b3fdb21cf0b7952307f740`。同一隔离 profile 两次原生退出重开，四个真实修复结果独立回解、产品浏览及再次解压通过；[报告和六张截图](evidence/legacy-zip-continuity-1.4.16/results.json)原样归档。该 EXE 为升版前 1.4.15 身份的 E2E 构建，不是正式安装器。全过程和原始失败见[实施审计](LEGACY_ZIP_PASSWORD_ENCODING_1.4.16.md)。

## 发布结果

八处身份已统一为 1.4.16，`node scripts/check-release-identity.mjs --expected 1.4.16 --tag v1.4.16` 与升版后普通生产构建通过。版本化资源为 `long_compress_shell_extension_1_4_16.dll`，SHA256 `9f3d1f1695f81983c36d6ff96c88a67745166b9263f814137db8709022aab007`；旧资源保存在 Git 历史。Windows 11 身份包因本机无签名环境按既有规则跳过，不冒称已生成；本版未改相关安装流程。

1. 最终候选 `651f35238cd0f153f25f06edaeac30a28c84801d` 的 [Windows package / 39009521633](https://github.com/Longyuyeee/long_Decompress/actions/runs/39009521633) 成功。附件 `windows-nsis-installer` ID `12092377333`，20,207,384 字节，GitHub 附件 ZIP 摘要 `8fbf9dd69a8d24551d28149a45d6bbf3e4d073f2ca6a3b17fff82d3bb2adbab2`。
2. 已回下载 `Long解压_1.4.16_x64-setup.exe`，20,221,501 字节，ProductVersion `1.4.16`，SHA256 `b4d56092772377e86e60dda16ee10b63fb97c4135b5a0322c123f6a8c1d857c5`。下载较慢，必需检查通过后并行合入/触发正式构建，随后完成下载核对；不是以附件元数据冒充已检查本地文件。原生下载备选在 CLI 下载成功后停止，未重复计结果。
3. 合入遵守保护规则、未删除开发分支；[正式 Release / 39010637423](https://github.com/Longyuyeee/long_Decompress/actions/runs/39010637423) 成功，发布时间为 `2026-10-17T06:53:06Z`（北京时间 14:53）。安装器 ProductVersion `1.4.16`；更新 ZIP 内的安装器与下载页 EXE 逐字节摘要相同，清单版本、更新地址及签名与实际 ZIP/签名文件一致。中文 Release 和更新清单说明同步后重新下载，两处正文一致，签名与更新地址未改；只替换说明清单，不重新打包或签名。
4. 安装流程本身未改，复用既有安装生命周期证据；本轮不改个人安装、历史或设置。正式资产齐全后以单独文档 PR 同步两份 README、中文说明和当前发布状态，保留原图标、旧标签及证据，不为回填文档重新打包。该文档 PR 按保护规则完成必需检查后合入，不冒用旧候选检查状态。

### 正式资产

以下为发布后实际回下载并与 GitHub 资产摘要核对的文件；候选摘要仅用于上方候选记录。

| 文件 | 字节数 | SHA256 |
| --- | ---: | --- |
| `Long-Decompress_1.4.16_x64-setup.exe` | 20,223,328 | `e27072cc3e490d38d4c3096d5d152959218bfcea18bf387a3e4ab4dd05caff23` |
| `Long-Decompress_1.4.16_x64-setup.nsis.zip` | 20,223,488 | `05bb6f9340aaf5f3e6b3e182e526c55da68e978b717293f8628e84390e204efe` |
| `Long-Decompress_1.4.16_x64-setup.nsis.zip.sig` | 428 | `f34674c0007e5cb9d875424d5bc8d5063d3f07329c78cc365bfd8c8479baa814` |
| `latest.json`（中文说明同步后） | 2,797 | `35d091507cfbf6e455b11fc620a5eef07bcf559d2330ce35ce0b2a3949343685` |

## 下一版

按原路线推进 v1.4.17 特殊分卷兼容：先取得原工具真实生成的好压 AES ZIP、7Z 特殊卷样本，再核实任意成员归组、内容/文件名密码、缺卷提示与补回继续。已有好压 ZipCrypto ZIP 证据复用，不把改名切片当新卷型，不提前宣传未验证能力。
