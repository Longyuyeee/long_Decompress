import { appWindow } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/tauri'
import type { UpdateManifest } from '@tauri-apps/api/updater'
import { useTaskStore, type TaskStatus } from '@/stores/task'
import { useUpdateStore } from '@/stores/update'

const TEST_TASK_ID = 'desktop-e2e-lifecycle-task'

export interface DesktopE2EBridge {
  startCancellableTask: (outputPath: string) => Promise<string>
  seedActiveTask: () => Promise<string>
  cancelTask: (taskId: string) => Promise<boolean>
  clearTasks: () => Promise<void>
  reset: () => Promise<void>
  taskStatus: (taskId: string) => TaskStatus | null
  showAvailableUpdate: () => void
  setCloseToTray: (enabled: boolean) => Promise<void>
  hideWindow: (markerPath: string) => Promise<void>
  isWindowVisible: () => Promise<boolean>
  desktopBehaviorState: () => Promise<{ close_to_tray: boolean; has_active_tasks: boolean }>
  requestExitConfirmation: () => Promise<boolean>
}

declare global {
  interface Window {
    __LONG_DECOMPRESS_DESKTOP_E2E__?: DesktopE2EBridge
  }
}

export const installDesktopE2EBridge = () => {
  const taskStore = useTaskStore()
  const updateStore = useUpdateStore()

  const addActiveTask = () => {
    taskStore.removeTask(TEST_TASK_ID)
    taskStore.addTask({
      id: TEST_TASK_ID,
      name: 'Desktop E2E lifecycle task',
      type: 'compression',
      sourceFiles: [],
      outputPath: '',
      format: 'zip',
    })
    taskStore.updateTaskStatus(TEST_TASK_ID, 'compressing')
    return TEST_TASK_ID
  }

  const syncActiveState = () =>
    invoke('set_has_active_tasks', { active: taskStore.activeTaskCount > 0 })

  const bridge: DesktopE2EBridge = {
    async startCancellableTask(outputPath) {
      const taskId = addActiveTask()
      const task = taskStore.tasks.find(item => item.id === taskId)
      if (task) task.outputPath = outputPath
      await syncActiveState()
      void invoke('desktop_e2e_run_cancellable_task', { taskId, outputPath })
        .then(() => {
          if (taskStore.tasks.find(item => item.id === taskId)?.status !== 'cancelled') {
            taskStore.updateTaskStatus(taskId, 'completed')
          }
        })
        .catch(error => {
          const current = taskStore.tasks.find(item => item.id === taskId)
          if (current && current.status !== 'cancelled') {
            current.error = String(error)
            taskStore.updateTaskStatus(taskId, 'failed')
          }
        })
      return taskId
    },

    async seedActiveTask() {
      const taskId = addActiveTask()
      await syncActiveState()
      return taskId
    },

    async cancelTask(taskId) {
      return taskStore.cancelTask(taskId)
    },

    async clearTasks() {
      taskStore.tasks.splice(0)
      await syncActiveState()
    },

    async reset() {
      taskStore.tasks.splice(0)
      await syncActiveState()
      updateStore.$patch({
        status: 'idle',
        manifest: null,
        errorMessage: '',
        dialogVisible: false,
      })
    },

    taskStatus(taskId) {
      return taskStore.tasks.find(item => item.id === taskId)?.status ?? null
    },

    showAvailableUpdate() {
      updateStore.$patch({
        status: 'available',
        manifest: {
          version: '99.0.0-e2e',
          date: '2026-07-27',
          body: 'Desktop lifecycle update blocking fixture.',
        } as UpdateManifest,
        errorMessage: '',
        dialogVisible: true,
      })
    },

    async setCloseToTray(enabled) {
      await invoke('set_close_to_tray', { enabled })
    },

    hideWindow: (markerPath) => invoke('desktop_e2e_hide_window', { markerPath }),
    isWindowVisible: () => appWindow.isVisible(),
    desktopBehaviorState: () => invoke('desktop_e2e_get_behavior_state'),
    requestExitConfirmation: () => invoke('desktop_e2e_request_exit_confirmation'),
  }

  window.__LONG_DECOMPRESS_DESKTOP_E2E__ = bridge
  return () => {
    delete window.__LONG_DECOMPRESS_DESKTOP_E2E__
  }
}
