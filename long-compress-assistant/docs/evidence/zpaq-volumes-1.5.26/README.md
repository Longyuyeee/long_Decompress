# v1.5.26 定向证据

`desktop-results.json` 逐条保留源提交、程序摘要、四条公共原生、六条用户流程、最后的预览显示补查及每张截图布局。二十张均已实看，原四张无 final 后缀的预览为改文案前记录；其余十六张为当前有效显示。前期通过记录没有改记为最后程序。

`rust.log` 六项通过；`rust-public-io.log` 是桌面发现旧拦截后新增的一项通过，没有重跑前六项。`frontend-zpaq.log` 与 `frontend-user-flow.log` 保留最初继承分卷/名单遗漏的失败，`frontend-batch-fixed.log`、`frontend-store.log` 及前两份日志内通过项合计六项不同用例，非将执行次数当不同测试。

桌面普通/密码独立原件及普通创建首轮通过，密码创建被公共旧校验拦下后仅续受影响项。后续脚本忙态/尾斜杠/元素选择器和滚动定位失败均保留对应日志；`desktop-conversion-final.log` 收口互转，`desktop-display-verified.log` 收口最后显示。Windows 7-Zip CLI 中文密码参数代码页误判后，以现有独立 Python UTF-8 AES 解码器确认完整内容。未把脚本或独立工具参数失败当产品损坏。

`frontend-polish-build.log` / `desktop-polish-build.log` 为最后显示构建，之前 build 日志仅对应各自当时来源；源码 C++ 在全部原生用例前已完成最终重建并锁定。`identity.log` / `shell.log` 固定 1.5.26 八处身份和 Shell。正式候选与四资产收据在 PR/Release 接续，测试桥程序不充当正式安装器。

普通/密码新 ZPAQ 卷及转换输出由作者未改工具完整回解，文件 SHA256 和空目录一致；测试端按序合并，产品不合并。下载和工具摘要见实施文档，未把原版压缩包或测试二进制复制进证据目录。个人程序、设置、密码库和历史未改。
