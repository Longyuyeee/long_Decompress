# Vue3前端架构文档

## 概述

本文档描述了"胧压缩·方便助手"的Vue3前端架构设计。前端采用现代化的技术栈，包括Vue3、TypeScript、Tailwind CSS、PrimeVue和Pinia，实现了苹果毛玻璃设计风格。

## 技术栈

### 核心框架
- **Vue 3.4+**: 渐进式JavaScript框架，使用组合式API
- **TypeScript 5.3+**: 类型安全的JavaScript超集
- **Vite 5.1+**: 下一代前端构建工具

### UI框架
- **Tailwind CSS 3.4+**: 实用优先的CSS框架
- **PrimeVue 3.47+**: Vue UI组件库
- **PrimeIcons**: 图标库

### 状态管理
- **Pinia 2.1+**: Vue状态管理库

### 桌面集成
- **Tauri 1.5+**: 构建小型、快速的桌面应用程序
- **@tauri-apps/api**: Tauri API客户端

## 项目结构

```
long-compress-assistant/
├── src/
│   ├── assets/                 # 静态资源
│   │   └── css/
│   │       └── main.css       # 主样式文件
│   ├── components/            # Vue组件
│   │   ├── layouts/          # 布局组件
│   │   │   └── MainLayout.vue # 主布局
│   │   └── ui/               # UI基础组件
│   │       ├── GlassCard.vue  # 玻璃卡片
│   │       ├── GlassButton.vue # 玻璃按钮
│   │       ├── FileDropzone.vue # 文件拖放
│   │       ├── ProgressBar.vue # 进度条
│   │       ├── ExampleComponents.vue # 示例
│   │       └── index.ts       # 组件导出
│   ├── composables/          # 组合式函数
│   │   └── useTauriCommands.ts # Tauri命令
│   ├── router/               # 路由配置
│   │   └── index.ts          # 路由定义
│   ├── stores/               # Pinia状态存储
│   │   └── app.ts            # 应用状态
│   ├── types/                # TypeScript类型定义
│   │   └── index.ts          # 类型导出
│   ├── utils/                # 工具函数
│   │   └── index.ts          # 工具函数导出
│   ├── views/                # 页面视图
│   │   ├── HomeView.vue      # 首页
│   │   ├── DecompressView.vue # 解压页
│   │   ├── SettingsView.vue  # 设置页
│   │   └── AboutView.vue     # 关于页
│   ├── App.vue               # 根组件
│   └── main.ts               # 应用入口
├── public/                   # 公共资源
│   └── vite.svg             # 图标
├── DESIGN_SYSTEM.md          # 设计系统文档
├── ARCHITECTURE.md           # 架构文档（本文档）
├── README.md                 # 项目说明
├── package.json              # 项目依赖
├── vite.config.ts            # Vite配置
├── tsconfig.json             # TypeScript配置
├── tailwind.config.js        # Tailwind配置
└── postcss.config.js         # PostCSS配置
```

## 架构设计

### 1. 组件架构

#### 1.1 原子设计模式
- **原子组件**: `GlassButton`, `GlassCard`等基础UI组件
- **分子组件**: `FileDropzone`, `ProgressBar`等复合组件
- **组织组件**: `MainLayout`等布局组件
- **模板**: 页面视图组件
- **页面**: 完整的路由页面

#### 1.2 组件通信
- **Props向下传递**: 父组件向子组件传递数据
- **Events向上传递**: 子组件向父组件发送事件
- **Provide/Inject**: 跨层级组件通信
- **Pinia Store**: 全局状态管理

### 2. 状态管理

#### 2.1 Pinia Store设计
```typescript
// 应用状态存储
const useAppStore = defineStore('app', () => {
  // 响应式状态
  const theme = ref<'light' | 'dark' | 'auto'>('auto')
  const selectedFiles = ref<FileItem[]>([])
  const decompressTasks = ref<DecompressTask[]>([])

  // 计算属性
  const currentTheme = computed(() => { /* ... */ })
  const activeTasks = computed(() => { /* ... */ })

  // 方法
  const addFile = (file: FileItem) => { /* ... */ }
  const createDecompressTask = (options: DecompressOptions) => { /* ... */ }

  return { theme, selectedFiles, decompressTasks, currentTheme, activeTasks, addFile, createDecompressTask }
})
```

#### 2.2 状态持久化
- 使用localStorage持久化用户设置
- 自动保存和加载应用状态
- 支持状态恢复

### 3. 路由设计

#### 3.1 路由配置
```typescript
const routes: Array<RouteRecordRaw> = [
  {
    path: '/',
    name: 'Home',
    component: () => import('@/views/HomeView.vue'),
    meta: { title: '胧压缩·方便助手' }
  },
  // 其他路由...
]
```

#### 3.2 路由守卫
- 页面标题自动更新
- 路由权限控制（预留）
- 导航进度指示

### 4. 样式系统

#### 4.1 Tailwind配置
```javascript
// tailwind.config.js
module.exports = {
  content: ['./index.html', './src/**/*.{vue,js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        primary: { /* 主色板 */ },
        secondary: { /* 次要色板 */ },
        // ...
      },
      fontFamily: {
        sans: ['Inter', 'system-ui'],
        mono: ['JetBrains Mono', 'monospace']
      }
    }
  }
}
```

#### 4.2 玻璃效果实现
```css
.glass-effect {
  backdrop-filter: blur(12px);
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.2);
  box-shadow: 0 8px 32px 0 rgba(31, 38, 135, 0.37);
}
```

### 5. Tauri集成

#### 5.1 命令调用
```typescript
// 使用Tauri命令
const decompressFile = async (filePath: string, options: DecompressOptions) => {
  try {
    const result = await invoke('decompress_file', {
      filePath,
      outputPath: options.outputPath,
      password: options.password,
      // ...
    })
    return result
  } catch (error) {
    console.error('解压失败:', error)
    throw error
  }
}
```

#### 5.2 文件系统操作
- 文件选择对话框
- 目录浏览
- 文件信息获取
- 文件操作（复制、移动、删除）

### 6. 性能优化

#### 6.1 代码分割
- 路由懒加载
- 组件异步加载
- 第三方库按需引入

#### 6.2 资源优化
- 图片懒加载
- 字体子集化
- CSS压缩和Purge

#### 6.3 渲染优化
- 虚拟滚动（预留）
- 列表项key优化
- 计算属性缓存

### 7. 开发规范

#### 7.1 代码规范
- 使用ESLint + Prettier
- TypeScript严格模式
- 组件命名规范（PascalCase）
- 文件命名规范（kebab-case）

#### 7.2 提交规范
- Conventional Commits
- 提交前代码检查
- 自动版本管理

#### 7.3 测试规范
- 单元测试（Vitest）
- 组件测试（Vue Test Utils）
- E2E测试（Playwright）

## 设计模式

### 1. 组合式函数模式
```typescript
// 使用组合式函数封装逻辑
export const useTauriCommands = () => {
  const selectFiles = async () => { /* ... */ }
  const decompressFile = async () => { /* ... */ }

  return { selectFiles, decompressFile }
}
```

### 2. 依赖注入模式
```typescript
// 使用Provide/Inject传递服务
const provideServices = () => {
  const fileService = useFileService()
  const decompressService = useDecompressService()

  provide('fileService', fileService)
  provide('decompressService', decompressService)
}
```

### 3. 观察者模式
```typescript
// 使用事件总线或Pinia插件
const useEventBus = () => {
  const events = new Map()

  const on = (event: string, callback: Function) => { /* ... */ }
  const emit = (event: string, data: any) => { /* ... */ }

  return { on, emit }
}
```

## 扩展性设计

### 1. 插件系统（预留）
- 支持功能插件扩展
- 插件生命周期管理
- 插件配置界面

### 2. 主题系统
- 多主题支持
- 主题切换动画
- 自定义主题配置

### 3. 国际化
- 多语言支持
- 动态语言切换
- 本地化资源管理

### 4. 无障碍支持
- 键盘导航
- 屏幕阅读器支持
- 高对比度模式

## 部署架构

### 1. 开发环境
- 热重载开发服务器
- 源代码映射
- 开发工具集成

### 2. 生产环境
- 代码压缩和优化
- 资源哈希缓存
- 错误监控和报告

### 3. 桌面打包
- Tauri应用打包
- 代码签名
- 自动更新

## 监控和调试

### 1. 错误监控
- 全局错误捕获
- 错误上报
- 性能监控

### 2. 开发工具
- Vue DevTools
- 浏览器开发者工具
- 性能分析工具

### 3. 日志系统
- 分级日志
- 日志持久化
- 日志分析

## 安全考虑

### 1. 输入验证
- 文件路径验证
- 用户输入清理
- 跨站脚本防护

### 2. 文件安全
- 文件类型验证
- 文件大小限制
- 病毒扫描集成

### 3. 数据保护
- 敏感数据加密
- 本地存储安全
- 通信加密

## 未来扩展

### 1. 云同步
- 用户配置云同步
- 文件历史记录
- 跨设备同步

### 2. 高级功能
- 批量处理
- 脚本支持
- 自动化工作流

### 3. 生态系统
- 插件市场
- 主题市场
- 社区贡献

---

*最后更新: 2024-03-08*
*版本: 1.0.0*