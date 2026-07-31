# Long解压 v1.0.20 正式发布验收记录

> 验收时间：2026-07-30 至 2026-08-01  
> 总体结论：**INCOMPLETE（应用内更新通过，传统右键二级菜单未通过）**

## 追溯信息

| 项目 | 证据 |
| --- | --- |
| 发布提交 | `5f4505686a8ea0770b5de5178d9ad6433967fb4e` |
| GitHub Release | [Long解压 v1.0.20](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.0.20) |
| Release Actions | [run 30556569837](https://github.com/Longyuyeee/long_Decompress/actions/runs/30556569837)（成功） |
| 验收 Issue | [#39](https://github.com/Longyuyeee/long_Decompress/issues/39) |
| 公开更新证据 | `test-results/public-update-validation/20260730-234442/result.json` |
| 修复 PR | [#38](https://github.com/Longyuyeee/long_Decompress/pull/38) |

## 正式资产

| 资产 | SHA-256 |
| --- | --- |
| `Long-Decompress_1.0.20_x64-setup.exe` | `D8774FD55E38203F5F0965A7FF5EBC84EE3AD674586D2EAF4BC4F96D2F2F3D96` |
| `Long-Decompress_1.0.20_x64-setup.nsis.zip` | `69B561CB7BF6C681072491F24EA5485ED83ED1E093F6BC105AC6AF37390DF572` |
| `Long-Decompress_1.0.20_x64-setup.nsis.zip.sig` | `FDA7744C900D09D750DF523EE3C0E30CE962343ABBDE1065173D0776532D4965` |
| `latest.json` | `C74C338ECEA7E86715636D601CEE5DF71517540D72A9C78D38095905CE4203C3` |

GitHub Release API、标签、Release 工作流提交和 `latest.json` 均指向 `1.0.20`；updater ZIP URL 与签名存在。

## 已通过项目

- 从公开安装的 v1.0.19 在真实设置页发现并安装 v1.0.20。
- 签名 updater 下载、覆盖安装、自动重启和产品版本切换成功。
- 原安装目录 `E:\long\Long解压` 保持不变。
- 两套用户数据目录 SHA-256 指纹保持不变。
- 安装目录仅保留 `long_compress_shell_extension_1_0_20.dll`。
- 正式安装包没有携带无商业签名的 Windows 11 identity MSIX。
- 原 `test:public-update` 共 18 项检查均返回通过。

## 未通过项目与测试缺口

Windows 11“显示更多选项”中的传统 `Long解压` 二级菜单会为空或消失。原验收只确认级联顶层键存在，
且已安装版本验收只抽查旧 CommandStore 的一个命令，没有证明 Explorer 能解析完整子菜单。因此原来的
“传统右键菜单通过”结论证据不足，不能继续作为发布通过依据。

根因是自定义 `SubCommands` 引用的 CommandStore verb 被写入 HKCU，而微软文档中的自定义 CommandStore
位置是 HKLM。修复改用支持按用户注册的 `ExtendedSubCommandsKey` 内联子命令，并要求验收四类菜单根：

- 普通文件：3 条压缩命令；
- ZIP 等归档：8 条解压、测试和压缩命令；
- 文件夹：3 条压缩命令；
- 文件夹空白处：3 条当前目录压缩命令。

合计 17 条命令必须全部存在、顺序稳定并指向当前安装的可执行文件。修复和严格回归见 PR #38。

## 判定与后续动作

v1.0.20 的 Release 资产、签名更新、自动重启、安装路径和用户数据保持验收通过；传统右键二级菜单失败，
因此总体状态为 `INCOMPLETE`，Issue #39 保持开放。只有下一补丁版本交付修复，并从 v1.0.20 执行真实
应用内更新后得到包含 `contextMenuCascade.valid=true`、`commandCount=17` 的新证据，才能关闭该问题。

以后可运行以下命令生成可追溯摘要；旧的顶层键证据会被明确判为 `INCOMPLETE`：

```powershell
npm.cmd run release:validation-report -- `
  -Version 1.0.20 `
  -PreviousVersion 1.0.19 `
  -ReleaseRunId 30556569837 `
  -ValidationIssue 39 `
  -EvidencePath test-results\public-update-validation\20260730-234442\result.json `
  -OutputPath test-results\release-validation\v1.0.20.md
```
