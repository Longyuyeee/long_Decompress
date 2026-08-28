# Long解压 v1.1.15 发布审计

审计日期：2026-08-28（Asia/Hong_Kong）

状态：**候选已就绪，尚未创建标签或公开 Release。**

## 结论

图片压缩 B-01 至 B-05.3 与其依赖的 S-00、B-00 基础节点已完成。版本身份已统一提升为 `1.1.15`，Release notes、本地正式 NSIS、完整回归及 `v1.1.14 → v1.1.15 → 卸载 → v1.1.14` 真实安装链均已完成，候选允许提交并推送开发分支。签名 updater 与公开应用内更新只能在发布提交合入、GitHub Actions 使用仓库 Secrets 生成四项资产之后复验，不以本地无签名产物代替。

## 需求对齐

| 范围 | 实际结果 | 判定 |
| --- | --- | --- |
| 图片输入与设置 | JPEG/PNG/WebP；格式、质量、缩放、元数据和冲突策略 | 已完成 |
| 生产编码与复验 | 真实编码、重新解码、格式/尺寸/方向/Alpha/元数据检查 | 已完成 |
| 统一任务与历史 | `compression/image`、阶段日志、取消、SQLite 跨重启历史 | 已完成 |
| 安全发布事务 | 容量预检、唯一暂存、目标竞争、写满、取消和清理 | 已完成 |
| 批量与边界 | 100 张、96/100.01 MP、中文长路径、冲突与故障矩阵 | 已完成 |
| 正式安装态图片闭环 | 无测试桥 v1.1.15 候选 17/17；安装/卸载/恢复 50/50 | 已完成 |
| `1.1.15` 版本身份 | npm、Tauri、两个 Cargo 清单/锁和唯一 DLL | 已完成 |
| `1.1.15` 正式候选 | NSIS、主程序身份、14 项载荷完整性 | 已完成 |
| 真实覆盖/恢复 | `1.1.14 → 1.1.15 → 卸载 → 1.1.14`、图片 17/17、生命周期 50/50 | 已完成 |
| 公开 updater 与升级 | 四项公开资产、签名、回下载、应用内升级 | 待正式发布后补齐 |

## 版本身份

- `package.json`、`package-lock.json` 根与 workspace、`tauri.conf.json`、主 Cargo 清单/锁、Shell Extension Cargo 清单/锁均为 `1.1.15`。
- 唯一版本化 Shell Extension 为 `long_compress_shell_extension_1_1_15.dll`，246,784 B，SHA-256 `628638E1EF00A6836CF78F9091B0899B6B037EE1E9A81C1B88A8A202A4F3FCDC`。
- `npm run test:release-identity -- --expected 1.1.15` 已通过，并联动通过媒体依赖、指标、发布门禁和图片基线静态检查。

## 候选与测试证据

- 正式构建命令为 `npm.cmd run tauri -- build`。本地默认 bundle 仅生成 NSIS；updater 由标签工作流显式 `--bundles nsis,updater` 生成。
- NSIS 为 8,691,488 B，SHA-256 `85CFBAD4230D3C1948278B34CFEC6327AC67368BC3730F66F35A8A99DBF8765A`；7-Zip 26.02 实测 14 个载荷完整，内含唯一 `long_compress_shell_extension_1_1_15.dll`。
- 主程序为 28,400,640 B，SHA-256 `7D11ED9673865B4F9BBF2B617AE215B8413EDEB4317154FFACB84BB78E476E05`；ProductName 为 `Long解压`，FileVersion/ProductVersion 均为 `1.1.15`。
- 首次完整性命令误用 npm 依赖附带的 7-Zip 21.07，该版本在当前控制台把中文安装包路径转换为乱码并拒绝打开。改用产品锁定的 7-Zip 26.02 后同一文件完整性通过；没有改变产物或放宽检查。
- 已安装公开 `v1.1.14` 覆盖候选后，安装 EXE SHA-256 与候选完全一致；版本、安装位置、两处用户数据指纹、唯一 Shell DLL、经典菜单 4 根/17 条子命令及 4 条快捷命令全部通过。
- 无测试桥安装版图片全流程 17/17：3 个真实 JPEG/PNG/WebP、可见质量 67/保持格式/限制尺寸/rename、执行前后预览、3 个真实输出、源哈希变化 0、3 条完成历史、完整重启和输出重开全部符合预期。
- 候选卸载后公开 `v1.1.14`、两处用户数据和原菜单目标完整恢复，最终无运行中应用进程；安装生命周期共 50/50。结构化证据：`test-results/installed-release-validation/20260828-125937/result.json`。
- 首轮安装门禁在变更系统前因新会话未设置 `EDGE_DRIVER_PATH` 安全失败；核对 WebView2 与 EdgeDriver 同为 `151.0.4129.107` 后从头重跑通过。该轮没有安装、卸载或修改用户数据。

## 完整回归

- TypeScript 类型检查通过；前端单元测试 44 个文件 254/254，集成测试 2 个文件 6/6。
- 媒体架构、6 项锁定依赖真实身份/许可、11 图/2 视频/6 PDF 固定夹具、19 项真实文件指标和发布门禁全部通过。
- 图片真实基线通过，峰值工作集 10,698,752 B；B-05.1 生产格式矩阵 9/9、重新解码差异 0；B-05.2.2 生产资源/故障边界差异 0。
- Rust debug `--all-targets`：库 319/319、4 项既定条件忽略；主程序及全部集成目标通过。与 Release workflow 一致的 `cargo test --release` 也完整通过；严格 Clippy `--all-targets --all-features -- -D warnings` 通过。
- npm 生产依赖安全审计为 0。首次使用机器配置的 npmmirror 时因镜像不实现 audit API 返回 404；显式切换 npm 官方 registry 后同一审计通过，未通过忽略告警或降低级别规避。
- 最终再次执行 `test:release-identity -- --expected 1.1.15` 通过。

## 签名与公开发布边界

- 本机未设置 `TAURI_PRIVATE_KEY` / `TAURI_KEY_PASSWORD`；历史 DPAPI 密码文件不能在当前机器状态解密，因此不在本地伪造 updater 签名。
- Release workflow 在 `v*` 标签触发，执行版本身份、前端、Rust Release、`--bundles nsis,updater`，并用 GitHub Actions Secrets 生成 `.nsis.zip`、`.sig` 和 `latest.json`。
- 当前分支相对 `origin/master` 领先且不落后，但公开发布仍须经过分支审计与合并；本步骤不直接创建标签或 Release。

## 发布后必做

1. 确认发布提交已合入受保护主分支，再创建 `v1.1.15` 标签。
2. 验证 Release Actions 成功且四项资产名称、版本、URL、签名一致。
3. 从公开 Release 回下载 NSIS 与 updater ZIP，执行完整性与 SHA-256 对账。
4. 在已安装公开 `v1.1.14` 的真实桌面环境运行 `test:public-update -PreviousVersion 1.1.14 -TargetVersion 1.1.15`。
5. 核对自动重启、安装位置、两处用户数据指纹、自动启动注册、唯一 Shell DLL、经典菜单 17 条子命令和 4 条快捷命令。
6. 将正式资产、Actions run、公开更新 evidence 和最终 PASS 回填本审计；任何差异先修正并从头复验。
