# 随包兼容引擎

当前为上游 **7-Zip 26.04 / Windows x64**，于 2026-10-13 更新。

来源：[7-Zip 官方下载页](https://www.7-zip.org/download.html)所链接的[官方安装包](https://github.com/ip7z/7zip/releases/download/26.04/7z2604-x64.exe)。仅解出控制台引擎、DLL、Readme 和许可证，未安装或修改本机 7-Zip。

| 文件 | SHA-256 |
| --- | --- |
| 上游 `7z2604-x64.exe` | `d54bf805f9f3704d1e8db2fa3498ae7ef2df0312b40b558e7c71c734430a665d` |
| `7z.exe` | `95fff19b73eff77c87c0d7bd32822e43fb90dd2932459f962eb532d8dfb6ba53` |
| `7z.dll` | `899cd73d7fc60e1c1c35c7de0f5bed3dc69bfe2cecbd48014d868a347970a0b1` |

分发许可见同目录 `7-Zip-License.txt`。原生 ZIP/7Z/TAR/LZ4/Brotli 路径独立于该外部兼容引擎。

## 原生 7Z 补充方法

v1.6.29 新增独立的未加密固实 PPMd 完整预览入口，按共享流映射与声明总量分段检查前后内容，只保留所选文件，确认逐项/块/打包流 CRC、压缩完整覆盖与 PPMd 结束。复用 v1.6.27 的 finish_7z 加法 API，未改 PPMd/AES 算法、默认解码或 ZIP 依赖，也未新增外部引擎；详细修改见 sevenz-rust/LONG_PATCHES.md。

v1.6.28 在同一完整 PPMd 入口接入 AES → PPMd，复用既有完整 PreviewAes（含消费回执、零填充、密文结束和 KDF 范围），逐级核对声明长度、实际消费与 PPMd 结束/CRC。未改 AES 算法或默认解码、未用部分读取绕过完整条件，也未新增依赖或外部引擎；补丁与必要原件见 sevenz-rust 的说明和应用测试。

v1.6.27 的未加密非固实 PPMd 图文预览，在 `vendor/ppmd-rust 1.4.0`（MIT-0/CC0）增加独立的 7z 声明大小结束检查入口，按所选文件大小读取后核对 range coder、压缩覆盖和 CRC。仅本地 sevenz-rust 使用此副本，ZIP 仍引用原 registry 版本；默认 Read、模型和编码器不改。原始文件 SHA-256、补丁说明和许可见该目录；既有随包 `licenses/ppmd-rust-MIT-0.md` 保持。没有新增外部运行引擎。

v1.5.7 在本地 `vendor/sevenz-rust 0.6.1`（Apache-2.0）接入 PPMdH、Deflate、Deflate64 流式读取，并启用其 Bzip2 特性。复用已锁定的 ppmd-rust 1.4.0、flate2、deflate64 与 bzip2；不改变默认编码器，不调用兼容引擎重试错密或损坏。补丁说明见 `vendor/sevenz-rust/LONG_PATCHES.md`，原文许可随包见 `licenses/sevenz-rust-Apache-2.0.txt`、`licenses/ppmd-rust-MIT-0.md`。7-Zip 独立工具只用于原包生成和回读。

## 原生 RAR 读取说明

v1.5.9 在 Rust 包装中保留原生 Unicode 换卷请求/通知及失败时状态，区分缺卷、不可读和前段密码已通过后的后续数据异常；打开、读头和处理均在同步返回前清除栈回调。未改 UnRAR 解码算法或重命名输入卷，修改原文见 `vendor/unrar/LONG_MODIFICATIONS.md`。

`unrar 0.5.8` Rust 包装与 `unrar_sys 0.5.8` 原生源码静态链接于主程序，源码分别固定在 `src-tauri/vendor/unrar` 和 `src-tauri/vendor/unrar-sys`。v1.5.5 增加同步数据检查点以及 MSVC `/EHsc`，支持实际块内暂停/取消并释放取消异常中的文件句柄；v1.5.6 增加同步解码块字节进度，取代预分配长度估算，不复制块内容或重复读取输出，未修改 RAR 解码算法。完整修改说明在各自 `LONG_MODIFICATIONS.md`；包装许可见 `licenses/unrar-rs-*`，UnRAR 原文见 `licenses/UnRAR-License.txt`。仅解码，不得用这些源码重建 RAR 兼容压缩算法，不随包分发 Rar.exe 编码器。

## LZIP 原生编解码

v1.4.12 开发接入 [lzma-rust2 0.22.0](https://github.com/hasenbanck/lzma-rust2/)，使用 `std`、`encoder`、`optimization` 和 `lzip` 特性，固定版本并静态链接，不调用外部 LZIP 程序。Apache-2.0 原文随包保存在 `licenses/lzma-rust2-Apache-2.0.txt`。独立 GNU lzip 仅生成测试样本和回解，不随软件分发。

## ZIP LZMA 原生创建

v1.5.38 为现有本地 zip 8.6.0 增加方法14写入，复用其已锁定且启用编码的 lzma-rust2 0.16.2（Apache-2.0），不新增外部引擎或依赖。写入 ZIP 专用属性头、原始 LZMA1 与 EOS，并保留普通、AES、ZipCrypto 和分卷流程。修改文件与原始来源见 `vendor/zip/LONG_COMPATIBILITY_PATCH.md`；既有 lzma-rust2 许可随包保留。独立 Python/liblzma 与 7-Zip 仅用于回解检查。
