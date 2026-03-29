import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/tauri'

export type TaskStatus = 'pending' | 'preparing' | 'running' | 'compressing' | 'extracting' | 'finalizing' | 'completed' | 'failed' | 'cancelled'
export type LogSeverity = 'info' | 'warning' | 'error' | 'success'

export interface TaskLog {
  task_id: string
  timestamp: string
  message: string
  severity: LogSeverity
}

export interface ConflictInfo {
  taskId: string
  fileName: string
  sourcePath: string
  destPath: string
  sourceSize: number
  destSize: number
  sourceModified: number
  destModified: number
}

export interface Task {
  id: string
  name: string
  type: 'compression' | 'decompression'
  status: TaskStatus
  progress: number
  startTime?: Date
  endTime?: Date
  error?: string
  logs: TaskLog[]
  sourceFiles: string[]
  outputPath: string
  format?: string
  conflicts: ConflictInfo[]
  extractToSubfolder?: boolean
  // 增强字段 [FE-INT-001]
  stage?: 'Pre-checking' | 'Extracting' | 'Finalizing'
  currentFile?: string
  currentPassword?: string
  speed?: string
}

export const useTaskStore = defineStore('task', () => {
  const tasks = ref<Task[]>([])
  const activeTaskCount = computed(() => tasks.value.filter(t => !['completed', 'failed', 'cancelled'].includes(t.status)).length)

  // 初始化监听器
  const initListeners = async () => {
    await listen<TaskLog>('task-log', (event) => {
      const { task_id, message, severity, timestamp } = event.payload
      const task = tasks.value.find(t => t.id === task_id)
      if (task) {
        task.logs.push({
          task_id,
          message,
          severity: severity.toLowerCase() as LogSeverity,
          timestamp
        })
      }
    })

    await listen<{ 
      task_id: string, 
      progress: number,
      stage?: string,
      current_file?: string,
      current_password?: string,
      speed?: string
    }>('task-progress', (event) => {
      const { task_id, progress, stage, current_file, current_password, speed } = event.payload
      const task = tasks.value.find(t => t.id === task_id)
      if (task) {
        task.progress = Math.round(progress * 100)
        task.stage = stage as any
        task.currentFile = current_file
        task.currentPassword = current_password
        task.speed = speed

        if (progress >= 1.0) {
          task.status = 'completed'
          task.endTime = new Date()
        }
      }
    })

    // 监听冲突事件
    await listen<ConflictInfo>('file-conflict', (event) => {
      const conflict = event.payload
      const task = tasks.value.find(t => t.id === conflict.taskId)
      if (task) {
        task.conflicts.push(conflict)
      }
    })
  }

  const addTask = (task: Omit<Task, 'logs' | 'progress' | 'status' | 'conflicts'>) => {
    const newTask: Task = {
      ...task,
      status: 'pending',
      progress: 0,
      logs: [],
      conflicts: []
    }
    tasks.value.push(newTask)
    return newTask.id
  }

  const updateTaskStatus = (taskId: string, status: TaskStatus) => {
    const task = tasks.value.find(t => t.id === taskId)
    if (task) {
      task.status = status
      if (status === 'running' && !task.startTime) {
        task.startTime = new Date()
      }
      if (['completed', 'failed', 'cancelled'].includes(status)) {
        task.endTime = new Date()
      }
    }
  }

  const removeTask = (taskId: string) => {
    const index = tasks.value.findIndex(t => t.id === taskId)
    if (index !== -1) {
      tasks.value.splice(index, 1)
    }
  }

  const clearFinishedTasks = () => {
    tasks.value = tasks.value.filter(t => !['completed', 'failed', 'cancelled'].includes(t.status))
  }

  const cancelTask = async (taskId: string) => {
    try {
      await invoke('cancel_compression', { taskId })
      updateTaskStatus(taskId, 'cancelled')
    } catch (e) {
      console.error('Failed to cancel task:', e)
    }
  }

  const fetchTasks = async () => {
    // 这是一个占位符，如果后端支持获取历史任务，可以在此实现
    console.log('Fetching tasks...')
  }

  return {
    tasks,
    activeTaskCount,
    initListeners,
    addTask,
    updateTaskStatus,
    removeTask,
    clearFinishedTasks,
    cancelTask,
    fetchTasks
  }
})
