# 压缩任务清除与历史保存一致性

## 本轮第二步

第一步 `5f46d9d` 已推送后，继续处理 [用户场景审计](USER_SCENARIO_AUDIT_2026-09-12.md) 中发现的问题。没有改动 Rust 或历史落库保护。

用户场景：

1. “我刚压缩完就点清除，为什么文件行没了，任务却还在？”
2. “历史保存失败了，别把我这条记录弄没；我点重试保存，不是让你再压缩一次。”

实际原因：压缩中心的批量清除、单项清除都先调用 `removeJobsByTaskIds` 删除工作项，再调用任务仓库。后者在 `historySaving/historySaveError` 存在时拒绝移除，造成两处状态不同步。新增 3 条组件场景在修复前均因工作项变为 0 条而失败，证实这不只是旧测试等待不足。

## 修复边界

- 先让任务仓库执行受保护的清除，再用实际剩余任务 ID 同步页面工作项；批量与单项共用同步逻辑。
- 未保存成功的任务和文件行均保留，告知用户稍后清除或在底部任务面板“重试保存”。不暗中重试压缩、不删除输出文件。
- 保留已有历史保存保护，不新增人为延迟或门禁；用户保存成功后可再次清除。
- 版本 1.3.2、schema 8 不变。

## 测试真实性说明

- 旧测试 `clears only finished compression tasks from the compression center` 原来只等待 Vue nextTick，没有等待异步历史保存。本轮保留“仅移除压缩任务、保留解压任务”的原断言，通过 `waitForHistoryPersistence` 等待真实任务仓库的保存 Promise，而非直接修改 historySaving 或伪造保存成功。
- 新增 bulk/single 两条：手动控制保存 Promise，在其完成前点击清除，任务与行必须保留；完成后再次点击才可同时移除。
- 新增失败/重试一条：注入实际 rejected Promise，保存失败后清除仍保留；调用既有保存重试后可清除，并断言未调用压缩命令。此处是组件场景，不伪称真实磁盘写入失败。
- 定向压缩页面、任务仓库、底部保存重试共 **52/52**；正式 `npm run test:unit`（含原有 PDF 合同和图片资料准备）**58 文件 372/372**；类型检查、生产构建、diff 检查通过。
- 未修改/跳过失败断言，未删除测试，未增加测试专用产品分支。第一步曾记录的 368/369 是修复前结果，保留用于追溯。
- 本轮真实桌面文件选择器定位失败的限制仍有效，不能因单测全绿宣称安装版验收已完成。下一步仍需真实完整/截断/重新下载的 UI 链与输出核对。

## 推送与构建证据

- 两步分别提交并推送：`5f46d9d`（失败原因优先）、`e5cec5c`（清除一致性）。[PR #137](https://github.com/Longyuyeee/long_Decompress/pull/137) 合入 `master@c9fd64a`。
- 最终代码 `e5cec5c` 的 [Windows package CI 34680346092](https://github.com/Longyuyeee/long_Decompress/actions/runs/34680346092) 成功，耗时 5m38s，版本核对、NSIS 构建及产物上传均通过。随后仅补录此交接证据。
- 本机最终代码也已通过 `cargo build --release --features custom-protocol,desktop-e2e`，测试程序 SHA-256 为 `7DABB4623D318D3FF421E2B6CE540FB8CAEC160B035DBB2BFEEE68B6A9D74E33`。此程序包含隔离测试功能，不是正式发布安装包；工具故障后没有再次启动进行 UI 验收。
