# v1.5.46 密码 ZIP 编辑与草稿恢复记录

[封板范围](../../ZIP_PASSWORD_DRAFTS_FREEZE_1.5.46.md) · [检查摘要](checks.json) · [实际桌面记录](desktop.json)。候选、合入、标签与正式附件收据在本版 PR 接续。

70 项不同前端、18 项相关 Rust、类型/Vite、Shell/版本身份与原生构建已通过。四条桌面流程使用同一最终二进制，包含错误密码后保留修改、保险箱选密、保留密码另存、新密码与仅修改密码、新包继续读取、真实正常关闭并用原隔离资料目录重启、缺失/变化来源修复、已有目标失败后改名重试，以及已完成任务残留草稿不重放。三份结果用 7-Zip 独立完整回解并核对内容。

正常关闭测试实际点击窗口关闭，先写入草稿再退出；最后一条另将带已完成任务 ID 的残留草稿写入专用测试目录，并重启读取数据库收据，两者分别记录。没有操作个人资料或正式安装。

首两条通过后保留收据，第三条测试夹具的 Python 追加保留了旧 Unicode 路径扩展字段 CRC，产品正确拒绝；改用 7-Zip 更新有效原包后接续。随后修正自动化选择器，使文件夹定位按钮按草稿 ID 选择，不再把文件夹路径发给文件项。运行时代码和二进制未变化，未重跑前两条。前端三处 isolated mock/异步等待修正只重跑对应项；Rust 两个加强用例只重跑受影响项。

八处界面各检查 1100×700、760×520，共 16 条布局记录，无横向溢出。复核宽窗两列各 426 CSS px，窄窗顺排 652 px，内容可滚动、底部按钮可见。以下八张已实际查看；来源问题图中的读取错误提示来自刻意移动原包的场景，随后定位/确认和另存成功见结果图。

- [password-review-1100x700](zip-edit1546-password-review-1100x700.png)
- [password-review-760x520](zip-edit1546-password-review-760x520.png)
- [new-password-settings-1100x700](zip-edit1546-new-password-settings-1100x700.png)
- [draft-recovery-list-760x520](zip-edit1546-draft-recovery-list-760x520.png)
- [recovery-sources-1100x700](zip-edit1546-recovery-sources-1100x700.png)
- [recovery-sources-760x520](zip-edit1546-recovery-sources-760x520.png)
- [recovered-result-1100x700](zip-edit1546-recovered-result-1100x700.png)
- [recovered-result-760x520](zip-edit1546-recovered-result-760x520.png)
