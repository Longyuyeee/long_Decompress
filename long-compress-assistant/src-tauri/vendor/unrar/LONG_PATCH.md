# Unicode 密码补丁

基于 crates.io `unrar 0.5.8`（上游 `https://github.com/muja/unrar.rs`），保留原 Rust 源码与 MIT/Apache-2.0 许可证。C++ UnRAR 仍来自未修改的 `unrar_sys 0.5.8`。

上游把密码的 UTF-8 字节传给 `RARSetPassword`，该旧接口在 Windows 通过本地字符集转为宽字符，中文等密码会被误解释。本补丁通过 `UCM_NEEDPASSWORDW` 提供宽字符密码，并在打开、读取文件头和处理载荷的回调中保持密码可用；卷切换及处理数据继续使用原回调。UTF-8 之外的原始字节参数保留原兼容路径。

回调按照 UnRAR 提供的缓冲区容量复制含 NUL 的密码，超长和内嵌 NUL 不截断。卷切换只复制实际 NUL 结尾的文件名，修正上游假定 2048 个宽字符缓冲区导致越界读取的错误。句柄关闭前清除回调；句柄 Debug 只显示是否配置密码。公开 Rust API 未改变。

回归使用真实 WinRAR 创建的无密码、内容密码和文件名密码 RAR5 分卷，通过应用的 RarSupportService 回解并逐字节比较，同时检查错密码、缺卷及归档浏览路径。测试密码包含中文、空格、符号和 emoji。详见应用 `rar_native_volumes_roundtrip_with_content_and_header_passwords`。
