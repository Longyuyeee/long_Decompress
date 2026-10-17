# v1.4.19 封板与发布记录

2026-10-18 已封板并[正式发布 v1.4.19](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.4.19)。功能冻结 `e6750766ba8b7765dece3d81b76db2fc7b4c564f`，八处身份、普通生产构建及必需 Windows package 通过，PR #177 已合入。标签 `31eb14c` 与候选文件树一致，正式四资产和中文说明均已回下载核对。

## 本版范围

RAR4 单包、旧式 `.rar/.r00` 和现代 `.partN.rar` 的恢复记录、恢复卷及现有任务接续。无密码、内容密码、文件名密码使用真实 RAR 6.24 原卷验证；Rar.exe 7.23 在副本中修复，最终内容校验不变。不新增创建格式，不外推所有 RAR4 历史卷型，不改 RAR5 原有路径。

## 检查与证据

真实桌面首轮 9 组恢复记录修复、6 组缺末卷补回，均由独立 Rar.exe 回解并按 SHA256 比对文件；恢复记录结果另由应用完整解压。真实失败任务完成缺首卷、修复另存、保留原密码/输出设置、重新入队和解压，原失败记录及原卷字节保留。页面还实际创建了 RAR4 恢复卷。

20 项不同 Rust（18 项 RAR 恢复相关及 2 项共用子进程取消）、26 项相关前端通过。最后补充无效恢复卷合并说明，避免重复界面键；真实页面确认两个损坏 REV 都明确说明，缺卷时不出现误导修复入口。

[最终结果](evidence/rar4-recovery-1.4.19/results.json)和 10 张实看截图已入库，完成时间 `2026-10-17T20:55:45.498Z`。程序为隔离 E2E、升版前 ProductVersion 1.4.18，SHA256 `ab6d21cebf23686c5e9570050f3b776abd8f3183c0825cdbd6f9bdfd891a69b1`，不作为正式安装器身份。Rar.exe 7.23 SHA256 `f561764bc3e9ed208744321a89a819b562edeaf06e203c02a06976121fda1991`。实际常用 1100×700 与窄窗 760×520 均已查看；保留弹窗内部滚动，长路径和按钮可读可达。

## 发布接续

八处版本统一 1.4.19，普通生产构建和必需 Windows package 成功，受保护 PR 合入后固定标签发布安装器及更新资产。候选与正式构建分别记录；发布后文档不移动标签或重新打包。

本地普通生产构建及八处身份检查已通过，ProductVersion `1.4.19`，EXE 33063936 字节，SHA256 `ffec09b0dfcef37d5480929b82d26139efacb03d7a4cde7f424d5be7431a8bec`；未启用 `desktop-e2e`。版本化 Shell DLL `long_compress_shell_extension_1_4_19.dll` 的 SHA256 `26af0dc3d57050289b9316ab555aeee65e2f8290288ded6e53e62377387ea962`，旧版生成资源由正常构建替换，可从 Git 历史恢复。未改个人安装。Windows 11 身份包因本机无签名配置按现有规则跳过，不宣称生成。

下一版为 v1.4.20 ZIP 部分抢救扩展；批准路线尚未全部完成，不重复规划。

## 候选与合入

[PR #177](https://github.com/Longyuyeee/long_Decompress/pull/177) 已受保护合入。功能冻结 `e6750766ba8b7765dece3d81b76db2fc7b4c564f`，候选 `502bafb9f5907696901f0d81d45ddd536a3a975d`，正式标签 `v1.4.19` 固定于合入 `31eb14ccac5197f9944ea64f38f4bf94bca72b96`；已核对两者 Git 文件树相同。

必需 [Windows package / 39069011965](https://github.com/Longyuyeee/long_Decompress/actions/runs/39069011965) 成功，实际回下载的 `Long解压_1.4.19_x64-setup.exe` 为 ProductVersion `1.4.19`、20256063 字节、SHA256 `512e494b3e2d3c677dcd9ac8d8ea31dbf6b1b421926074d5cc4eeab9abc7f8c5`。候选附件 ID `12114776495`、20241945 字节、附件 ZIP 摘要 `b825ab259067c121e46b608b5e94b16469d5b652d25523716d4e625ef3d81b9c`。这些是候选构建，不冒充正式发布资产。

[正式构建 / 39069589604](https://github.com/Longyuyeee/long_Decompress/actions/runs/39069589604) 成功，发布时间 `2026-10-17T21:19:29Z`（北京时间 10 月 18 日 05:19）。四项正式资产已回下载；安装器 ProductVersion `1.4.19`，更新 ZIP 内安装器与下载页 EXE 哈希相同，签名及地址与清单一致。中文 Release 与 latest.json 说明一致，清单只替换 notes，版本、签名、地址和发布时间保持。重复主线 CI `39069560700` 已确认取消。

## 正式资产

以下文件大小及 SHA256 均与 GitHub 资产摘要核对，机器记录见 [release.json](evidence/rar4-recovery-1.4.19/release.json)。文档独立接续，不改封板标签或重打包。

| 文件 | 字节数 | SHA256 |
| --- | ---: | --- |
| Long-Decompress_1.4.19_x64-setup.exe | 20261519 | d9a0b1f7d26d01c034b42d980efe351dc92fac430caa4082bcfdb23536b5dd63 |
| Long-Decompress_1.4.19_x64-setup.nsis.zip | 20261679 | f8bcb66d12cbd1dde4a65e439237364933bd9d3f064f07110c7306d28fe3a893 |
| Long-Decompress_1.4.19_x64-setup.nsis.zip.sig | 428 | 332b4502578399d156f92ad6fdc3c1735d615617cd2e9d37a0e291df9002a13d |
| latest.json | 2788 | 545359258bf0b3417df30555bb4bc134f3ad5178f3e9ff9301a6e85eaa22c1e8 |
