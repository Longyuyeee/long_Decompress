<template>
  <div class="max-w-4xl mx-auto">
    <!-- 页面标题 -->
    <div class="mb-8">
      <h1 class="text-2xl font-bold text-gray-900 dark:text-white">关于</h1>
      <p class="text-gray-600 dark:text-gray-400">了解更多关于胧压缩 · 方便助手的信息</p>
    </div>

    <main class="space-y-8">
      <!-- 应用信息 -->
      <div class="glass-card">
        <div class="flex items-center space-x-6 mb-6">
          <div class="w-20 h-20 rounded-2xl bg-gradient-to-br from-primary to-accent-500 flex items-center justify-center">
            <i class="pi pi-box text-white text-3xl"></i>
          </div>
          <div>
            <h2 class="text-2xl font-bold text-gray-900 dark:text-white">胧压缩 · 方便助手</h2>
            <p class="text-gray-600 dark:text-gray-400">版本 1.0.0</p>
            <p class="text-gray-500 dark:text-gray-400 text-sm mt-1">构建时间: 2024-03-08</p>
          </div>
        </div>

        <p class="text-gray-700 dark:text-gray-300 mb-4">
          胧压缩 · 方便助手是一个现代化的文件解压工具，专注于提供安全、高效、美观的文件解压体验。
          支持多种压缩格式，具备智能文件管理功能，采用苹果毛玻璃设计风格，带来优雅的用户界面。
        </p>

        <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mt-6">
          <div class="text-center p-4 rounded-lg bg-gray-50 dark:bg-gray-800">
            <div class="text-2xl font-bold text-primary mb-1">100+</div>
            <div class="text-sm text-gray-600 dark:text-gray-400">支持格式</div>
          </div>
          <div class="text-center p-4 rounded-lg bg-gray-50 dark:bg-gray-800">
            <div class="text-2xl font-bold text-accent mb-1">∞</div>
            <div class="text-sm text-gray-600 dark:text-gray-400">文件大小</div>
          </div>
          <div class="text-center p-4 rounded-lg bg-gray-50 dark:bg-gray-800">
            <div class="text-2xl font-bold text-green-500 mb-1">AES-256</div>
            <div class="text-sm text-gray-600 dark:text-gray-400">加密强度</div>
          </div>
          <div class="text-center p-4 rounded-lg bg-gray-50 dark:bg-gray-800">
            <div class="text-2xl font-bold text-purple-500 mb-1">8x</div>
            <div class="text-sm text-gray-600 dark:text-gray-400">解压速度</div>
          </div>
        </div>
      </div>

      <!-- 功能特性 -->
      <div class="glass-card">
        <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-6">功能特性</h2>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div class="flex items-start space-x-4">
            <div class="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center flex-shrink-0">
              <i class="pi pi-shield text-primary"></i>
            </div>
            <div>
              <h3 class="font-medium text-gray-900 dark:text-white mb-1">安全可靠</h3>
              <p class="text-gray-600 dark:text-gray-400 text-sm">内置可疑文件检测，密码加密存储，确保文件安全。</p>
            </div>
          </div>
          <div class="flex items-start space-x-4">
            <div class="w-10 h-10 rounded-lg bg-accent/10 flex items-center justify-center flex-shrink-0">
              <i class="pi pi-bolt text-accent"></i>
            </div>
            <div>
              <h3 class="font-medium text-gray-900 dark:text-white mb-1">高效快速</h3>
              <p class="text-gray-600 dark:text-gray-400 text-sm">多线程解压，智能缓存，大幅提升处理速度。</p>
            </div>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
// 这里可以添加逻辑
</script>

<style scoped>
.glass-card {
  @apply p-6 rounded-2xl border border-white/20 bg-white/10 backdrop-blur-md shadow-lg dark:border-white/10 dark:bg-black/20;
}
</style>
