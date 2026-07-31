# Long解压 1.0.21

本补丁版本修复 v1.0.20 发布后确认的 Windows 传统右键二级菜单问题，并补齐菜单、性能与依赖治理证据。

## Windows 右键菜单

- 传统级联菜单改用 `ExtendedSubCommandsKey` 内联子命令，不再依赖无法可靠解析的 HKCU CommandStore 引用。
- 普通文件、归档文件、文件夹和文件夹空白处四类菜单根共注册 17 条有序命令。
- 状态检测会把旧 `SubCommands` 布局判为过期，并核对每条命令都指向当前安装的可执行文件。
- 安装版与公开更新脚本均执行严格 17 条子命令验证；只存在顶层菜单不能判定为通过。

## 性能与工程质量

- 新增结构化固定机器性能运行器，采集机器指纹、电源计划、Git/工具链、逐次样本和中位数区间。
- 覆盖大文件 ZIP、小文件 ZIP、原生 7Z 和 AES v2 的真实压缩/解压或加解密往返，并校验内容。
- 基线和当前结果必须同机、同配置且双方各至少 10 次样本，才会应用回归阈值。
- Vue、Playwright、Vue Test Utils、GSAP、PostCSS 和 Autoprefixer 已完成同主版本更新；生产依赖审计为 0。

## 发布验收

- [Release v1.0.21](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.0.21) 已由
  [Actions run 30657834110](https://github.com/Longyuyeee/long_Decompress/actions/runs/30657834110) 从发布提交
  `97cc7e63e15dc0870f7603eb13edcacea754043a` 构建并发布，五项发布前 CI 全部通过。
- 已从官方 v1.0.20 通过软件内可见更新界面升级到公开 v1.0.21；21/21 项检查通过，包含签名下载、升级前进程退出、
  新版独立重启、安装目录与两套用户数据保持、唯一 `long_compress_shell_extension_1_0_21.dll`，以及四类传统菜单根的 17 条命令。
- 严格菜单证据为 `contextMenuCascade.valid=true`、`commandCount=17`。Issue
  [#39](https://github.com/Longyuyeee/long_Decompress/issues/39) 与
  [#46](https://github.com/Longyuyeee/long_Decompress/issues/46) 均已完成关闭，正式结果见
  [RELEASE_VALIDATION_1.0.21.md](RELEASE_VALIDATION_1.0.21.md)。

公开资产 SHA-256：

- 安装器：`E82F1A4BC60D1F6A87FBE62B9045F6C85B0C35F054813F03EBD9E80516CC6ADD`
- updater ZIP：`4CA2037D637E1205F34A120CF0D8680DD0BA0069D07848AE04165F703514D353`
- updater 签名资产：`FC96CE376ED106A992EF0269CA48E341176AD37BDE76F5E7A7D80393BB576E1F`
- `latest.json`：`823A61505F443F24DEED69EF6A718370EA49DDC06743B7F99D3DF365C54E72F1`

## 已知限制

当前安装包没有可信 Windows 商业代码签名证书，因此 Windows 11 新式顶层菜单身份包仍不随正式安装器分发；
快捷操作位于“显示更多选项”的传统菜单中。创建 RAR 仍需要本机安装 WinRAR/rar.exe，解压 RAR 不受影响。
