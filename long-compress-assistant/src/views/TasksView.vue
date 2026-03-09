<template>
  <div class="max-w-7xl mx-auto">
    <!-- 页面标题 -->
    <div class="mb-8">
      <h1 class="text-2xl font-bold text-gray-900 dark:text-white">任务管理</h1>
      <p class="text-gray-600 dark:text-gray-400">查看和管理正在进行或已完成的任务</p>
    </div>

    <main class="space-y-6">
      <!-- 任务概览 -->
      <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
        <div class="glass-card">
          <p class="text-sm text-gray-500 mb-1">正在运行</p>
          <p class="text-2xl font-bold text-primary">0</p>
        </div>
        <div class="glass-card">
          <p class="text-sm text-gray-500 mb-1">队列中</p>
          <p class="text-2xl font-bold text-accent">0</p>
        </div>
        <div class="glass-card">
          <p class="text-sm text-gray-500 mb-1">已完成</p>
          <p class="text-2xl font-bold text-green-500">0</p>
        </div>
      </div>

      <!-- 任务列表 -->
      <div class="glass-card">
        <div class="flex items-center justify-between mb-6">
          <h2 class="text-xl font-semibold text-gray-900 dark:text-white">所有任务</h2>
          <div class="flex gap-2">
            <button class="glass-button px-4 py-2 text-sm">清空已完成</button>
          </div>
        </div>

        <!-- 任务列表组件 -->
        <div class="space-y-4">
          <div class="text-center py-12 text-gray-500">
            <i class="pi pi-inbox text-4xl mb-4 block"></i>
            <p>暂无任务</p>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
// 这里可以添加任务管理逻辑
</script>

<style scoped>
.glass-card {
  @apply p-6 rounded-2xl border border-white/20 bg-white/10 backdrop-blur-md shadow-lg dark:border-white/10 dark:bg-black/20;
}
.glass-button {
  @apply bg-primary/10 text-primary border border-primary/20 rounded-lg transition-colors hover:bg-primary/20;
}
</style>
