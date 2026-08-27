# S-00.1 密码保险箱本机保护审计

日期：2026-08-27

基线：`209da3c` / Long解压 `1.1.14` 发布后开发分支

节点性质：基础能力安全语义修复，不升版本、不发布

## 1. 开发目标

1. 保持用户已经确认的无主密码、无手工锁定体验；
2. Windows 磁盘上的安装密钥只允许当前 Windows 用户通过 DPAPI 解锁；
3. `passwords/*.json` 不再保存解压密码明文；
4. 旧明文安装密钥和旧明文密码条目可以无损迁移；
5. 密文损坏、DPAPI 失败或密钥不匹配时保留原数据，不静默生成新密钥、不覆盖原记录；
6. 安装密钥不得再通过 Tauri 命令返回给 WebView；
7. 迁移后密码保险箱仍能在应用重启后自动命中真实加密归档，并正确记录当天调用趋势。

## 2. 实现边界

- 新增 Windows 当前用户级 DPAPI 适配层，使用 `CRYPTPROTECT_UI_FORBIDDEN`，不会弹出系统凭据窗口；
- `installation.key` 使用 `long-dpapi:v1:` 包装格式；旧明文随机密钥只有在 DPAPI 保护和原子替换成功后才迁移；
- 解压密码正文使用内存数据密钥执行 AES-256-GCM，磁盘字段使用 `long-vault:v2:` 格式；随机安装密钥在解锁时一次性转为 32 字节数据密钥，避免为每条记录重复执行慢派生；
- 文件发布使用同目录临时文件与 Windows `MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH)`；失败会删除临时文件并保留旧文件；
- Rust 锁定路径会清零内存数据密钥；
- 前端改为调用无参数 `ensure_encrypted_password_service`；旧的密钥返回、外部初始化、外部解锁和外部锁定命令均不再注册到 invoke handler，避免 WebView 读取密钥或重写保护状态；
- 本节点只收口密码正文和安装密钥保护。传统网站密码字段的领域模型清理继续属于 S-00.2。

## 3. 预期—实际—修正

| 阶段 | 预期 | 首次实际 | 修正 | 最终实际 |
| --- | --- | --- | --- | --- |
| Windows 数据保护 | DPAPI 可往返，篡改密文必须失败 | 真实 API 测试通过 | 无产品修正 | 往返通过，单字节篡改被拒绝 |
| 旧安装密钥迁移 | 返回原密钥，磁盘不再含明文 | 通过 | 无产品修正 | 文件变为 `long-dpapi:v1:`，二次读取结果一致 |
| 旧密码条目迁移 | 密码值不变，磁盘不含明文 | 通过 | 无产品修正 | 重启服务后仍可解密，记录标记为 v2 |
| 损坏密文 | 返回错误且原文件字节不变 | 通过 | 无产品修正 | 未发生覆盖或自动重置 |
| 性能 | 密码条目数量增加时不重复执行慢密钥派生 | 首版实现每条记录执行 Argon2，存在加载退化风险 | 改为解锁时一次性生成内存数据密钥，每条记录只执行 AES-GCM | 三项迁移测试由约 0.91 秒降至约 0.27 秒 |
| 前端初始化 | WebView 不接触安装密钥 | 单元测试更新后通过 | 新增无参数后端就绪命令，删除密钥返回命令注册 | 前端 234/234、类型检查通过 |
| 真实桌面门禁 | 真实写盘、退出重启、密码命中、趋势更新 | 首次未配置 EdgeDriver；第二次误用开发地址；第三次未编入 E2E 桥 | 使用本机 Edge 151.0.4129.101 对应驱动；按文档依次构建 `VITE_DESKTOP_E2E=1` 前端和 `custom-protocol,desktop-e2e` Rust 二进制 | 磁盘无密码明文、DPAPI/v2 标识正确；完全退出重启后真实加密 7Z 解压成功，当天趋势为 1 |
| 正式构建 | 最终产物不包含测试桥 | 生产前端首次通过；Rust 命令误在仓库根目录执行而未启动 | 在 `src-tauri` 重跑 | `npm.cmd run build` 与 `cargo build --release --features custom-protocol` 通过 |

## 4. 验证结果

- `cargo test --release encrypted_password_service::tests -- --nocapture`：3/3 通过；
- `cargo test --release real_windows_dpapi_round_trip_and_tamper_rejection -- --nocapture`：1/1 通过；
- `cargo test --release --test password_auto_attempt_flow_test -- --nocapture`：真实 7Z 密码本、导入词表、未授权字典和未加密归档 6/6 通过；
- `cargo clippy --release --all-targets -- -D warnings`：通过；
- `npm.cmd run test:unit`：40 个文件、234/234 通过；
- `npm.cmd run type-check`：通过；
- `npm.cmd run test:e2e:desktop:vault-usage`：真实 Windows Tauri/WebView2、真实磁盘、真实 DPAPI、真实加密 7Z、应用退出重启和当天趋势门禁通过；
- `npm.cmd run build`：不含 E2E 桥的正式前端构建通过；
- `cargo build --release --features custom-protocol`：正式 Rust Release 构建通过。

## 5. 需求对齐与剩余风险

本节点解决了审计中最高优先级的“公开保护描述与实际明文存储不一致”，并保留无主密码、无手工锁定的用户体验。归档解压自动密码命中、使用次数和趋势语义没有改变。

仍未完成的 S-00 项：

1. 收敛 `username`、`url`、传统密码强度和过期等通用密码管理器字段；
2. 清理 `task.ts` 的历史任务占位接口；
3. 统一格式支持等级和 HFSX 历史描述；
4. 确认并归档未参与构建的 `src-tauri/TranslateSoftware` 旧工程。

上述项目完成并通过相应真实门禁前，B-00 继续保持阻断，图片、视频和 PDF 引擎不进入产品代码。
