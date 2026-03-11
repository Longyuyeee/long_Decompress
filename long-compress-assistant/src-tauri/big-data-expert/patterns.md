# 数字大屏开发模式库

## 1. 响应式布局模式 (Layout Patterns)
- **三栏布局 (1:2:1)**: 侧边栏固定宽 (如 460px), 中间自适应地图。
- **Header 设计**: 高度固定 (如 80px), 采用背景图 + 倾斜标题 + 实时时钟。

## 2. ECharts 高级样式模式 (Visual Patterns)
### 暗黑科技风配色
- **主色**: #00ccff (青蓝色)
- **次色**: #409eff (经典蓝)
- **警告**: #ff3300 (红色)
- **渐变**: \linear-gradient(to bottom, #00ccff, #0066ff)\

### 3D 地图模拟
- 使用 3 层 \geo\ 叠加，底层设置较大的 \shadowBlur\，中层位移 1%，顶层作为交互层。

## 3. 性能优化模式 (Performance Patterns)
- **实例销毁**: 必须在 \onUnmounted\ 调用 \chartInstance.dispose()\。
- **自适应**: 必须监听 \window.onresize\ 并调用 \chartInstance.resize()\。
- **定时器清理**: 所有数据轮播定时器必须在组件卸载时 \clearInterval\。

## 4. 数据架构模式 (Data Architectures)
- **Props 透传**: \EChart.vue\ 仅接收 \option\, 不持有业务数据。
- **Mock 驱动**: 组件内定义 \static\ 数据数组，通过 \-for\ 渲染，保持模板整洁。