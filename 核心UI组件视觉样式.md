# 胧压缩·方便助手 - 核心UI组件视觉样式

## 1. 按钮组件 (Button)

### 1.1 主要按钮 (Primary Button)
```css
/* 主要按钮 - 标准样式 */
.btn-primary {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 12px 24px;
  background: linear-gradient(135deg, #0ea5e9 0%, #0284c7 100%);
  color: white;
  border: none;
  border-radius: 12px;
  font-size: 14px;
  font-weight: 600;
  line-height: 1;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: 0 4px 12px rgba(14, 165, 233, 0.3);
}

/* 悬停状态 */
.btn-primary:hover {
  background: linear-gradient(135deg, #0284c7 0%, #0369a1 100%);
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(14, 165, 233, 0.4);
}

/* 激活状态 */
.btn-primary:active {
  transform: translateY(0);
  box-shadow: 0 2px 8px rgba(14, 165, 233, 0.3);
}

/* 禁用状态 */
.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  transform: none;
  box-shadow: none;
}
```

### 1.2 次要按钮 (Secondary Button)
```css
/* 次要按钮 - 毛玻璃效果 */
.btn-secondary {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 12px 24px;
  background: rgba(255, 255, 255, 0.8);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  color: #334155;
  border: 1px solid rgba(255, 255, 255, 0.3);
  border-radius: 12px;
  font-size: 14px;
  font-weight: 600;
  line-height: 1;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.btn-secondary:hover {
  background: rgba(255, 255, 255, 0.95);
  backdrop-filter: blur(30px);
  -webkit-backdrop-filter: blur(30px);
  border-color: rgba(255, 255, 255, 0.5);
  transform: translateY(-1px);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1);
}
```

### 1.3 图标按钮 (Icon Button)
```css
/* 图标按钮 - 圆形毛玻璃 */
.btn-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  background: rgba(255, 255, 255, 0.7);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 50%;
  color: #334155;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.btn-icon:hover {
  background: rgba(255, 255, 255, 0.9);
  backdrop-filter: blur(30px);
  -webkit-backdrop-filter: blur(30px);
  transform: scale(1.05);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
}

/* 小图标按钮 */
.btn-icon-sm {
  width: 36px;
  height: 36px;
  font-size: 16px;
}

/* 大图标按钮 */
.btn-icon-lg {
  width: 64px;
  height: 64px;
  font-size: 24px;
}
```

### 1.4 按钮尺寸变体
```css
/* 小按钮 */
.btn-sm {
  padding: 8px 16px;
  font-size: 12px;
  border-radius: 8px;
}

/* 中按钮 (默认) */
.btn-md {
  padding: 12px 24px;
  font-size: 14px;
  border-radius: 12px;
}

/* 大按钮 */
.btn-lg {
  padding: 16px 32px;
  font-size: 16px;
  border-radius: 16px;
}
```

## 2. 输入框组件 (Input)

### 2.1 文本输入框 (Text Input)
```css
/* 基础文本输入框 */
.input-text {
  width: 100%;
  padding: 12px 16px;
  background: rgba(255, 255, 255, 0.9);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  border: 1px solid rgba(203, 213, 225, 0.5);
  border-radius: 12px;
  font-size: 14px;
  color: #334155;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.input-text:focus {
  outline: none;
  background: rgba(255, 255, 255, 0.95);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border-color: #0ea5e9;
  box-shadow: 0 0 0 3px rgba(14, 165, 233, 0.1),
              0 4px 16px rgba(0, 0, 0, 0.1);
}

/* 错误状态 */
.input-text.error {
  border-color: #ef4444;
  background: rgba(239, 68, 68, 0.05);
}

.input-text.error:focus {
  box-shadow: 0 0 0 3px rgba(239, 68, 68, 0.1);
}

/* 成功状态 */
.input-text.success {
  border-color: #22c55e;
  background: rgba(34, 197, 94, 0.05);
}
```

### 2.2 文件拖放区域 (File Drop Zone)
```css
/* 文件拖放区域 */
.file-drop-zone {
  position: relative;
  width: 100%;
  min-height: 200px;
  padding: 48px 24px;
  background: rgba(255, 255, 255, 0.7);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 2px dashed rgba(203, 213, 225, 0.5);
  border-radius: 20px;
  text-align: center;
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

/* 拖放悬停状态 */
.file-drop-zone.dragover {
  background: rgba(14, 165, 233, 0.1);
  backdrop-filter: blur(30px);
  -webkit-backdrop-filter: blur(30px);
  border-color: #0ea5e9;
  border-style: solid;
  transform: scale(1.02);
  box-shadow: 0 8px 32px rgba(14, 165, 233, 0.2);
}

/* 拖放区域图标 */
.file-drop-icon {
  font-size: 48px;
  color: #0ea5e9;
  margin-bottom: 16px;
  transition: all 0.3s ease;
}

.file-drop-zone.dragover .file-drop-icon {
  transform: scale(1.1);
  color: #0284c7;
}

/* 拖放区域文本 */
.file-drop-text {
  font-size: 16px;
  font-weight: 600;
  color: #334155;
  margin-bottom: 8px;
}

.file-drop-hint {
  font-size: 14px;
  color: #64748b;
}
```

## 3. 进度条组件 (Progress Bar)

### 3.1 基础进度条
```css
/* 进度条容器 */
.progress-container {
  width: 100%;
  height: 8px;
  background: rgba(226, 232, 240, 0.5);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  border-radius: 4px;
  overflow: hidden;
}

/* 进度条填充 */
.progress-bar {
  height: 100%;
  background: linear-gradient(90deg, #0ea5e9 0%, #38bdf8 100%);
  border-radius: 4px;
  transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
  overflow: hidden;
}

/* 进度条动画效果 */
.progress-bar::after {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(
    90deg,
    transparent 0%,
    rgba(255, 255, 255, 0.3) 50%,
    transparent 100%
  );
  animation: progress-shimmer 2s infinite;
}

@keyframes progress-shimmer {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(100%); }
}
```

### 3.2 状态进度条变体
```css
/* 成功状态进度条 */
.progress-bar.success {
  background: linear-gradient(90deg, #22c55e 0%, #4ade80 100%);
}

/* 警告状态进度条 */
.progress-bar.warning {
  background: linear-gradient(90deg, #f97316 0%, #fb923c 100%);
}

/* 错误状态进度条 */
.progress-bar.error {
  background: linear-gradient(90deg, #ef4444 0%, #f87171 100%);
}

/* 完成状态进度条 */
.progress-bar.complete {
  background: linear-gradient(90deg, #22c55e 0%, #86efac 100%);
  animation: progress-pulse 2s infinite;
}

@keyframes progress-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.8; }
}
```

### 3.3 带标签进度条
```css
/* 带标签进度条容器 */
.progress-with-label {
  display: flex;
  align-items: center;
  gap: 16px;
}

/* 进度标签 */
.progress-label {
  min-width: 80px;
  font-size: 14px;
  font-weight: 600;
  color: #334155;
  text-align: right;
}

/* 进度百分比 */
.progress-percentage {
  min-width: 40px;
  font-size: 14px;
  font-weight: 600;
  color: #0ea5e9;
  text-align: left;
}

/* 进度详情 */
.progress-details {
  font-size: 12px;
  color: #64748b;
  margin-top: 4px;
}
```

## 4. 表格组件 (Table)

### 4.1 基础表格
```css
/* 表格容器 */
.table-container {
  width: 100%;
  background: rgba(255, 255, 255, 0.7);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 16px;
  overflow: hidden;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.1);
}

/* 表格 */
.table {
  width: 100%;
  border-collapse: collapse;
}

/* 表头 */
.table thead {
  background: rgba(248, 250, 252, 0.8);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
}

.table th {
  padding: 16px;
  font-size: 14px;
  font-weight: 600;
  color: #334155;
  text-align: left;
  border-bottom: 1px solid rgba(226, 232, 240, 0.5);
}

/* 表格行 */
.table td {
  padding: 16px;
  font-size: 14px;
  color: #475569;
  border-bottom: 1px solid rgba(226, 232, 240, 0.3);
  transition: background-color 0.2s ease;
}

/* 行悬停效果 */
.table tbody tr:hover {
  background: rgba(241, 245, 249, 0.5);
}

/* 最后一行 */
.table tbody tr:last-child td {
  border-bottom: none;
}
```

### 4.2 可选中表格
```css
/* 选择列 */
.table-selectable th:first-child,
.table-selectable td:first-child {
  width: 48px;
  text-align: center;
  padding: 0;
}

/* 复选框样式 */
.table-checkbox {
  width: 20px;
  height: 20px;
  border: 2px solid #cbd5e1;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s ease;
  position: relative;
}

.table-checkbox:checked {
  background: #0ea5e9;
  border-color: #0ea5e9;
}

.table-checkbox:checked::after {
  content: '✓';
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  color: white;
  font-size: 12px;
  font-weight: bold;
}

/* 选中行样式 */
.table tbody tr.selected {
  background: rgba(14, 165, 233, 0.1);
}
```

### 4.3 状态列样式
```css
/* 状态指示器 */
.status-indicator {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

/* 状态点 */
.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

/* 状态颜色 */
.status-waiting .status-dot {
  background: #94a3b8;
}

.status-processing .status-dot {
  background: #0ea5e9;
  animation: status-pulse 2s infinite;
}

.status-success .status-dot {
  background: #22c55e;
}

.status-error .status-dot {
  background: #ef4444;
}

@keyframes status-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
```

## 5. 卡片组件 (Card)

### 5.1 基础卡片
```css
/* 基础卡片 */
.card {
  background: rgba(255, 255, 255, 0.7);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 20px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1);
  overflow: hidden;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.card:hover {
  transform: translateY(-4px);
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.15);
}

/* 卡片头部 */
.card-header {
  padding: 24px;
  border-bottom: 1px solid rgba(226, 232, 240, 0.5);
  background: rgba(248, 250, 252, 0.5);
}

.card-title {
  font-size: 18px;
  font-weight: 700;
  color: #0f172a;
  margin: 0;
}

.card-subtitle {
  font-size: 14px;
  color: #64748b;
  margin-top: 4px;
}

/* 卡片内容 */
.card-body {
  padding: 24px;
}

/* 卡片底部 */
.card-footer {
  padding: 16px 24px;
  border-top: 1px solid rgba(226, 232, 240, 0.5);
  background: rgba(248, 250, 252, 0.5);
}
```

### 5.2 交互式卡片
```css
/* 可点击卡片 */
.card-clickable {
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.card-clickable:hover {
  background: rgba(255, 255, 255, 0.9);
  backdrop-filter: blur(30px);
  -webkit-backdrop-filter: blur(30px);
  transform: translateY(-4px) scale(1.02);
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.2);
}

.card-clickable:active {
  transform: translateY(-2px) scale(1.01);
}

/* 选中卡片 */
.card-selected {
  background: rgba(14, 165, 233, 0.1);
  border-color: rgba(14, 165, 233, 0.3);
  box-shadow: 0 8px 32px rgba(14, 165, 233, 0.2);
}
```

## 6. 模态框组件 (Modal)

### 6.1 模态框遮罩
```css
/* 模态框遮罩 */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  animation: fadeIn 0.3s ease;
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}
```

### 6.2 模态框内容
```css
/* 模态框内容 */
.modal-content {
  width: 90%;
  max-width: 500px;
  max-height: 90vh;
  background: rgba(255, 255, 255, 0.9);
  backdrop-filter: blur(40px);
  -webkit-backdrop-filter: blur(40px);
  border: 1px solid rgba(255, 255, 255, 0.3);
  border-radius: 24px;
  box-shadow: 0 32px 64px rgba(0, 0, 0, 0.2);
  overflow: hidden;
  animation: modalSlideIn 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

@keyframes modalSlideIn {
  from {
    opacity: 0;
    transform: translateY(20px) scale(0.95);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

/* 模态框头部 */
.modal-header {
  padding: 24px;
  border-bottom: 1px solid rgba(226, 232, 240, 0.5);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.modal-title {
  font-size: 20px;
  font-weight: 700;
  color: #0f172a;
  margin: 0;
}

.modal-close {
  width: 36px;
  height: 36px;
  background: rgba(241, 245, 249, 0.8);
  border: none;
  border-radius: 50%;
  color: #64748b;
  cursor: pointer;
  transition: all 0.2s ease;
}

.modal-close:hover {
  background: rgba(239, 68, 68, 0.1);
  color: #ef4444;
}

/* 模态框内容区域 */
.modal-body {
  padding: 24px;
  overflow-y: auto;
}

/* 模态框底部 */
.modal-footer {
  padding: 24px;
  border-top: 1px solid rgba(226, 232, 240, 0.5);
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  background: rgba(248, 250, 252, 0.5);
}
```

## 7. 标签页组件 (Tabs)

### 7.1 标签页容器
```css
/* 标签页容器 */
.tabs-container {
  background: rgba(255, 255, 255, 0.7);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 16px;
  overflow: hidden;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.1);
}

/* 标签页导航 */
.tabs-nav {
  display: flex;
  background: rgba(248, 250, 252, 0.8);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  border-bottom: 1px solid rgba(226, 232, 240, 0.5);
}

/* 标签页按钮 */
.tab-button {
  flex: 1;
  padding: 16px 24px;
  background: none;
  border: none;
  font-size: 14px;
  font-weight: 600;
  color: #64748b;
  cursor: pointer;
  transition: all 0.2s ease;
  position: relative;
}

.tab-button:hover {
  color: #334155;
  background: rgba(255, 255, 255, 0.5);
}

/* 激活标签页 */
.tab-button.active {
  color: #0ea5e9;
}

.tab-button.active::after {
  content: '';
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 3px;
  background: linear-gradient(90deg, #0ea5e9 0%, #38bdf8 100%);
  border-radius: 3px 3px 0 0;
}

/* 标签页内容 */
.tab-content {
  padding: 24px;
  animation: fadeIn 0.3s ease;
}
```

## 8. 通知组件 (Notification)

### 8.1 基础通知
```css
/* 通知容器 */
.notification {
  position: fixed;
  top: 24px;
  right: 24px;
  min-width: 300px;
  max-width: 400px;
  background: rgba(255, 255, 255, 0.95);
  backdrop-filter: blur(40px);
  -webkit-backdrop-filter: blur(40px);
  border: 1px solid rgba(255, 255, 255, 0.3);
  border-radius: 16px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
  padding: 16px;
  z-index: 1001;
  animation: slideInRight 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

@keyframes slideInRight {
  from {
    opacity: 0;
    transform: translateX(100%);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

/* 通知内容 */
.notification-content {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

/* 通知图标 */
.notification-icon {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

/* 通知类型 */
.notification-success .notification-icon {
  background: rgba(34, 197, 94, 0.1);
  color: #22c55e;
}

.notification-error .notification-icon {
  background: rgba(239, 68, 68, 0.1);
  color: #ef4444;
}

.notification-warning .notification-icon {
  background: rgba(249, 115, 22, 0.1);
  color: #f97316;
}

.notification-info .notification-icon {
  background: rgba(14, 165, 233, 0.1);
  color: #0ea5e9;
}

/* 通知文本 */
.notification-text {
  flex: 1;
}

.notification-title {
  font-size: 14px;
  font-weight: 600;
  color: #0f172a;
  margin: 0 0 4px 0;
}

.notification-message {
  font-size: 13px;
  color: #475569;
  margin: 0;
  line-height: 1.4;
}

/* 关闭按钮 */
.notification-close {
  background: none;
  border: none;
  color: #94a3b8;
  cursor: pointer;
  padding: 4px;
  margin-left: 8px;
  transition: color 0.2s ease;
}

.notification-close:hover {
  color: #64748b;
}
```

## 9. 加载指示器 (Loading Spinner)

### 9.1 旋转加载器
```css
/* 旋转加载器 */
.spinner {
  width: 40px;
  height: 40px;
  border: 3px solid rgba(226, 232, 240, 0.5);
  border-top-color: #0ea5e9;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* 尺寸变体 */
.spinner-sm {
  width: 20px;
  height: 20px;
  border-width: 2px;
}

.spinner-lg {
  width: 60px;
  height: 60px;
  border-width: 4px;
}

/* 颜色变体 */
.spinner-success {
  border-top-color: #22c55e;
}

.spinner-warning {
  border-top-color: #f97316;
}

.spinner-error {
  border-top-color: #ef4444;
}
```

### 9.2 进度加载器
```css
/* 进度加载器 */
.progress-spinner {
  width: 40px;
  height: 40px;
  position: relative;
}

.progress-spinner::before,
.progress-spinner::after {
  content: '';
  position: absolute;
  border-radius: 50%;
}

.progress-spinner::before {
  width: 100%;
  height: 100%;
  background: conic-gradient(
    #0ea5e9 0%,
    #38bdf8 25%,
    #7dd3fc 50%,
    #bae6fd 75%,
    #0ea5e9 100%
  );
  animation: spin 1s linear infinite;
}

.progress-spinner::after {
  width: 32px;
  height: 32px;
  background: white;
  top: 4px;
  left: 4px;
}
```

## 10. 工具提示 (Tooltip)

### 10.1 基础工具提示
```css
/* 工具提示容器 */
.tooltip-container {
  position: relative;
  display: inline-block;
}

/* 工具提示 */
.tooltip {
  position: absolute;
  bottom: 100%;
  left: 50%;
  transform: translateX(-50%);
  margin-bottom: 8px;
  padding: 8px 12px;
  background: rgba(15, 23, 42, 0.9);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  color: white;
  font-size: 12px;
  font-weight: 500;
  border-radius: 8px;
  white-space: nowrap;
  z-index: 100;
  opacity: 0;
  visibility: hidden;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
}

.tooltip::after {
  content: '';
  position: absolute;
  top: 100%;
  left: 50%;
  transform: translateX(-50%);
  border: 6px solid transparent;
  border-top-color: rgba(15, 23, 42, 0.9);
}

/* 显示工具提示 */
.tooltip-container:hover .tooltip {
  opacity: 1;
  visibility: visible;
  transform: translateX(-50%) translateY(0);
}

/* 位置变体 */
.tooltip-top {
  bottom: 100%;
  top: auto;
  margin-bottom: 8px;
}

.tooltip-bottom {
  top: 100%;
  bottom: auto;
  margin-top: 8px;
}

.tooltip-bottom::after {
  top: auto;
  bottom: 100%;
  border-top-color: transparent;
  border-bottom-color: rgba(15, 23, 42, 0.9);
}
```

## 11. 颜色状态指示器

### 11.1 状态徽章 (Badge)
```css
/* 状态徽章 */
.badge {
  display: inline-flex;
  align-items: center;
  padding: 4px 8px;
  font-size: 11px;
  font-weight: 600;
  border-radius: 6px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

/* 状态颜色 */
.badge-success {
  background: rgba(34, 197, 94, 0.1);
  color: #16a34a;
  border: 1px solid rgba(34, 197, 94, 0.2);
}

.badge-warning {
  background: rgba(249, 115, 22, 0.1);
  color: #ea580c;
  border: 1px solid rgba(249, 115, 22, 0.2);
}

.badge-error {
  background: rgba(239, 68, 68, 0.1);
  color: #dc2626;
  border: 1px solid rgba(239, 68, 68, 0.2);
}

.badge-info {
  background: rgba(14, 165, 233, 0.1);
  color: #0284c7;
  border: 1px solid rgba(14, 165, 233, 0.2);
}

.badge-neutral {
  background: rgba(100, 116, 139, 0.1);
  color: #475569;
  border: 1px solid rgba(100, 116, 139, 0.2);
}
```

## 12. 图标系统

### 12.1 图标尺寸
```css
/* 图标尺寸 */
.icon-xs {
  width: 12px;
  height: 12px;
}

.icon-sm {
  width: 16px;
  height: 16px;
}

.icon-md {
  width: 20px;
  height: 20px;
}

.icon-lg {
  width: 24px;
  height: 24px;
}

.icon-xl {
  width: 32px;
  height: 32px;
}

.icon-2xl {
  width: 48px;
  height: 48px;
}
```

### 12.2 图标颜色
```css
/* 图标颜色类 */
.icon-primary {
  color: #0ea5e9;
}

.icon-success {
  color: #22c55e;
}

.icon-warning {
  color: #f97316;
}

.icon-error {
  color: #ef4444;
}

.icon-muted {
  color: #94a3b8;
}

.icon-white {
  color: white;
}
```

## 13. 实现注意事项

### 13.1 毛玻璃效果兼容性
```css
/* 毛玻璃效果回退方案 */
.glass-effect {
  background: rgba(255, 255, 255, 0.95);
  /* 标准毛玻璃效果 */
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
}

/* 不支持backdrop-filter的浏览器 */
@supports not (backdrop-filter: blur(20px)) {
  .glass-effect {
    background: rgba(255, 255, 255, 0.98);
  }
}
```

### 13.2 性能优化
1. **减少重绘**：使用transform和opacity进行动画
2. **硬件加速**：对动画元素使用transform: translateZ(0)
3. **will-change**：对需要频繁变化的属性使用will-change
4. **图片优化**：使用SVG图标，避免PNG雪碧图
5. **字体优化**：使用系统字体，避免自定义字体加载延迟

### 13.3 可访问性
1. **颜色对比度**：确保文本与背景对比度至少4.5:1
2. **焦点样式**：为所有可交互元素提供清晰的焦点指示
3. **键盘导航**：确保所有功能可通过键盘访问
4. **屏幕阅读器**：使用适当的ARIA标签和属性
5. **字体大小**：支持浏览器字体大小调整

---

**组件样式版本**: v1.0
**创建时间**: 2026-03-08
**更新记录**: 初始版本
**适用框架**: Vue3 + CSS Modules / Tailwind CSS
**设计状态**: ✅ 完成