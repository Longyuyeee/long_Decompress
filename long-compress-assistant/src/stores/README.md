# 状态管理架构

本文档描述了胧压缩·方便助手项目的状态管理架构。

## 概述

项目使用 [Pinia](https://pinia.vuejs.org/) 作为状态管理库，它是一个轻量级、类型安全且易于使用的 Vue 状态管理解决方案。

## Store 结构

项目包含三个主要的 store：

### 1. `app` Store (`app.ts`)
**职责**：应用核心状态和设置管理

**状态**：
- `theme` - 应用主题（light/dark/auto）
- `language` - 当前语言
- `error` - 全局错误信息
- `decompressTasks` - 解压任务列表
- `settings` - 应用设置

**计算属性**：
- `currentTheme` - 计算后的当前主题（考虑auto模式）
- `activeTasks` - 进行中的任务
- `completedTasks` - 已完成的任务
- `totalProgress` - 总进度

**主要方法**：
- `createDecompressTask()` - 创建解压任务
- `updateTaskProgress()` - 更新任务进度
- `markTaskAsError()` - 标记任务为错误状态
- `clearCompletedTasks()` - 清理已完成任务
- `updateSettings()` - 更新应用设置
- `resetSettings()` - 重置设置为默认值

### 2. `file` Store (`file.ts`)
**职责**：文件管理相关状态

**状态**：
- `files` - 文件列表
- `selectedFiles` - 选中的文件ID列表
- `fileHistory` - 文件操作历史
- `favoriteFiles` - 收藏的文件
- `currentDirectory` - 当前目录
- `isLoading` - 加载状态
- `error` - 错误信息

**计算属性**：
- `selectedFileItems` - 选中的文件对象列表
- `totalSelectedSize` - 选中文件的总大小
- `recentHistory` - 最近的操作历史
- `favoritesByTag` - 按标签分类的收藏文件

**主要方法**：
- `addFile()` / `removeFile()` - 文件增删
- `selectFile()` / `deselectFile()` - 文件选择
- `addToHistory()` - 添加历史记录
- `addToFavorites()` - 添加到收藏夹
- `formatFileSize()` - 格式化文件大小
- `getFileExtension()` - 获取文件扩展名

### 3. `ui` Store (`ui.ts`)
**职责**：用户界面状态管理

**状态**：
- `sidebarOpen` - 侧边栏开关状态
- `notifications` - 通知列表
- `modals` - 模态框状态
- `toasts` - Toast提示列表
- `loading` - 加载状态
- `loadingText` - 加载文本
- `darkMode` - 暗色模式状态
- `currentView` - 当前视图

**计算属性**：
- `unreadNotifications` - 未读通知数量
- `activeModals` - 活动的模态框
- `hasActiveModals` - 是否有活动模态框
- `isLoading` - 是否正在加载

**主要方法**：
- `toggleSidebar()` - 切换侧边栏
- `addNotification()` - 添加通知
- `openModal()` / `closeModal()` - 模态框控制
- `showToast()` - 显示Toast提示
- `startLoading()` / `stopLoading()` - 加载状态控制
- `toggleDarkMode()` - 切换暗色模式
- `showSuccess()` / `showError()` - 快捷提示方法

## 使用示例

### 基本使用
```typescript
// 导入store
import { useAppStore, useFileStore, useUIStore } from '@/stores'

// 在setup中使用
const appStore = useAppStore()
const fileStore = useFileStore()
const uiStore = useUIStore()

// 访问状态
const theme = computed(() => appStore.currentTheme)
const selectedFiles = computed(() => fileStore.selectedFileItems)
const sidebarOpen = computed(() => uiStore.sidebarOpen)

// 调用方法
const handleFileSelect = (file: File) => {
  const fileId = fileStore.addFile({
    name: file.name,
    path: file.path,
    size: file.size,
    type: file.type
  })

  uiStore.showSuccess(`文件 ${file.name} 已添加`)
}

// 监听状态变化
watch(() => appStore.currentTheme, (newTheme) => {
  console.log('主题已切换:', newTheme)
})
```

### 在组件中使用
```vue
<template>
  <div :class="{ 'dark': uiStore.darkMode }">
    <button @click="uiStore.toggleSidebar">
      切换侧边栏
    </button>

    <div v-if="fileStore.isLoading">
      加载中...
    </div>

    <div v-for="file in fileStore.files" :key="file.id">
      {{ file.name }} - {{ fileStore.formatFileSize(file.size) }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { useFileStore, useUIStore } from '@/stores'

const fileStore = useFileStore()
const uiStore = useUIStore()

// 初始化加载文件
onMounted(async () => {
  uiStore.startLoading('加载文件中...')
  try {
    // 加载文件逻辑
    await loadFiles()
  } catch (error) {
    uiStore.showError('加载文件失败')
  } finally {
    uiStore.stopLoading()
  }
})
</script>
```

## 最佳实践

### 1. 状态分离
- 将相关状态放在同一个store中
- 避免在不同store中重复状态
- 使用计算属性派生状态

### 2. 类型安全
- 为所有状态定义TypeScript接口
- 使用严格的类型检查
- 导出类型供其他模块使用

### 3. 本地存储
- 重要状态自动保存到localStorage
- 提供加载和保存方法
- 处理存储失败的情况

### 4. 错误处理
- 统一错误状态管理
- 提供用户友好的错误提示
- 自动清理错误信息

### 5. 性能优化
- 使用计算属性缓存派生状态
- 避免在store中存储大量数据
- 及时清理不再需要的数据

## 扩展指南

### 添加新的Store
1. 在 `src/stores/` 目录下创建新的store文件
2. 使用 `defineStore` 定义store
3. 在 `index.ts` 中导出
4. 更新本文档

### 添加新的状态
1. 在相应的store中添加状态
2. 添加对应的类型定义
3. 添加必要的计算属性
4. 添加操作方法
5. 考虑是否需要持久化存储

### 添加新的操作方法
1. 在store中添加方法
2. 确保方法职责单一
3. 提供适当的错误处理
4. 更新类型定义

## 注意事项

1. **避免循环依赖**：store之间不要相互引用
2. **保持纯净**：store中不要包含UI逻辑
3. **适度使用**：不是所有状态都需要放在store中
4. **测试覆盖**：为store添加单元测试
5. **文档更新**：修改store时更新本文档