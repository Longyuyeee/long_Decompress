# ZIP PPMd v1.5.35 证据

程序源提交`1a5063e6ac088b593e7400ea81a9079bf757080c`，SHA256 `aa520b6deca6aa10a7b32ebc658f3c773c2abd08b2a90ff47868d43bfae46019`；所有通过项使用同一程序。完整状态见[桌面记录](desktop-progress.json)，本次检查范围与外部工具差异见[定向记录](checks.md)。这里保存七张代表截图，完整22张位于本地test-results/desktop-e2e，未生成示意图冒充界面。

实际混合输入4196108字节，创建标准三卷共2147649字节。这是合成资料，不代表通用压缩率。9级公共原生暂停恢复链总计14156ms（含250ms暂停、IPC/验证与采集），宿主进程观测峰值工作集111050752字节，含其他运行开销，不含WebView/独立7-Zip子进程；256MiB是9级字典预算，不能用此样本的工作集数值当上限。小样本参数检查的进程观测单列[rust资源记录](rust-resources.json)。

独立中文AES解码器使用cryptography与pyppmd 1.3.1（仅检查目录；[上游](https://github.com/miurahr/pyppmd)），可用`python -m pip install --only-binary=:all: --target test-results/zip-ppmd-1535/python pyppmd==1.3.1`准备。`scripts/verify-zip-ppmd-1535.py`认证后读取实际方法98参数再解码，不用产品Rust读回自证。
