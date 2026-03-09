<template>
  <div class="max-w-4xl mx-auto">
    <!-- 页面标题 -->
    <div class="mb-8">
      <h1 class="text-2xl font-bold text-gray-900 dark:text-white">设置</h1>
      <p class="text-gray-600 dark:text-gray-400">管理应用偏好和功能配置</p>
    </div>

    <main class="space-y-6">
      <!-- 常规设置 -->
      <section class="glass-card">
        <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center">
          <i class="pi pi-cog mr-2"></i> 常规设置
        </h2>
        <div class="space-y-4">
          <div class="flex items-center justify-between">
            <div>
              <p class="font-medium text-gray-900 dark:text-white">深色模式</p>
              <p class="text-sm text-gray-500">跟随系统或手动切换主题</p>
            </div>
            <!-- 这里可以放切换组件 -->
          </div>
          <div class="flex items-center justify-between border-t border-gray-100 dark:border-gray-800 pt-4">
            <div>
              <p class="font-medium text-gray-900 dark:text-white">开机自启动</p>
              <p class="text-sm text-gray-500">系统启动时自动运行应用</p>
            </div>
          </div>
        </div>
      </section>

      <!-- 压缩设置 -->
      <section class="glass-card">
        <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center">
          <i class="pi pi-file-excel mr-2"></i> 压缩选项
        </h2>
        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">默认解压路径</label>
            <div class="flex gap-2">
              <input type="text" class="glass-input flex-1" placeholder="选择路径..." readonly />
              <button class="glass-button px-4 py-2">浏览</button>
            </div>
          </div>
        </div>
      </section>
    </main>
  </div>
</template>

<script setup lang="ts">
// 这里可以添加设置逻辑
</script>

<style scoped>
.glass-card {
  @apply p-6 rounded-2xl border border-white/20 bg-white/10 backdrop-blur-md shadow-lg dark:border-white/10 dark:bg-black/20;
}
.glass-input {
  @apply bg-white/5 border border-white/10 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-primary/50 dark:bg-black/20;
}
.glass-button {
  @apply bg-primary/10 text-primary border border-primary/20 rounded-lg transition-colors hover:bg-primary/20;
}
</style>
