# Long解压 v1.1.15 发布审计

审计日期：2026-08-28（Asia/Hong_Kong）

状态：**正式发布与公开更新验证已完成。**

## 结论

图片压缩 B-01 至 B-05.3 与其依赖的 S-00、B-00 基础节点已完成。发布提交经 PR #87 合入受保护主分支，`v1.1.15` 标签固定在 `82b1b8f8fe9f596ef4cbe55aeac4c100b7d6c316`。GitHub Actions 已使用仓库 Secrets 生成并公开四项签名更新资产；公开回下载、哈希/签名对账及真实 `v1.1.14 → v1.1.15` 应用内更新 24/24 均通过。

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
| 公开 updater 与升级 | 四项公开资产、签名、回下载、应用内升级 24/24 | 已完成 |

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

## 签名与公开发布

- 本机未设置 `TAURI_PRIVATE_KEY` / `TAURI_KEY_PASSWORD`，因此候选阶段没有伪造 updater 签名。正式签名仅由 GitHub Actions 仓库 Secrets 生成。
- PR #87 以 squash 合入受保护主分支，合并提交 `82b1b8f8fe9f596ef4cbe55aeac4c100b7d6c316` 与最终跑绿 PR 头的文件树完全一致；annotated tag `v1.1.15` 精确指向该提交。
- Release workflow run [33146766724](https://github.com/Longyuyeee/long_Decompress/actions/runs/33146766724) 用时 20m45s，版本身份、干净检出真实图片夹具单测、Rust Release、签名 `nsis,updater` 构建、identity package 排除和公开 manifest 回读全部成功。

## 合并前远端审计

- PR #87 首轮真实 Windows CI 中 Browser shell E2E 通过；Frontend checks 在 `test:unit:coverage` 发现 4 个 `ENOENT`，均指向被 Git 忽略的真实图片夹具。预期是干净检出能自建夹具，实际是本地已有 `test-results/media-fixture-audit` 掩盖了隐式前置条件。
- 修正为 `test:unit` 和 `test:unit:coverage` 的 npm 前置生命周期都先执行 `test:fixtures:media:images`。这样 PR CI、开发者本地命令和正式 Release workflow 使用同一真实夹具生成/冻结哈希校验入口，不以 mock 或提交生成物规避问题。
- 修正后两次把新生成目录完整移出项目，确认 `test-results/media-fixture-audit` 不存在再分别启动命令：coverage 重新生成 11 个真实图片与 1 个 PDF 拒绝边界后 47 文件 276/276；普通单测再次从空状态生成并通过 44 文件 254/254。类型检查、生产构建和 `1.1.15` 发布身份同时通过。
- 修正提交 `b74cabeda6266dbd6b1b814194799e8d9a8d33c7` 的远端 CI run [33144654827](https://github.com/Longyuyeee/long_Decompress/actions/runs/33144654827) 已在干净 runner 全部通过：Frontend checks 1m53s、Browser shell E2E 51s、Windows desktop E2E build 5m05s、Rust and shell-extension checks 14m36s、Windows installer 10m11s。最后一项实际完成无签名 NSIS 构建并上传产物，不以静态配置检查代替打包。
- 对齐结果：首次实际为 4 个夹具 `ENOENT`，修正后实际为五个 CI job 全绿、四个受保护分支必需上下文全绿；预期与实际差异归零。PR #87 随后以 `MERGED` 状态完成，受保护主分支没有被绕过。

## 正式资产与公开更新审计

- 正式 Release：[Long解压 v1.1.15](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.15)，非草稿、非预发布，目标提交为 `82b1b8f8fe9f596ef4cbe55aeac4c100b7d6c316`。
- `latest.json`：950 B，SHA-256 `8265EFE654A773D85C1A06EF894FA5D1C278DC792EC2072E93DFD5577B3A3E78`。
- NSIS：8,658,170 B，SHA-256 `DBFF77AE6C1C642B32EFC03C4726D14284A46CAED9AFA9EEA7F6BF4487DD2C51`。
- updater ZIP：8,658,330 B，SHA-256 `DE591FDBF78F07813CCF91F0ACBBEB9A593FA2CC9A3D714E9A9CB1B3FA241D00`；解包得到的唯一 EXE 与独立 NSIS 大小、SHA-256 完全一致。
- `.sig`：428 B，SHA-256 `BE82F7E06F563F6B84B1F25418CA6BA302D8342AB7C6D45F5A4ED4C24902E2AC`；内容与 `latest.json` 的 Windows x86_64 签名逐字一致，manifest 版本和 ZIP URL 正确。
- 首轮真实公开更新在安装前安全失败：预期 `autoStart=true` 与 Windows Run 项一致，实际偏好为 `true` 但注册项缺失。这是此前安装恢复测试留下的基线不一致；生产设计禁止启动时静默修复。通过公开 `v1.1.14` 的真实设置 UI 先同步实际关闭状态、再显式点击开启，生产 Tauri 命令恢复精确注册值；没有直接写注册表或放宽断言。
- 从一致基线重新执行 `test:public-update -PreviousVersion 1.1.14 -TargetVersion 1.1.15`，24/24 通过：签名更新 UI 交接、旧进程退出、目标版本安装、原路径保留、自动重启、经典菜单 4 根/17 条子命令/4 条快捷命令、自动启动、两处用户数据指纹、唯一 `long_compress_shell_extension_1_1_15.dll` 和无 MSIX identity package 均符合预期。结构化证据：`test-results/public-update-validation/20260828-143320/result.json`。
- 本机最终安装公开 `v1.1.15`，测试完成后无运行中应用进程；发布预期与当前实际差异为 0。
