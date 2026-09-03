# 桌面门禁 Node 兼容性审计（2026-09-03）

## 问题与根因

压缩详情密度节点首次用本机默认 Node 22.12.0 启动 `test:e2e:desktop:responsive-layout` 时，脚本尚未创建桌面会话便因 `node:zlib` 不导出 `zstdCompressSync` 退出。`package.json` 明确接纳 Node `^20.19.0 || ^22.12.0 || ^24.0.0`，所以要求所有桌面聚焦入口仅因顶层导入就必须使用更新 Node，已经偏离仓库运行时契约。

该 API 只用于归档浏览门禁生成 25 字节明文的 `.zst` 能力夹具，与响应式、壳层、图片、视频、PDF 等入口无关。问题属于测试基础设施耦合，不是产品 Zstandard 引擎故障。

## 修复

- 删除 `zstdCompressSync` 顶层导入，保留各 Node 20/22/24 均支持的 `deflateSync`。
- 夹具改为脚本内生成标准 Zstandard 单段 frame：固定 magic、单字节内容大小、一个 final raw block 和原始载荷。
- 生成器限制载荷不超过 255 字节，防止未来在不调整 frame header 的情况下误用。
- 媒体架构门禁新增静态契约：桌面脚本不得重新依赖 `zstdCompressSync`，并必须保留可移植生成器。

产品 Rust 代码、打包归档引擎、公开格式支持和用户文件处理均未修改。

## 验证

- 默认 Node `v22.12.0`：`npm.cmd run test:e2e:desktop:responsive-layout` 真实 Windows Tauri/WebView2 通过。
- Node `v22.12.0` 生成的 34 字节测试 frame 经产品随包 `7z.exe 26.02` 识别为 `Type = zstd`，解压大小 25 字节，`Everything is Ok`。
- `npm.cmd run test:media-architecture`：通过，锁定兼容性契约。

尝试用 `test:e2e:desktop:archive-browser` 覆盖完整能力来源路径时，门禁在进入该路径前发现既有公共往返流程会在快速解压后删除临时 `roundtrip-payload.zip`，随后诊断阶段读取源包时报 `ENOENT`。该失败与新 frame 无关（生成器尚未被调用）；不得记为归档浏览通过。后续应单独审计该聚焦入口的源包保留/测试设置隔离，不在本兼容节点顺手改变产品删除语义。

## 接续点

下一步先审计并修复 `archive-browser-only` 聚焦入口的临时 ZIP 生命周期，使 Zstandard 能力来源重新由完整真实桌面流程覆盖；完成后独立提交、推送。随后继续高 DPI 与活动态/终态任务的窄窗视觉审计。
