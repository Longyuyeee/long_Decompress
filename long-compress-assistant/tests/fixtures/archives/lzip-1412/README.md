# GNU lzip 原始样本

生成器为 Ubuntu noble 官方软件包 `lzip 1.24.1-1build1` 的 GNU lzip 1.24.1，TAR 由 GNU tar 生成；本应用和 lzma-rust2 没有参与样本编码。载荷是自有中文文本、32,791 字节确定性二进制、空文件及空目录；无外部用户数据。源文件和归档 SHA256 见 `manifest.json`。

- `说明.txt.lz`：单个文本文件。
- `资料.tar.lz`：含中文目录、空文件和空目录的 TAR.LZIP。
- `资料别名.tlz`：上述 `.tar.lz` 的逐字节相同标准别名。
- `多成员.txt.lz`：两个独立 GNU lzip 成员顺序连接，解压后组成一个文本文件。

每个文件已通过 GNU lzip `-t`。应用读取、转换、创建和独立回解结果另见 `docs/LZIP_AND_TAR_LZIP_1.4.12.md`，不能只凭本目录声明用户流程通过。

重建脚本为 `scripts/prepare-lzip-fixtures.mjs`，需要 WSL Ubuntu 和位于忽略目录的 GNU lzip；生成器仅供测试，不随软件分发。TAR 的文件时间会影响重建归档哈希，因此回归使用此处固定原始产物，重建样本记录各自的实际清单。
