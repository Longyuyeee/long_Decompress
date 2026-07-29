# 开发交接

## 当前情况

- 当前分支：`codex/all-format-real-e2e`，对应 PR [#22](https://github.com/Longyuyeee/long_Decompress/pull/22)。
- 压缩与解压主流程、任务状态、取消、冲突处理和真实桌面 E2E 已建立。
- 已完成 ZIP、7Z、TAR 系列及多种只读归档、文件系统、虚拟磁盘的真实载荷验证。
- 本轮新增 MSI、MSM、MSP、APFS 和 UEFI 固件的可复现测试样本，并已通过 Release Tauri 桌面闭环；MSM 会继续解开内嵌 CAB，不再只输出中间容器。
- 全格式桌面 E2E 新增严格模式；缺少任一外部生成器时会汇总失败，不再允许将静默跳过误记为全格式通过。
- 2026-07-30 已运行 `npm.cmd run test:prepare:full-format` 和
  `npm.cmd run test:e2e:desktop:full-format`：25 种可创建格式、虚拟磁盘、FAT16/NTFS、APFS、
  SquashFS、MSI/MSM/MSP、UEFI 与固定上游样本均完成真实载荷闭环。
- Windows 11 顶层右键菜单仍受签名证书限制，当前不作为开发阻塞项。
- HFSX 目前只能可靠生成空镜像，尚未找到可写入已知载荷的可信方案，因此不能标记为完整通过。
- v1.0.18 候选已修复无签名 Windows 11 菜单降级与覆盖安装残留，并通过公开 v1.0.17
  覆盖安装、41 项状态/数据检查、卸载和基线恢复。

## 接手后要做什么

1. 找到可再分发、非空的 HFSX 样本或可靠写入工具，补齐 HFSX 真实载荷验证。
2. v1.0.18 正式发布后，从保留的公开 v1.0.17 环境执行应用内升级并回填验收记录；当前
   “latest 正式 Release”端点无法在发布前向 v1.0.17 安全提供候选版。
3. 用正式安装版复测文件选择、拖放、批量开始、进度、取消、冲突处理、清除已完成和输出目录。
4. 后续改动继续使用 `npm.cmd run test:e2e:desktop:full-format` 作为严格回归；只识别文件头、空镜像或损坏样本不算通过。
5. 安装、升级与全量回归通过后再提升版本、打安装包并更新 README 与 Release。
