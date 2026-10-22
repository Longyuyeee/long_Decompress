# v1.6.5 ALZ Bzip2 封板依据

- [旧 helper 实际结果](baseline.json)：原 CC0 Bzip2 条目的尾随和缺结束标记均错误返回 verified=true。原件不改。
- [终止红灯](red-end.log)、[声明边界红灯](red-boundary.log)：原实现真实失败；[四项解码器](decoder.log)与[一项应用服务](application.log)通过。零项筛选不计通过数量。
- [helper 八类故障与不确定边界](helper-verification.json)：密码/三卷是明确标注的受控派生，不是新的原 ALZip 组合来源。原件身份与逐文件预期保留在样本清单。
- [普通类型/Vite](frontend-build.log)、[Shell](shell-build.log)、[helper](helper-build.log)通过。版本八处和资源身份通过；本地桌面为 opt-level=1、LTO=false，正式构建默认优化不变。
- [桌面候选](candidate.json)、[构建](native-desktop.log)、[执行](desktop.log)与[五项结果](desktop.json)：三条实际生产 IPC 记录，两条实际界面流程（失败→重新获取后新任务；历史原因）。四张宽窄图均已逐张查看，横向溢出为零。
- 首次桌面启动因缺测试构建开关，尚未进入场景；首次脚本把 CP949 当成公共 ZIP 编码枚举而被参数检查拒绝，改为原 ALZ 的默认 null 后接续。两者不计产品红灯，也不放宽断言。普通生产、已过后端与旧格式矩阵不重跑。

只保存本次候选、受影响的实际流程和图，不复制安装或盘满旧矩阵。正常 PR/标签与发布资产状态在本版 PR/Release 接续，不回填提交反复触发同一候选的打包。
