# Long解压 v1.2.8 发布审计

日期：2026-09-04

正式提交：`master@bdaf708228d0485b5fdb211c1b17100c9b9feedd`

标签：`v1.2.8`

## 发布范围

- 归档显示顺序与真实执行顺序统一。
- 并发上限、同目标安全串行和全局/单独配置来源明确化。
- 单项/全部暂停、恢复、停止形成真实后端控制闭环。
- 密码尝试、完整性校验、最终发布和外部进程纳入暂停检查点。

## 关键预期—实际

| 门禁 | 预期 | 正式实际 |
| --- | --- | --- |
| 原生暂停 | 暂停后工作线程阻塞，恢复继续，停止可唤醒 | Rust 2/2 通过 |
| 外部进程暂停 | Windows 子进程暂停期间输出字节不增长 | 真实 PowerShell 子进程通过 |
| 桌面控制 | 单项/全部暂停、恢复、停止与后台状态一致 | 真实 Tauri/WebView2 通过 |
| 停止清理 | 停止后任务取消，临时输出删除，队列可接续 | 真实桌面门禁通过 |
| 配置与队列 | 自然顺序、并发上限、来源切换无回归 | 前端完整回归通过 |
| 归档主流程 | 并发、遥测、串行提交、加密往返保持正确 | archive-flow 真实门禁通过 |

## 候选验证

- Vue / TypeScript 类型检查：通过。
- 前端单元：53 文件、303/303 通过。
- Rust 严格 Clippy：`--all-targets --all-features -- -D warnings` 通过。
- Rust 全量：主库 389 通过、10 项专用环境测试明确忽略；全部常规集成测试通过。
- 密码与加密归档聚焦回归：48 项主库测试及 ZIP、7Z、保险箱、词表矩阵通过。
- 生产前端构建：通过，正式构建不包含桌面 E2E bridge。
- 八处版本身份与版本化 Shell 扩展：`v1.2.8` 一致并通过身份门禁。
- 正式 NSIS 候选：通过；生产前端不含测试桥，经典资源管理器菜单与版本化 Shell 扩展已随包，未配置签名时按既有契约跳过 Windows 11 身份包。

## PR 与正式工作流

- [PR #121](https://github.com/Longyuyeee/long_Decompress/pull/121) 在五项检查全绿后合入 `master`：前端、Rust/壳扩展、浏览器 E2E、真实 Windows 桌面 E2E、Windows 安装器均通过。
- 合并提交为 `bdaf708228d0485b5fdb211c1b17100c9b9feedd`；annotated `v1.2.8` 标签精确指向该提交。
- [Release workflow 33837785345](https://github.com/Longyuyeee/long_Decompress/actions/runs/33837785345) 通过版本身份、类型检查、前端测试、Release Rust 全量测试、Tauri/NSIS 打包和 updater manifest 校验。
- [v1.2.8 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.2.8) 已公开，状态为非草稿、非预发布。

## 本机候选产物

| 产物 | 字节 | SHA-256 |
| --- | ---: | --- |
| `Long解压_1.2.8_x64-setup.exe` | `19,437,215` | `114999A707DA2910ED8BFF849CAE89DE2C7E9BEB23CC7F9C443DE571B1E66184` |
| `Long解压.exe` | `29,914,624` | `35CC439E052BD4E5684A0FB2544FD2474714B962D46B86CBD00FD724AA8EB1D0` |
| `long_compress_shell_extension_1_2_8.dll` | `246,784` | `1AE330BEA48A9B803AE5CC0EDC61F43BC2ABF6CCDB269E54F64BEC06BD822861` |

本机候选只用于身份、可构建性和哈希审计；正式 updater 与公开资产必须由标签触发的干净 GitHub Actions Runner 重新生成。

## 公开资产回下载

| 公开资产 | 字节 | SHA-256 |
| --- | ---: | --- |
| `Long-Decompress_1.2.8_x64-setup.exe` | `19,369,686` | `F3EE04952FA8442E102F261BD3D5C294DAFA001263982CB0EA846180D942E22B` |
| `Long-Decompress_1.2.8_x64-setup.nsis.zip` | `19,369,844` | `083E906C26E14F5EF6217525CE197EA5941E8DEE65695A1EA1BE771C9B209FD8` |
| `Long-Decompress_1.2.8_x64-setup.nsis.zip.sig` | `428` | `2C4063B6E2670D58B59D04FFD91C5B04438D19AAC6C9AB769256E29CC509C020` |
| `latest.json` | `954` | `F712D5E8AB874FF3B6C5BAE8DC125C90A91462F2A2A7319B380FB26ADC758DA2` |

四项资产已从公开 Release 回下载并重新计算哈希；结果与 GitHub 资产摘要一致。`latest.json.version == 1.2.8`，Windows x86_64 URL 精确指向本标签 updater ZIP，manifest 内签名与独立 `.sig` 的 428 字节内容一致。

## 明确边界

- 原生 RAR API 不能安全打断单个条目内部读取，因此在条目边界暂停；使用外部 RAR/7-Zip 时由 Windows 系统级挂起即时暂停。
- 图片、视频、PDF 继续使用各自取消协议，本版本不伪造其可恢复暂停能力。
- Windows 11 第一层菜单仍需要生产代码签名身份；未配置签名时只构建经典菜单。
- 本机未执行会替换用户正式安装的安装/升级生命周期；本项不冒充已通过。公开资产已由干净 GitHub Actions Runner 从 `v1.2.8` 标签重新生成并完成回下载对账。

## 正式发布状态

`v1.2.8` 已正式公开：PR、五项 CI、合并、annotated tag、Release workflow、四项公开资产和 updater 清单对账全部闭环。本版本发布阶段关闭，下一轮开发从最新 `master` 开始。
