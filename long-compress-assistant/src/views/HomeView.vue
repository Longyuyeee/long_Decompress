<template>
  <div class="max-w-7xl mx-auto">
    <!-- 欢迎区域 -->

      <!-- 主内容区 -->
      <main class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- 左侧：欢迎区域 -->
        <div class="lg:col-span-2">
          <div class="glass-card mb-6">
            <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">欢迎使用胧压缩·方便助手</h2>
            <p class="text-gray-700 dark:text-gray-300 mb-4">
              这是一个智能的文件解压工具，支持多种压缩格式，提供安全、高效的文件管理体验。
            </p>
            <div class="flex flex-col sm:flex-row space-y-3 sm:space-y-0 sm:space-x-4">
              <button @click="$router.push('/decompress')" class="glass-button-primary px-4 sm:px-6 py-3 w-full sm:w-auto flex items-center justify-center hover:scale-105 active:scale-95 transition-transform" aria-label="开始解压文件">
                <i class="pi pi-play mr-2" aria-hidden="true"></i>
                开始解压文件
              </button>
              <button class="glass-button px-4 sm:px-6 py-3 w-full sm:w-auto flex items-center justify-center hover:scale-105 active:scale-95 transition-transform" aria-label="查看使用教程">
                <i class="pi pi-question-circle mr-2" aria-hidden="true"></i>
                查看教程
              </button>
            </div>
          </div>

          <!-- 功能卡片 -->
          <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-2 gap-4 sm:gap-6">
            <div class="glass-card" role="article" aria-labelledby="feature-safe">
              <div class="w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center mb-4" aria-hidden="true">
                <i class="pi pi-shield text-primary text-xl"></i>
              </div>
              <h3 id="feature-safe" class="font-semibold text-gray-900 dark:text-white mb-2">安全解压</h3>
              <p class="text-gray-600 dark:text-gray-400 text-sm">
                支持密码保护文件，自动检测可疑文件，确保解压过程安全可靠。
              </p>
            </div>

            <div class="glass-card">
              <div class="w-12 h-12 rounded-lg bg-accent/10 flex items-center justify-center mb-4">
                <i class="pi pi-bolt text-accent text-xl"></i>
              </div>
              <h3 class="font-semibold text-gray-900 dark:text-white mb-2">快速处理</h3>
              <p class="text-gray-600 dark:text-gray-400 text-sm">
                多线程解压，智能缓存机制，大幅提升大文件处理速度。
              </p>
            </div>

            <div class="glass-card">
              <div class="w-12 h-12 rounded-lg bg-green-500/10 flex items-center justify-center mb-4">
                <i class="pi pi-file text-green-500 text-xl"></i>
              </div>
              <h3 class="font-semibold text-gray-900 dark:text-white mb-2">多格式支持</h3>
              <p class="text-gray-600 dark:text-gray-400 text-sm">
                支持 ZIP、RAR、7Z、TAR、GZ 等多种常见压缩格式。
              </p>
            </div>

            <div class="glass-card">
              <div class="w-12 h-12 rounded-lg bg-purple-500/10 flex items-center justify-center mb-4">
                <i class="pi pi-eye text-purple-500 text-xl"></i>
              </div>
              <h3 class="font-semibold text-gray-900 dark:text-white mb-2">预览功能</h3>
              <p class="text-gray-600 dark:text-gray-400 text-sm">
                解压前预览文件内容，支持图片、文本、PDF等多种文件预览。
              </p>
            </div>
          </div>
        </div>

        <!-- 右侧：状态面板 -->
        <div class="space-y-4 sm:space-y-6">
          <div class="glass-card">
            <h3 class="font-semibold text-gray-900 dark:text-white mb-4">系统状态</h3>
            <div class="space-y-3">
              <div class="flex items-center justify-between">
                <span class="text-gray-600 dark:text-gray-400">存储空间</span>
                <span class="font-medium text-gray-900 dark:text-white">2.3 GB / 10 GB</span>
              </div>
              <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                <div class="bg-primary h-2 rounded-full" style="width: 23%"></div>
              </div>

              <div class="flex items-center justify-between">
                <span class="text-gray-600 dark:text-gray-400">最近解压</span>
                <span class="font-medium text-gray-900 dark:text-white">5 个文件</span>
              </div>

              <div class="flex items-center justify-between">
                <span class="text-gray-600 dark:text-gray-400">运行时间</span>
                <span class="font-medium text-gray-900 dark:text-white">2 小时 15 分</span>
              </div>
            </div>
          </div>

          <div class="glass-card">
            <h3 class="font-semibold text-gray-900 dark:text-white mb-4">快速操作</h3>
            <div class="space-y-2 sm:space-y-3">
              <button class="w-full glass-button text-left px-3 sm:px-4 py-2 sm:py-3 hover:scale-105 active:scale-95 transition-transform focus:ring-2 focus:ring-primary focus:outline-none" aria-label="打开最近使用的文件">
                <i class="pi pi-folder-open mr-2 sm:mr-3" aria-hidden="true"></i>
                打开最近文件
              </button>
              <button class="w-full glass-button text-left px-3 sm:px-4 py-2 sm:py-3 hover:scale-105 active:scale-95 transition-transform focus:ring-2 focus:ring-primary focus:outline-none" aria-label="查看解压历史记录">
                <i class="pi pi-history mr-2 sm:mr-3" aria-hidden="true"></i>
                查看历史记录
              </button>
              <button class="w-full glass-button text-left px-3 sm:px-4 py-2 sm:py-3 hover:scale-105 active:scale-95 transition-transform focus:ring-2 focus:ring-primary focus:outline-none" aria-label="查看收藏的文件">
                <i class="pi pi-star mr-2 sm:mr-3" aria-hidden="true"></i>
                收藏的文件
              </button>
            </div>
          </div>

          <div class="glass-card">
            <h3 class="font-semibold text-gray-900 dark:text-white mb-4">支持格式</h3>
            <div class="flex flex-wrap gap-1 sm:gap-2">
              <span class="px-2 sm:px-3 py-1 bg-primary/10 text-primary rounded-full text-xs sm:text-sm">ZIP</span>
              <span class="px-2 sm:px-3 py-1 bg-accent/10 text-accent rounded-full text-xs sm:text-sm">RAR</span>
              <span class="px-2 sm:px-3 py-1 bg-green-500/10 text-green-500 rounded-full text-xs sm:text-sm">7Z</span>
              <span class="px-2 sm:px-3 py-1 bg-purple-500/10 text-purple-500 rounded-full text-xs sm:text-sm">TAR</span>
              <span class="px-2 sm:px-3 py-1 bg-yellow-500/10 text-yellow-500 rounded-full text-xs sm:text-sm">GZ</span>
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
.router-link-active {
  @apply bg-primary/20 text-primary border-primary/40;
}
</style>