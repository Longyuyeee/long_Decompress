# v1.6.1 定向记录

已通过的步骤保留各自程序身份，不重复运行。`desktop.json` 记录接续身份、四条流程与八条布局；最后两个更新/结果流程在长路径修正后的候选完成。

| 界面 | 宽窗口 | 窄窗口 |
| --- | --- | --- |
| 多选配置 | [1100×700](bulk-1100x700.png) | [760×520](bulk-760x520.png) |
| 原生组配置 | [1100×700](mapping-1100x700.png) | [760×520](mapping-760x520.png) |
| 更新结果 | [1100×700](result-1100x700.png) | [760×520](result-760x520.png) |
| 全部部分 | [1100×700](parts-1100x700.png) | [760×520](parts-760x520.png) |

运行依据：`engine-probe.json` 为官方/适配器各普通/密码的第四事务及四快照回解；`rust.log` 为三项新增行为，`long-reference-final.log` 为深路径修正，`single-update-regression.log` 为受影响的四种原输入输出组合。`desktop-path-fix.log` 为只接续余下两条实机流程。

初次相关前端命令为 `npx vitest run src/stores/__tests__/zpaqNative161.test.ts src/stores/__tests__/zpaqNative1564.test.ts src/components/compression/__tests__/ZpaqUpdate1536.test.ts`，工具返回三文件、六项通过（2026-10-22 17:37:28，29.12 秒）。恢复修正后补两项：原缺失新增来源提示通过，原生模式恢复的测试修正其 `suggest_zpaq_multipart` 空返回模拟后通过；最终原生项输出见 `frontend-native-recovery.log`。共八项不同案例，不计重复运行。类型/Vite、版本身份/Shell 与原生构建记录保存在本地 `test-results/zpaq-native-161/`，不把所有旧测试声称为通过。
