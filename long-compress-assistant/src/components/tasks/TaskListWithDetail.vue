<template>
  <div class="task-list-with-detail">
    <!-- 任务列表 -->
    <TaskList
      ref="taskListRef"
      @task-click="handleTaskClick"
    />

    <!-- 任务详情模态框 -->
    <Modal
      v-model:visible="detailModalVisible"
      :title="selectedTask ? `任务详情 - ${selectedTask.fileName}` : '任务详情'"
      icon="pi pi-info-circle"
      size="xl"
      :show-footer="false"
      @close="handleModalClose"
    >
      <TaskDetailPanel
        v-if="selectedTask"
        :task="selectedTask"
        :is-processing="isProcessing"
        @close="handleModalClose"
        @retry="handleTaskRetry"
        @cancel="handleTaskCancel"
        @open-output="handleOpenOutput"
        @copy-path="handleCopyPath"
        @show-in-explorer="handleShowInExplorer"
        @export-log="handleExportLog"
        @delete="handleTaskDelete"
      />

      <div v-else class="text-center py-12">
        <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-gray-100 dark:bg-gray-800 flex items-center justify-center">
          <i class="pi pi-exclamation-circle text-gray-400 text-2xl"></i>
        </div>
        <h3 class="font-medium text-gray-900 dark:text-white mb-2">任务信息加载失败</h3>
        <p class="text-gray-600 dark:text-gray-400 text-sm">
          无法加载任务详情，请刷新页面后重?
        </p>
      </div>
    </Modal>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { Modal } from '@/components/ui'
import TaskList from './TaskList.vue'
import TaskDetailPanel from './TaskDetailPanel.vue'
import type { DecompressTask } from '@/stores'
import { useAppStore, useUIStore } from '@/stores'

const appStore = useAppStore()
const uiStore = useUIStore()

// 状?
const detailModalVisible = ref(false)
const selectedTask = ref<DecompressTask | null>(null)
const isProcessing = ref(false)
const taskListRef = ref<InstanceType<typeof TaskList>>()

// 事件处理
const handleTaskClick = (taskId: string) => {
  // 从store中获取任务详情
  const task = appStore.decompressTasks.find(t => t.id === taskId)
  if (task) {
    selectedTask.value = task
    detailModalVisible.value = true
  } else {
    uiStore.showError('未找到任务信息')
  }
}

const handleModalClose = () => {
  detailModalVisible.value = false
  selectedTask.value = null
}

const handleTaskRetry = async (taskId: string) => {
  isProcessing.value = true
  try {
    // 这里应该调用重试任务的API
    uiStore.showInfo('重试功能开发中')
    // 模拟API调用
    await new Promise(resolve => setTimeout(resolve, 1000))
    uiStore.showSuccess('任务重试请求已发送')
  } catch (error) {
    uiStore.showError('重试失败')
  } finally {
    isProcessing.value = false
  }
}

const handleTaskCancel = async (taskId: string) => {
  isProcessing.value = true
  try {
    // 这里应该调用取消任务的API
    appStore.markTaskAsError(taskId, '任务已取消')
    uiStore.showWarning('任务已取消')
    // 刷新任务列表
    if (taskListRef.value) {
      // 调用TaskList的刷新方法
      // taskListRef.value.refreshTasks()
    }
  } catch (error) {
    uiStore.showError('取消失败')
  } finally {
    isProcessing.value = false
    handleModalClose()
  }
}

const handleOpenOutput = (path: string) => {
  // 这里应该实现打开目录的逻辑
  uiStore.showInfo(`打开目录: ${path}`)
}

const handleCopyPath = (path: string) => {
  // 复制路径到剪贴板
  navigator.clipboard.writeText(path)
    .then(() => uiStore.showSuccess('路径已复制到剪贴板'))
    .catch(() => uiStore.showError('复制失败'))
}

const handleShowInExplorer = (path: string) => {
  // 这里应该实现在资源管理器中显示文件的逻辑
  uiStore.showInfo(`在资源管理器中显示: ${path}`)
}

const handleExportLog = async (taskId: string) => {
  isProcessing.value = true
  try {
    // 这里应该实现导出日志的逻辑
    uiStore.showInfo('导出日志功能开发中')
    await new Promise(resolve => setTimeout(resolve, 1000))
    uiStore.showSuccess('日志导出成功')
  } catch (error) {
    uiStore.showError('导出失败')
  } finally {
    isProcessing.value = false
  }
}

const handleTaskDelete = async (taskId: string) => {
  if (!confirm('确定要删除此任务记录吗？此操作不可撤销')) {
    return
  }

  isProcessing.value = true
  try {
    // 从store中删除任务
    const index = appStore.decompressTasks.findIndex(t => t.id === taskId)
    if (index !== -1) {
      appStore.decompressTasks.splice(index, 1)
      uiStore.showSuccess('任务记录已删除')
      handleModalClose()
    }
  } catch (error) {
    uiStore.showError('删除失败')
  } finally {
    isProcessing.value = false
  }
}

// 暴露方法给父组件
defineExpose({
  openTaskDetail: (taskId: string) => handleTaskClick(taskId),
  closeTaskDetail: () => handleModalClose()
})
</script>

<style scoped>
.task-list-with-detail {
  @apply relative;
}

/* 动画效果 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
