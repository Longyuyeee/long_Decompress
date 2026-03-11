# big-data-expert 数字大屏专家 Skill

这是一个专门为 Vue 3 + TS 环境定制的数字大屏自动化开发工具。

## 快速开始

### 1. 启动建模
告知 Gemini 参考图片的位置：
`gemini --skill big-data-expert "分析这个图片并建模：E:\Project\WebProject\ui\home.png"`

### 2. 确认描述
Skill 会生成 `页面描述文档.md`。请仔细阅读并确认其中的指标和布局是否准确。

### 3. 生成代码
一旦描述文档确认无误，运行：
`/generate`

## 包含的模板
- **responsive.scss**: vw/vh 响应式适配引擎。
- **BlockItem.vue**: 带有扫描光效的科技感标题容器。
- **EChart.vue**: 封装了自适应和生命周期销毁的 ECharts 组件。
- **Layout.vue**: 标准 1:2:1 三栏大屏布局。

## 依赖要求
生成的代码依赖以下库，请确保项目已安装：
- `echarts`
- `sass` (或 `sass-embedded`)