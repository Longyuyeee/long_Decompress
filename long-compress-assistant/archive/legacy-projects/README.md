# 历史工程归档

此目录只保存不参与当前产品构建的历史原型，便于追溯，不属于 Long解压 的运行时、安装资源或发布源代码入口。

- `TranslateSoftware`：早期 React/Tauri 模板残片；没有 `package.json`、Cargo manifest 或正式构建引用，已从 `src-tauri` 移出。

归档内容不得被加入 `tauri.conf.json > bundle.resources`，也不得作为当前功能文档引用。若未来需要复用，应先建立独立工作项、依赖审计和真实测试门禁。
