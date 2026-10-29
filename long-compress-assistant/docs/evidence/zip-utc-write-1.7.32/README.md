# v1.7.32 检查记录

- `bounds.json`：负秒、epoch 0、奇数秒、亚秒截取、签名正值上限、超界与 DOS 年份边界；源文件纳秒保持不变。
- `native.json`：普通 ZIP、AES / ZipCrypto ZIP、AES ZIPX，实际标准及数字分卷；源文件、明确非空/空目录与最终提交日期。
- `choices.json`：真正使用多个工作线程的并行读回，以及平铺同名来源独立编号与各自时间。
- `conversion.json`：上一版 bsdtar 真实 UTC ZIP 原件转换为已验证 AES ZIP / ZIPX；发布回执、原件 SHA 和私有暂存清理。
- `verified.json`：独立 Python 解析本地/中央头、来源日期及 DOS 兼容字段，并以外部 7-Zip 26.04 读回 9 组新包。标准 ZIPX 为工具识别而复制为 `.zNN/.zip` 名称后校验相同字节，数字切片按序合并；没有更改软件的输出命名。
- `checks.json`：4 项新原生、2 项相关回归、Shell 与版本身份摘要。

复核脚本：[verify-zip-utc-write-1732.py](../../../scripts/verify-zip-utc-write-1732.py)。本机生成物和原始日志位于忽略的 `test-results/zip-utc-write-1732/`，回执保留实际路径供追溯；转换私有来源在测试结束后按设计清理。正式 CI、合入、标签和附件由本版发布流程接续，不为回填编号再次打包。
