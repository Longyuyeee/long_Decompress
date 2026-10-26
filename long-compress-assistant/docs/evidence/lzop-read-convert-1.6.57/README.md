# v1.6.57 本地候选证据

最终候选 SHA256 与四条实际桌面业务、四张宽窄截图见 desktop.json；源码/合并/正式附件身份在本版 PR 接续。桌面候选使用 custom-protocol,desktop-e2e 和本地 opt-level=1/LTO=false，正式安装包由未修改的生产 CI/Release 参数生成，不能把候选等同于正式签名附件。

核心四组 Rust、前端两项、类型与生产 Vite、桌面前端资产、版本身份与 Shell、实际桌面通过。前端初次第二项误引入私有函数，frontend-initial.log 保留失败及第一项通过；修正后 frontend.log 只续跑第二项。未重复旧格式矩阵、安装或盘满检查。新增所选/筛选回归通过，受影响工作区能力回归通过。

截图已逐张实看；text-narrow 中的“先填写密码”是沿用公开测试 TXT 的原文，不是 LZOP 密码要求或界面提示。原件生产/回解摘要见 tests/fixtures/archives/lzop-read-1657/manifest.json，官方测试工具不随应用分发。
