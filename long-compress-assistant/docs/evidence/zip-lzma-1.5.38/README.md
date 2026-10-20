# v1.5.38 证据范围

七项不同 Rust、六项新增前端、三条公共原生与六条实际 WebView2 UI 通过。desktop-progress.json 保留真实程序身份、输出、独立内容哈希、资源采样与全部22张宽/窄布局记录；本目录保留七张已查看代表截图。checks.json 汇总，不替代原始输出。

probe-release.log 是版本号提升前的方法14原型，Python/liblzma 与7-Zip完整回解通过。rust-final.log 中五项通过，九组合检查发现 AES 外层隐藏了 LZMA 的 EOS/版本标记；修复后仅接续失败的组合检查，rust-aes-fixed.log 九组合全部通过。其他已通过项保留，不重跑。

前端首次5项通过、1项因新增独立案例缺 Pinia 初始化失败；补测试初始化后只接续这一项。桌面检查器最初沿用 PPMD 方法名正则而未识别实际 LZMA，修正检查器后执行完整链；未将其计为产品缺陷。最终运行时源02418eb9415b6c8fe1eec5f7ea5dcc77ddb976c4，SHA256 6cbac22e40fa6d0e7ff4500e583ab0cca9d6ec86bf434e8d50261220abf59f2e。

未重跑旧格式/密码矩阵、真实盘满，未改个人安装。候选、受保护合入、标签和正式四附件收据在本版PR接续，避免文档回填再触发重复打包。
