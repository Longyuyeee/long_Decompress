# Long解压 v1.2.6 发布审计

日期：2026-09-03

分支：`codex/decompression-runtime-ux`

## 发布范围

- 解压运行态信息架构、速度与进度显示、密码日志采样。
- Windows 卷根目录同卷暂存与事务发布。
- 标题栏拖动、八方向缩放和真实 Windows 系统输入门禁。
- 过期测试与文档清理、当前测试入口和接续文档纠偏。

## 关键预期—实际

| 门禁 | 预期 | 实际 |
| --- | --- | --- |
| 真实标题栏拖动 | Windows 原生窗口位置发生变化 | `48×28`，通过 |
| 真实窗口缩放 | Windows 原生尺寸发生变化且松键后停止 | `40×30`，释放后尺寸保持，通过 |
| 真实响应式详情 | 920×620 / 760×520 双栏可读且无横向溢出 | 解压与压缩两类均通过 |
| 隔离卷根产品 IPC | 根目录发布成功、字节一致、源包保留、暂存清零 | 全部通过 |
| 标题栏单元门禁 | 空白区启动原生拖动，窗口按钮不拖动 | 通过 |
| 八方向缩放单元门禁 | 8 个方向均转交 Windows 原生循环 | 通过 |

## 发布状态

`v1.2.6` 已正式发布：PR [#119](https://github.com/Longyuyeee/long_Decompress/pull/119) 的五项门禁全部通过，合并提交为 `master@0e04574b0e7f0df73a0d9deaf586a59f9678aff6`，annotated tag `v1.2.6` 指向该提交。Release workflow [33714184134](https://github.com/Longyuyeee/long_Decompress/actions/runs/33714184134) 的全部步骤通过，公开 Release 位于 [Long解压 v1.2.6](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.2.6)。

## 回归与产物

| 项目 | 实际结果 |
| --- | --- |
| 前端单元测试 | `51` 个文件、`286/286` 通过 |
| 前端集成测试 | `6/6` 通过 |
| 性能测试 | `17/17` 通过 |
| Chromium E2E | `12/12` 通过 |
| Rust Release | 主程序及集成共 `494` 通过、`14` 忽略、`0` 失败 |
| Rust Clippy | 主程序与 Shell 扩展均以 `-D warnings` 通过 |
| Shell 扩展 | Release 测试 `5/5` 通过，版本资源为 `1.2.6` |
| 正式构建 | `npm run tauri build` 通过；生产前端不含桌面 E2E 桥 |

正式 NSIS：`Long解压_1.2.6_x64-setup.exe`，`19,399,407` 字节，SHA-256 `4FE451D5DC7164FF959873A46BD08FF4D1F24FE13E0C1C616C6E129F89BD5376`。

正式主程序：`Long解压.exe`，`29,771,776` 字节，SHA-256 `39941B92A02ABDF876C06A3AC02F534371B4FD4880985F545AA174D37386ED06`。

Shell 扩展：`long_compress_shell_extension_1_2_6.dll`，`246,784` 字节，SHA-256 `F816E29ED4E1A7A0699D54AD84E3D2FAAC8693F2970A7BFF6300038AA6D34C79`。

## 公开资产回下载

| 资产 | 字节 | SHA-256 |
| --- | ---: | --- |
| `Long-Decompress_1.2.6_x64-setup.exe` | `19,367,216` | `522EF802D0380062C711979F1DE789E05C4683780832BF2CF8794E84DB658E75` |
| `Long-Decompress_1.2.6_x64-setup.nsis.zip` | `19,367,374` | `860EE8EA55A13A06E9A8BC2940BCF4B9DE1CC719161EE70734B394B05BC76527` |
| `Long-Decompress_1.2.6_x64-setup.nsis.zip.sig` | `428` | `800A0ADA7D1E7C4826036F972F007AC7A0065E75A8AE23974A2279BF24490AEE` |
| `latest.json` | `932` | `FE018D989008088130EB7C967D7C9EA22EE0EBE5653FB2C43AC3142F78DF5D4D` |

公开 `latest.json` 的版本为 `1.2.6`，下载 URL 精确指向公开更新 ZIP，manifest 签名与公开 `.sig` 逐字一致；Release 为非草稿、非预发布，发布说明已替换为本版本完整说明。

## 明确未伪造为通过的边界

- Windows 11 第一层资源管理器菜单需要生产代码签名证书；当前无签名环境只构建经典菜单资源，不生成 MSIX。
- `test:context-menu-package` 需要管理员权限并会向本机证书存储写入测试证书，当前非提升会话未执行成功。
- 安装生命周期脚本会替换/停止当前安装态应用。为保留用户正在运行的正式版，本机未执行该破坏性门禁；发布工作流在干净 Windows Runner 中重新构建、测试并生成带 updater 签名的更新资产。
