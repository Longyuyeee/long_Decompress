# v1.4.0 发布审计

## 发布状态

2026-10-12：**v1.4.0 已封板并公开发布**，PR #147 已合并。标签 `v1.4.0` 指向 `6a1a02bfca531617b83ad22f00d15f122cd7c5f0`，与候选提交 `cf8cda6aa6456b2f3b19b88fa2ca643579c9b923` 文件树一致。[发布工作流 38161217441](https://github.com/Longyuyeee/long_Decompress/actions/runs/38161217441) 成功，安装器、更新 ZIP、签名、latest.json 均已上传；[Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.4.0) 为公开正式版，发布于北京时间 01:59。

## 公开附件复验

- 公开安装器 19,606,359 字节，SHA256 `36d48e52e848fa0a7eaad107602060519dc1a161fae04accc09783e274ce71ed`；安装 EXE SHA256 `3d1621b7f67c3d753f024950871abcf18344de50c6c391f4d7eb57ed2387d2a8`，与候选构建分别记录。
- 从公开附件下载后，最终安装生命周期 **52/52**、工作区 **18/18**，北京时间 02:07 完成；原 1.3.9 安装、用户数据、右键菜单、自启动均恢复，数据指纹一致。实际查看公开包转换重试页面，版本 1.4.0、草稿密码/输出为空，取消任务不显示进行中。结果及截图见 `evidence/release-1.4.0/published-*`。
- `releases/latest` 指向 v1.4.0；公开 latest.json 版本、更新 ZIP 地址、签名存在性和中文说明同步检查通过。此处没有执行应用内自动更新，不将安装器覆盖安装冒充自动更新全链或签名密码学验证。
- **保留首轮失败**：启动后 20 秒内菜单仍指向本机旧 debug 路径。相同包、相同等待条件后两轮菜单检查通过；根因未确定，不声称偶发启动问题已修复，后续若复现应补启动耗时/退出码证据。失败后安装与数据已恢复，原备份留在本机。
- **保留第二轮失败**：转换填表触发 Selenium `StaleElementReferenceError`。仅修正脚本的弹窗退出等待、失效引用最多三次重新定位，完整值/产物/历史断言保留；不修改产品、公开附件或标签。修正后完整流程通过。脚本后续提交不属于已发布二进制。

## 本版范围

UI 六项反馈、现有 ZIP/7Z 密码与数字分卷、7Z 文件名加密、显式 ZipCrypto/Windows 密码编码、八类目标转换/改密/重新分卷、历史与全局重试闭环。未新增格式，快速打包已移除。完整说明见 [发布说明](RELEASE_NOTES_1.4.0.md)。

## 验证

- 封板代码 52f47be：前端 579/579，Rust library 475 通过、14 个既有 ignored 不计通过；相关后端集成 61/61，浏览器页面专项 5/5。正式安装工作区及页面证据见 `evidence/freeze-2026-10-12/`。
- 升版后 `npm run test:unit`（src 与 tests/fixtures）75 文件、557/557；与封板全量调用覆盖范围不同，不将二者叠加。`test:release-identity -- --tag v1.4.0` 通过，八处版本和右键扩展 DLL 一致。
- 本机前端与 Rust release 编译通过；本机旧有 NSIS 运行依赖故障导致安装器生成失败，未变更系统组件。改用成功的 [Windows CI 38160539477](https://github.com/Longyuyeee/long_Decompress/actions/runs/38160539477) 精确候选产物，PR 检查亦通过。
- 无测试桥候选安装生命周期 52/52、工作区 18/18。真实页面检查 1.4.0 版本、转换设置；实际 ZIP/7Z/RAR 解压、密码分卷、八类转换、再次执行及读写取消均通过。原 1.3.9 安装、用户数据、右键菜单和自启动恢复成功；两个数据目录路径/长度/哈希与备份一致。北京时间 01:48 完成。
- 候选安装器 SHA256 `13169503b3645bf7d8bd17384ba92a9f8388cafa4103bd64ddffc44baec9cab6`；安装 EXE SHA256 `c25ad90cb707567854eed67fba12f13072a571e2e0148a742ef4da99bdfaa13e`。本目录 `evidence/release-1.4.0/` 保存候选结果与页面截图；正式发布重新构建的附件需独立记录身份。

## 支持边界

- 中文 ZIP 密码与使用 Windows 系统编码的工具互操作须显式匹配编码，不自动降低加密方式。ZipCrypto 保护较弱。
- ZIP 36 个读取组合覆盖 Store/Deflate/Deflate64/BZip2/LZMA/XZ × 无密码/AES/ZipCrypto × 单包/数字卷；Zstd 仅独立无密码样本，不外推全部组合。
- 转换限 ZIP/7Z/TAR/TGZ/TBZ/TXZ/TZST/WIM 目标；不宣称 RAR、单文件流、AES 包装转换。
- 性能单次同机数据不代表加速倍数，内存为后端进程生命周期峰值，临时空间为采样；未覆盖完整 WebView 进程树、每阶段物理磁盘耗尽、全部硬件/DPI 或原生选择器操作。Windows N 不保证支持。
