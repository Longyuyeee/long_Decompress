# S-00.4 格式支持与仓库卫生审计

日期：2026-08-27

分支：`codex/archive-media-roadmap`

公开版本：`v1.1.14`（本步骤不升版）

## 开发目标

1. 用统一证据等级区分“完整创建—解压”“真实只读”“仅引擎识别”和“不作为归档导入”；
2. 解决 HFSX 文档中“已验证”和“只有空镜像”的矛盾；
3. 将不参与构建的旧工程从产品源码目录移出，同时保留可追溯历史；
4. 使用真实非空载荷和 Windows Release Tauri 验证，不用扩展名、空镜像或单元测试代替用户路径。

## 预期—实际—修正

| 检查项 | 预期 | 首次实际 | 修正 | 最终实际 |
| --- | --- | --- | --- | --- |
| HFSX 夹具 | 固定上游版本，生成含已知文件的非空 HFSX，并校验内容 | npm 子进程中的 PowerShell 环境无法识别 `Get-FileHash`，夹具在哈希阶段失败 | 改用 .NET `SHA256` 流式计算，移除对该 cmdlet 的依赖 | 随包 7-Zip 识别 `Method = HFSX`，解出 `Firefox/known-payload.txt`，SHA-256 为 `0A7130487543AF627E9C15512AE6DBE0A6FD9D6ED5F4C2C89942E56CB3B14023` |
| Windows 桌面闭环 | Release Tauri 从同一 HFSX 解出已知载荷并逐字节比较 | 首次启动命中了此前生成的普通 Release 可执行文件，未包含桌面 E2E 通道，WebDriver 会话无法建立 | 重新以 `custom-protocol,desktop-e2e` 构建 Release 后复验 | `test:e2e:desktop:hfsx` 通过，真实 WebView2/Tauri 解压内容精确匹配 |
| 支持口径 | README 与专项文档不得把动态处理器等同于公开支持 | 历史交接仍同时保留“非空通过”和“尚未完成” | 新增权威等级表，并在旧验证文档标注历史结论已被替代 | README、格式表和真实验证记录使用同一 A/B/C/D 口径 |
| 旧工程治理 | 不参与构建的原型不应留在 `src-tauri` | `src-tauri/TranslateSoftware` 无 manifest、无产品构建引用但仍位于源码树 | 验证路径与引用后使用 Git 重命名移入 `archive/legacy-projects`，增加归档说明 | 产品构建入口不再包含该目录，历史文件仍可追溯 |

## 验收结果

- `npm.cmd run test:fixtures:hfsx`：通过；真实非空 HFSX 内容与固定 SHA-256 一致；
- `npm.cmd run test:e2e:desktop:hfsx`：通过；Windows Release Tauri + WebView2 用户路径通过；
- `npm.cmd run type-check`：通过；
- `npm.cmd run test:unit -- --run`：40/40 测试文件、235/235 测试通过；
- `cargo test --test archive_engine_matrix_test`：2/2 通过，覆盖随包引擎动态能力和真实归档样本矩阵；
- `node --check scripts/test-tauri-desktop.mjs`：通过；
- `npm.cmd run build`：正式生产前端构建通过；
- `cargo build --release --features custom-protocol`：正式 Rust Release 构建通过，已恢复非 E2E 生产可执行文件。

## 需求对齐与下一步

本步骤没有增加媒体功能，也没有扩大未经验证的格式声明；它直接服务于“真实可用、状态诚实”的基础需求。S-00.4 完成后进入 S-00 总验收，复跑密码保险箱、历史持久化、应用重启和安装覆盖等跨步骤真实门禁。总验收通过前不进入媒体编码器实现，也不提升公开版本。
