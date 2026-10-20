# v1.5.31 WIM XPRESS 创建实施

2026-10-20，接续已正式发布的 v1.5.30：PR #225，标签0cf82057e1341d016016a380b6e481829ee9cca4，正式运行39425363245成功，四附件及中文 Release/updater 一致。

已按既定路线完成并进入发布，具体检查与身份见[封板记录](WIM_XPRESS_FREEZE_1.5.31.md)。Windows WIMGAPI 捕获在普通权限下返回1314，未提权、未安装系统组件；本版选择独立 wimlib1.14.5 CLI 编码（两个二进制合计610332字节），随包固定原始源码压缩包、构建说明和完整许可。只对明确选择 XPRESS 的创建/转换使用，旧请求仍存储与去重，读取保持原引擎。

依次接通方法选择、单条/多选、配置组/模板/草稿/历史、文件目录创建、数字切卷、转换、取消/失败及结果；用7-Zip独立回解和资源表确认真实XPRESS。不启动LZX/固实、原生SWM创建或系统部署，WIM无密码。保留单条展开、多选弹窗、自然继承、50/50及窄窗滚动；只补本版受影响链。

依据：[WIMCreateFile](https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/wim/dd851934(v=msdn.10))、[wimlib capture](https://wimlib.net/man1/wimcapture.html)、[wimlib CLI与许可](https://wimlib.net/man1/wimlib-imagex.html)。源码/二进制/通知身份见wim-engine.lock.json。

实现了显式wim_method/wimMethod（store/xpress），缺省保留存储，配置/模板和历史分别保存。源列表只列普通文件与空目录，确保重复名称经已有收集器保留，数字卷沿既有分割/发布事务。编码器真实百分比反馈，成功后记录纳入归档的文件大小，分卷前固定来源统计；不声称逐字节即时读取测量或使用旧存储预测XPRESS。

六项不同Rust与六项新增前端、实际公共原生及六条UI链完成；只补统计受影响检查，先前内容检查保留程序身份。源码、二进制、说明和许可均锁定并随候选/正式包核对，不新增全量验收。下一版[WIM LZX](ARCHIVE_ROADMAP_1.5.32.md)仅规划。
