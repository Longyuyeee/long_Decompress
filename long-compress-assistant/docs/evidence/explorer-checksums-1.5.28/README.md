# v1.5.28 定向记录

范围与程序身份见[封板](../../EXPLORER_CHECKSUMS_FREEZE_1.5.28.md)。desktop-results.json保留四链实际结果、初轮/补查程序身份和截图映射；六张补查只涉及来源、长路径清单名与失败通知。十张实看，逻辑尺寸对应DPI2双倍像素。

rust.log共三项，不将其他零项过滤套件计入通过数。frontend.log首轮五过一失败，frontend-fixed.log仅补关联五项；frontend-error.log新增一项，合计七项不同前端。初期desktop.log/desktop-wait-fixed.log是WebDriver args不能传入应用CLI的失败记录，未冒充冷启动通过。desktop-real-cli.log四条真实单实例/UI链通过，desktop-polish.log只补三组显示。

shell.log、identity.log与最终frontend-final-polish-build.log/desktop-final-polish-build.log记录最终构建；早期构建保留诊断身份，不重记为最终构建。没有真实个人菜单点击或安装；菜单注册在隔离HKCU树检查。候选与正式包身份在PR接续，避免回填引发重复打包。
