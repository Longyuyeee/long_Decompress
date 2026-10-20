# 本版检查收据

- Rust：cargo test --manifest-path src-tauri/Cargo.toml --lib wim1533 -- --nocapture，7 passed，855 filtered，实际用例运行1.71秒。
- 前端：6个相关测试文件以 -t wim1533 执行，6 passed、150 skipped；覆盖方法/配置组、混合方法批量、Store/XPRESS/LZX/LZMS草稿、转换历史和不使用旧采样预测。
- vue-tsc与前端构建、Shell、8处版本身份和WIM锁定通过。布局变化后只重新构建对应前端/桌面及补查WIM转换，后端功能和旧算法矩阵不重跑。
- 公共原生与六条实际UI、受影响补查见desktop-progress.json，各产物由独立7-Zip或Python ZIP解码器回解摘要/空目录。
