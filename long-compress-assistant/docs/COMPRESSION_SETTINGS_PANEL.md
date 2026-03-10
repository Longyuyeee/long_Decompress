# 压缩设置面板 (FE-007) 实现文档

## 概述
压缩设置面板是胧解压项目的前端核心组件之一，负责提供用户友好的压缩参数配置界面。该组件基于Vue 3.4 + TypeScript 5.3 + PrimeVue 3.47 + Tailwind CSS 3.4技术栈实现。

## 功能特性

### 1. 压缩格式选择
- 支持多种压缩格式：ZIP、7z、TAR、GZIP、BZIP2、XZ、RAR等
- 格式组合支持：TAR.GZ、TAR.BZ2、TAR.XZ
- 可视化格式图标和描述
- 格式特性提示（密码保护、分卷压缩等）

### 2. 压缩级别配置
- 1-9级压缩级别滑块控制
- 级别标签显示（最快、平衡、最小等）
- 实时压缩率估算

### 3. 输出设置
- 输出路径选择（集成Tauri文件对话框）
- 自定义文件名设置
- 自动文件扩展名匹配

### 4. 密码保护
- 密码输入和确认
- 密码可见性切换
- 密码一致性验证
- AES-256加密支持提示

### 5. 高级选项
- 分卷压缩（支持ZIP、7z格式）
- 分卷大小选择（100MB-4GB）
- 保持目录结构
- 压缩后删除原文件
- 固实压缩（7z格式专用）

## 组件架构

### 主要组件
1. **CompressionSettingsPanel.vue** - 压缩设置面板主组件
2. **compression.ts** - Pinia压缩状态管理
3. **CompressView.vue** - 压缩页面（集成面板）

### 状态管理
使用Pinia进行状态管理，包含以下核心状态：

```typescript
interface CompressionOptions {
  format: 'zip' | '7z' | 'tar' | 'gz' | 'bz2' | 'tar.gz' | 'tar.bz2' | 'xz' | 'tar.xz' | 'rar'
  level: number
  password: string
  filename: string
  splitArchive: boolean
  splitSize: string
  keepStructure: boolean
  deleteAfter: boolean
  createSolidArchive: boolean
}

interface CompressionTask {
  id: string
  files: FileItem[]
  options: CompressionOptions
  outputPath: string
  status: 'pending' | 'processing' | 'completed' | 'failed'
  progress: number
  // ...其他字段
}
```

### 组件接口

#### Props
```typescript
interface Props {
  modelValue?: CompressionOptions  // 压缩选项
  outputPath?: string             // 输出路径
}
```

#### Emits
```typescript
interface Emits {
  (e: 'update:modelValue', value: CompressionOptions): void
  (e: 'update:outputPath', value: string): void
  (e: 'format-changed', format: string): void
  (e: 'options-changed', options: CompressionOptions): void
}
```

#### 暴露的方法
```typescript
defineExpose({
  getOptions: () => CompressionOptions,      // 获取当前选项
  getOutputPath: () => string,              // 获取输出路径
  validate: () => { valid: boolean, error?: string }  // 验证配置
})
```

## 技术实现

### 1. 响应式设计
- 使用Tailwind CSS实现完全响应式布局
- 移动端适配：网格布局自动调整
- 暗色/亮色主题支持

### 2. 无障碍访问
- 所有交互元素都有适当的ARIA标签
- 键盘导航支持
- 屏幕阅读器友好

### 3. 类型安全
- 完整的TypeScript类型定义
- 运行时类型验证
- 编译时类型检查

### 4. 测试覆盖
- 单元测试：组件功能测试
- 集成测试：与存储和API的集成
- 测试用例包括：
  - 组件渲染测试
  - 用户交互测试
  - 格式验证测试
  - 错误处理测试

## 使用示例

### 基本使用
```vue
<template>
  <CompressionSettingsPanel
    v-model="compressionOptions"
    v-model:outputPath="outputPath"
    @format-changed="handleFormatChanged"
    @options-changed="handleOptionsChanged"
  />
</template>

<script setup lang="ts">
import { ref } from 'vue'
import CompressionSettingsPanel from '@/components/compression/CompressionSettingsPanel.vue'
import type { CompressionOptions } from '@/stores'

const compressionOptions = ref<CompressionOptions>({
  format: 'zip',
  level: 6,
  password: '',
  filename: '',
  splitArchive: false,
  splitSize: '1024',
  keepStructure: true,
  deleteAfter: false,
  createSolidArchive: false
})

const outputPath = ref('')

const handleFormatChanged = (format: string) => {
  console.log('格式已更改:', format)
}

const handleOptionsChanged = (options: CompressionOptions) => {
  console.log('选项已更改:', options)
}
</script>
```

### 使用存储
```vue
<script setup lang="ts">
import { useCompressionStore } from '@/stores'

const compressionStore = useCompressionStore()

// 计算属性绑定
const compressionOptions = computed({
  get: () => compressionStore.compressionOptions,
  set: (value) => compressionStore.updateCompressionOptions(value)
})

const outputPath = computed({
  get: () => compressionStore.outputPath,
  set: (value) => compressionStore.setOutputPath(value)
})
</script>
```

## 与后端集成

### API调用
```typescript
// 调用Tauri压缩API
const result = await invoke('compress_file', {
  files: filePaths,
  outputPath: fullOutputPath,
  options: {
    password: compressionOptions.value.password || null,
    compression_level: compressionOptions.value.level,
    split_size: compressionOptions.value.splitArchive && compressionOptions.value.splitSize
      ? parseInt(compressionOptions.value.splitSize) * 1024 * 1024
      : null,
    preserve_paths: compressionOptions.value.keepStructure,
    exclude_patterns: [],
    include_patterns: [],
    create_subdirectories: true,
    overwrite_existing: true
  }
})
```

### 错误处理
- 网络错误处理
- API错误处理
- 用户输入验证
- 进度跟踪和取消

## 性能优化

### 1. 组件优化
- 使用Vue 3的Composition API
- 响应式数据懒计算
- 事件防抖处理

### 2. 渲染优化
- 虚拟滚动（大量文件时）
- 组件懒加载
- 记忆化计算属性

### 3. 状态管理优化
- 状态持久化
- 历史记录管理
- 任务队列管理

## 扩展性

### 1. 格式扩展
添加新压缩格式只需：
1. 在`compressionFormats`数组中添加格式定义
2. 更新TypeScript类型定义
3. 添加格式特性支持

### 2. 选项扩展
添加新压缩选项只需：
1. 扩展`CompressionOptions`接口
2. 在组件中添加对应的UI控件
3. 更新存储逻辑

### 3. 主题扩展
- 支持自定义颜色主题
- 支持图标替换
- 支持布局调整

## 测试策略

### 单元测试
```typescript
// 测试组件渲染
it('renders correctly with default props', () => {
  const wrapper = createWrapper()
  expect(wrapper.find('h2').text()).toBe('压缩配置')
})

// 测试用户交互
it('emits format-changed event when format is selected', async () => {
  const wrapper = createWrapper()
  await wrapper.find('button[aria-label^="选择"]').trigger('click')
  expect(wrapper.emitted('format-changed')).toBeTruthy()
})

// 测试验证逻辑
it('validates password confirmation correctly', () => {
  const component = wrapper.vm as any
  const validation = component.validate()
  expect(validation.valid).toBe(true)
})
```

### 集成测试
- 与Pinia存储的集成测试
- 与Tauri API的集成测试
- 端到端工作流测试

## 部署和维护

### 构建配置
```javascript
// vite.config.ts
export default defineConfig({
  plugins: [vue()],
  build: {
    target: 'esnext',
    minify: 'terser'
  }
})
```

### 版本控制
- 遵循语义化版本控制
- 变更日志记录
- 向后兼容性保证

### 监控和日志
- 错误监控
- 性能监控
- 用户行为分析

## 总结

压缩设置面板实现了完整、用户友好的压缩参数配置功能，具有以下特点：

1. **功能完整**：支持所有主流压缩格式和选项
2. **用户体验优秀**：直观的界面设计和交互反馈
3. **技术先进**：使用现代前端技术栈
4. **可维护性强**：清晰的架构和完整的测试覆盖
5. **扩展性好**：易于添加新功能和格式支持

该组件为胧解压项目提供了核心的压缩配置能力，是项目前端架构的重要组成部分。