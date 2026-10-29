# v1.7.30 检查记录

- `producer.json`：真实 GNU USTAR/GNU/PAX 创建、删除内容记录、拼接后置目标的配方、固定 SHA 及独立内容/自身日期记录；没有改写归档头。
- `native.json`：10 份第二层包及 6 份第三层包的实际物化收据、浏览信息、图文预览 SHA、20 个所选文件目录和会话缓存清理。密码为固定公开测试密码。
- `verified.json`：独立 Python TAR 原件字节、外部 7-Zip 读回、第三层 ZIP 及所选文件的比对结果。
- `checks.json`：4 项新原生、2 项相关回归、测试调用线程修正、Shell DLL、版本身份和差异空白摘要。UI 未改，沿用既有 UI 检查。

固定原件位于 `tests/fixtures/archives/tar-forward-nested-1730/`，本机生成物和原始日志位于忽略的 `test-results/tar-forward-nested-1730/`。正式 PR/CI/合入/标签与附件由本版发布流程接续，不为回填编号再次打包。
