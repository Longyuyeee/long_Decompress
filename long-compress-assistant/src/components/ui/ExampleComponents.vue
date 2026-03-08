<template>
  <div class="p-6 max-w-7xl mx-auto">
    <h1 class="text-3xl font-bold text-gray-900 dark:text-white mb-8">设计系统示例</h1>

    <!-- 颜色示例 -->
    <section class="mb-12">
      <h2 class="text-2xl font-semibold text-gray-900 dark:text-white mb-6">颜色系统</h2>

      <div class="grid grid-cols-2 md:grid-cols-4 gap-6">
        <div class="glass-card">
          <div class="w-full h-20 rounded-lg bg-primary-500 mb-3"></div>
          <h3 class="font-medium text-gray-900 dark:text-white">主蓝</h3>
          <p class="text-sm text-gray-600 dark:text-gray-400">#0ea5e9</p>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">主要操作、重要元素</p>
        </div>

        <div class="glass-card">
          <div class="w-full h-20 rounded-lg bg-secondary-500 mb-3"></div>
          <h3 class="font-medium text-gray-900 dark:text-white">中性灰</h3>
          <p class="text-sm text-gray-600 dark:text-gray-400">#64748b</p>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">正文文本、图标</p>
        </div>

        <div class="glass-card">
          <div class="w-full h-20 rounded-lg bg-green-500 mb-3"></div>
          <h3 class="font-medium text-gray-900 dark:text-white">成功绿</h3>
          <p class="text-sm text-gray-600 dark:text-gray-400">#10b981</p>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">成功状态、完成提示</p>
        </div>

        <div class="glass-card">
          <div class="w-full h-20 rounded-lg bg-red-500 mb-3"></div>
          <h3 class="font-medium text-gray-900 dark:text-white">错误红</h3>
          <p class="text-sm text-gray-600 dark:text-gray-400">#ef4444</p>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">错误提示、危险操作</p>
        </div>
      </div>
    </section>

    <!-- 按钮示例 -->
    <section class="mb-12">
      <h2 class="text-2xl font-semibold text-gray-900 dark:text-white mb-6">按钮组件</h2>

      <div class="glass-card">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-8">
          <!-- 变体 -->
          <div>
            <h3 class="font-medium text-gray-900 dark:text-white mb-4">按钮变体</h3>
            <div class="space-y-4">
              <GlassButton text="默认按钮" />
              <GlassButton variant="primary" text="主要按钮" />
              <GlassButton variant="secondary" text="次要按钮" />
              <GlassButton variant="danger" text="危险按钮" />
            </div>
          </div>

          <!-- 尺寸 -->
          <div>
            <h3 class="font-medium text-gray-900 dark:text-white mb-4">按钮尺寸</h3>
            <div class="space-y-4">
              <div class="flex items-center space-x-4">
                <GlassButton size="sm" text="小按钮" />
                <GlassButton size="md" text="中按钮" />
                <GlassButton size="lg" text="大按钮" />
                <GlassButton size="xl" text="超大按钮" />
              </div>

              <div class="pt-4 border-t border-gray-200 dark:border-gray-700">
                <h4 class="font-medium text-gray-900 dark:text-white mb-3">按钮状态</h4>
                <div class="flex items-center space-x-4">
                  <GlassButton text="正常" />
                  <GlassButton text="禁用" disabled />
                  <GlassButton text="加载中" loading />
                  <GlassButton text="带图标" icon="pi-check" />
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- 卡片示例 -->
    <section class="mb-12">
      <h2 class="text-2xl font-semibold text-gray-900 dark:text-white mb-6">卡片组件</h2>

      <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
        <GlassCard>
          <div class="w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center mb-4">
            <i class="pi pi-shield text-primary text-xl"></i>
          </div>
          <h3 class="font-semibold text-gray-900 dark:text-white mb-2">安全解压</h3>
          <p class="text-gray-600 dark:text-gray-400 text-sm">
            支持密码保护文件，自动检测可疑文件，确保解压过程安全可靠。
          </p>
        </GlassCard>

        <GlassCard hoverable>
          <div class="w-12 h-12 rounded-lg bg-accent/10 flex items-center justify-center mb-4">
            <i class="pi pi-bolt text-accent text-xl"></i>
          </div>
          <h3 class="font-semibold text-gray-900 dark:text-white mb-2">快速处理</h3>
          <p class="text-gray-600 dark:text-gray-400 text-sm">
            多线程解压，智能缓存机制，大幅提升大文件处理速度。
          </p>
        </GlassCard>

        <GlassCard compact>
          <div class="w-12 h-12 rounded-lg bg-green-500/10 flex items-center justify-center mb-4">
            <i class="pi pi-file text-green-500 text-xl"></i>
          </div>
          <h3 class="font-semibold text-gray-900 dark:text-white mb-2">多格式支持</h3>
          <p class="text-gray-600 dark:text-gray-400 text-sm">
            支持 ZIP、RAR、7Z、TAR、GZ 等多种常见压缩格式。
          </p>
        </GlassCard>
      </div>
    </section>

    <!-- 进度条示例 -->
    <section class="mb-12">
      <h2 class="text-2xl font-semibold text-gray-900 dark:text-white mb-6">进度条组件</h2>

      <div class="glass-card">
        <div class="space-y-8">
          <!-- 变体 -->
          <div>
            <h3 class="font-medium text-gray-900 dark:text-white mb-4">进度条变体</h3>
            <div class="space-y-6">
              <ProgressBar variant="primary" :value="75" label="主要进度" />
              <ProgressBar variant="success" :value="90" label="成功进度" />
              <ProgressBar variant="warning" :value="60" label="警告进度" />
              <ProgressBar variant="danger" :value="30" label="危险进度" />
            </div>
          </div>

          <!-- 样式 -->
          <div class="pt-6 border-t border-gray-200 dark:border-gray-700">
            <h3 class="font-medium text-gray-900 dark:text-white mb-4">进度条样式</h3>
            <div class="space-y-6">
              <ProgressBar variant="primary" :value="65" label="条纹效果" striped />
              <ProgressBar variant="info" :value="45" label="动画效果" animated />
              <ProgressBar variant="secondary" indeterminate label="不确定状态" />
            </div>
          </div>

          <!-- 尺寸 -->
          <div class="pt-6 border-t border-gray-200 dark:border-gray-700">
            <h3 class="font-medium text-gray-900 dark:text-white mb-4">进度条尺寸</h3>
            <div class="space-y-6">
              <ProgressBar variant="primary" :value="50" size="sm" label="小尺寸" />
              <ProgressBar variant="primary" :value="50" size="md" label="中尺寸" />
              <ProgressBar variant="primary" :value="50" size="lg" label="大尺寸" />
              <ProgressBar variant="primary" :value="50" size="xl" label="超大尺寸" />
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- 文件拖放示例 -->
    <section class="mb-12">
      <h2 class="text-2xl font-semibold text-gray-900 dark:text-white mb-6">文件拖放组件</h2>

      <div class="glass-card">
        <FileDropzone
          :multiple="true"
          :max-size="1024 * 1024 * 100" <!-- 100MB -->
          :max-files="5"
          @files-selected="handleFilesSelected"
          @file-removed="handleFileRemoved"
          @error="handleFileError"
        />

        <div v-if="droppedFiles.length > 0" class="mt-6">
          <h3 class="font-medium text-gray-900 dark:text-white mb-3">已选择文件</h3>
          <div class="space-y-2">
            <div v-for="file in droppedFiles" :key="file.id"
                 class="flex items-center justify-between p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
              <div class="flex items-center">
                <i :class="getFileIcon(file)" class="mr-3"></i>
                <div>
                  <p class="font-medium text-gray-900 dark:text-white">{{ file.name }}</p>
                  <p class="text-sm text-gray-500 dark:text-gray-400">{{ formatFileSize(file.size) }}</p>
                </div>
              </div>
              <button @click="removeFile(file.id)" class="text-red-500 hover:text-red-700">
                <i class="pi pi-times"></i>
              </button>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- 排版示例 -->
    <section class="mb-12">
      <h2 class="text-2xl font-semibold text-gray-900 dark:text-white mb-6">排版系统</h2>

      <div class="glass-card">
        <div class="space-y-6">
          <div>
            <h1 class="text-4xl font-bold text-gray-900 dark:text-white">H1 标题 - 2.5rem</h1>
            <p class="text-gray-600 dark:text-gray-400 mt-1">页面主标题，用于最重要的内容</p>
          </div>

          <div>
            <h2 class="text-3xl font-semibold text-gray-900 dark:text-white">H2 标题 - 2rem</h2>
            <p class="text-gray-600 dark:text-gray-400 mt-1">区块标题，用于主要章节</p>
          </div>

          <div>
            <h3 class="text-2xl font-semibold text-gray-900 dark:text-white">H3 标题 - 1.5rem</h3>
            <p class="text-gray-600 dark:text-gray-400 mt-1">卡片标题，用于内容区块</p>
          </div>

          <div>
            <h4 class="text-xl font-medium text-gray-900 dark:text-white">H4 标题 - 1.25rem</h4>
            <p class="text-gray-600 dark:text-gray-400 mt-1">小节标题，用于细分内容</p>
          </div>

          <div class="pt-4 border-t border-gray-200 dark:border-gray-700">
            <p class="text-lg text-gray-700 dark:text-gray-300">正文大号 - 1.125rem</p>
            <p class="text-gray-700 dark:text-gray-300">正文常规 - 1rem</p>
            <p class="text-sm text-gray-600 dark:text-gray-400">正文小号 - 0.875rem</p>
            <p class="text-xs text-gray-500 dark:text-gray-400">说明文字 - 0.75rem</p>
          </div>

          <div class="pt-4 border-t border-gray-200 dark:border-gray-700">
            <p class="font-mono text-gray-700 dark:text-gray-300">等宽字体 - JetBrains Mono</p>
            <p class="text-sm text-gray-600 dark:text-gray-400">用于代码片段、文件路径、技术信息</p>
          </div>
        </div>
      </div>
    </section>

    <!-- 间距示例 -->
    <section>
      <h2 class="text-2xl font-semibold text-gray-900 dark:text-white mb-6">间距系统</h2>

      <div class="glass-card">
        <div class="space-y-8">
          <div>
            <h3 class="font-medium text-gray-900 dark:text-white mb-4">基础间距单位</h3>
            <div class="flex items-center space-x-4">
              <div class="text-center">
                <div class="w-4 h-4 bg-primary rounded mb-1"></div>
                <span class="text-xs text-gray-600 dark:text-gray-400">xs (4px)</span>
              </div>
              <div class="text-center">
                <div class="w-8 h-8 bg-primary rounded mb-1"></div>
                <span class="text-xs text-gray-600 dark:text-gray-400">sm (8px)</span>
              </div>
              <div class="text-center">
                <div class="w-16 h-16 bg-primary rounded mb-1"></div>
                <span class="text-xs text-gray-600 dark:text-gray-400">md (16px)</span>
              </div>
              <div class="text-center">
                <div class="w-24 h-24 bg-primary rounded mb-1"></div>
                <span class="text-xs text-gray-600 dark:text-gray-400">lg (24px)</span>
              </div>
            </div>
          </div>

          <div class="pt-6 border-t border-gray-200 dark:border-gray-700">
            <h3 class="font-medium text-gray-900 dark:text-white mb-4">实际应用示例</h3>
            <div class="space-y-4">
              <div class="p-2 bg-gray-100 dark:bg-gray-800 rounded">
                <p class="text-sm text-gray-600 dark:text-gray-400">内边距: p-2 (8px)</p>
              </div>
              <div class="p-4 bg-gray-100 dark:bg-gray-800 rounded">
                <p class="text-sm text-gray-600 dark:text-gray-400">内边距: p-4 (16px)</p>
              </div>
              <div class="p-6 bg-gray-100 dark:bg-gray-800 rounded">
                <p class="text-sm text-gray-600 dark:text-gray-400">内边距: p-6 (24px)</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import GlassCard from './GlassCard.vue'
import GlassButton from './GlassButton.vue'
import FileDropzone from './FileDropzone.vue'
import ProgressBar from './ProgressBar.vue'

interface FileItem {
  id: string
  name: string
  size: number
  type: string
}

const droppedFiles = ref<FileItem[]>([])

const handleFilesSelected = (files: any[]) => {
  console.log('文件已选择:', files)
  // 这里可以添加文件处理逻辑
}

const handleFileRemoved = (fileId: string) => {
  console.log('文件已移除:', fileId)
  droppedFiles.value = droppedFiles.value.filter(f => f.id !== fileId)
}

const handleFileError = (error: string) => {
  console.error('文件错误:', error)
  // 这里可以显示错误提示
}

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
    default:
      return 'pi pi-file text-gray-500'
  }
}

const removeFile = (fileId: string) => {
  droppedFiles.value = droppedFiles.value.filter(f => f.id !== fileId)
}
</script>

<style scoped>
/* 自定义样式示例 */
</style>