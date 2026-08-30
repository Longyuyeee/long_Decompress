# D-01.2.1 qpdf 正式资源与生产预检审计

日期：2026-08-30
状态：**D-01.2.1 完成；D-01 尚未关闭，PDF 分析/执行 UI 继续冻结。**

## 本步对齐范围

D-01 原始需求要求固定 qpdf 版本、哈希、许可和安装态能力检查。D-01.1 已完成官方能力契约与十类真实 PDF 基线；本步只完成“仓库正式资源 + 与产品共用的生产预检”，不提前实现 D-02 前端或 D-03 执行路径，也不把尚未测量的安装包增量写成已完成。

| 原始要求 | 实际实现 | 状态 |
| --- | --- | --- |
| 固定正式运行时 | 收录 qpdf 12.4.0 官方 MinGW64 五文件子集，逐文件锁定字节与 SHA-256 | 完成 |
| 携带许可义务 | 收录 qpdf Apache-2.0、NOTICE、GCC runtime、MinGW-w64 notice 和来源说明 | 完成 |
| 生产身份与能力检查 | 同一验证器先校验全部 10 个资源，再执行版本、crypto、JSON v2 与图片优化能力探针 | 完成 |
| 缺失或替换时安全拒绝 | 资源缺失、类型、大小、哈希、启动、版本或能力任一不符即失败关闭 | 完成 |
| 正式安装包增量 | 未在本步填写推测值 | D-01.2.2 |

## 固定载荷

二进制运行子集为 12,637,211 B：`qpdf.exe`、`qpdf30.dll`、`libgcc_s_seh-1.dll`、`libstdc++-6.dll`、`libwinpthread-1.dll`。连同 `SOURCE.txt` 和四份许可/notice，仓库正式资源共 10 文件、12,765,477 B。身份明细由 `config/media-dependencies.json` 与 Rust `EXPECTED_RESOURCES` 双向门禁；Tauri 资源清单逐文件显式列出，不使用运行时下载或通配下载。

生产预检只报告以下可证明事实：qpdf 版本必须为 12.4.0；crypto provider 必须同时包含 `openssl` 与 `native`；帮助输出必须证明 JSON v2 和 DCT/JPEG 图片优化阈值能力。Tauri 命令仅暴露身份预检，未暴露 PDF 优化命令；内部 `--internal-pdf-engine-preflight-report` 在窗口、数据库和单实例初始化前运行，供下一步正式安装态门禁复用。

## 审计纠偏

1. 首轮图片能力探针错误地要求帮助页回显 `--optimize-images` 字面量；qpdf 12.4.0 的实际选项帮助说明 DCT/JPEG，并列出 `--oi-min-width`、`--oi-min-height`、`--oi-min-area`。已改为核对真实官方输出，未放宽版本或能力边界。
2. 首轮执行全仓 `cargo fmt` 暴露仓库历史文件并未统一格式化，产生约百个无关文件变化。审计在提交前逐补丁撤回，只对两个新增 Rust 文件运行定向 `rustfmt`；功能改动文件集合恢复到本节点范围。
3. 首轮干净 CI 暴露仓库只对 `video-engine` 禁止文本换行转换，Windows checkout 将 qpdf 许可文本转成 CRLF 后触发字节门禁。根 `.gitattributes` 已把整个 `pdf-engine` 声明为逐字节资源并关闭上游 notice 的空白改写；锁定内容与哈希未改变。
4. 安装器压缩比不能由 12,765,477 B 原始资源直接推算，因此 `compressedInstallerDeltaBytes` 与 updater 增量继续为 `null`，明确交给 D-01.2.2 的同提交基线/候选正式构建。

## 验证证据

- `cargo test --release pdf_engine --manifest-path src-tauri/Cargo.toml`：4/4 通过；覆盖正式候选能力、资源路径、缺失拒绝和替换拒绝。
- `cargo build --release` 后由正式 Release 主程序执行 `--internal-pdf-engine-preflight-report`：退出成功，报告 `passed=true`，10 个资源身份及全部能力一致。
- `cargo clippy --all-targets --manifest-path src-tauri/Cargo.toml -- -D warnings`：通过。
- `npm.cmd run test:media-dependencies` 与 `:real`：通过；真实上游制品、checksum 与运行能力重新核验。
- `npm.cmd run test:pdf-contract`、`test:media-architecture`：通过；只允许身份预检，PDF 执行命令与 UI 仍被门禁禁止。
- `npm.cmd run type-check`、`npm.cmd run test:unit:coverage`、`npm.cmd run build`：通过；48 个前端测试文件、284/284 用例通过。

## 下一接续点

唯一下一步为 **D-01.2.2 正式 NSIS/updater 增量与安装态预检**：从同一基线提交构建不含/含 qpdf 的正式候选，记录字节与 SHA-256；安装候选后从正式 EXE 调用内部预检，实测完整、缺失和替换三种状态，并在卸载后恢复当前公开版本。完成并写回结构化证据前，不关闭 D-01、不进入 D-02、不提升 `1.1.17`。
