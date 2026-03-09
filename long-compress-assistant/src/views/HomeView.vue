<template>
  <div class="max-w-7xl mx-auto">
    <!-- 欢迎区域 -->
    <div class="mb-8">
      <h1 class="text-3xl font-bold text-gray-900 dark:text-white mb-2">胧压缩 · 方便助手</h1>
      <p class="text-gray-600 dark:text-gray-400">智能解压，安全高效，您的文件管理好伙伴。</p>
    </div>

    <!-- 主内容区 -->
    <main class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- 左侧：功能区域 -->
      <div class="lg:col-span-2 space-y-6">
        <div class="glass-card">
          <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">开始使用</h2>
          <p class="text-gray-700 dark:text-gray-300 mb-6">
            这是一个智能的文件解压工具，支持多种压缩格式，提供安全、高效的文件管理体验。
          </p>
          <div class="flex flex-col sm:flex-row gap-4">
            <button @click="$router.push('/decompress')" class="glass-button-primary px-6 py-3 flex items-center justify-center button-interaction">
              <i class="pi pi-play mr-2"></i>
              开始解压文件
            </button>
            <button @click="$router.push('/about')" class="glass-button px-6 py-3 flex items-center justify-center button-interaction">
              <i class="pi pi-question-circle mr-2"></i>
              查看教程
            </button>
          </div>
        </div>

        <!-- 功能卡片 -->
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-6">
          <div class="glass-card card-hover-lift">
            <div class="w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center mb-4">
              <i class="pi pi-shield text-primary text-xl"></i>
            </div>
            <h3 class="font-semibold text-gray-900 dark:text-white mb-2">安全解压</h3>
            <p class="text-gray-600 dark:text-gray-400 text-sm">
              支持密码保护文件，自动检测可疑文件，确保解压过程安全可靠。
            </p>
          </div>

          <div class="glass-card card-hover-lift">
            <div class="w-12 h-12 rounded-lg bg-accent/10 flex items-center justify-center mb-4">
              <i class="pi pi-bolt text-accent text-xl"></i>
            </div>
            <h3 class="font-semibold text-gray-900 dark:text-white mb-2">快速处理</h3>
            <p class="text-gray-600 dark:text-gray-400 text-sm">
              多线程解压，智能缓存机制，大幅提升大文件处理速度。
            </p>
          </div>
        </div>
      </div>

      <!-- 右侧：状态面板 -->
      <div class="space-y-6">
        <div class="glass-card">
          <h3 class="font-semibold text-gray-900 dark:text-white mb-4">系统状态</h3>
          <div class="space-y-4">
            <div>
              <div class="flex items-center justify-between mb-1">
                <span class="text-sm text-gray-600 dark:text-gray-400">存储空间</span>
                <span class="text-sm font-medium text-gray-900 dark:text-white">2.3 GB / 10 GB</span>
              </div>
              <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                <div class="bg-primary h-2 rounded-full" style="width: 23%"></div>
              </div>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-sm text-gray-600 dark:text-gray-400">运行时间</span>
              <span class="text-sm font-medium text-gray-900 dark:text-white">2 小时 15 分</span>
            </div>
          </div>
        </div>

        <div class="glass-card">
          <h3 class="font-semibold text-gray-900 dark:text-white mb-4">支持格式</h3>
          <div class="flex flex-wrap gap-2">
            <span class="px-2 py-1 bg-primary/10 text-primary rounded-full text-xs">ZIP</span>
            <span class="px-2 py-1 bg-accent/10 text-accent rounded-full text-xs">RAR</span>
            <span class="px-2 py-1 bg-green-500/10 text-green-500 rounded-full text-xs">7Z</span>
            <span class="px-2 py-1 bg-purple-500/10 text-purple-500 rounded-full text-xs">TAR</span>
            <span class="px-2 py-1 bg-yellow-500/10 text-yellow-500 rounded-full text-xs">GZ</span>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'

const router = useRouter()
</script>

<style scoped>
.button-interaction {
  @apply transition-all duration-200 active:scale-95;
}
.card-hover-lift {
  @apply transition-transform duration-300 hover:-translate-y-1;
}
</style>
