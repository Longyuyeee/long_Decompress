# 🔧 Bug 修复详细方案

## Bug #1: 无密码文件错误提示密码错误

### 当前问题代码

**位置**: `src/views/DecompressView.vue:258-270`

```typescript
// 问题代码：无条件给所有任务添加密码
if (!task.password) {
  const candidates = passwordStore.findCandidatePasswords(fileName)
  if (candidates.length > 0) {
    task.password = candidates[0] // ❌ 错误：给无密码文件也加密码
    task.logs.push({
      task_id: task.id,
      message: appStore.t('decompress.auto_trying').replace('{0}', String(candidates.length)),
      severity: 'info',
      timestamp: new Date().toISOString()
    })
  }
}
```

### 修复方案

```typescript
// ✅ 修复：只在后端明确要求密码时才尝试
// 先不添加密码，让后端检测
const options = {
  outputPath: task.outputPath,
  keepStructure: true,
  overwrite: false,
  deleteAfter: appStore.settings.autoDeleteSource,
  createSubdirectory: task.extractToSubfolder ?? false,
  password: task.password || undefined, // 只使用用户输入的密码
  fileFilter: task.fileFilter || null
}

try {
  taskStore.updateTaskStatus(task.id, 'preparing')
  task.passwordRequired = false
  await tauriCommands.decompressFile(task.sourceFiles[0], options, task.id)
} catch (error) {
  const errorMsg = extractErrorMessage(error) || String(error)
  
  // ✅ 只在明确的密码错误时才尝试保险箱密码
  const isPasswordError = errorMsg.includes('PasswordRequired') || 
                          errorMsg.includes('Wrong password') ||
                          errorMsg.includes('InvalidPassword')
  
  if (isPasswordError && !task.password) {
    // 现在才尝试保险箱密码
    const candidates = passwordStore.findCandidatePasswords(fileName)
    if (candidates.length > 0) {
      // ... 尝试候选密码
    }
  } else {
    // 非密码错误，直接失败
    taskStore.updateTaskStatus(task.id, 'failed')
    task.error = errorMsg
  }
}
```

---

## Bug #2: 完整性校验界面溢出

### 当前问题

**位置**: `src/views/FileIntegrityView.vue:272-313`

- 结果列表没有最大高度限制
- 长文件名和校验和没有换行

### 修复方案

```vue
<!-- 修改结果列表容器 -->
<section v-if="checksumResults.length > 0" class="aero-card p-10">
  <h2 class="text-sm font-black text-content uppercase tracking-[0.3em] mb-6">
    {{ appStore.t('integrity.results') }}
  </h2>
  
  <!-- ✅ 添加滚动容器，限制高度 -->
  <div class="max-h-[400px] overflow-y-auto custom-scrollbar space-y-3">
    <div
      v-for="result in checksumResults"
      :key="result.path"
      class="p-6 rounded-2xl bg-input/30 border border-subtle hover:border-primary/50 transition-all"
    >
      <div class="flex items-start justify-between gap-4">
        <!-- ✅ 添加 min-w-0 和文字换行 -->
        <div class="flex-1 min-w-0">
          <!-- ✅ 使用 break-all 允许换行 -->
          <div class="text-sm font-black text-content break-all uppercase tracking-widest">
            {{ result.fileName }}
          </div>
          <div class="mt-3 flex items-start gap-3">
            <span class="text-xs text-muted uppercase tracking-widest font-bold shrink-0">
              {{ result.algorithm }}:
            </span>
            <!-- ✅ 校验和允许换行 -->
            <code class="text-xs font-mono text-primary break-all">
              {{ result.checksum || '计算中...' }}
            </code>
          </div>
        </div>
        
        <!-- 保持按钮区域固定 -->
        <div class="flex items-center gap-3 shrink-0">
          <span v-if="result.status === 'success'" class="text-green-500 text-2xl">✓</span>
          <span v-else-if="result.status === 'error'" class="text-red-500 text-2xl">✗</span>
          <button
            v-if="result.status === 'success'"
            @click="copyChecksum(result.checksum)"
            class="px-4 py-2 rounded-xl bg-primary/10 hover:bg-primary/20 text-primary text-sm font-black uppercase tracking-widest transition-all"
          >
            {{ appStore.t('integrity.copy') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</section>
```

---

## Bug #3: 任务生命周期重构（大改）

### 当前问题

任务开始后立即消失，没有状态变化过程。

### 重构方案

#### 1. 修改任务显示逻辑

**位置**: `src/views/DecompressView.vue` 和 `CompressionView.vue`

```vue
<!-- 当前：只显示 pending 任务 -->
<div v-for="task in taskStore.tasks.filter(t => t.status === 'pending')" :key="task.id">

<!-- ✅ 修改为：显示所有任务 -->
<div v-for="task in visibleTasks" :key="task.id">
```

```typescript
// 新增计算属性
const visibleTasks = computed(() => {
  // 显示所有非已完成的任务，或最近5分钟内完成的任务
  const fiveMinutesAgo = Date.now() - 5 * 60 * 1000
  return taskStore.tasks.filter(t => {
    if (t.status === 'pending' || t.status === 'running' || t.status === 'extracting') {
      return true
    }
    if ((t.status === 'completed' || t.status === 'failed') && t.endTime) {
      return t.endTime.getTime() > fiveMinutesAgo
    }
    return false
  })
})
```

#### 2. 添加状态图标和颜色

```vue
<!-- 状态指示器 -->
<div class="flex items-center gap-2">
  <!-- 状态图标 -->
  <div 
    class="w-3 h-3 rounded-full shrink-0"
    :class="{
      'bg-gray-400': task.status === 'pending',
      'bg-blue-500 animate-pulse': task.status === 'running' || task.status === 'extracting',
      'bg-green-500': task.status === 'completed',
      'bg-red-500': task.status === 'failed'
    }"
  ></div>
  
  <!-- 状态文字 -->
  <span class="text-xs font-bold uppercase tracking-widest" :class="{
    'text-muted': task.status === 'pending',
    'text-blue-500': task.status === 'running' || task.status === 'extracting',
    'text-green-500': task.status === 'completed',
    'text-red-500': task.status === 'failed'
  }">
    {{ getStatusText(task.status) }}
  </span>
  
  <!-- 进度条（仅运行时显示） -->
  <div v-if="task.status === 'running' || task.status === 'extracting'" 
       class="flex-1 h-1 bg-input rounded-full overflow-hidden">
    <div class="h-full bg-primary transition-all duration-300" 
         :style="{ width: `${task.progress}%` }">
    </div>
  </div>
</div>
```

#### 3. 添加"清理已完成"按钮

**位置**: 界面右上角

```vue
<!-- 在"开始解压"按钮旁边 -->
<button
  v-if="hasCompletedTasks"
  @click="clearCompletedTasks"
  class="h-9 px-5 rounded-lg bg-input border border-subtle text-muted text-xs font-bold uppercase tracking-wider hover:text-primary hover:border-primary transition-all flex items-center gap-2"
>
  <i class="pi pi-trash text-xs"></i>
  <span>{{ appStore.t('decompress.clear_completed') }}</span>
</button>
```

```typescript
const hasCompletedTasks = computed(() => {
  return taskStore.tasks.some(t => t.status === 'completed' || t.status === 'failed')
})

const clearCompletedTasks = () => {
  taskStore.tasks = taskStore.tasks.filter(t => 
    t.status !== 'completed' && t.status !== 'failed'
  )
}
```

#### 4. i18n 翻译添加

```typescript
// src/i18n/index.ts
'decompress.clear_completed': '清理已完成',
'decompress.status.pending': '等待中',
'decompress.status.running': '解压中',
'decompress.status.extracting': '解压中',
'decompress.status.completed': '已完成',
'decompress.status.failed': '失败',
```

---

## Bug #4: 压缩中心布局重构（大改）

### 当前问题

- 全局设置摊开在页面上，占用大量空间
- 目标格式行过长导致溢出

### 重构方案

#### 1. 创建全局设置弹窗组件

**新文件**: `src/components/compression/GlobalSettingsModal.vue`

```vue
<template>
  <Modal 
    :visible="visible" 
    @close="$emit('close')"
    title="全局压缩设置"
    :width="600"
  >
    <div class="space-y-6 p-6">
      <!-- 复用 CompressionSettingsPanel 组件 -->
      <CompressionSettingsPanel
        v-model="localSettings"
        v-model:outputPath="localOutputPath"
        :allow-single-file-formats="allowSingleFileFormats"
      />
      
      <!-- 底部按钮 -->
      <div class="flex gap-3 justify-end pt-4 border-t border-subtle">
        <button
          @click="$emit('close')"
          class="px-6 py-2 rounded-xl bg-input border border-subtle text-content hover:border-primary transition-all"
        >
          取消
        </button>
        <button
          @click="handleSave"
          class="px-6 py-2 rounded-xl bg-primary text-white hover:bg-primary/90 transition-all"
        >
          保存设置
        </button>
      </div>
    </div>
  </Modal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import Modal from '@/components/ui/Modal.vue'
import CompressionSettingsPanel from './CompressionSettingsPanel.vue'

// ... props and emits
</script>
```

#### 2. 简化主界面

**位置**: `src/views/CompressionView.vue`

```vue
<!-- ❌ 删除：全局设置摊开区域 (332-385行) -->
<!-- 
<div class="rounded-2xl border border-primary/20 bg-primary/5 p-6">
  <CompressionSettingsPanel ... />
</div>
-->

<!-- ✅ 新增：紧凑的全局设置按钮 -->
<div class="flex items-center justify-between mb-6">
  <div class="flex gap-3">
    <button
      @click="handleSelectFiles"
      class="px-6 py-3 rounded-xl bg-primary text-white hover:bg-primary/90 transition-all flex items-center gap-2"
    >
      <i class="pi pi-plus"></i>
      <span>添加文件</span>
    </button>
    
    <button
      @click="handleSelectFolders"
      class="px-6 py-3 rounded-xl bg-input border border-subtle text-content hover:border-primary transition-all flex items-center gap-2"
    >
      <i class="pi pi-folder"></i>
      <span>添加文件夹</span>
    </button>
    
    <!-- ✅ 新增：全局设置按钮 -->
    <button
      @click="showGlobalSettingsModal = true"
      class="px-6 py-3 rounded-xl bg-input border border-subtle text-content hover:border-primary transition-all flex items-center gap-2"
    >
      <i class="pi pi-cog"></i>
      <span>全局设置</span>
    </button>
  </div>
  
  <!-- 右侧：开始压缩按钮 -->
  <button
    v-if="totalPayload > 0"
    @click="handleCompress"
    class="px-6 py-3 rounded-xl bg-primary text-white hover:bg-primary/90 transition-all flex items-center gap-2"
  >
    <i class="pi pi-play-circle"></i>
    <span>开始压缩</span>
  </button>
</div>

<!-- 全局设置弹窗 -->
<GlobalSettingsModal
  v-model:visible="showGlobalSettingsModal"
  v-model:settings="compressionStore.globalSettings"
  v-model:outputPath="compressionStore.globalOutputPath"
  :allow-single-file-formats="canGlobalUseSingleFileFormats"
/>
```

#### 3. 常用选项提取

```vue
<!-- ✅ 在按钮下方保留常用选项 -->
<div class="flex items-center gap-6 p-4 rounded-xl bg-input/30 border border-subtle">
  <!-- 目标路径 -->
  <div class="flex items-center gap-3 flex-1">
    <span class="text-xs font-bold text-muted uppercase tracking-widest shrink-0">
      目标路径:
    </span>
    <input
      v-model="compressionStore.globalOutputPath"
      type="text"
      placeholder="选择输出目录..."
      class="flex-1 bg-card border border-subtle rounded-lg px-4 py-2 text-sm"
      readonly
    />
    <button
      @click="selectGlobalOutputPath"
      class="px-4 py-2 rounded-lg bg-primary/10 text-primary hover:bg-primary/20 transition-all"
    >
      浏览
    </button>
  </div>
  
  <!-- 压缩格式（下拉选择） -->
  <div class="flex items-center gap-3">
    <span class="text-xs font-bold text-muted uppercase tracking-widest shrink-0">
      格式:
    </span>
    <select
      v-model="compressionStore.globalSettings.format"
      class="bg-card border border-subtle rounded-lg px-4 py-2 text-sm"
    >
      <option v-for="fmt in COMPRESSIBLE_FORMATS" :key="fmt.value" :value="fmt.value">
        {{ fmt.label }}
      </option>
    </select>
  </div>
</div>
```

---

## Bug #5: 密码生成器按钮

### 问题定位

需要找到密码输入框旁边的"骰子"图标。

### 修复方案

如果找到了该按钮：

```vue
<!-- 密码输入框 -->
<div class="flex items-center gap-2">
  <input
    v-model="task.password"
    type="password"
    placeholder="解压密码（可选）"
    class="flex-1 bg-input border border-subtle rounded-lg px-4 py-2"
  />
  
  <!-- ✅ 添加密码生成器按钮 -->
  <button
    @click="openPasswordGenerator(task)"
    class="w-10 h-10 rounded-lg bg-primary/10 text-primary hover:bg-primary/20 transition-all flex items-center justify-center"
    title="生成随机密码"
  >
    <i class="pi pi-refresh"></i>
  </button>
</div>

<!-- 密码生成器弹窗 -->
<PasswordGeneratorDialog
  v-model:visible="showPasswordGenerator"
  @generated="handlePasswordGenerated"
/>
```

```typescript
const showPasswordGenerator = ref(false)
const currentTask = ref<Task | null>(null)

const openPasswordGenerator = (task: Task) => {
  currentTask.value = task
  showPasswordGenerator.value = true
}

const handlePasswordGenerated = (password: string) => {
  if (currentTask.value) {
    currentTask.value.password = password
  }
  showPasswordGenerator.value = false
}
```

---

## 📊 修复影响评估

| Bug | 文件修改数 | 代码行变更 | 风险等级 | 测试需求 |
|-----|-----------|-----------|---------|---------|
| #1 | 1 个文件 | ~30 行 | 低 | 测试有/无密码文件 |
| #2 | 1 个文件 | ~10 行 | 低 | 测试长文件名 |
| #3 | 4 个文件 | ~150 行 | **高** | 完整任务流程测试 |
| #4 | 3 个文件（新增1个） | ~200 行 | **高** | UI 测试 |
| #5 | 1 个文件 | ~20 行 | 低 | 功能测试 |

---

## ✅ 修复顺序建议

### 第一阶段：快速修复（低风险）
1. Bug #2 - 完整性校验溢出 ✅ (~10 分钟)
2. Bug #5 - 密码生成器按钮 ✅ (~15 分钟)
3. Bug #1 - 密码错误提示 ✅ (~20 分钟)

### 第二阶段：重构（高风险，需充分测试）
4. Bug #4 - 压缩中心布局 ⚠️ (~1 小时)
5. Bug #3 - 任务生命周期 ⚠️ (~1.5 小时)

---

## 🎯 你的决定

请确认：
- ✅ **A) 方案看起来合理，开始修复所有 Bug**
- ✅ **B) 只修复简单的 Bug (#1, #2, #5)，大重构 (#3, #4) 另找时间**
- ✅ **C) 先修复某个特定的 Bug，其他暂缓**
- ✅ **D) 方案需要调整，我有其他想法**
