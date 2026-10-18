# v1.5.1 ZPAQ 创建封板与发布

## 功能冻结

2026-10-18，最终功能冻结提交 `f6f2d9222dc754873d1a76954761d396fda2d78a`。前一步实现 `b88bd4f`；首次冻结 `f0e5106` 及候选 `235ea65` 已撤回封板资格：复查发现暂停只阻止发布，没有暂停实际子进程，现已修复。按批准路线完成新日志式 ZPAQ、原生密码、配置复用、历史再执行与转换输出；优先补 Windows 写入错误传播，不扩张固定大小分卷、增量备份或历史恢复。

23 项不同 Rust、九个前端文件 128 项、18 项受控 I/O 故障、七组真实桌面流程通过。新增心跳基线先复现暂停仍写入，再验证停止、恢复及暂停中取消；最终桌面流程亦验证实际编码暂停。12 张常用/窄窗页面已实际查看，保存配置弹窗超高及主题配色已修复。作者原版 ZPAQ 独立回解与已知载荷通过，固定产品产物、JSON 与 PNG 均入库，见[实施审计](ZPAQ_CREATE_1.5.1.md)。

最终 E2E 完成 UTC `2026-10-18T00:16:14.227Z`，程序 ProductVersion 1.5.1，33497600 字节、SHA256 `5fc167fbefd909a465fd25de07f135245ec1e0f94266862059ef77fb9717b0dc`。这是带测试桥的真实桌面程序，不是正式安装包；旧 E2E 证据已被本轮替代。一次快速点击后格式仍为 ZIP 的失败截图保留，脚本改为等待动画并确认实际选中标签，未放宽断言或改产品设置。随附引擎 494592 字节，SHA256 `5ad239810009dc499c79159c60f8cd2e61764b9bb88f43dc0eb6d242846c937b`，源与二进制锁定检查通过。

## 发布进展

八处版本身份已统一为 1.5.1，最终修复后的普通生产构建及 `check-release-identity.mjs --expected 1.5.1` 成功。生产 EXE 33501696 字节、ProductVersion 1.5.1、SHA256 `aed6ef7b06d32266f00d556446edd2c53eca5b8d79db0e98b0ee79b3986c18b8`；版本化 Shell DLL 248832 字节、SHA256 `9753df2083773e68e5e79d110e06844f5cf9febbf2049efeeecc4d756b6deffe`。Windows 11 菜单身份包因本机未配置签名环境按既定策略跳过，未记为生成成功。

最终候选 `7324e1befaffe749c25a3b0b42a87fbd4ed3367a` 的必需 [Windows package / 39082434311](https://github.com/Longyuyeee/long_Decompress/actions/runs/39082434311) 成功。[PR #183](https://github.com/Longyuyeee/long_Decompress/pull/183) 已受保护合入，合入及 v1.5.1 标签固定于 `3508c68b7cf943d60d1425235dbcabdabbf6a03c`；候选与标签文件树同为 `03ec5bf35d6c41331816ef29c93909249295b33b`。旧候选构建 `39081172773` 不覆盖暂停修复，不算最终证据。同树重复 master CI `39082869245` 已取消，不影响已通过的必需候选。

候选安装器 ProductVersion 1.5.1、20536025 字节、SHA256 `f07bbd61ec61fd1a604130866dd7d81eeb9bcd92f529909841105cb0945ff651`。实际从 NSIS 提取引擎与许可证核对锁定哈希，读取普通/中文密码原包，并使用包内引擎创建普通和原生密码 ZPAQ，经作者原版工具独立回解已知载荷通过，见[候选收据](evidence/zpaq-create-1.5.1/candidate.json)。未执行安装器。

正式 [Release 构建 / 39082879858](https://github.com/Longyuyeee/long_Decompress/actions/runs/39082879858) 全部成功，[v1.5.1 已正式发布](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.1)。发布时间 UTC `2026-10-18T00:40:07Z`（北京时间 10 月 18 日 08:40:07）。正式四资产均已回下载，与 GitHub digest/字节数一致；安装器 ProductVersion 1.5.1，更新 ZIP 内 EXE 与独立安装器相同，清单签名、下载地址、发布时间及中文说明一致。仅替换更新清单 notes，未改签名、地址或安装包，见[正式收据](evidence/zpaq-create-1.5.1/release.json)。

正式 NSIS 也已实际提取引擎与许可证，锁定哈希一致；包内引擎读取普通/中文密码当前文件，以及创建普通/密码 ZPAQ 后由作者原版工具独立回解通过。没有执行安装升级，不将候选或带测试桥的 EXE 当成正式安装包。发布后独立文档 PR 仅回填 README、审计和收据，不改产品、重打包或移动标签。

个人安装、设置和历史未改。生产构建将旧版本生成的 Shell DLL 换成 1.5.1 资源，旧资源可从 Git 历史恢复。后续按原路线进入 v1.5.2 ALZ，只读范围需先核实真实引擎、方法、密码及原生卷样本；本路线仍有四版，不重复规划。

## 正式资产

| 文件 | 字节 | SHA256 |
| --- | ---: | --- |
| Long-Decompress_1.5.1_x64-setup.exe | 20541813 | `2f66634cc8a1e8c2b2bee0557c84516f0cc49eafd469645d23a70e1c57db00c0` |
| Long-Decompress_1.5.1_x64-setup.nsis.zip | 20541971 | `6a184e25cf36d69111372809ceb8266f395f0d34da6d586934a68aab30a65150` |
| Long-Decompress_1.5.1_x64-setup.nsis.zip.sig | 428 | `f9d1988d1665fc7a14842911354061fe4fb3d0cd99d95d559540f57dfcb564aa` |
| latest.json | 3113 | `bb154a8c8f5e7b7d13f625e4c81867d57c5959db725328c368aeb16e33ffa6a2` |

以上为正式资产，不与候选安装器哈希混用。
