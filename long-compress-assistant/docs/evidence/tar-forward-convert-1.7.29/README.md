# v1.7.29 检查记录

- `native.json`：7 份实际新包的原件/读取卷、输出文件收据、原生最终取回目录、配置及预期条目。密码仅为固定测试密码。
- `verified.json`：独立 Python TAR 原件解析和外部 7-Zip 26.04 读回核对，7 份新包、108 个条目、87 个文件的精确目录树、内容、各自日期与独立副本身份。
- `checks.json`：本次原生 5 项、相关旧 2 项、Shell DLL、版本身份与差异空白摘要；首轮反例构造修正和定向复核单独记录。

原件沿用 `tests/fixtures/archives/tar-forward-links-1728/` 的固定 GNU tar 生产者样本，本机生成文件和原始日志在忽略的 `test-results/tar-forward-convert-1729/`。正式 PR、CI、合入、标签和附件由本版发布流程接续，不为回填运行号再打包。
