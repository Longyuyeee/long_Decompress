# 随包兼容引擎

当前为上游 **7-Zip 26.04 / Windows x64**，于 2026-10-13 更新。

来源：[7-Zip 官方下载页](https://www.7-zip.org/download.html)所链接的[官方安装包](https://github.com/ip7z/7zip/releases/download/26.04/7z2604-x64.exe)。仅解出控制台引擎、DLL、Readme 和许可证，未安装或修改本机 7-Zip。

| 文件 | SHA-256 |
| --- | --- |
| 上游 `7z2604-x64.exe` | `d54bf805f9f3704d1e8db2fa3498ae7ef2df0312b40b558e7c71c734430a665d` |
| `7z.exe` | `95fff19b73eff77c87c0d7bd32822e43fb90dd2932459f962eb532d8dfb6ba53` |
| `7z.dll` | `899cd73d7fc60e1c1c35c7de0f5bed3dc69bfe2cecbd48014d868a347970a0b1` |

分发许可见同目录 `7-Zip-License.txt`。原生 ZIP/7Z/TAR/LZ4/Brotli 路径独立于该外部兼容引擎。

## LZIP 原生编解码

v1.4.12 开发接入 [lzma-rust2 0.22.0](https://github.com/hasenbanck/lzma-rust2/)，使用 `std`、`encoder`、`optimization` 和 `lzip` 特性，固定版本并静态链接，不调用外部 LZIP 程序。Apache-2.0 原文随包保存在 `licenses/lzma-rust2-Apache-2.0.txt`。独立 GNU lzip 仅生成测试样本和回解，不随软件分发。
