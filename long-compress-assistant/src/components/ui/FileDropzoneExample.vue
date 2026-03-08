<template>
  <div class="file-dropzone-example space-y-8">
    <!-- 标题 -->
    <div class="text-center">
      <h1 class="text-3xl font-bold text-gray-900 dark:text-white mb-2">文件选择组件示例</h1>
      <p class="text-lg text-gray-600 dark:text-gray-400">支持拖放和系统对话框的文件选择器</p>
    </div>

    <!-- 基本用法 -->
    <section class="space-y-4">
      <h2 class="text-2xl font-semibold text-gray-900 dark:text-white">基本用法</h2>

      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <!-- 原始FileDropzone -->
        <div class="space-y-4">
          <h3 class="text-xl font-medium text-gray-800 dark:text-gray-200">原始FileDropzone</h3>
          <p class="text-gray-600 dark:text-gray-400">使用原生HTML文件输入和拖放API</p>
          <FileDropzone
            :multiple="true"
            :maxSize="1024 * 1024 * 10" <!-- 10MB -->
            :maxFiles="5"
            @files-selected="handleFilesSelected"
            @error="handleError"
          />
        </div>

        <!-- 增强版EnhancedFileDropzone -->
        <div class="space-y-4">
          <h3 class="text-xl font-medium text-gray-800 dark:text-gray-200">增强版EnhancedFileDropzone</h3>
          <p class="text-gray-600 dark:text-gray-400">集成Tauri系统对话框，支持文件格式检测</p>
          <EnhancedFileDropzone
            :multiple="true"
            :maxSize="1024 * 1024 * 50" <!-- 50MB -->
            :maxFiles="10"
            :useTauriDialog="true"
            @files-selected="handleEnhancedFilesSelected"
            @error="handleError"
            @preview="handlePreview"
          />
        </div>
      </div>
    </section>

    <!-- 配置选项 -->
    <section class="space-y-4">
      <h2 class="text-2xl font-semibold text-gray-900 dark:text-white">配置选项</h2>

      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        <!-- 单文件模式 -->
        <div class="space-y-4">
          <h3 class="text-lg font-medium text-gray-800 dark:text-gray-200">单文件模式</h3>
          <EnhancedFileDropzone
            :multiple="false"
            :maxSize="1024 * 1024 * 100" <!-- 100MB -->
            @files-selected="handleSingleFileSelected"
            @error="handleError"
          />
        </div>

        <!-- 特定格式 -->
        <div class="space-y-4">
          <h3 class="text-lg font-medium text-gray-800 dark:text-gray-200">仅ZIP格式</h3>
          <EnhancedFileDropzone
            :multiple="true"
            accept=".zip"
            :maxFiles="3"
            @files-selected="handleZipFilesSelected"
            @error="handleError"
          />
        </div>

        <!-- 小文件限制 -->
        <div class="space-y-4">
          <h3 class="text-lg font-medium text-gray-800 dark:text-gray-200">小文件限制 (1MB)</h3>
          <EnhancedFileDropzone
            :multiple="true"
            :maxSize="1024 * 1024" <!-- 1MB -->
            :maxFiles="20"
            @files-selected="handleSmallFilesSelected"
            @error="handleError"
          />
        </div>
      </div>
    </section>

    <!-- 状态显示 -->
    <section class="space-y-4">
      <h2 class="text-2xl font-semibold text-gray-900 dark:text-white">状态显示</h2>

      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <!-- 已选择文件列表 -->
        <div class="space-y-4">
          <h3 class="text-xl font-medium text-gray-800 dark:text-gray-200">已选择文件</h3>

          <div v-if="selectedFiles.length === 0" class="text-center py-8 rounded-lg border-2 border-dashed border-gray-300 dark:border-gray-600">
            <i class="pi pi-inbox text-gray-400 text-4xl mb-4"></i>
            <p class="text-gray-500 dark:text-gray-400">暂无文件</p>
          </div>

          <div v-else class="space-y-3">
            <div
              v-for="file in selectedFiles"
              :key="file.id"
              class="p-4 rounded-lg glass-effect"
            >
              <div class="flex items-start justify-between">
                <div class="flex-1">
                  <div class="flex items-center mb-2">
                    <i :class="getFileIcon(file)" class="text-xl mr-3"></i>
                    <div>
                      <p class="font-medium text-gray-900 dark:text-white truncate">{{ file.name }}</p>
                      <div class="flex items-center space-x-3 mt-1">
                        <span class="text-sm text-gray-500 dark:text-gray-400">{{ formatFileSize(file.size) }}</span>
                        <span v-if="file.format" class="text-xs px-2 py-0.5 rounded-full bg-primary/10 text-primary">
                          {{ file.format.toUpperCase() }}
                        </span>
                        <span v-if="file.encrypted" class="text-xs px-2 py-0.5 rounded-full bg-warning/10 text-warning">
                          加密
                        </span>
                      </div>
                    </div>
                  </div>
                  <div class="text-sm text-gray-600 dark:text-gray-400">
                    <p v-if="file.path">路径: {{ file.path }}</p>
                    <p v-else>来源: 拖放</p>
                  </div>
                </div>
                <button
                  @click="removeSelectedFile(file.id)"
                  class="text-gray-400 hover:text-red-500 transition-colors ml-4"
                  title="删除文件"
                >
                  <i class="pi pi-times"></i>
                </button>
              </div>
            </div>

            <div class="pt-4 border-t border-gray-200 dark:border-gray-700">
              <div class="flex justify-between items-center">
                <div>
                  <p class="text-sm text-gray-600 dark:text-gray-400">
                    共 {{ selectedFiles.length }} 个文件，总大小 {{ formatFileSize(totalSelectedSize) }}
                  </p>
                </div>
                <button
                  @click="clearSelectedFiles"
                  class="glass-button-danger px-4 py-2 text-sm"
                >
                  <i class="pi pi-trash mr-2"></i>
                  清空所有
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- 操作面板 -->
        <div class="space-y-4">
          <h3 class="text-xl font-medium text-gray-800 dark:text-gray-200">操作面板</h3>

          <div class="space-y-4">
            <!-- 组件控制 -->
            <div class="glass-card p-6">
              <h4 class="font-medium text-gray-900 dark:text-white mb-4">组件控制</h4>
              <div class="space-y-3">
                <div class="flex items-center justify-between">
                  <span class="text-gray-700 dark:text-gray-300">使用Tauri对话框</span>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input type="checkbox" v-model="useTauriDialog" class="sr-only peer">
                    <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-primary/50 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-primary"></div>
                  </label>
                </div>

                <div class="flex items-center justify-between">
                  <span class="text-gray-700 dark:text-gray-300">显示预览按钮</span>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input type="checkbox" v-model="showPreview" class="sr-only peer">
                    <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-primary/50 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-primary"></div>
                  </label>
                </div>
              </div>
            </div>

            <!-- 批量操作 -->
            <div class="glass-card p-6">
              <h4 class="font-medium text-gray-900 dark:text-white mb-4">批量操作</h4>
              <div class="space-y-3">
                <button
                  @click="processSelectedFiles"
                  :disabled="selectedFiles.length === 0"
                  class="w-full glass-button-primary py-3 flex items-center justify-center"
                  :class="{ 'opacity-50 cursor-not-allowed': selectedFiles.length === 0 }"
                >
                  <i class="pi pi-play mr-2"></i>
                  处理选中的文件 ({{ selectedFiles.length }})
                </button>

                <button
                  @click="exportFileList"
                  :disabled="selectedFiles.length === 0"
                  class="w-full glass-button py-3 flex items-center justify-center"
                  :class="{ 'opacity-50 cursor-not-allowed': selectedFiles.length === 0 }"
                >
                  <i class="pi pi-download mr-2"></i>
                  导出文件列表
                </button>
              </div>
            </div>

            <!-- 统计信息 -->
            <div class="glass-card p-6">
              <h4 class="font-medium text-gray-900 dark:text-white mb-4">统计信息</h4>
              <div class="space-y-2">
                <div class="flex justify-between">
                  <span class="text-gray-600 dark:text-gray-400">文件总数</span>
                  <span class="font-medium text-gray-900 dark:text-white">{{ selectedFiles.length }}</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-gray-600 dark:text-gray-400">总大小</span>
                  <span class="font-medium text-gray-900 dark:text-white">{{ formatFileSize(totalSelectedSize) }}</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-gray-600 dark:text-gray-400">加密文件</span>
                  <span class="font-medium text-gray-900 dark:text-white">{{ encryptedFilesCount }}</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-gray-600 dark:text-gray-400">ZIP文件</span>
                  <span class="font-medium text-gray-900 dark:text-white">{{ zipFilesCount }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- 使用说明 -->
    <section class="space-y-4">
      <h2 class="text-2xl font-semibold text-gray-900 dark:text-white">使用说明</h2>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div class="glass-card p-6">
          <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-3">组件特性</h3>
          <ul class="space-y-2 text-gray-600 dark:text-gray-400">
            <li class="flex items-start">
              <i class="pi pi-check-circle text-green-500 mr-2 mt-0.5"></i>
              <span>支持拖放文件和系统对话框选择</span>
            </li>
            <li class="flex items-start">
              <i class="pi pi-check-circle text-green-500 mr-2 mt-0.5"></i>
              <span>集成Tauri文件系统API，获取完整文件信息</span>
            </li>
            <li class="flex items-start">
              <i class="pi pi-check-circle text-green-500 mr-2 mt-0.5"></i>
              <span>自动检测文件格式和加密状态</span>
            </li>
            <li class="flex items-start">
              <i class="pi pi-check-circle text-green-500 mr-2 mt-0.5"></i>
              <span>支持文件大小和数量限制</span>
            </li>
            <li class="flex items-start">
              <i class="pi pi-check-circle text-green-500 mr-2 mt-0.5"></i>
              <span>提供丰富的状态反馈和错误处理</span>
            </li>
          </ul>
        </div>

        <div class="glass-card p-6">
          <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-3">代码示例</h3>
          <div class="space-y-2 font-mono text-sm">
            <code class="block p-3 bg-gray-900 text-gray-100 rounded overflow-x-auto">
&lt;EnhancedFileDropzone<br>
&nbsp;&nbsp;:multiple="true"<br>
&nbsp;&nbsp;:maxSize="1024 * 1024 * 50"<br>
&nbsp;&nbsp;:maxFiles="10"<br>
&nbsp;&nbsp;:useTauriDialog="true"<br>
&nbsp;&nbsp;@files-selected="handleFiles"<br>
&nbsp;&nbsp;@error="handleError"<br>
/&gt;
            </code>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { FileDropzone, EnhancedFileDropzone } from './index'
import type { FileItem } from './EnhancedFileDropzone.vue'

// 状态
const selectedFiles = ref<FileItem[]>([])
const useTauriDialog = ref(true)
const showPreview = ref(true)

// 计算属性
const totalSelectedSize = computed(() => {
  return selectedFiles.value.reduce((sum, file) => sum + file.size, 0)
})

const encryptedFilesCount = computed(() => {
  return selectedFiles.value.filter(file => file.encrypted).length
})

const zipFilesCount = computed(() => {
  return selectedFiles.value.filter(file => file.format === 'zip' || file.name.endsWith('.zip')).length
})

// 方法
const formatFileSize = (bytes: number): string => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

const getFileIcon = (file: FileItem): string => {
  const extension = file.name.split('.').pop()?.toLowerCase()

  switch (extension) {
    case 'zip':
      return 'pi pi-file-archive text-blue-500'
    case 'rar':
      return 'pi pi-file-archive text-red-500'
    case '7z':
      return 'pi pi-file-archive text-green-500'
    case 'tar':
    case 'gz':
    case 'bz2':
      return 'pi pi-file-archive text-purple-500'
    default:
      return 'pi pi-file text-gray-500'
  }
}

const handleFilesSelected = (files: FileItem[]) => {
  console.log('原始FileDropzone选择的文件:', files)
  selectedFiles.value = [...selectedFiles.value, ...files]
}

const handleEnhancedFilesSelected = (files: FileItem[]) => {
  console.log('EnhancedFileDropzone选择的文件:', files)
  selectedFiles.value = [...selectedFiles.value, ...files]
}

const handleSingleFileSelected = (files: FileItem[]) => {
  console.log('单文件选择:', files)
  selectedFiles.value = files // 替换而不是追加
}

const handleZipFilesSelected = (files: FileItem[]) => {
  console.log('ZIP文件选择:', files)
  selectedFiles.value = [...selectedFiles.value, ...files]
}

const handleSmallFilesSelected = (files: FileItem[]) => {
  console.log('小文件选择:', files)
  selectedFiles.value = [...selectedFiles.value, ...files]
}

const handleError = (error: string) => {
  console.error('文件选择错误:', error)
  alert(`错误: ${error}`)
}

const handlePreview = (file: FileItem) => {
  console.log('预览文件:', file)
  alert(`预览文件: ${file.name}\n路径: ${file.path || '拖放文件'}`)
}

const removeSelectedFile = (fileId: string) => {
  selectedFiles.value = selectedFiles.value.filter(file => file.id !== fileId)
}

const clearSelectedFiles = () => {
  selectedFiles.value = []
}

const processSelectedFiles = () => {
  if (selectedFiles.value.length === 0) return

  console.log('处理文件:', selectedFiles.value)
  alert(`开始处理 ${selectedFiles.value.length} 个文件`)

  // 这里可以调用解压API
}

const exportFileList = () => {
  if (selectedFiles.value.length === 0) return

  const fileList = selectedFiles.value.map(file => ({
    名称: file.name,
    大小: formatFileSize(file.size),
    格式: file.format || '未知',
    加密: file.encrypted ? '是' : '否',
    路径: file.path || '拖放文件'
  }))

  console.log('导出文件列表:', fileList)

  // 这里可以生成并下载CSV文件
  const csvContent = 'data:text/csv;charset=utf-8,'
    + '名称,大小,格式,加密,路径\n'
    + fileList.map(f => `"${f.名称}","${f.大小}","${f.格式}","${f.加密}","${f.路径}"`).join('\n')

  const encodedUri = encodeURI(csvContent)
  const link = document.createElement('a')
  link.setAttribute('href', encodedUri)
  link.setAttribute('download', '文件列表.csv')
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
}
</script>

<style scoped>
.file-dropzone-example {
  max-width: 1200px;
  margin: 0 auto;
  padding: 2rem;
}
</style>