# v1.5.18 自有 AES 数字分卷封板快照

2026-10-19，基线正式 v1.5.17 的 `6f41d9e`，独立原件/红灯提交 `c26dea9`，产品运行时冻结 `06101cc`。本文件记录合入前范围；候选安装器、受保护合入、固定标签及正式四附件收据接续在[本版 PR](https://github.com/Longyuyeee/long_Decompress/pulls?q=is%3Apr+head%3Acodex%2Fv1.5.18-aes-volumes)和[正式 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.18)，避免回填文档再触发整包构建。

## 用户可以做什么

九种 Long 自有 AES 格式支持密码数字分卷创建：TAR.AES、TAR.GZ/BZ2/XZ/ZST.AES、GZ/BZ2/XZ/ZST.AES。整组同目录，从任意成员进入浏览、支持的文本/图片预览、检查、所选/整包解压；认证完整后才读取内层。转换沿原目标列表提供五种 TAR.AES 输出，可与密码 ZIP 互转、重新拆卷或合并。四种普通流 AES 不新增为转换输出选项。

V2 按声明总长度发现缺尾；前段已认证后的后段认证失败明确为内容异常。首块失败保留“密码或数据问题”不确定性，不能可靠区分时不强行判断。补齐卷组后通过原纠正弹窗创建并主动启动新任务，旧失败记录保持。修复 TAR.AES 多源目录创建，沿用普通 TAR 的名称、空目录与所选提取策略。

单条行内展开、多选弹窗、自然继承、左右等宽与窄窗滚动保持；只补格式旁说明，不增加常驻大面板。

## 已完成证据

[检查与身份](evidence/aes-volumes-1.5.18/checks.json)、[实际程序收据](evidence/aes-volumes-1.5.18/desktop-results.json)、[独立 AES 回解](evidence/aes-volumes-1.5.18/independent.json)、[独立 ZIP 回解](evidence/aes-volumes-1.5.18/zip-independent.json)。

- 十一份固定独立原件：九种 V2 与两种 V1；CPython 3.12.10 / cryptography 49.0.0 按既有布局生成，工具、内层与外层哈希见[原件清单](../tests/fixtures/archives/aes-1518/manifest.json)。不调用产品加密实现生成正面原件。最初红灯确认能力表拒绝与头部跨卷读取失败；当时小包仅一卷的测试已改名为单成员对照，不冒称中卷证据。
- 22 项不同 Rust：八项新增、十四项相关既有。前七项在版本号仍为 1.5.17 时通过，之后只修真正失败的 TAR.AES 目录创建并通过第八项及相关项；早期七项按原日志身份复用，未将后来的测试 EXE 哈希倒填给它们。
- 805 项前端单元 / 86 文件通过一次。误用 npm 总入口导致执行全量单元，后续 Playwright 参数不适用已停止；只计实际完成的单元，不声称整个 npm test 或 Playwright 通过，也不重跑已通过单元。类型检查、普通及 E2E 前端构建、Shell 版本 DLL 构建通过。
- 24 条真实原生 IPC：十一原件中卷全内容解压、九种产品创建的 33 卷全量往返、末卷图片完整哈希、两种 TAR.AES 只取末尾文件、后段认证失败不发布。不是注入成功状态。
- 五条普通 UI：行内密码分卷创建、中卷末尾文本、撤回卷造成真实失败后补卷/弹窗新任务/完整输出与旧历史保留、AES 卷组转密码 ZIP、密码 ZIP 转三卷 TAR.XZ.AES。产品输出由独立 cryptography 解密，再由独立格式工具全内容回解十组 AES 产物；ZIP 用 UTF-8 密码的 PBKDF2/HMAC/AES-CTR 独立认证三文件并核对完整哈希及空目录。
- 常用外窗 1100×700 与窄窗 760×520 共十张页面逐张实看，保存各次实际内视口几何，横向溢出为 0。超长路径换行、弹窗主体纵向滚动和底部操作可用；名称为外窗设置，像素受设备缩放影响。

## 程序与脚本身份

全部 24 原生记录、五条 UI、十张页面来自运行时 `06101cc` 的同一 custom-protocol,desktop-e2e 程序 SHA-256 `63c1b3426fef3fb3508cdd623dac83fdd4990bdad61ac299ab6d556edac49008`，构建日志单列。普通生产候选由必需 Windows package 提供，不将桌面程序冒称正式包或实际安装。

脚本修正只接续未完成阶段：WebView 155 使用匹配 EdgeDriver；先打开归档再填写密码，等待可用输入，转换选择新增条目，分卷历史输出接受真实首卷 `.001`。最初 plain 原件校验误读工作区 CRLF 文件，已改用固定原件清单的 decodedSha256。Windows 7-Zip 的中文命令行密码编码不能作为本次 UTF-8 ZIP 正确性的依据，改为独立字节级认证，没有改密码规避问题。这些与产品 TAR.AES 目录缺陷分开记录；脚本改动后没有再修改或重建产品。

## 真实盘满补充与后续边界

用户补充 workflow 授权后，使用已有有权限登录推送独立 CI 入口，运行 [39211588338](https://github.com/Longyuyeee/long_Decompress/actions/runs/39211588338)。仅 Actions 自有 256 MiB NTFS VHD，不触碰个人盘符；当前正在运行，尚不计真实盘满通过，最终结果在本版 PR 接续。既有注入 StorageFull、Windows 文件占用证据不替代真实盘满。

这是自有容器数字切卷，接收方使用 Long解压，不是通用 TAR 原生密码/分卷标准。加密布局、算法、KDF 与默认选项未改。V1 有限只读兼容保持 512 MiB 既有限制；缺尾无法可靠与错密/损坏区别。完整认证需要临时空间；文本/图片继续沿用既有读取预算，不承诺大包瞬时预览。个人安装、设置、历史未改。

下一步为[既定路线](ARCHIVE_ROADMAP_1.5.17_TO_1.5.19.md)中的 v1.5.19 逐文件校验原因与待补列表，尚未开始。ALZ 方法 3、EGG 加密固实、Windows N 原边界保持，不追加重复路线。
