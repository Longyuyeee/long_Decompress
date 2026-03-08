<template>
  <div class="max-w-7xl mx-auto">
    <!-- 页面标题 -->
    <div class="mb-6">
      <h1 class="text-2xl font-bold text-gray-900 dark:text-white">任务管理</h1>
      <p class="text-gray-600 dark:text-gray-400">查看和管理解压任务</p>
    </div>

    <!-- 任务列表组件 -->
    <TaskList />

    <!-- 使用说明 -->
    <div class="mt-8 glass-card">
      <h3 class="font-semibold text-gray-900 dark:text-white mb-4">使用说明</h3>
      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div class="space-y-3">
          <div class="flex items-start space-x-3">
            <div class="w-8 h-8 rounded-full bg-primary/10 flex items-center justify-center flex-shrink-0">
              <i class="pi pi-play text-primary"></i>
            </div>
            <div>
              <h4 class="font-medium text-gray-900 dark:text-white">任务状态</h4>
              <p class="text-gray-600 dark:text-gray-400 text-sm mt-1">
                任务有四种状态：等待中、进行中、已完成、失败。不同状态有对应的颜色标识。
              </p>
            </div>
          </div>

          <div class="flex items-start space-x-3">
            <div class="w-8 h-8 rounded-full bg-green-500/10 flex items-center justify-center flex-shrink-0">
              <i class="pi pi-filter text-green-500"></i>
            </div>
            <div>
              <h4 class="font-medium text-gray-900 dark:text-white">筛选和排序</h4>
              <p class="text-gray-600 dark:text-gray-400 text-sm mt-1">
                使用筛选功能查看特定状态的任务，使用排序按钮切换时间顺序。
              </p>
            </div>
          </div>
        </div>

        <div class="space-y-3">
          <div class="flex items-start space-x-3">
            <div class="w-8 h-8 rounded-full bg-yellow-500/10 flex items-center justify-center flex-shrink-0">
              <i class="pi pi-cog text-yellow-500"></i>
            </div>
            <div>
              <h4 class="font-medium text-gray-900 dark:text-white">批量操作</h4>
              <p class="text-gray-600 dark:text-gray-400 text-sm mt-1">
                选择多个任务可以进行批量暂停、取消或删除操作。
              </p>
            </div>
          </div>

          <div class="flex items-start space-x-3">
            <div class="w-8 h-8 rounded-full bg-purple-500/10 flex items-center justify-center flex-shrink-0">
              <i class="pi pi-chart-bar text-purple-500"></i>
            </div>
            <div>
              <h4 class="font-medium text-gray-900 dark:text-white">统计信息</h4>
              <p class="text-gray-600 dark:text-gray-400 text-sm mt-1">
                查看任务统计信息，了解整体进度和状态分布。
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 常见问题 -->
    <div class="mt-6 glass-card">
      <h3 class="font-semibold text-gray-900 dark:text-white mb-4">常见问题</h3>
      <div class="space-y-4">
        <div>
          <h4 class="font-medium text-gray-900 dark:text-white mb-2">任务长时间没有进度怎么办？</h4>
          <p class="text-gray-600 dark:text-gray-400 text-sm">
            可以尝试取消任务后重新开始，或者检查文件是否损坏、磁盘空间是否充足。
          </p>
        </div>

        <div>
          <h4 class="font-medium text-gray-900 dark:text-white mb-2">如何查看失败任务的详细信息？</h4>
          <p class="text-gray-600 dark:text-gray-400 text-sm">
            点击失败任务卡片上的错误信息区域，可以查看详细的错误原因和解决方案。
          </p>
        </div>

        <div>
          <h4 class="font-medium text-gray-900 dark:text-white mb-2">任务列表会自动刷新吗？</h4>
          <p class="text-gray-600 dark:text-gray-400 text-sm">
            是的，进行中的任务会自动更新进度。你也可以手动点击刷新按钮获取最新状态。
          </p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import TaskList from '@/components/tasks/TaskList.vue'
</script>

<style scoped>
/* 自定义样式 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>