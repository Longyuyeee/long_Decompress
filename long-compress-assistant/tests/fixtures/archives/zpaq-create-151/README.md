# v1.5.1 产品创建产物

由真实 Windows Tauri 后端创建，再用作者未修改的 ZPAQ 7.15 `zpaq64.exe` 独立回解，三个文件 SHA256 和空目录一致。固定文件对应[桌面证据](../../../../docs/evidence/zpaq-create-1.5.1/results.json)的前两组，不是把其他归档改名。

| 产物 | SHA256 |
| --- | --- |
| plain.zpaq | `753192eb6c159a282278eff440bebed1de5a60023b840b21261364afb0c8d36b` |
| native-key.zpaq | `234deb3de5db3072c1cd66d36f16b17cab0cc900e0212a22f6e6e7d5c488d512` |

密码为公开测试值 `中文 密码!-钥匙`，不是用户数据。归档内为 `项目 空格/`，包含 `说明.txt`、`空文件.txt`、`子目录/数据.bin` 和 `子目录/空目录/`。

| 文件 | SHA256 |
| --- | --- |
| 空文件.txt | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 说明.txt | `4443bd95e6e5dff0c46fa8e56a2fde59d91f3cfa2dc1e0f3207424bd100caaf1` |
| 子目录/数据.bin | `e32d92510f7e1c6acbf14afec28643fb51ef2a877202d3f1d73105425891775d` |

`说明.txt` 是 UTF-8 文本 `这是要分享给同事的中文资料。` 加 CRLF，数据文件为 65536 字节，字节 i 为 `(i*37)&255`。重生成脚本为 `scripts/zpaq-151-gate.mjs`。由于时间戳和密码随机盐，新生成包不要求与固定包逐字节相同，必须比较回解后的内容。

独立工具：[作者原包](https://www.mattmahoney.net/dc/zpaq715.zip)，`zpaq64.exe` SHA256 为 `7a94b4f1d6323a758c7b0b6344036f166bff0fd44f1c3c86f05b3688023496cb`。使用 `zpaq64.exe x native-key.zpaq -key "中文 密码!-钥匙" -to <新的输出目录>` 回解；普通包不传 `-key`。不要用产品适配器代替独立工具的这条证据。
