# 胧压缩·方便助手 - Vue3前端

现代化的文件解压工具前端应用，采用Vue3 + TypeScript + Tailwind CSS + PrimeVue构建，具有苹果毛玻璃设计风格。

## 技术栈

- **Vue 3** - 渐进式JavaScript框架
- **TypeScript** - 类型安全的JavaScript超集
- **Tailwind CSS** - 实用优先的CSS框架
- **PrimeVue** - Vue UI组件库
- **Pinia** - Vue状态管理
- **Vite** - 下一代前端构建工具
- **Tauri** - 构建小型、快速的桌面应用程序

## 项目结构

```
long-compress-assistant/
├── src/
│   ├── assets/           # 静态资源
│   │   └── css/         # 样式文件
│   ├── components/       # Vue组件
│   │   ├── layouts/     # 布局组件
│   │   ├── ui/          # UI基础组件
│   │   └── views/       # 页面组件
│   ├── composables/     # 组合式函数
│   ├── router/          # 路由配置
│   ├── stores/          # Pinia状态存储
│   ├── types/           # TypeScript类型定义
│   ├── utils/           # 工具函数
│   ├── App.vue          # 根组件
│   └── main.ts          # 应用入口
├── public/              # 公共资源
├── index.html           # HTML模板
└── package.json         # 项目依赖
```

## 设计系统

### 颜色主题

- **主色 (Primary)**: `#0ea5e9` - 用于主要操作和重要元素
- **次要色 (Secondary)**: `#64748b` - 用于次要操作和文本
- **强调色 (Accent)**: `#d946ef` - 用于强调和特殊状态
- **成功色 (Success)**: `#10b981` - 用于成功状态
- **警告色 (Warning)**: `#f59e0b` - 用于警告状态
- **危险色 (Danger)**: `#ef4444` - 用于错误和危险状态

### 字体

- **主要字体**: Inter - 用于界面文本
- **等宽字体**: JetBrains Mono - 用于代码和文件路径

### 玻璃效果

项目采用苹果风格的毛玻璃效果，通过以下CSS类实现：

- `.glass-effect` - 基础玻璃效果
- `.glass-card` - 玻璃卡片
- `.glass-button` - 玻璃按钮
- `.glass-input` - 玻璃输入框

## 核心组件

### 1. GlassCard
可复用的玻璃效果卡片组件，支持悬停效果和自定义样式。

### 2. GlassButton
玻璃效果按钮组件，支持多种变体、尺寸和加载状态。

### 3. FileDropzone
文件拖放区域组件，支持多文件选择、格式验证和大小限制。

### 4. ProgressBar
进度条组件，支持多种变体、条纹效果和不确定状态。

## 页面路由

- `/` - 首页：应用概览和快速操作
- `/decompress` - 解压页面：文件选择和配置
- `/settings` - 设置页面：应用配置选项
- `/about` - 关于页面：应用信息和许可

## 开发指南

### 安装依赖

```bash
npm install
```

### 开发模式

```bash
npm run dev
```

### 构建生产版本

```bash
npm run build
```

### 预览生产版本

```bash
npm run preview
```

### Tauri开发

```bash
npm run tauri dev
```

## 代码规范

### Vue组件
- 使用组合式API (`<script setup>`)
- 使用TypeScript进行类型检查
- 组件名使用PascalCase
- 单文件组件结构：template → script → style

### 样式
- 优先使用Tailwind CSS工具类
- 自定义样式放在`@layer components`中
- 响应式设计使用Tailwind断点

### 状态管理
- 全局状态使用Pinia
- 组件状态使用ref/reactive
- 复杂逻辑使用组合式函数

## 浏览器支持

- Chrome 90+
- Firefox 88+
- Safari 14+
- Edge 90+

## 许可证

MIT License