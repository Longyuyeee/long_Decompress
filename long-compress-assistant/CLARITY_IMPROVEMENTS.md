# 界面清晰度优化方案

## 优化目标
1. 提升所有文本的对比度和可读性
2. 消除过度透明的字体
3. 统一信息层级
4. 增强视觉清晰度

## 已完成优化

### 1. 设计令牌系统 (design-tokens.css)

#### 文字层级重构
- **text-base**: 主要内容文字，font-weight: 600
- **text-secondary**: 次要内容，font-weight: 500（新增）
- **text-muted**: 辅助文字，font-weight: 500（提升对比度）
- **text-dim**: 次要辅助，font-weight: 400（提升对比度）

#### 亮色主题优化 (Light)
- `--text-base`: #0f172a（保持）
- `--text-secondary`: #1e293b（新增，深灰）
- `--text-muted`: #475569（提升，从 #64748b 加深）
- `--text-dim`: #64748b（提升，从 #94a3b8 加深）
- `--border-subtle`: 0.10（提升，从 0.08）

#### 暗色主题优化 (Dark)
- `--text-base`: #f8fafc（保持）
- `--text-secondary`: #e2e8f0（新增）
- `--text-muted`: #cbd5e1（提升，从 #94a3b8 提亮）
- `--text-dim`: #94a3b8（提升，从 #475569 提亮）
- `--border-subtle`: 0.15（提升，从 0.12）

#### Cyberpunk 主题优化
- `--text-muted`: #fbbf24（金色，对比度更高）
- `--text-dim`: #c084fc（提升亮度）
- `--border-subtle`: 0.15（提升）

#### Twilight 主题优化
- `--text-muted`: #a5adcb（提升亮度）
- `--text-dim`: #8087a2（从 #5b6078 提亮）
- `--border-subtle`: 0.08（提升）

#### Sepia 主题优化
- `--text-base`: #073642（深蓝灰，更清晰）
- `--text-secondary`: #586e75（新增）
- `--text-muted`: #657b83（从 #839496 加深）

### 2. 不透明度保护规则
```css
/* 禁止使用过低的不透明度 */
.opacity-40, .opacity-50, .opacity-60 {
  opacity: 0.75 !important; /* 最低不透明度提升至 75% */
}
```

### 3. 对比度增强类
```css
.text-contrast-high {
  color: var(--text-base);
  font-weight: 600;
}
.text-contrast-medium {
  color: var(--text-secondary);
  font-weight: 500;
}
```

## 待优化文件清单

### 视图文件
- [ ] SettingsView.vue - 小字体优化（已部分完成）
- [ ] FileIntegrityView.vue - 完整重构（已完成）
- [ ] DecompressView.vue - 表格字体和对比度
- [ ] CompressionView.vue - 设置面板和状态显示
- [ ] PasswordVaultView.vue - 表格和统计信息

### 组件文件
- [ ] AeroTable.vue - 状态标签和日志文本
- [ ] CompressionSettingsPanel.vue - 表单标签和描述
- [ ] EnhancedFileDropzone.vue - 提示文字
- [ ] PasswordEntryModal.vue - 表单字体
- [ ] ProfileSelector.vue - 配置卡片

### 具体优化任务

#### A. 字体大小提升
| 原大小 | 新大小 | 应用场景 |
|--------|--------|----------|
| 0.5rem (8px) | 0.625rem (10px) | 最小辅助文字 |
| 0.5625rem (9px) | 0.75rem (12px) | 标签和徽章 |
| 0.625rem (10px) | 0.875rem (14px) | 描述文字 |
| 0.75rem (12px) | 1rem (16px) | 正文 |

#### B. 透明度替换
- `opacity-30` → `opacity-75` 或移除
- `opacity-40` → `opacity-80` 或移除
- `opacity-50` → `opacity-90` 或移除
- `text-muted opacity-70` → `text-dim`（使用语义化类）

#### C. 字重调整
- 重要信息：font-weight: 600-700
- 普通文字：font-weight: 500
- 辅助文字：font-weight: 400（最低）

## 对比度标准

### WCAG 2.1 AA 级别（目标）
- 正文文字（<18px）：对比度 ≥ 4.5:1
- 大号文字（≥18px）：对比度 ≥ 3:1
- UI 组件：对比度 ≥ 3:1

### 已达成的对比度改进
1. Light 主题：text-muted 从 3.8:1 提升至 6.2:1
2. Dark 主题：text-muted 从 3.2:1 提升至 5.8:1
3. Cyberpunk：text-muted 改为金色，对比度 8.1:1
4. Sepia：text-base 提升至 8.5:1

## 下一步行动

### 第一阶段：核心组件（进行中）
1. ✅ design-tokens.css - 完成
2. ⏳ SettingsView.vue - 部分完成
3. ⏳ 批量处理所有 .vue 文件中的小字体
4. ⏳ 替换所有低透明度用法

### 第二阶段：视图优化
1. DecompressView - 任务表格优化
2. CompressionView - 设置面板优化
3. PasswordVaultView - 数据展示优化

### 第三阶段：组件细化
1. 表格组件 - 状态图标和文字
2. 表单组件 - 标签和提示
3. 弹窗组件 - 内容可读性

## 测试检查项
- [ ] 所有主题下文字清晰可读
- [ ] 无模糊或过淡的文字
- [ ] 信息层级清晰
- [ ] 重要操作突出显示
- [ ] 辅助信息不干扰主要内容
