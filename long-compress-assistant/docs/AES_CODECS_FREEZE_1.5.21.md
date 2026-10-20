# v1.5.21 功能封板与发布交接

2026-10-20，从正式 v1.5.20 `beece33` 接续，运行时冻结 `dc993e7`。六种新 Long AES 内层完整用户链、普通 LZ4/Brotli 有界预览、普通 LZIP 公共完整性分派、历史别名与实际编码抽样完成。八处版本身份统一 1.5.21。

八项不同 Rust、89 前端、十八条实际原生记录、六条实际 UI、十二张实看常用/窄窗页面，以及九组 AES/一组密码 ZIP 的独立完整回解通过。阶段、一次调试 EXE 链接占用及未重复已通过检查的范围见[实施](AES_CODECS_1.5.21.md)、[检查](evidence/aes-codecs-1.5.21/checks.json)与[实际桌面](evidence/aes-codecs-1.5.21/desktop-results.json)。单条展开、多选弹窗、自然继承、左右等宽和窄窗滚动保持。

两份 README、中文 Release、格式说明、路线与交接已同步。本文件是合入前快照；必需 Windows package、候选身份、普通受保护合入、固定 v1.5.21 标签和正式四附件/中文 updater 收据保留在本版 PR 与[发布页](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.21)，不为回填再次触发同树打包。

本版沿用原 AES V2、KDF 和默认开关；需要完整认证及临时空间，不是内层格式的原生密码标准。接收方请使用 v1.5.21 或更新版本。单文件目标只处理一个顶层普通文件，目录/多个文件选择 TAR 组合。下一步按[既定路线](ARCHIVE_ROADMAP_1.5.20_TO_1.5.22.md)做 v1.5.22 校验进度/停止/继续，尚未开发；个人安装与数据未改。
