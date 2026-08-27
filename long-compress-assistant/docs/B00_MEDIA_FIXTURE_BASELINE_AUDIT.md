# B-00.4 真实媒体样本基线审计

日期：2026-08-27

分支：`codex/archive-media-roadmap`

公开版本：`1.1.14`（本工作项不升版、不发布）

## 1. 结论

B-00.4 已建立可再生成、无用户数据、带机器可验证预期属性的真实媒体夹具。仓库提交生成器、精确依赖版本、测试工具身份和属性清单；实际图片、视频、PDF、渲染图与结果 JSON 位于 Git 忽略目录 `test-results/media-fixture-audit`，不扩大仓库和安装包。

这些夹具以解码/探测后的属性为验收依据，不承诺不同工具链运行产生逐字节相同文件，也不得直接作为性能基准。特别是 PDF 签名、容器元数据和编码器版本可能改变文件哈希。B-01 必须另行冻结可处理输入的 SHA-256 清单；这一边界已写入 `manifest.json` 并由 B-00.5 真实门禁检查。

本节点仍不实现媒体压缩。用于生成视频的 GPL FFmpeg 只允许作为测试工具，固定 GitHub asset ID、字节数和 SHA-256，清单明确 `productIntegrationAllowed=false`；它不会进入 `src-tauri/resources`、Cargo 依赖或 NSIS。

## 2. 固定样本与真实属性

| 类别 | 样本 | 真实验收属性 |
| --- | --- | --- |
| 图片 | 透明 PNG | 256×256，真实 RGBA alpha 通道 |
| 图片 | EXIF JPEG | 640×360，Make=`LongDecompressFixture`，Orientation=6，无 GPS/用户信息 |
| 图片 | 动画 GIF | 320×180，3 帧，逐帧时长 100/200/300 ms |
| 图片 | 超大 PNG | 12000×8000，共 96,000,000 像素；不是改扩展名或空占位 |
| 视频 | H.264 综合样本 | 640×360，H.264 + AAC + mov_text，三种实际包时长，90° Display Matrix |
| 视频 | H.265 样本 | 640×360，HEVC/hvc1，固定帧率，无伪造音轨或字幕 |
| PDF | 文本/扫描/透明 | 可搜索矢量文本；纯栅格扫描无 PDF 文本对象；透明图形状态存在 |
| PDF | 表单 | 两个真实 AcroForm 字段及有效外观流 |
| PDF | 签名 | 单个 detached CMS 签名，`valid=true`、`intact=true`；测试证书自签名，所以 `trusted=false` |
| PDF | 加密拒绝 | AES-256；未授权读取页树失败，固定测试密码授权后读取 1 页 |

PDF 六个首屏均由 Poppler 渲染为 PNG 并逐张检查：无裁切、重叠、黑块或不可读内容。表单值和勾选外观可见，签名外观可见，加密 PDF 仅在提供固定测试密码后可渲染。

## 3. 预期、首次实际、修正与最终实际

| 检查 | 预期 | 首次实际差异 | 修正 | 最终实际 |
| --- | --- | --- | --- | --- |
| ReportLab 表单 | 生成两个 AcroForm 字段 | 代码误用 `acroform`，真实运行抛出属性错误 | 按当前固定版本 API 改为 `acroForm` | 两字段名称、值和渲染外观通过 |
| pyHanko 签名 | 生成真实 CMS 签名 | 当前 API 要求显式 `other_certs`，首次调用被拒绝 | 显式传入空额外证书链，并增加密码保护 PKCS#12 | 签名 valid/intact，整文件覆盖；自签名保持不受信任 |
| 加密 PDF | 未授权拒绝、授权成功 | 首次检查在解密前读取页数，pypdf 正确抛出 `FileNotDecryptedError` | 把异常作为拒绝证据，随后用固定密码解密再检查页数 | 未授权拒绝=true，授权=true，页数=1 |
| 90° 视频旋转 | ffprobe 必须看到 Display Matrix | `-metadata rotate=90` 写入后实际旋转仍为 0，门禁失败 | 使用输入侧 `-display_rotation:v:0 90` 后无损复制 | side data rotation=90 |
| VFR | 不用容器标签冒充可变帧率 | 预期三段时长 0.20/0.70/0.10 秒 | 以 ffprobe packet duration 去重作为事实来源 | 实际为 0.200000/0.720000/0.040000，VFR=true |
| PDF 视觉 | 六种 PDF 都能清晰渲染 | 扫描样本首次字体过小，虽结构通过但视觉不合格 | 改为嵌入栅格的大字号字体后重新生成、重渲染 | 六个首屏视觉复核通过 |

## 4. 可复验命令

```powershell
$env:LONG_MEDIA_FIXTURE_PYTHON = '<具备 Python 3.12 的路径>'
$env:LONG_MEDIA_PDFTOPPM = '<Poppler pdftoppm.exe 路径>'
npm.cmd run test:fixtures:media
npm.cmd run test:media-dependencies
npm.cmd run test:media-architecture
npm.cmd run type-check
```

`test:fixtures:media` 会在忽略目录隔离安装 `tests/fixtures/media/requirements.txt` 的精确版本，核对 FFmpeg 测试资产字节和 SHA-256，生成全部夹具，再用 Pillow、pypdf、pyHanko、ffprobe 与 Poppler 验证。最终机器证据为 `test-results/media-fixture-audit/result.json`，其中 `differences` 必须为空。

## 5. 需求对齐与下一步

- 样本全部是合成数据，不含用户路径、照片、文档、密码或聊天附件。
- 没有将测试用 GPL FFmpeg 当成产品候选；B-00.3 的 LGPL 产品边界保持不变。
- 没有新增媒体 Tab、媒体队列、模拟进度或占位成功状态，归档主流程没有被替换。
- B-00 尚未整体完成，因此不升版本、不打包、不创建 Release。
- 下一步为 B-00.5：定义图片、视频、PDF 的真实进度、已处理字节、输出大小、压缩率、速度和 ETA 的唯一事实来源；无法从引擎或文件系统证明的指标必须明确为不可用。
